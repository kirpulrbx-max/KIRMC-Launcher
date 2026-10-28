//! Sistema de cuentas dual:
//! - Premium: Microsoft OAuth (ventana de login oficial) -> Xbox Live -> XSTS -> Minecraft Services.
//!   El refresh token vive en el almacén seguro del SO; el access token solo en memoria.
//! - No-Premium: nombre libre + UUID offline determinista (compatible con servidores offline).

pub mod microsoft;
pub mod offline;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::oneshot;

use crate::{
    error::{Error, Result},
    paths,
    state::{AppState, CachedToken},
    yggdrasil,
};

const KEYRING_SERVICE: &str = "kirmc-launcher";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    Microsoft,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cape {
    pub id: String,
    pub alias: String,
    pub url: String,
    pub active: bool,
}

/// Lo que se guarda en accounts.json y se envía al frontend. Sin secretos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub uuid: String,
    pub username: String,
    pub kind: AccountKind,
    #[serde(default)]
    pub skin_url: Option<String>,
    #[serde(default)]
    pub skin_variant: Option<String>,
    #[serde(default)]
    pub capes: Vec<Cape>,
    /// Capa servida por el servidor de skins local (solo cuentas no-premium; no se guarda)
    #[serde(default, skip_deserializing)]
    pub cape_url: Option<String>,
}

/// Sesión lista para el juego. Nunca se serializa.
pub struct Session {
    pub uuid: String,
    pub username: String,
    pub access_token: String,
    pub user_type: &'static str,
    pub kind: AccountKind,
}

pub fn load_accounts() -> Vec<Account> {
    std::fs::read(paths::accounts_file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_accounts(list: &[Account]) -> Result<()> {
    paths::ensure(&paths::root())?;
    std::fs::write(paths::accounts_file(), serde_json::to_vec_pretty(list)?)?;
    Ok(())
}

fn upsert(acc: Account) -> Result<Account> {
    let mut list = load_accounts();
    list.retain(|a| a.uuid != acc.uuid);
    list.push(acc.clone());
    save_accounts(&list)?;
    Ok(acc)
}

fn keyring(uuid: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(KEYRING_SERVICE, uuid)?)
}

/// Cuentas para la UI. En las no-premium, la skin/capa vienen del servidor de skins local.
#[tauri::command]
pub fn list_accounts() -> Vec<Account> {
    load_accounts()
        .into_iter()
        .map(|mut a| {
            if a.kind == AccountKind::Offline {
                let (skin, cape) = yggdrasil::texture_urls(&a.uuid);
                a.skin_url = skin;
                a.cape_url = cape;
                a.skin_variant = Some(if yggdrasil::is_slim(&a.uuid) { "slim" } else { "classic" }.into());
            }
            a
        })
        .collect()
}

#[tauri::command]
pub fn add_offline_account(username: String) -> Result<Account> {
    let username = username.trim().to_string();
    offline::validate(&username)?;
    upsert(Account {
        uuid: offline::offline_uuid(&username),
        username,
        kind: AccountKind::Offline,
        skin_url: None,
        skin_variant: None,
        capes: vec![],
        cape_url: None,
    })
}

#[tauri::command]
pub fn remove_account(state: State<'_, AppState>, uuid: String) -> Result<()> {
    let mut list = load_accounts();
    list.retain(|a| a.uuid != uuid);
    save_accounts(&list)?;
    state.tokens.lock().unwrap().remove(&uuid);
    if let Ok(entry) = keyring(&uuid) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

/// Abre la página oficial de Microsoft en una ventana propia (sin acceso al IPC del launcher),
/// espera la redirección a `oauth20_desktop.srf?code=…` y completa el login.
#[tauri::command]
pub async fn ms_login_window(app: AppHandle, state: State<'_, AppState>) -> Result<Account> {
    const LABEL: &str = "ms-login";
    if let Some(old) = app.get_webview_window(LABEL) {
        let _ = old.close();
    }
    let url = tauri::Url::parse(&microsoft::authorize_url()?).map_err(|e| Error::Msg(e.to_string()))?;

    // El primer resultado (código, error o ventana cerrada) gana; el resto se ignora
    let (tx, rx) = oneshot::channel::<Result<String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let deliver = move |r: Result<String>| {
        if let Some(sender) = tx.lock().unwrap().take() {
            let _ = sender.send(r);
        }
    };

    let on_redirect = deliver.clone();
    let window = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::External(url))
        .title("Iniciar sesión con Microsoft · KIRMC")
        .inner_size(500.0, 700.0)
        .center()
        .incognito(true) // cada login empieza limpio: permite elegir otra cuenta
        .on_navigation(move |nav| {
            if !nav.as_str().starts_with(microsoft::REDIRECT_URI) {
                return true;
            }
            let params: HashMap<String, String> = nav.query_pairs().into_owned().collect();
            on_redirect(match (params.get("code"), params.get("error")) {
                (Some(code), _) => Ok(code.clone()),
                (None, Some(err)) if err == "access_denied" => Err(Error::Msg("Inicio de sesión cancelado".into())),
                (None, Some(err)) => Err(Error::Msg(format!(
                    "Microsoft: {}",
                    params.get("error_description").unwrap_or(err)
                ))),
                _ => Err(Error::Msg("Microsoft no devolvió un código de acceso".into())),
            });
            false // no cargamos la página de redirección
        })
        .build()?;

    let on_close = deliver.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            on_close(Err(Error::Msg("Inicio de sesión cancelado".into())));
        }
    });

    let result = rx.await.unwrap_or_else(|_| Err(Error::Msg("Inicio de sesión cancelado".into())));
    let _ = window.close();
    let ms = microsoft::exchange_code(&state.http, &result?).await?;
    finish_microsoft(&state, ms).await
}

async fn finish_microsoft(state: &AppState, ms: microsoft::MsToken) -> Result<Account> {
    let mc = microsoft::minecraft_login(&state.http, &ms.access_token).await?;
    let profile = microsoft::profile(&state.http, &mc.access_token).await?;
    if let Some(rt) = &ms.refresh_token {
        keyring(&profile.id)?.set_password(rt)?;
    }
    state.tokens.lock().unwrap().insert(
        profile.id.clone(),
        CachedToken { token: mc.access_token, expires: Instant::now() + Duration::from_secs(mc.expires_in) },
    );
    upsert(profile.into_account())
}

/// Obtiene una sesión válida, renovando silenciosamente los tokens de Microsoft si hace falta.
pub async fn session_for(state: &AppState, uuid: &str) -> Result<Session> {
    let acc = load_accounts()
        .into_iter()
        .find(|a| a.uuid == uuid)
        .ok_or_else(|| Error::Msg("Cuenta no encontrada".into()))?;

    if acc.kind == AccountKind::Offline {
        return Ok(Session {
            uuid: acc.uuid.clone(),
            username: acc.username,
            access_token: acc.uuid,
            // "mojang": así authlib-injector trata la sesión como Yggdrasil (skins del servidor local)
            user_type: "mojang",
            kind: AccountKind::Offline,
        });
    }

    let cached = state
        .tokens
        .lock()
        .unwrap()
        .get(uuid)
        .filter(|t| t.expires > Instant::now() + Duration::from_secs(120))
        .map(|t| t.token.clone());

    let token = match cached {
        Some(t) => t,
        None => {
            let rt = keyring(uuid)?
                .get_password()
                .map_err(|_| Error::Msg("La sesión de Microsoft expiró, vuelve a iniciar sesión".into()))?;
            let ms = microsoft::refresh(&state.http, &rt).await?;
            finish_microsoft(state, ms).await?;
            state.tokens.lock().unwrap().get(uuid).map(|t| t.token.clone()).unwrap_or_default()
        }
    };

    Ok(Session { uuid: acc.uuid, username: acc.username, access_token: token, user_type: "msa", kind: AccountKind::Microsoft })
}

async fn premium_session(state: &AppState, uuid: &str) -> Result<Session> {
    let s = session_for(state, uuid).await?;
    if s.kind != AccountKind::Microsoft {
        return Err(Error::Msg("Las cuentas no-premium gestionan su skin en el servidor de skins de KIRMC".into()));
    }
    Ok(s)
}

#[tauri::command]
pub async fn upload_skin(state: State<'_, AppState>, uuid: String, bytes: Vec<u8>, variant: String) -> Result<Account> {
    let s = premium_session(&state, &uuid).await?;
    microsoft::upload_skin(&state.http, &s.access_token, bytes, &variant).await?;
    upsert(microsoft::profile(&state.http, &s.access_token).await?.into_account())
}

#[tauri::command]
pub async fn set_cape(state: State<'_, AppState>, uuid: String, cape_id: Option<String>) -> Result<Account> {
    let s = premium_session(&state, &uuid).await?;
    microsoft::set_cape(&state.http, &s.access_token, cape_id.as_deref()).await?;
    upsert(microsoft::profile(&state.http, &s.access_token).await?.into_account())
}
