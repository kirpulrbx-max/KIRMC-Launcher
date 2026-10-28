//! Skins y capas con descarga directa:
//! - MineSkin (api.mineskin.org/v2/skins): galería pública de skins, sin API key.
//! - capes.dev: capas de un jugador en Minecraft, OptiFine, LabyMod, MinecraftCapes, 5zig, TLauncher y SkinMC.
//! - Ely.by (skinsystem.ely.by): skins/capas por nombre, popular entre jugadores no-premium.
//! - API pública de Mojang para jugadores premium.
//! Además, biblioteca local de skins y capas en %APPDATA%/KIRMC/skins.

use std::{fs, path::PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::{
    error::{Error, Result},
    instances::now,
    paths,
    state::AppState,
};

const ALLOWED_HOSTS: &[&str] = &[
    "textures.minecraft.net",
    "api.capes.dev",
    "skinsystem.ely.by",
    "ely.by",
    "mc-heads.net",
    "crafatar.com",
];
const MAX_PNG: usize = 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GallerySkin {
    pub id: String,
    pub name: Option<String>,
    pub texture_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryPage {
    pub skins: Vec<GallerySkin>,
    pub next: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapeEntry {
    /// minecraft | optifine | labymod | minecraftcapes | 5zig | tlauncher | skinmc
    pub provider: String,
    pub image_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerTextures {
    pub uuid: String,
    pub name: String,
    pub skin_url: Option<String>,
    pub slim: bool,
    pub cape_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSkin {
    pub id: String,
    pub name: String,
    /// "classic" | "slim"
    pub variant: String,
    /// "skin" | "cape"
    #[serde(default = "default_kind")]
    pub kind: String,
    pub added: u64,
}

fn default_kind() -> String {
    "skin".into()
}

fn https(url: &str) -> String {
    url.replacen("http://", "https://", 1)
}

fn check_png(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_PNG || !bytes.starts_with(b"\x89PNG") {
        return Err(Error::Msg("El archivo no es una skin PNG válida".into()));
    }
    Ok(())
}

/// Texturas públicas (skin + capa) de cualquier jugador premium
#[tauri::command]
pub async fn lookup_player(state: State<'_, AppState>, name: String) -> Result<PlayerTextures> {
    let name = name.trim();
    let res = state.http.get(format!("https://api.mojang.com/users/profiles/minecraft/{name}")).send().await?;
    if !res.status().is_success() || res.status() == reqwest::StatusCode::NO_CONTENT {
        return Err(Error::Msg(format!("No existe ningún jugador premium llamado «{name}»")));
    }
    let profile: Value = res.json().await?;
    let id = profile["id"].as_str().ok_or_else(|| Error::Msg("Respuesta inesperada de Mojang".into()))?;

    let session: Value = state
        .http
        .get(format!("https://sessionserver.mojang.com/session/minecraft/profile/{id}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let encoded = session["properties"]
        .as_array()
        .and_then(|props| props.iter().find(|p| p["name"] == "textures"))
        .and_then(|p| p["value"].as_str())
        .ok_or_else(|| Error::Msg("El jugador no tiene texturas".into()))?;
    let decoded = STANDARD.decode(encoded).map_err(|e| Error::Msg(e.to_string()))?;
    let textures: Value = serde_json::from_slice(&decoded)?;
    let skin = &textures["textures"]["SKIN"];

    Ok(PlayerTextures {
        uuid: id.to_string(),
        name: profile["name"].as_str().unwrap_or(name).to_string(),
        skin_url: skin["url"].as_str().map(https),
        slim: skin["metadata"]["model"].as_str() == Some("slim"),
        cape_url: textures["textures"]["CAPE"]["url"].as_str().map(https),
    })
}

/// Solo descargamos texturas de servicios conocidos (evita usar el launcher como proxy arbitrario)
fn normalize_texture_url(input: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(input.trim()).map_err(|_| Error::Msg("Enlace no válido".into()))?;
    if !url.host_str().is_some_and(|h| ALLOWED_HOSTS.contains(&h)) {
        return Err(Error::Msg("Origen de textura no permitido".into()));
    }
    let _ = url.set_scheme("https");
    Ok(url.to_string())
}

/// Galería pública de MineSkin (más recientes primero). `after` = cursor de paginación.
#[tauri::command]
pub async fn mineskin_gallery(state: State<'_, AppState>, after: Option<String>) -> Result<GalleryPage> {
    let mut req = state.http.get("https://api.mineskin.org/v2/skins").query(&[("size", "24")]);
    if let Some(a) = after.as_deref().filter(|a| a.chars().all(|c| c.is_ascii_alphanumeric())) {
        req = req.query(&[("after", a)]);
    }
    let v: Value = req.send().await?.error_for_status()?.json().await?;
    let skins = v["skins"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|s| {
                    let hash = s["texture"].as_str()?;
                    Some(GallerySkin {
                        id: s["uuid"].as_str().unwrap_or(hash).to_string(),
                        name: s["name"].as_str().filter(|n| !n.is_empty()).map(String::from),
                        texture_url: format!("https://textures.minecraft.net/texture/{hash}"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(GalleryPage { skins, next: v["pagination"]["next"]["after"].as_str().map(String::from) })
}

/// Todas las capas de un jugador en los servicios que agrega capes.dev
#[tauri::command]
pub async fn lookup_capes(state: State<'_, AppState>, name: String) -> Result<Vec<CapeEntry>> {
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Error::Msg("Nombre de jugador no válido".into()));
    }
    let v: Value = state.http.get(format!("https://api.capes.dev/load/{name}")).send().await?.error_for_status()?.json().await?;
    Ok(v.as_object()
        .map(|providers| {
            providers
                .iter()
                .filter(|(_, c)| c["exists"].as_bool() == Some(true))
                .filter_map(|(provider, c)| {
                    Some(CapeEntry { provider: provider.clone(), image_url: c["imageUrl"].as_str()?.to_string() })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Skin de un usuario de Ely.by (sistema de skins usado por muchos jugadores no-premium)
#[tauri::command]
pub async fn elyby_skin(state: State<'_, AppState>, name: String) -> Result<Vec<u8>> {
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Error::Msg("Nombre de jugador no válido".into()));
    }
    let res = state.http.get(format!("https://skinsystem.ely.by/skins/{name}.png")).send().await?;
    if !res.status().is_success() {
        return Err(Error::Msg(format!("«{name}» no tiene skin en Ely.by")));
    }
    let bytes = res.bytes().await?;
    check_png(&bytes)?;
    Ok(bytes.to_vec())
}

/// Descarga una textura (skin/capa) y la devuelve como bytes PNG para previsualizar o guardar
#[tauri::command]
pub async fn fetch_texture(state: State<'_, AppState>, url: String) -> Result<Vec<u8>> {
    let url = normalize_texture_url(&url)?;
    let bytes = state.http.get(&url).send().await?.error_for_status()?.bytes().await?;
    check_png(&bytes)?;
    Ok(bytes.to_vec())
}

// ---------- Biblioteca local ----------

fn library_dir() -> PathBuf {
    paths::root().join("skins")
}

fn load_index() -> Vec<SavedSkin> {
    fs::read(library_dir().join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_index(list: &[SavedSkin]) -> Result<()> {
    paths::ensure(&library_dir())?;
    fs::write(library_dir().join("index.json"), serde_json::to_vec_pretty(list)?)?;
    Ok(())
}

fn check_id(id: &str) -> Result<()> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Ok(())
    } else {
        Err(Error::Msg("Id de skin inválido".into()))
    }
}

#[tauri::command]
pub fn list_skins() -> Vec<SavedSkin> {
    let mut list = load_index();
    list.sort_by_key(|s| std::cmp::Reverse(s.added));
    list
}

#[tauri::command]
pub fn save_skin(name: String, bytes: Vec<u8>, variant: String, kind: Option<String>) -> Result<SavedSkin> {
    check_png(&bytes)?;
    let skin = SavedSkin {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().chars().take(32).collect(),
        variant: if variant == "slim" { "slim".into() } else { "classic".into() },
        kind: if kind.as_deref() == Some("cape") { "cape".into() } else { "skin".into() },
        added: now(),
    };
    paths::ensure(&library_dir())?;
    fs::write(library_dir().join(format!("{}.png", skin.id)), &bytes)?;
    let mut list = load_index();
    list.push(skin.clone());
    save_index(&list)?;
    Ok(skin)
}

#[tauri::command]
pub fn read_skin(id: String) -> Result<Vec<u8>> {
    check_id(&id)?;
    Ok(fs::read(library_dir().join(format!("{id}.png")))?)
}

#[tauri::command]
pub fn delete_skin(id: String) -> Result<()> {
    check_id(&id)?;
    let mut list = load_index();
    list.retain(|s| s.id != id);
    save_index(&list)?;
    let _ = fs::remove_file(library_dir().join(format!("{id}.png")));
    Ok(())
}
