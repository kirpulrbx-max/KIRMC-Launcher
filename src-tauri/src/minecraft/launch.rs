//! Preparación (descargas) y arranque del proceso de Minecraft.
//! Eventos emitidos: game://started, game://log (por lotes), game://exit

use std::{collections::HashMap, process::Stdio, time::Duration};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::oneshot,
};

use super::{
    download::{self, Job},
    loaders, manifest, rules,
    version::{self, VersionJson},
};
use crate::{
    auth::{self, AccountKind, Session},
    config::{self, LauncherConfig},
    error::{Error, Result},
    instances::{self, Instance},
    java, notify, paths,
    state::AppState,
    yggdrasil,
};

/// Flags G1GC afinados para cliente (pausas cortas, sin tirones)
const OPTIMIZED_JVM: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=200",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1HeapRegionSize=8M",
    "-XX:G1ReservePercent=20",
    "-XX:InitiatingHeapOccupancyPercent=15",
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogBatch<'a> {
    instance_id: &'a str,
    err: bool,
    lines: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitEvent {
    instance_id: String,
    code: Option<i32>,
}

/// Descarga todo lo necesario y devuelve la versión resuelta + el Java con el que lanzarla
async fn prepare(app: &AppHandle, state: &AppState, inst: &mut Instance, cfg: &LauncherConfig) -> Result<(VersionJson, String)> {
    let http = &state.http;
    let par = cfg.concurrent_downloads;
    let verify = cfg.verify_files;
    let tag = inst.id.clone();

    manifest::ensure_vanilla_json(http, &inst.mc_version).await?;
    // Java exacto que pide la versión (8 para 1.12.2, 21 para 1.21, 25 para 26.x…); se descarga si falta
    let required = version::load(&inst.mc_version)?.java_version.map(|j| j.major_version).unwrap_or(8);
    let java = match inst.java_path.as_deref().or(cfg.java_path.as_deref()).filter(|p| !p.trim().is_empty()) {
        Some(explicit) => explicit.to_string(),
        None => java::ensure(app, http, &tag, required).await?,
    };
    let id = loaders::install(http, inst.loader, &inst.mc_version, inst.loader_version.as_deref(), &java).await?;
    if inst.version_id.as_deref() != Some(id.as_str()) {
        inst.version_id = Some(id.clone());
        instances::save(inst)?;
    }
    let v = version::resolve(&id)?;

    // 1) client.jar + librerías
    let mut jobs = Vec::new();
    if let Some(client) = v.downloads.as_ref().and_then(|d| d.client.clone()) {
        jobs.push(Job { url: client.url, path: paths::version_jar(&v.jar_id()), sha1: client.sha1, size: client.size });
    }
    for lib in v.libraries.iter().filter(|l| rules::allowed(l.rules.as_ref())) {
        for a in lib.artifact().into_iter().chain(lib.native()) {
            let path = paths::libraries().join(a.path.unwrap_or_default());
            jobs.push(Job { url: a.url, path, sha1: a.sha1, size: a.size });
        }
    }
    download::run(app, http, &tag, "libraries", jobs, par, verify).await?;

    // 2) assets (índice + objetos direccionados por hash)
    if let Some(idx) = &v.asset_index {
        let idx_path = paths::assets().join("indexes").join(format!("{}.json", idx.id));
        let idx_job = Job { url: idx.url.clone(), path: idx_path.clone(), sha1: idx.sha1.clone(), size: None };
        download::run(app, http, &tag, "assets", vec![idx_job], 1, verify).await?;
        let index: Value = serde_json::from_slice(&std::fs::read(&idx_path)?)?;
        let jobs: Vec<Job> = index["objects"]
            .as_object()
            .map(|objects| {
                objects
                    .values()
                    .filter_map(|o| {
                        let hash = o["hash"].as_str()?;
                        let sub = hash.get(..2)?;
                        Some(Job {
                            url: format!("https://resources.download.minecraft.net/{sub}/{hash}"),
                            path: paths::assets().join("objects").join(sub).join(hash),
                            sha1: Some(hash.to_string()),
                            size: o["size"].as_u64(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        download::run(app, http, &tag, "assets", jobs, par, verify).await?;
    }

    extract_natives(&v)?;
    Ok((v, java))
}

/// Solo versiones legacy; LWJGL 3.3+ extrae sus natives desde el classpath.
fn extract_natives(v: &VersionJson) -> Result<()> {
    let dir = paths::natives(&v.id);
    paths::ensure(&dir)?;
    for lib in v.libraries.iter().filter(|l| rules::allowed(l.rules.as_ref())) {
        let Some(native) = lib.native() else { continue };
        let jar = paths::libraries().join(native.path.unwrap_or_default());
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&jar)?)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if entry.is_dir() || entry.name().starts_with("META-INF") {
                continue;
            }
            let Some(file_name) = std::path::Path::new(entry.name()).file_name().map(|f| f.to_owned()) else { continue };
            let out = dir.join(file_name);
            if !out.exists() {
                std::io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
            }
        }
    }
    Ok(())
}

fn flatten(args: &[Value], out: &mut Vec<String>) {
    for a in args {
        match a {
            Value::String(s) => out.push(s.clone()),
            Value::Object(o) if rules::allowed(o.get("rules").and_then(Value::as_array)) => match o.get("value") {
                Some(Value::String(s)) => out.push(s.clone()),
                Some(Value::Array(list)) => out.extend(list.iter().filter_map(|x| x.as_str().map(String::from))),
                _ => {}
            },
            _ => {}
        }
    }
}

fn build_command(
    v: &VersionJson,
    inst: &Instance,
    s: &Session,
    cfg: &LauncherConfig,
    java: &str,
    server: Option<&str>,
    capture_output: bool,
    agent: Option<&str>,
) -> Command {
    let game_dir = instances::game_dir(&inst.id);
    let str_of = |p: std::path::PathBuf| p.to_string_lossy().into_owned();

    let mut cp: Vec<String> = v
        .libraries
        .iter()
        .filter(|l| rules::allowed(l.rules.as_ref()))
        .filter_map(|l| l.artifact()?.path)
        .map(|p| str_of(paths::libraries().join(p)))
        .collect();
    cp.push(str_of(paths::version_jar(&v.jar_id())));

    let assets_index = v.asset_index.as_ref().map(|a| a.id.clone()).or_else(|| v.assets.clone()).unwrap_or_default();
    let vars: HashMap<&str, String> = HashMap::from([
        ("auth_player_name", s.username.clone()),
        ("version_name", v.id.clone()),
        ("game_directory", str_of(game_dir.clone())),
        ("assets_root", str_of(paths::assets())),
        ("game_assets", str_of(paths::assets())),
        ("assets_index_name", assets_index),
        ("auth_uuid", s.uuid.clone()),
        ("auth_access_token", s.access_token.clone()),
        ("auth_session", s.access_token.clone()),
        ("auth_xuid", "0".into()),
        ("clientid", String::new()),
        ("user_type", s.user_type.into()),
        ("user_properties", "{}".into()),
        ("version_type", "KIRMC".into()),
        ("natives_directory", str_of(paths::natives(&v.id))),
        ("launcher_name", "kirmc-launcher".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("classpath", cp.join(paths::cp_sep())),
        ("classpath_separator", paths::cp_sep().into()),
        ("library_directory", str_of(paths::libraries())),
    ]);
    let sub = |arg: &String| -> String {
        if !arg.contains("${") {
            return arg.clone();
        }
        vars.iter().fold(arg.clone(), |acc, (k, val)| acc.replace(&format!("${{{k}}}"), val))
    };

    let min = inst.min_ram_mb.unwrap_or(cfg.min_ram_mb);
    let max = inst.max_ram_mb.unwrap_or(cfg.max_ram_mb).max(min);
    let mut jvm = vec![format!("-Xms{min}M"), format!("-Xmx{max}M")];
    // Skinfix no-premium: authlib-injector apuntando al servidor de skins (local de KIRMC o el configurado)
    if let Some(agent) = agent {
        jvm.push(agent.to_string());
    }
    jvm.extend(OPTIMIZED_JVM.iter().map(|f| f.to_string()));
    jvm.extend(inst.jvm_args.iter().cloned());

    let mut game = Vec::new();
    if let Some(args) = &v.arguments {
        flatten(&args.jvm, &mut jvm);
        flatten(&args.game, &mut game);
    }
    if let Some(legacy) = &v.minecraft_arguments {
        jvm.extend(["-Djava.library.path=${natives_directory}".into(), "-cp".into(), "${classpath}".into()]);
        game.extend(legacy.split_whitespace().map(String::from));
    }

    // QuickPlay: entrar directo a un servidor
    if let Some(addr) = server {
        let modern = v.arguments.as_ref().is_some_and(|a| a.game.iter().any(|x| x.to_string().contains("quickPlayMultiplayer")));
        if modern {
            game.extend(["--quickPlayMultiplayer".into(), addr.to_string()]);
        } else {
            let (host, port) = addr.rsplit_once(':').unwrap_or((addr, "25565"));
            game.extend(["--server".into(), host.to_string(), "--port".into(), port.to_string()]);
        }
    }

    // Resolución de ventana (funciona en todas las versiones, sin depender de las reglas "features")
    if cfg.fullscreen {
        game.push("--fullscreen".into());
    } else {
        game.extend(["--width".into(), cfg.game_width.to_string(), "--height".into(), cfg.game_height.to_string()]);
    }

    let output = || if capture_output { Stdio::piped() } else { Stdio::null() };
    let mut cmd = Command::new(java);
    cmd.args(jvm.iter().map(&sub))
        .arg(&v.main_class)
        .args(game.iter().map(&sub))
        .current_dir(&game_dir)
        .stdin(Stdio::null())
        .stdout(output())
        .stderr(output());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Reenvía la salida del juego en lotes (máx. ~10/s) para no saturar el IPC ni React.
async fn pump<R: AsyncRead + Unpin>(app: AppHandle, id: String, reader: R, err: bool) {
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::with_capacity(512);
    let mut batch: Vec<String> = Vec::with_capacity(64);
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    let flush = |batch: &mut Vec<String>| {
        if !batch.is_empty() {
            let _ = app.emit("game://log", LogBatch { instance_id: &id, err, lines: std::mem::take(batch) });
        }
    };
    loop {
        tokio::select! {
            r = reader.read_until(b'\n', &mut buf) => match r {
                Ok(n) if n > 0 => {
                    // from_utf8_lossy: la consola de Windows no siempre emite UTF-8
                    batch.push(String::from_utf8_lossy(&buf).trim_end().to_string());
                    buf.clear();
                    if batch.len() >= 256 { flush(&mut batch); }
                }
                _ => { flush(&mut batch); break; }
            },
            _ = tick.tick() => flush(&mut batch),
        }
    }
}

#[tauri::command]
pub async fn install_instance(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    let cfg = config::load();
    let mut inst = instances::load(&id)?;
    prepare(&app, &state, &mut inst, &cfg).await?;
    // Deja listo authlib-injector para cuentas no-premium (no bloquea la instalación si falla)
    let _ = yggdrasil::ensure_injector(&state.http).await;
    notify::send(&app, notify::Note::InstallDone(&inst.name));
    Ok(())
}

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    account_uuid: String,
    server: Option<String>,
) -> Result<()> {
    let cfg = config::load();
    {
        let running = state.running.lock().unwrap();
        if running.contains_key(&id) {
            return Err(Error::Msg("La instancia ya está en ejecución".into()));
        }
        if !cfg.allow_multiple_instances && !running.is_empty() {
            return Err(Error::Msg("Ya hay otra instancia abierta. Actívalo en Ajustes → Varias instancias".into()));
        }
    }
    let mut inst = instances::load(&id)?;
    let session = auth::session_for(&state, &account_uuid).await?;
    let (v, java) = prepare(&app, &state, &mut inst, &cfg).await?;

    // No-premium: skins y capas vía authlib-injector + servidor Yggdrasil (local salvo que se configure otro)
    let agent = if session.kind == AccountKind::Offline {
        match cfg.skin_server.clone().filter(|u| !u.trim().is_empty()).or_else(yggdrasil::root_url) {
            Some(url) => {
                let jar = yggdrasil::ensure_injector(&state.http).await?;
                Some(format!("-javaagent:{}={url}", jar.display()))
            }
            None => None,
        }
    } else {
        None
    };

    // Si el launcher se cierra, no capturamos la salida (nadie la leería)
    let closing = cfg.after_launch == "close";
    let mut child = build_command(&v, &inst, &session, &cfg, &java, server.as_deref(), !closing, agent.as_deref())
        .spawn()
        .map_err(|e| Error::Msg(format!("No se pudo iniciar Java ({java}): {e}")))?;

    inst.last_played = Some(instances::now());
    instances::save(&inst)?;

    if let Some(out) = child.stdout.take() {
        tauri::async_runtime::spawn(pump(app.clone(), id.clone(), out, false));
    }
    if let Some(err) = child.stderr.take() {
        tauri::async_runtime::spawn(pump(app.clone(), id.clone(), err, true));
    }

    let (kill_tx, kill_rx) = oneshot::channel::<()>();
    state.running.lock().unwrap().insert(id.clone(), kill_tx);
    let _ = app.emit("game://started", &id);

    if closing {
        // El juego sigue vivo: el proceso hijo no se mata al salir el launcher
        app.exit(0);
        return Ok(());
    }
    // "hide": el WebView deja de renderizar mientras se juega y la RAM baja
    if cfg.after_launch == "hide" {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    }

    let name = inst.name.clone();
    tauri::async_runtime::spawn(async move {
        let mut killed = false;
        let code = tokio::select! {
            status = child.wait() => status.ok().and_then(|s| s.code()),
            _ = kill_rx => { killed = true; None }
        };
        if killed {
            let _ = child.kill().await;
        }
        app.state::<AppState>().running.lock().unwrap().remove(&id);
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.set_focus();
        }
        if let Some(c) = code.filter(|c| *c != 0) {
            notify::send(&app, notify::Note::GameCrashed(&name, c));
        }
        let _ = app.emit("game://exit", ExitEvent { instance_id: id, code });
    });
    Ok(())
}

#[tauri::command]
pub fn kill_instance(state: State<'_, AppState>, id: String) {
    if let Some(tx) = state.running.lock().unwrap().remove(&id) {
        let _ = tx.send(());
    }
}

#[tauri::command]
pub fn running_instances(state: State<'_, AppState>) -> Vec<String> {
    state.running.lock().unwrap().keys().cloned().collect()
}
