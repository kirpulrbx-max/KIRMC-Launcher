//! Skinfix para cuentas no-premium: un mini servidor Yggdrasil en 127.0.0.1 que
//! authlib-injector (javaagent) usa en lugar de los servidores de Mojang.
//! Sirve el perfil de cada cuenta offline con su skin y capa elegidas en el launcher,
//! firmadas con una clave RSA propia (authlib-injector la toma de `signaturePublickey`).
//!
//! Resultado: la skin y la capa se ven en el juego (un jugador, LAN y servidores no-premium),
//! en cualquier versión, incluidas las antiguas como 1.12.2.
//! Spec: https://github.com/yushijinhun/authlib-injector/wiki

use std::{
    fs,
    path::PathBuf,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Client;
use rsa::{
    pkcs1v15::SigningKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    auth::{self, Account, AccountKind},
    error::{Error, Result},
    paths,
};

struct Server {
    port: u16,
    signer: SigningKey<Sha1>,
    public_pem: String,
}

static SERVER: OnceLock<Server> = OnceLock::new();
const MAX_REQUEST: usize = 64 * 1024;

fn err(e: impl std::fmt::Display) -> Error {
    Error::Msg(e.to_string())
}

/// URL raíz del servidor local (None si aún no arrancó)
pub fn root_url() -> Option<String> {
    SERVER.get().map(|s| format!("http://127.0.0.1:{}", s.port))
}

// ---------- Texturas activas por cuenta: skins/active/<uuid>.png, <uuid>-cape.png, <uuid>.json ----------

fn active_dir() -> PathBuf {
    paths::root().join("skins").join("active")
}
fn skin_path(uuid: &str) -> PathBuf {
    active_dir().join(format!("{uuid}.png"))
}
fn cape_path(uuid: &str) -> PathBuf {
    active_dir().join(format!("{uuid}-cape.png"))
}
fn meta_path(uuid: &str) -> PathBuf {
    active_dir().join(format!("{uuid}.json"))
}

fn sha1_hex(bytes: &[u8]) -> String {
    hex::encode(Sha1::digest(bytes))
}

/// URLs de skin y capa de una cuenta offline. El nombre del archivo es el hash del PNG:
/// Minecraft cachea las texturas por ese nombre, así un cambio de skin se ve al instante.
pub fn texture_urls(uuid: &str) -> (Option<String>, Option<String>) {
    let Some(root) = root_url() else { return (None, None) };
    let url = |p: PathBuf| fs::read(p).ok().map(|b| format!("{root}/textures/{}", sha1_hex(&b)));
    (url(skin_path(uuid)), url(cape_path(uuid)))
}

pub fn is_slim(uuid: &str) -> bool {
    fs::read(meta_path(uuid))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .is_some_and(|v| v["variant"] == "slim")
}

fn check_offline(uuid: &str) -> Result<Account> {
    auth::load_accounts()
        .into_iter()
        .find(|a| a.uuid == uuid && a.kind == AccountKind::Offline)
        .ok_or_else(|| Error::Msg("Solo las cuentas no-premium usan el servidor de skins local".into()))
}

fn check_png(bytes: &[u8]) -> Result<()> {
    if bytes.len() > 1024 * 1024 || !bytes.starts_with(b"\x89PNG") {
        return Err(Error::Msg("El archivo no es un PNG válido".into()));
    }
    Ok(())
}

#[tauri::command]
pub fn set_offline_skin(uuid: String, bytes: Vec<u8>, variant: String) -> Result<()> {
    check_offline(&uuid)?;
    check_png(&bytes)?;
    paths::ensure(&active_dir())?;
    fs::write(skin_path(&uuid), &bytes)?;
    let variant = if variant == "slim" { "slim" } else { "classic" };
    fs::write(meta_path(&uuid), json!({ "variant": variant }).to_string())?;
    Ok(())
}

/// `bytes = None` quita la capa
#[tauri::command]
pub fn set_offline_cape(uuid: String, bytes: Option<Vec<u8>>) -> Result<()> {
    check_offline(&uuid)?;
    match bytes {
        Some(b) => {
            check_png(&b)?;
            paths::ensure(&active_dir())?;
            fs::write(cape_path(&uuid), &b)?;
        }
        None => {
            let _ = fs::remove_file(cape_path(&uuid));
        }
    }
    Ok(())
}

// ---------- authlib-injector ----------

/// Descarga (una vez) la última versión de authlib-injector y verifica su SHA-256
pub async fn ensure_injector(http: &Client) -> Result<PathBuf> {
    let path = paths::root().join("authlib-injector.jar");
    if path.exists() {
        return Ok(path);
    }
    let info: Value = http
        .get("https://authlib-injector.yushi.moe/artifact/latest.json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let url = info["download_url"].as_str().ok_or_else(|| err("authlib-injector: respuesta inesperada"))?;
    let bytes = http.get(url).send().await?.error_for_status()?.bytes().await?;
    if let Some(expected) = info["checksums"]["sha256"].as_str() {
        use sha2::Sha256;
        if hex::encode(Sha256::digest(&bytes)) != expected {
            return Err(err("authlib-injector: checksum SHA-256 inválido"));
        }
    }
    paths::ensure(&paths::root())?;
    fs::write(&path, &bytes)?;
    Ok(path)
}

// ---------- Servidor ----------

fn load_or_create_key() -> Result<RsaPrivateKey> {
    let path = paths::root().join("yggdrasil-key.pem");
    if let Ok(pem) = fs::read_to_string(&path) {
        if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(&pem) {
            return Ok(key);
        }
    }
    let key = RsaPrivateKey::new(&mut rand::thread_rng(), 2048).map_err(err)?;
    paths::ensure(&paths::root())?;
    fs::write(&path, key.to_pkcs8_pem(LineEnding::LF).map_err(err)?.as_bytes())?;
    Ok(key)
}

/// Arranca el servidor en un puerto libre de 127.0.0.1 (solo accesible desde este PC)
pub async fn start() -> Result<()> {
    let key = tauri::async_runtime::spawn_blocking(load_or_create_key).await.map_err(err)??;
    let public_pem = key.to_public_key().to_public_key_pem(LineEnding::LF).map_err(err)?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let _ = SERVER.set(Server { port, signer: SigningKey::<Sha1>::new(key), public_pem });
    tauri::async_runtime::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            tauri::async_runtime::spawn(handle(socket));
        }
    });
    Ok(())
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

async fn handle(mut socket: TcpStream) {
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
        if let Some(p) = find_subsequence(&buf, b"\r\n\r\n") {
            break p + 4;
        }
        if buf.len() > MAX_REQUEST {
            return;
        }
    };

    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.lines();
    let mut request_line = lines.next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_string();
    let target = request_line.next().unwrap_or_default().to_string();
    let content_length = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
        .min(MAX_REQUEST);
    while buf.len() < header_end + content_length {
        match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let body = &buf[header_end..buf.len().min(header_end + content_length)];

    let (status, content_type, payload) = route(&method, &target, body);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.write_all(&payload).await;
    let _ = socket.shutdown().await;
}

type Reply = (&'static str, &'static str, Vec<u8>);

fn reply_json(v: Value) -> Reply {
    ("200 OK", "application/json; charset=utf-8", v.to_string().into_bytes())
}
const NO_CONTENT: Reply = ("204 No Content", "text/plain", Vec::new());
const NOT_FOUND: Reply = ("404 Not Found", "text/plain", Vec::new());

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| v)
}

fn offline_accounts() -> impl Iterator<Item = Account> {
    auth::load_accounts().into_iter().filter(|a| a.kind == AccountKind::Offline)
}

fn by_uuid(id: &str) -> Option<Account> {
    let id = id.replace('-', "");
    offline_accounts().find(|a| a.uuid.eq_ignore_ascii_case(&id))
}

fn by_name(name: &str) -> Option<Account> {
    offline_accounts().find(|a| a.username.eq_ignore_ascii_case(name))
}

fn metadata() -> Value {
    let pem = SERVER.get().map(|s| s.public_pem.clone()).unwrap_or_default();
    json!({
        "meta": {
            "serverName": "KIRMC Skins",
            "implementationName": "kirmc-launcher",
            "implementationVersion": env!("CARGO_PKG_VERSION"),
            "feature.non_email_login": true
        },
        "skinDomains": ["127.0.0.1", "localhost"],
        "signaturePublickey": pem
    })
}

fn profile(acc: &Account) -> Value {
    let (skin, cape) = texture_urls(&acc.uuid);
    let mut textures = Map::new();
    if let Some(url) = skin {
        let mut skin = json!({ "url": url });
        if is_slim(&acc.uuid) {
            skin["metadata"] = json!({ "model": "slim" });
        }
        textures.insert("SKIN".into(), skin);
    }
    if let Some(url) = cape {
        textures.insert("CAPE".into(), json!({ "url": url }));
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let payload = json!({
        "timestamp": timestamp,
        "profileId": acc.uuid,
        "profileName": acc.username,
        "textures": textures,
    });
    let value = STANDARD.encode(payload.to_string());
    let signature = SERVER.get().map(|s| STANDARD.encode(s.signer.sign(value.as_bytes()).to_bytes()));
    json!({
        "id": acc.uuid,
        "name": acc.username,
        "properties": [{ "name": "textures", "value": value, "signature": signature }]
    })
}

/// Busca el PNG activo cuyo hash coincide (las URLs de textura son /textures/<sha1>)
fn find_texture(hash: &str) -> Option<Vec<u8>> {
    fs::read_dir(active_dir())
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
        .filter_map(|e| fs::read(e.path()).ok())
        .find(|b| sha1_hex(b) == hash)
}

fn route(method: &str, target: &str, body: &[u8]) -> Reply {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let path = path.trim_end_matches('/');
    let user = |acc: Option<Account>| match acc {
        Some(a) => reply_json(json!({ "id": a.uuid, "name": a.username })),
        None => NO_CONTENT,
    };

    match method {
        "GET" if path.is_empty() => reply_json(metadata()),
        "GET" => {
            if let Some(id) = path.strip_prefix("/sessionserver/session/minecraft/profile/") {
                by_uuid(id).map(|a| reply_json(profile(&a))).unwrap_or(NO_CONTENT)
            } else if path == "/sessionserver/session/minecraft/hasJoined" {
                query_param(query, "username").and_then(by_name).map(|a| reply_json(profile(&a))).unwrap_or(NO_CONTENT)
            } else if let Some(name) = path.strip_prefix("/api/users/profiles/minecraft/") {
                user(by_name(name))
            } else if let Some(name) = path.strip_prefix("/api/minecraft/profile/lookup/name/") {
                user(by_name(name))
            } else if let Some(hash) = path.strip_prefix("/textures/") {
                find_texture(hash).map(|b| ("200 OK", "image/png", b)).unwrap_or(NOT_FOUND)
            } else {
                NOT_FOUND
            }
        }
        "POST" if path == "/sessionserver/session/minecraft/join" => NO_CONTENT,
        "POST" if path == "/api/profiles/minecraft" || path == "/api/minecraft/profile/lookup/bulk/byname" => {
            let names: Vec<String> = serde_json::from_slice(body).unwrap_or_default();
            reply_json(Value::Array(
                names.iter().filter_map(|n| by_name(n)).map(|a| json!({ "id": a.uuid, "name": a.username })).collect(),
            ))
        }
        _ => NOT_FOUND,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::{
        pkcs1v15::{Signature, VerifyingKey},
        pkcs8::DecodePublicKey,
        signature::Verifier,
        RsaPublicKey,
    };

    /// La firma SHA1withRSA del valor `textures` debe verificarse con la clave publicada en `signaturePublickey`
    #[test]
    fn textures_signature_verifies_with_published_key() {
        let key = RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
        let pem = key.to_public_key().to_public_key_pem(LineEnding::LF).unwrap();
        let value = STANDARD.encode(r#"{"profileName":"Steve","textures":{}}"#);
        let sig = SigningKey::<Sha1>::new(key).sign(value.as_bytes()).to_bytes();

        let public = RsaPublicKey::from_public_key_pem(&pem).unwrap();
        let verifier = VerifyingKey::<Sha1>::new(public);
        let sig = Signature::try_from(STANDARD.decode(STANDARD.encode(&sig)).unwrap().as_slice()).unwrap();
        assert!(verifier.verify(value.as_bytes(), &sig).is_ok());
    }

    #[test]
    fn routes_metadata_and_unknown_paths() {
        let (status, ctype, body) = route("GET", "/", b"");
        assert_eq!(status, "200 OK");
        assert!(ctype.starts_with("application/json"));
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["skinDomains"][0], "127.0.0.1");
        assert_eq!(route("GET", "/nope", b"").0, "404 Not Found");
        assert_eq!(route("POST", "/sessionserver/session/minecraft/join", b"{}").0, "204 No Content");
    }
}