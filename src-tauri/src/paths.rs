//! Layout en disco (compartido entre instancias para no duplicar GBs):
//! %APPDATA%/KIRMC/{versions,libraries,assets,natives,instances,java}

use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("KIRMC")
}

pub fn versions() -> PathBuf {
    root().join("versions")
}

pub fn libraries() -> PathBuf {
    root().join("libraries")
}

pub fn assets() -> PathBuf {
    root().join("assets")
}

pub fn instances() -> PathBuf {
    root().join("instances")
}

pub fn natives(version: &str) -> PathBuf {
    root().join("natives").join(version)
}

pub fn config_file() -> PathBuf {
    root().join("config.json")
}

pub fn accounts_file() -> PathBuf {
    root().join("accounts.json")
}

pub fn version_json(id: &str) -> PathBuf {
    versions().join(id).join(format!("{id}.json"))
}

pub fn version_jar(id: &str) -> PathBuf {
    versions().join(id).join(format!("{id}.jar"))
}

pub fn ensure(p: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(p)
}

/// Nombre de SO tal como lo usan las reglas de Mojang
pub fn os_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub fn cp_sep() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}
