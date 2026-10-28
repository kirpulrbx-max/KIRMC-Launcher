use std::{collections::HashMap, sync::Mutex, time::Instant};

use tokio::sync::oneshot;

pub struct CachedToken {
    pub token: String,
    pub expires: Instant,
}

/// Estado global. Todo es ligero: un cliente HTTP con pool compartido y dos mapas pequeños.
pub struct AppState {
    pub http: reqwest::Client,
    /// Tokens de Minecraft en memoria (nunca en disco)
    pub tokens: Mutex<HashMap<String, CachedToken>>,
    /// instancia -> canal para matar el proceso del juego
    pub running: Mutex<HashMap<String, oneshot::Sender<()>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("KIRMC-Launcher/", env!("CARGO_PKG_VERSION")))
                .pool_max_idle_per_host(16)
                .build()
                .expect("cliente HTTP"),
            tokens: Default::default(),
            running: Default::default(),
        }
    }
}
