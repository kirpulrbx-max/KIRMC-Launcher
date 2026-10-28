//! Login de Microsoft con el flujo "authorization code" de login.live.com (app de escritorio):
//! el launcher abre una ventana con la página oficial de Microsoft y captura la redirección a
//! `oauth20_desktop.srf?code=…`. Luego: token MS -> Xbox Live -> XSTS -> Minecraft Services.

use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{Account, AccountKind, Cape};
use crate::error::{Error, Result};

/// Client ID de la app KIRMC en Azure Entra ID (identificador público, no es un secreto).
/// Se puede sustituir desde Ajustes → Cuentas sin recompilar.
const DEFAULT_CLIENT_ID: &str = "5f5fbf52-723d-4d75-bf7f-771be83f54a6";
/// URI de redirección registrada en Azure (plataforma "Móvil y escritorio")
pub const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
const AUTHORIZE_URL: &str = "https://login.live.com/oauth20_authorize.srf";
const TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
const SCOPE: &str = "XboxLive.signin offline_access";
const MC_API: &str = "https://api.minecraftservices.com";

/// Client ID configurado en Ajustes o el de KIRMC
fn client_id() -> Result<String> {
    let id = crate::config::load()
        .ms_client_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string());
    if id.len() < 32 {
        return Err(Error::Msg("El Client ID de Azure configurado en Ajustes → Cuentas no es válido".into()));
    }
    Ok(id)
}

#[derive(Debug, Deserialize)]
pub struct MsToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

pub struct McToken {
    pub access_token: String,
    pub expires_in: u64,
}

/// Página oficial de inicio de sesión (siempre deja elegir cuenta)
pub fn authorize_url() -> Result<String> {
    let mut url = reqwest::Url::parse(AUTHORIZE_URL).map_err(|e| Error::Msg(e.to_string()))?;
    url.query_pairs_mut()
        .append_pair("client_id", &client_id()?)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("scope", SCOPE)
        .append_pair("prompt", "select_account");
    Ok(url.to_string())
}

async fn token_request(http: &Client, params: &[(&str, &str)]) -> Result<MsToken> {
    let res = http.post(TOKEN_URL).form(params).send().await?;
    if !res.status().is_success() {
        let body: Value = res.json().await.unwrap_or_default();
        let detail = body["error_description"].as_str().or(body["error"].as_str()).unwrap_or("error desconocido");
        return Err(Error::Msg(format!("Microsoft rechazó el inicio de sesión: {detail}")));
    }
    Ok(res.json().await?)
}

/// Canjea el `code` de la redirección por tokens de Microsoft
pub async fn exchange_code(http: &Client, code: &str) -> Result<MsToken> {
    let cid = client_id()?;
    token_request(
        http,
        &[
            ("client_id", cid.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", REDIRECT_URI),
            ("scope", SCOPE),
        ],
    )
    .await
}

pub async fn refresh(http: &Client, refresh_token: &str) -> Result<MsToken> {
    let cid = client_id()?;
    token_request(
        http,
        &[
            ("client_id", cid.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("redirect_uri", REDIRECT_URI),
            ("scope", SCOPE),
        ],
    )
    .await
    .map_err(|_| Error::Msg("La sesión de Microsoft expiró, vuelve a iniciar sesión".into()))
}
fn field<'a>(v: &'a Value, ptr: &str) -> Result<&'a str> {
    v.pointer(ptr).and_then(Value::as_str).ok_or_else(|| Error::Msg(format!("Respuesta inesperada ({ptr})")))
}

/// Microsoft access token -> XBL -> XSTS -> token de Minecraft
pub async fn minecraft_login(http: &Client, ms_access: &str) -> Result<McToken> {
    let xbl: Value = http
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={ms_access}") },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let xbl_token = field(&xbl, "/Token")?;
    let uhs = field(&xbl, "/DisplayClaims/xui/0/uhs")?;

    let res = http
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?;
    let status = res.status();
    let xsts: Value = res.json().await.unwrap_or_default();
    if !status.is_success() {
        return Err(Error::Msg(match xsts["XErr"].as_u64() {
            Some(2148916233) => "Esta cuenta no tiene perfil de Xbox. Créalo en xbox.com".to_string(),
            Some(2148916235) => "Xbox Live no está disponible en tu país".to_string(),
            Some(2148916236 | 2148916237) => "La cuenta requiere verificación de edad".to_string(),
            Some(2148916238) => "Cuenta de menor: debe pertenecer a una familia de Microsoft".to_string(),
            _ => format!("XSTS rechazó el inicio de sesión ({status})"),
        }));
    }
    let xsts_token = field(&xsts, "/Token")?;

    let res = http
        .post(format!("{MC_API}/authentication/login_with_xbox"))
        .json(&json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }))
        .send()
        .await?;
    // 403 = Microsoft/Xbox aceptaron la cuenta, pero la app de Azure no está aprobada para la API de Minecraft
    if res.status() == StatusCode::FORBIDDEN {
        return Err(Error::Msg(
            "Tu cuenta de Microsoft es válida, pero Mojang todavía no ha aprobado este Client ID para la API de Minecraft. \
             Solicítalo en https://aka.ms/mce-reviewappid (suele tardar unos días)."
                .into(),
        ));
    }
    let mc: Value = res.error_for_status()?.json().await?;
    Ok(McToken {
        access_token: field(&mc, "/access_token")?.to_string(),
        expires_in: mc["expires_in"].as_u64().unwrap_or(86_400),
    })
}

#[derive(Deserialize)]
struct ProfileSkin {
    url: String,
    state: String,
    #[serde(default)]
    variant: Option<String>,
}

#[derive(Deserialize)]
struct ProfileCape {
    id: String,
    url: String,
    state: String,
    #[serde(default)]
    alias: String,
}

#[derive(Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    skins: Vec<ProfileSkin>,
    #[serde(default)]
    capes: Vec<ProfileCape>,
}

impl Profile {
    pub fn into_account(self) -> Account {
        let skin = self.skins.into_iter().find(|s| s.state == "ACTIVE");
        Account {
            cape_url: None,
            uuid: self.id,
            username: self.name,
            kind: AccountKind::Microsoft,
            skin_url: skin.as_ref().map(|s| s.url.clone()),
            skin_variant: skin.and_then(|s| s.variant).map(|v| v.to_lowercase()),
            capes: self
                .capes
                .into_iter()
                .map(|c| Cape { active: c.state == "ACTIVE", id: c.id, alias: c.alias, url: c.url })
                .collect(),
        }
    }
}

pub async fn profile(http: &Client, mc_token: &str) -> Result<Profile> {
    let res = http.get(format!("{MC_API}/minecraft/profile")).bearer_auth(mc_token).send().await?;
    if res.status() == StatusCode::NOT_FOUND {
        return Err(Error::Msg("Esta cuenta no posee Minecraft: Java Edition".into()));
    }
    Ok(res.error_for_status()?.json().await?)
}

/// variant: "classic" | "slim"
pub async fn upload_skin(http: &Client, mc_token: &str, png: Vec<u8>, variant: &str) -> Result<()> {
    let part = reqwest::multipart::Part::bytes(png).file_name("skin.png").mime_str("image/png")?;
    let form = reqwest::multipart::Form::new().text("variant", variant.to_string()).part("file", part);
    http.post(format!("{MC_API}/minecraft/profile/skins"))
        .bearer_auth(mc_token)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

/// None = ocultar capa
pub async fn set_cape(http: &Client, mc_token: &str, cape_id: Option<&str>) -> Result<()> {
    let url = format!("{MC_API}/minecraft/profile/capes/active");
    let req = match cape_id {
        Some(id) => http.put(url).json(&json!({ "capeId": id })),
        None => http.delete(url),
    };
    req.bearer_auth(mc_token).send().await?.error_for_status()?;
    Ok(())
}
