//! Cargadores de mods. Fabric y Quilt publican un "profile JSON" que hereda de vanilla:
//! basta con guardarlo. Forge necesita ejecutar su instalador (procesadores que parchean el jar).

use std::collections::HashMap;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::{
    error::{Error, Result},
    paths,
    state::AppState,
};

const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
const QUILT_META: &str = "https://meta.quiltmc.org/v3";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
/// El maven ya no publica maven-metadata.json; el índice JSON (mc -> versiones) vive en files.minecraftforge.net
const FORGE_INDEX: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
}

fn meta_base(loader: Loader) -> &'static str {
    if loader == Loader::Fabric {
        FABRIC_META
    } else {
        QUILT_META
    }
}

/// Versiones del cargador para una versión de Minecraft, la más reciente primero
#[tauri::command]
pub async fn list_loader_versions(state: State<'_, AppState>, loader: Loader, mc_version: String) -> Result<Vec<String>> {
    let http = &state.http;
    Ok(match loader {
        Loader::Vanilla => vec![],
        Loader::Fabric | Loader::Quilt => {
            let res = http.get(format!("{}/versions/loader/{mc_version}", meta_base(loader))).send().await?;
            // 400/404 = el cargador no existe para esa versión (Fabric/Quilt: 1.14+). Lista vacía, no error.
            if matches!(res.status().as_u16(), 400 | 404) {
                return Ok(vec![]);
            }
            let list: Vec<Value> = res.error_for_status()?.json().await?;
            list.iter().filter_map(|v| v["loader"]["version"].as_str().map(String::from)).collect()
        }
        Loader::Forge => {
            let meta: HashMap<String, Vec<String>> = http.get(FORGE_INDEX).send().await?.error_for_status()?.json().await?;
            let prefix = format!("{mc_version}-");
            meta.get(&mc_version)
                .map(|list| list.iter().rev().map(|f| f.trim_start_matches(prefix.as_str()).to_string()).collect())
                .unwrap_or_default()
        }
    })
}

/// Instala el cargador (si falta) y devuelve el id de versión a lanzar.
pub async fn install(http: &Client, loader: Loader, mc: &str, loader_version: Option<&str>, java: &str) -> Result<String> {
    let need = || loader_version.ok_or_else(|| Error::Msg("Falta la versión del cargador".into()));
    match loader {
        Loader::Vanilla => Ok(mc.to_string()),

        Loader::Fabric | Loader::Quilt => {
            let lv = need()?;
            let prefix = if loader == Loader::Fabric { "fabric-loader" } else { "quilt-loader" };
            let id = format!("{prefix}-{lv}-{mc}");
            if paths::version_json(&id).exists() {
                return Ok(id);
            }
            let mut profile: Value = http
                .get(format!("{}/versions/loader/{mc}/{lv}/profile/json", meta_base(loader)))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            profile["id"] = Value::String(id.clone());
            paths::ensure(&paths::versions().join(&id))?;
            std::fs::write(paths::version_json(&id), serde_json::to_vec_pretty(&profile)?)?;
            Ok(id)
        }

        Loader::Forge => {
            let fv = need()?;
            let id = format!("{mc}-forge-{fv}");
            if paths::version_json(&id).exists() {
                return Ok(id);
            }
            let full = format!("{mc}-{fv}");
            let tmp = paths::root().join("tmp");
            let installer = tmp.join(format!("forge-{full}-installer.jar"));
            if !installer.exists() {
                let url = format!("{FORGE_MAVEN}/{full}/forge-{full}-installer.jar");
                let bytes = http.get(url).send().await?.error_for_status()?.bytes().await?;
                paths::ensure(&tmp)?;
                std::fs::write(&installer, &bytes)?;
            }
            // El instalador exige un launcher_profiles.json en el directorio destino
            let profiles = paths::root().join("launcher_profiles.json");
            if !profiles.exists() {
                std::fs::write(&profiles, r#"{"profiles":{}}"#)?;
            }
            let mut cmd = tokio::process::Command::new(java);
            cmd.arg("-jar").arg(&installer).arg("--installClient").arg(paths::root());
            #[cfg(windows)]
            {
                cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            }
            let out = cmd.output().await?;
            if !out.status.success() || !paths::version_json(&id).exists() {
                let log = String::from_utf8_lossy(&out.stdout);
                let tail: Vec<&str> = log.lines().collect();
                let tail = tail[tail.len().saturating_sub(15)..].join("\n");
                return Err(Error::Msg(format!("El instalador de Forge falló:\n{tail}")));
            }
            Ok(id)
        }
    }
}
