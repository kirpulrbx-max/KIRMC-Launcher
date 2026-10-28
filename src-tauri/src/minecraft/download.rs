//! Descargador paralelo con verificación SHA1.
//! Comprobación rápida: si conocemos el tamaño solo leemos metadata (no hasheamos miles de assets).

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::{stream, StreamExt};
use reqwest::Client;
use serde::Serialize;
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Emitter};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Job {
    pub url: String,
    pub path: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    /// Id de la instancia (o tarea) a la que pertenece la descarga
    tag: &'a str,
    stage: &'a str,
    done: usize,
    total: usize,
}

fn sha1_matches(job: &Job, hash: &str) -> bool {
    std::fs::read(&job.path).map(|b| hex::encode(Sha1::digest(&b)) == hash).unwrap_or(false)
}

fn is_valid(job: &Job, verify: bool) -> bool {
    let Ok(meta) = std::fs::metadata(&job.path) else { return false };
    if let (true, Some(hash)) = (verify, &job.sha1) {
        return sha1_matches(job, hash);
    }
    match (job.size, &job.sha1) {
        (Some(size), _) => meta.len() == size,
        (None, Some(hash)) => sha1_matches(job, hash),
        (None, None) => true,
    }
}

async fn try_fetch(http: &Client, job: &Job) -> Result<()> {
    let bytes = http.get(&job.url).send().await?.error_for_status()?.bytes().await?;
    if let Some(expected) = &job.sha1 {
        if hex::encode(Sha1::digest(&bytes)) != *expected {
            return Err(Error::Msg(format!("SHA1 inválido: {}", job.url)));
        }
    }
    if let Some(parent) = job.path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    // Escritura atómica: .part y luego rename, para no dejar archivos corruptos
    let mut tmp = job.path.clone().into_os_string();
    tmp.push(".part");
    tokio::fs::write(&tmp, &bytes).await?;
    tokio::fs::rename(&tmp, &job.path).await?;
    Ok(())
}

async fn fetch(http: &Client, job: &Job) -> Result<()> {
    let mut last = None;
    for attempt in 1..=3u64 {
        match try_fetch(http, job).await {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
        }
        tokio::time::sleep(Duration::from_millis(400 * attempt)).await;
    }
    Err(last.expect("al menos un intento"))
}

/// Emite `download://progress` { tag, stage, done, total }
pub async fn run(
    app: &AppHandle,
    http: &Client,
    tag: &str,
    stage: &str,
    jobs: Vec<Job>,
    parallel: usize,
    verify: bool,
) -> Result<()> {
    let pending: Vec<Job> = tokio::task::spawn_blocking(move || {
        jobs.into_iter().filter(|j| !j.url.is_empty() && !is_valid(j, verify)).collect()
    })
    .await
    .map_err(|e| Error::Msg(e.to_string()))?;

    let total = pending.len();
    let _ = app.emit("download://progress", Progress { tag, stage, done: 0, total });
    if total == 0 {
        return Ok(());
    }

    // Cada tarea posee sus datos (Client y AppHandle son Arc internamente): futuros `Send` sin lifetimes
    let done = Arc::new(AtomicUsize::new(0));
    let (tag, stage): (Arc<str>, Arc<str>) = (tag.into(), stage.into());
    let tasks = pending.into_iter().map(|job| {
        let (http, app, done, tag, stage) = (http.clone(), app.clone(), done.clone(), tag.clone(), stage.clone());
        async move {
            let r = fetch(&http, &job).await;
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n == total || n % 16 == 0 {
                let _ = app.emit("download://progress", Progress { tag: &tag, stage: &stage, done: n, total });
            }
            r
        }
    });
    let results: Vec<Result<()>> = stream::iter(tasks).buffer_unordered(parallel.max(1)).collect().await;
    results.into_iter().collect()
}
