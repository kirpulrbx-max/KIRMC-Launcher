//! Cada instancia tiene su propio .minecraft (mods, configs, mundos) pero comparte
//! librerías y assets con las demás.

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    minecraft::loaders::Loader,
    paths,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub loader: Loader,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub min_ram_mb: Option<u32>,
    #[serde(default)]
    pub max_ram_mb: Option<u32>,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub jvm_args: Vec<String>,
    #[serde(default)]
    pub last_played: Option<u64>,
    pub created: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    pub file: String,
    pub enabled: bool,
    pub size: u64,
}

const SUBDIRS: [&str; 5] = ["mods", "resourcepacks", "shaderpacks", "config", "saves"];

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Evita path traversal: los ids son UUID
fn check_id(id: &str) -> Result<()> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Ok(())
    } else {
        Err(Error::Msg("Id de instancia inválido".into()))
    }
}

fn check_file_name(name: &str) -> Result<()> {
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return Err(Error::Msg("Nombre de archivo inválido".into()));
    }
    Ok(())
}

pub fn dir(id: &str) -> PathBuf {
    paths::instances().join(id)
}

pub fn game_dir(id: &str) -> PathBuf {
    dir(id).join(".minecraft")
}

pub fn load(id: &str) -> Result<Instance> {
    check_id(id)?;
    Ok(serde_json::from_slice(&fs::read(dir(id).join("instance.json"))?)?)
}

pub fn save(inst: &Instance) -> Result<()> {
    check_id(&inst.id)?;
    paths::ensure(&dir(&inst.id))?;
    fs::write(dir(&inst.id).join("instance.json"), serde_json::to_vec_pretty(inst)?)?;
    Ok(())
}

#[tauri::command]
pub fn list_instances() -> Vec<Instance> {
    let Ok(rd) = fs::read_dir(paths::instances()) else { return vec![] };
    let mut list: Vec<Instance> = rd.flatten().filter_map(|e| load(&e.file_name().to_string_lossy()).ok()).collect();
    list.sort_by_key(|i| std::cmp::Reverse(i.last_played.unwrap_or(i.created)));
    list
}

#[tauri::command]
pub fn create_instance(name: String, mc_version: String, loader: Loader, loader_version: Option<String>) -> Result<Instance> {
    create(name, mc_version, loader, loader_version)
}

pub fn create(name: String, mc_version: String, loader: Loader, loader_version: Option<String>) -> Result<Instance> {
    let inst = Instance {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        mc_version,
        loader,
        loader_version,
        version_id: None,
        min_ram_mb: None,
        max_ram_mb: None,
        java_path: None,
        jvm_args: vec![],
        last_played: None,
        created: now(),
    };
    for sub in SUBDIRS {
        fs::create_dir_all(game_dir(&inst.id).join(sub))?;
    }
    save(&inst)?;
    Ok(inst)
}

#[tauri::command]
pub fn update_instance(instance: Instance) -> Result<Instance> {
    load(&instance.id)?; // debe existir
    save(&instance)?;
    Ok(instance)
}

#[tauri::command]
pub fn delete_instance(id: String) -> Result<()> {
    check_id(&id)?;
    fs::remove_dir_all(dir(&id))?;
    Ok(())
}

/// Ruta para abrir en el explorador (raíz del .minecraft o una subcarpeta conocida)
#[tauri::command]
pub fn instance_dir(id: String, sub: Option<String>) -> Result<String> {
    check_id(&id)?;
    let mut p = game_dir(&id);
    if let Some(sub) = sub {
        if !SUBDIRS.contains(&sub.as_str()) {
            return Err(Error::Msg("Carpeta desconocida".into()));
        }
        p = p.join(sub);
    }
    paths::ensure(&p)?;
    Ok(p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn list_mods(id: String) -> Result<Vec<ModFile>> {
    check_id(&id)?;
    let Ok(rd) = fs::read_dir(game_dir(&id).join("mods")) else { return Ok(vec![]) };
    let mut mods: Vec<ModFile> = rd
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            let enabled = file.ends_with(".jar");
            (enabled || file.ends_with(".jar.disabled")).then(|| ModFile {
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
                file,
                enabled,
            })
        })
        .collect();
    mods.sort_by(|a, b| a.file.to_lowercase().cmp(&b.file.to_lowercase()));
    Ok(mods)
}

/// Activa/desactiva un mod renombrando .jar <-> .jar.disabled
#[tauri::command]
pub fn toggle_mod(id: String, file: String) -> Result<()> {
    check_id(&id)?;
    check_file_name(&file)?;
    let mods = game_dir(&id).join("mods");
    let target = match file.strip_suffix(".disabled") {
        Some(enabled) => enabled.to_string(),
        None => format!("{file}.disabled"),
    };
    fs::rename(mods.join(&file), mods.join(target))?;
    Ok(())
}
