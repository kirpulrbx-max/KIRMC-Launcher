use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Red: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Disco: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ZIP: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Almacén seguro: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("Ventana: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("{0}")]
    Msg(String),
}

// Los errores llegan al frontend como string legible
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
