use serde::{Deserialize, Serialize};

use crate::{error::Result, paths};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherConfig {
    /// "es" | "en" | "ja"
    pub language: String,
    pub java_path: Option<String>,
    pub min_ram_mb: u32,
    pub max_ram_mb: u32,
    /// Qué hace el launcher al arrancar el juego: "keep" | "hide" | "close"
    pub after_launch: String,
    pub allow_multiple_instances: bool,
    pub notifications: bool,
    pub animations: bool,
    pub game_width: u32,
    pub game_height: u32,
    pub fullscreen: bool,
    pub concurrent_downloads: usize,
    /// Verificar SHA1 de todos los archivos antes de lanzar (más lento, más seguro)
    pub verify_files: bool,
    pub active_account: Option<String>,
    /// Servidor Yggdrasil para skins de cuentas no-premium (authlib-injector)
    pub skin_server: Option<String>,
    pub favorite_servers: Vec<ServerEntry>,
    /// Client ID de la app de Azure para el login de Microsoft
    pub ms_client_id: Option<String>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            language: "es".into(),
            java_path: None,
            min_ram_mb: 1024,
            max_ram_mb: 4096,
            after_launch: "hide".into(),
            allow_multiple_instances: true,
            notifications: true,
            animations: true,
            game_width: 1280,
            game_height: 720,
            fullscreen: false,
            concurrent_downloads: 24,
            verify_files: false,
            active_account: None,
            skin_server: None,
            favorite_servers: vec![ServerEntry { name: "Hypixel".into(), address: "mc.hypixel.net".into() }],
            ms_client_id: None,
        }
    }
}

pub fn load() -> LauncherConfig {
    std::fs::read(paths::config_file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn store(cfg: &LauncherConfig) -> Result<()> {
    paths::ensure(&paths::root())?;
    std::fs::write(paths::config_file(), serde_json::to_vec_pretty(cfg)?)?;
    Ok(())
}

#[tauri::command]
pub fn get_config() -> LauncherConfig {
    load()
}

#[tauri::command]
pub fn save_config(cfg: LauncherConfig) -> Result<()> {
    store(&cfg)
}

#[tauri::command]
pub fn data_dir() -> Result<String> {
    paths::ensure(&paths::root())?;
    Ok(paths::root().to_string_lossy().into_owned())
}
