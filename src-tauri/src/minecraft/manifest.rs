use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    error::{Error, Result},
    paths,
    state::AppState,
};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub sha1: String,
    pub release_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<VersionEntry>,
}

/// Red primero; si falla, caché en disco (modo offline).
pub async fn fetch(http: &Client) -> Result<Manifest> {
    let cache = paths::root().join("version_manifest_v2.json");
    match http.get(MANIFEST_URL).send().await.and_then(|r| r.error_for_status()) {
        Ok(res) => {
            let bytes = res.bytes().await?;
            paths::ensure(&paths::root())?;
            std::fs::write(&cache, &bytes)?;
            Ok(serde_json::from_slice(&bytes)?)
        }
        Err(e) => std::fs::read(&cache)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .ok_or(Error::Http(e)),
    }
}

#[tauri::command]
pub async fn list_versions(state: State<'_, AppState>) -> Result<Manifest> {
    fetch(&state.http).await
}

pub async fn ensure_vanilla_json(http: &Client, id: &str) -> Result<()> {
    let path = paths::version_json(id);
    if path.exists() {
        return Ok(());
    }
    let manifest = fetch(http).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == id)
        .ok_or_else(|| Error::Msg(format!("Versión {id} no encontrada")))?;
    let bytes = http.get(&entry.url).send().await?.error_for_status()?.bytes().await?;
    paths::ensure(&paths::versions().join(id))?;
    std::fs::write(&path, &bytes)?;
    Ok(())
}
