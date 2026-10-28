//! Feed de noticias: notas de parche oficiales + feed propio de la comunidad KIRMC.

use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::{error::Result, state::AppState};

const PATCH_NOTES: &str = "https://launchercontent.mojang.com/v2/javaPatchNotes.json";
const CONTENT_BASE: &str = "https://launchercontent.mojang.com";
/// Endpoint propio (JSON: lista de NewsItem). Cámbialo por tu backend.
const COMMUNITY_FEED: &str = "https://kirmc.example/api/news.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub image: Option<String>,
    /// ISO 8601, sirve para ordenar
    pub date: String,
    /// "release" | "snapshot" | "community" | ...
    pub tag: String,
    pub url: Option<String>,
}

fn s(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_string()
}

async fn patch_notes(http: &Client) -> Result<Vec<NewsItem>> {
    let v: Value = http.get(PATCH_NOTES).send().await?.error_for_status()?.json().await?;
    let entries = v["entries"].as_array().cloned().unwrap_or_default();
    Ok(entries
        .iter()
        .take(20)
        .map(|n| NewsItem {
            id: s(n, "id"),
            title: s(n, "title"),
            summary: s(n, "shortText"),
            image: n["image"]["url"]
                .as_str()
                .map(|u| if u.starts_with("http") { u.to_string() } else { format!("{CONTENT_BASE}{u}") }),
            date: s(n, "date"),
            tag: n["type"].as_str().unwrap_or("release").to_string(),
            url: n["version"].as_str().map(|ver| format!("https://minecraft.wiki/w/Java_Edition_{ver}")),
        })
        .collect())
}

async fn community(http: &Client) -> Result<Vec<NewsItem>> {
    Ok(http.get(COMMUNITY_FEED).timeout(Duration::from_secs(4)).send().await?.error_for_status()?.json().await?)
}

#[tauri::command]
pub async fn fetch_news(state: State<'_, AppState>) -> Result<Vec<NewsItem>> {
    let (patch, comm) = tokio::join!(patch_notes(&state.http), community(&state.http));
    let mut items = comm.unwrap_or_default();
    match patch {
        Ok(p) => items.extend(p),
        Err(e) if items.is_empty() => return Err(e),
        Err(_) => {}
    }
    items.sort_by(|a, b| b.date.cmp(&a.date));
    items.truncate(30);
    Ok(items)
}
