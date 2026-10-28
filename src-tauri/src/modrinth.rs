//! Integración con la API v2 de Modrinth: búsqueda, instalación de mods/resource packs/shaders
//! (con dependencias requeridas) e instalación de modpacks `.mrpack`.
//! Docs: https://docs.modrinth.com/api/

use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::{
    config,
    error::{Error, Result},
    instances::{self, Instance},
    minecraft::{
        download::{self, Job},
        loaders::Loader,
    },
    notify,
    state::AppState,
};

const API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Hit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub project_type: String,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SearchResult {
    pub hits: Vec<Hit>,
    pub offset: u32,
    pub total_hits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hashes {
    pub sha1: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    pub hashes: Hashes,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct MrDep {
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct MrVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    pub files: Vec<MrFile>,
    #[serde(default)]
    pub dependencies: Vec<MrDep>,
    #[serde(default)]
    pub date_published: String,
}

/// Quilt carga mods de Fabric, así que también los aceptamos
fn loader_names(l: Loader) -> Vec<&'static str> {
    match l {
        Loader::Fabric => vec!["fabric"],
        Loader::Quilt => vec!["quilt", "fabric"],
        Loader::Forge => vec!["forge"],
        Loader::Vanilla => vec![],
    }
}

fn dest_dir(project_type: &str) -> &'static str {
    match project_type {
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        _ => "mods",
    }
}

/// Rutas relativas seguras (sin `..` ni absolutas) dentro de la instancia
fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let p = Path::new(rel);
    p.components().all(|c| matches!(c, Component::Normal(_))).then(|| base.join(p))
}

fn primary_file(v: &MrVersion) -> Option<&MrFile> {
    v.files.iter().find(|f| f.primary).or_else(|| v.files.first())
}

async fn get_version(http: &Client, id: &str) -> Result<MrVersion> {
    Ok(http.get(format!("{API}/version/{id}")).send().await?.error_for_status()?.json().await?)
}

async fn project_versions(http: &Client, project: &str, game_version: Option<&str>, loaders: &[&str]) -> Result<Vec<MrVersion>> {
    let mut req = http.get(format!("{API}/project/{project}/version"));
    if let Some(v) = game_version {
        req = req.query(&[("game_versions", serde_json::to_string(&[v])?)]);
    }
    if !loaders.is_empty() {
        req = req.query(&[("loaders", serde_json::to_string(loaders)?)]);
    }
    Ok(req.send().await?.error_for_status()?.json().await?)
}

#[tauri::command]
pub async fn modrinth_search(
    state: State<'_, AppState>,
    query: String,
    project_type: String,
    game_version: Option<String>,
    loader: Option<Loader>,
    offset: u32,
) -> Result<SearchResult> {
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{project_type}")]];
    if let Some(v) = game_version.filter(|v| !v.is_empty()) {
        facets.push(vec![format!("versions:{v}")]);
    }
    if project_type == "mod" {
        let names = loader.map(loader_names).unwrap_or_default();
        if !names.is_empty() {
            facets.push(names.iter().map(|n| format!("categories:{n}")).collect());
        }
    }
    let facets = serde_json::to_string(&facets)?;
    let index = if query.trim().is_empty() { "downloads" } else { "relevance" };
    let offset = offset.to_string();
    Ok(state
        .http
        .get(format!("{API}/search"))
        .query(&[
            ("query", query.as_str()),
            ("facets", facets.as_str()),
            ("index", index),
            ("limit", "24"),
            ("offset", offset.as_str()),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

#[tauri::command]
pub async fn modrinth_versions(
    state: State<'_, AppState>,
    project_id: String,
    game_version: Option<String>,
    loader: Option<Loader>,
) -> Result<Vec<MrVersion>> {
    let loaders = loader.map(loader_names).unwrap_or_default();
    project_versions(&state.http, &project_id, game_version.as_deref(), &loaders).await
}

/// Instala un proyecto en una instancia, resolviendo dependencias requeridas.
/// Devuelve los nombres de archivo instalados.
#[tauri::command]
pub async fn modrinth_install(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
    project_type: String,
    version_id: Option<String>,
) -> Result<Vec<String>> {
    let http = &state.http;
    let inst = instances::load(&instance_id)?;
    let cfg = config::load();
    let is_mod = project_type == "mod";
    let loaders = if is_mod { loader_names(inst.loader) } else { vec![] };
    if is_mod && loaders.is_empty() {
        return Err(Error::Msg("Las instancias vanilla no admiten mods: crea una con Fabric, Quilt o Forge".into()));
    }
    let dir = instances::game_dir(&inst.id).join(dest_dir(&project_type));

    let mut queue: Vec<(Option<String>, Option<String>)> = vec![(Some(project_id), version_id)];
    let mut seen = HashSet::new();
    let (mut jobs, mut names) = (Vec::new(), Vec::new());

    while let Some((pid, vid)) = queue.pop() {
        let version = match (&pid, vid) {
            (_, Some(v)) => get_version(http, &v).await?,
            (Some(p), None) => project_versions(http, p, Some(&inst.mc_version), &loaders)
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| Error::Msg(format!("No hay versión de «{p}» para Minecraft {}", inst.mc_version)))?,
            (None, None) => continue,
        };
        if !seen.insert(version.project_id.clone()) {
            continue;
        }
        let Some(file) = primary_file(&version) else { continue };
        let Some(path) = safe_join(&dir, &file.filename) else { continue };
        jobs.push(Job { url: file.url.clone(), path, sha1: Some(file.hashes.sha1.clone()), size: Some(file.size) });
        names.push(file.filename.clone());

        for dep in version.dependencies.iter().filter(|d| d.dependency_type == "required") {
            if dep.project_id.as_ref().is_some_and(|p| seen.contains(p)) {
                continue;
            }
            queue.push((dep.project_id.clone(), dep.version_id.clone()));
        }
    }

    download::run(&app, http, &inst.id, "mods", jobs, cfg.concurrent_downloads, true).await?;
    notify::send(&app, notify::Note::ModsInstalled(names.len()));
    Ok(names)
}

#[derive(Deserialize)]
struct MrIndex {
    name: String,
    dependencies: HashMap<String, String>,
    files: Vec<IndexFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexFile {
    path: String,
    hashes: Hashes,
    #[serde(default)]
    env: Option<HashMap<String, String>>,
    downloads: Vec<String>,
    #[serde(default)]
    file_size: Option<u64>,
}

/// Lee el .mrpack, crea la instancia y copia los overrides. Síncrono (sin awaits).
fn unpack_mrpack(bytes: Vec<u8>) -> Result<(Instance, Vec<Job>)> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
    let index: MrIndex = {
        let mut s = String::new();
        zip.by_name("modrinth.index.json")?.read_to_string(&mut s)?;
        serde_json::from_str(&s)?
    };
    let deps = &index.dependencies;
    let mc = deps.get("minecraft").cloned().ok_or_else(|| Error::Msg("El modpack no indica la versión de Minecraft".into()))?;
    let (loader, lv) = if let Some(v) = deps.get("fabric-loader") {
        (Loader::Fabric, Some(v.clone()))
    } else if let Some(v) = deps.get("quilt-loader") {
        (Loader::Quilt, Some(v.clone()))
    } else if let Some(v) = deps.get("forge") {
        (Loader::Forge, Some(v.clone()))
    } else if deps.contains_key("neoforge") {
        return Err(Error::Msg("Los modpacks de NeoForge aún no están soportados".into()));
    } else {
        (Loader::Vanilla, None)
    };

    let inst = instances::create(index.name.clone(), mc, loader, lv)?;
    let game = instances::game_dir(&inst.id);

    // overrides/ y client-overrides/ se copian tal cual sobre el .minecraft
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name() else { continue };
        let Ok(rel) = name.strip_prefix("overrides").or_else(|_| name.strip_prefix("client-overrides")) else { continue };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = game.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
    }

    let jobs = index
        .files
        .iter()
        .filter(|f| f.env.as_ref().and_then(|e| e.get("client")).map_or(true, |c| c != "unsupported"))
        .filter_map(|f| {
            Some(Job {
                url: f.downloads.first()?.clone(),
                path: safe_join(&game, &f.path)?,
                sha1: Some(f.hashes.sha1.clone()),
                size: f.file_size,
            })
        })
        .collect();
    Ok((inst, jobs))
}

#[tauri::command]
pub async fn modrinth_install_modpack(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    version_id: Option<String>,
) -> Result<Instance> {
    let http = &state.http;
    let cfg = config::load();
    let version = match version_id {
        Some(v) => get_version(http, &v).await?,
        None => project_versions(http, &project_id, None, &[])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::Msg("El modpack no tiene versiones publicadas".into()))?,
    };
    let file = primary_file(&version).ok_or_else(|| Error::Msg("La versión no tiene archivo .mrpack".into()))?;
    let bytes = http.get(&file.url).send().await?.error_for_status()?.bytes().await?.to_vec();

    let (inst, jobs) = tauri::async_runtime::spawn_blocking(move || unpack_mrpack(bytes))
        .await
        .map_err(|e| Error::Msg(e.to_string()))??;
    download::run(&app, http, &inst.id, "modpack", jobs, cfg.concurrent_downloads, true).await?;
    notify::send(&app, notify::Note::InstallDone(&inst.name));
    Ok(inst)
}
