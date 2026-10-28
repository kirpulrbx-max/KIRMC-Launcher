//! Java: detección de instalaciones y descarga automática (Eclipse Temurin vía API de Adoptium)
//! de la versión exacta que pide cada versión de Minecraft (`javaVersion.majorVersion`).

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use tauri::AppHandle;

use crate::{
    error::{Error, Result},
    minecraft::download::{self, Job},
    paths,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstall {
    pub path: String,
    pub version: String,
    pub major: u32,
}

fn parse_major(v: &str) -> Option<u32> {
    let mut parts = v.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok() // "1.8.0_382" -> 8
    } else {
        Some(first)
    }
}

/// En Windows lanzamos con javaw.exe (sin ventana de consola)
fn launch_binary(java: &Path) -> String {
    let javaw = java.with_file_name("javaw.exe");
    if cfg!(windows) && javaw.exists() {
        javaw.to_string_lossy().into_owned()
    } else {
        java.to_string_lossy().into_owned()
    }
}

fn probe(java: &Path) -> Option<JavaInstall> {
    let mut cmd = std::process::Command::new(java);
    cmd.arg("-version");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    let version = text.split('"').nth(1)?.to_string();
    Some(JavaInstall { path: launch_binary(java), major: parse_major(&version)?, version })
}

fn candidates() -> Vec<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let mut out = Vec::new();
    if let Ok(home) = std::env::var("JAVA_HOME") {
        out.push(PathBuf::from(home).join("bin").join(exe));
    }
    if let Some(path) = std::env::var_os("PATH") {
        out.extend(std::env::split_paths(&path).map(|d| d.join(exe)));
    }
    let mut roots: Vec<PathBuf> = if cfg!(windows) {
        ["Java", "Eclipse Adoptium", "Microsoft", "Zulu", "BellSoft", "Amazon Corretto"]
            .iter()
            .map(|v| PathBuf::from(r"C:\Program Files").join(v))
            .collect()
    } else if cfg!(target_os = "macos") {
        vec!["/Library/Java/JavaVirtualMachines".into()]
    } else {
        vec!["/usr/lib/jvm".into()]
    };
    roots.push(paths::root().join("java")); // runtimes descargados por KIRMC
    for root in roots {
        let Ok(rd) = fs::read_dir(root) else { continue };
        for e in rd.flatten() {
            out.push(e.path().join("bin").join(exe));
            out.push(e.path().join("Contents/Home/bin").join(exe));
        }
    }
    out
}

pub fn scan() -> Vec<JavaInstall> {
    let mut seen = HashSet::new();
    let mut found: Vec<JavaInstall> = candidates()
        .into_iter()
        .filter(|p| p.is_file())
        .filter(|p| seen.insert(fs::canonicalize(p).unwrap_or_else(|_| p.clone())))
        .filter_map(|p| probe(&p))
        .collect();
    found.sort_by_key(|j| std::cmp::Reverse(j.major));
    found
}

/// Java instalado compatible: Java 8 exacto para versiones antiguas (LWJGL 2 / Forge viejo no
/// funcionan con Java moderno); para el resto, la versión pedida o la más cercana por encima.
fn find_installed(req: u32) -> Option<String> {
    let all = scan();
    if req <= 8 {
        return all.iter().find(|j| j.major == 8).map(|j| j.path.clone());
    }
    all.iter()
        .find(|j| j.major == req)
        .or_else(|| all.iter().filter(|j| j.major >= req).min_by_key(|j| j.major))
        .map(|j| j.path.clone())
}

/// Copia el zip quitando la carpeta raíz (jdk-25.0.5+8-jre/…) para que quede en `dest/bin/…`
fn extract_strip_root(zip_path: &Path, dest: &Path) -> Result<()> {
    let _ = fs::remove_dir_all(dest);
    let mut zip = zip::ZipArchive::new(fs::File::open(zip_path)?)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let Some(name) = entry.enclosed_name() else { continue };
        let rel: PathBuf = name.components().skip(1).collect();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        std::io::copy(&mut entry, &mut fs::File::create(&out)?)?;
    }
    let _ = fs::remove_file(zip_path);
    Ok(())
}

/// Devuelve un Java válido para `required`, descargándolo de Adoptium si no hay ninguno.
/// Emite progreso con stage "java" bajo el `tag` de la instancia.
pub async fn ensure(app: &AppHandle, http: &Client, tag: &str, required: u32) -> Result<String> {
    // No existe JRE 16 de Temurin; Java 17 ejecuta igual las versiones que pedían 16
    let req = if required == 16 { 17 } else { required.max(8) };
    if let Ok(Some(path)) = tauri::async_runtime::spawn_blocking(move || find_installed(req)).await {
        return Ok(path);
    }
    if !cfg!(windows) {
        return Err(Error::Msg(format!("Esta versión necesita Java {req}. Instálalo y vuelve a intentarlo.")));
    }

    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x64" };
    let info: Value = http
        .get(format!("https://api.adoptium.net/v3/assets/latest/{req}/hotspot"))
        .query(&[("architecture", arch), ("image_type", "jre"), ("os", "windows"), ("vendor", "eclipse")])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let package = &info[0]["binary"]["package"];
    let link = package["link"]
        .as_str()
        .ok_or_else(|| Error::Msg(format!("No se encontró Java {req} para descargar")))?;

    let zip_path = paths::root().join("tmp").join(format!("temurin-{req}.zip"));
    let job = Job { url: link.to_string(), path: zip_path.clone(), sha1: None, size: package["size"].as_u64() };
    download::run(app, http, tag, "java", vec![job], 1, false).await?;

    let dest = paths::root().join("java").join(format!("temurin-{req}"));
    let dest2 = dest.clone();
    tauri::async_runtime::spawn_blocking(move || extract_strip_root(&zip_path, &dest2))
        .await
        .map_err(|e| Error::Msg(e.to_string()))??;

    let javaw = dest.join("bin").join("javaw.exe");
    if !javaw.exists() {
        return Err(Error::Msg(format!("La descarga de Java {req} no contiene javaw.exe")));
    }
    Ok(javaw.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn detect_java() -> Vec<JavaInstall> {
    tauri::async_runtime::spawn_blocking(scan).await.unwrap_or_default()
}
