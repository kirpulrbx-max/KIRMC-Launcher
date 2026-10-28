//! Modelo del "version JSON" de Mojang y fusión de herencia (`inheritsFrom`),
//! que es como Fabric, Quilt y Forge se apoyan sobre la versión vanilla.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{error::Result, paths};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Value>,
    #[serde(default)]
    pub jvm: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Downloads {
    pub client: Option<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    /// Base de repositorio Maven (estilo Fabric/Quilt)
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub rules: Option<Vec<Value>>,
    /// Natives estilo legacy (<1.19): { "windows": "natives-windows" }
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
}

impl Library {
    /// Jar principal para el classpath. None en librerías solo-natives (legacy).
    pub fn artifact(&self) -> Option<Artifact> {
        if let Some(d) = &self.downloads {
            return d.artifact.clone().map(|mut a| {
                a.path.get_or_insert_with(|| maven_path(&self.name));
                a
            });
        }
        let path = maven_path(&self.name);
        let base = self.url.as_deref().unwrap_or("https://libraries.minecraft.net/");
        let sep = if base.ends_with('/') { "" } else { "/" };
        Some(Artifact { url: format!("{base}{sep}{path}"), path: Some(path), sha1: self.sha1.clone(), size: self.size })
    }

    /// Jar de natives a extraer (solo formato legacy).
    pub fn native(&self) -> Option<Artifact> {
        let classifier = self.natives.as_ref()?.get(paths::os_name())?.replace("${arch}", "64");
        self.downloads.as_ref()?.classifiers.as_ref()?.get(&classifier).cloned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    #[serde(default)]
    pub jar: Option<String>,
    #[serde(default)]
    pub main_class: String,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub downloads: Option<Downloads>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
}

impl VersionJson {
    /// Versión cuyo client.jar se usa (la vanilla raíz en versiones heredadas)
    pub fn jar_id(&self) -> String {
        self.jar.clone().unwrap_or_else(|| self.id.clone())
    }
}

/// "group:artifact:version[:classifier][@ext]" -> ruta relativa Maven
pub fn maven_path(coord: &str) -> String {
    let (coord, ext) = coord.split_once('@').unwrap_or((coord, "jar"));
    let p: Vec<&str> = coord.split(':').collect();
    if p.len() < 3 {
        return coord.to_string();
    }
    let (group, artifact, version) = (p[0].replace('.', "/"), p[1], p[2]);
    let file = match p.get(3) {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    format!("{group}/{artifact}/{version}/{file}")
}

/// Clave de deduplicación: misma librería en otra versión -> gana la del hijo
fn lib_key(name: &str) -> String {
    let base = name.split('@').next().unwrap_or(name);
    match base.split(':').collect::<Vec<_>>().as_slice() {
        [g, a, _v, c, ..] => format!("{g}:{a}:{c}"),
        [g, a, ..] => format!("{g}:{a}"),
        _ => name.to_string(),
    }
}

pub fn load(id: &str) -> Result<VersionJson> {
    Ok(serde_json::from_slice(&std::fs::read(paths::version_json(id))?)?)
}

/// Carga una versión aplicando toda su cadena de herencia.
pub fn resolve(id: &str) -> Result<VersionJson> {
    let mut v = load(id)?;
    let Some(parent_id) = v.inherits_from.take() else { return Ok(v) };
    let p = resolve(&parent_id)?;

    let mut seen = HashSet::new();
    v.libraries = std::mem::take(&mut v.libraries)
        .into_iter()
        .chain(p.libraries)
        .filter(|l| seen.insert(lib_key(&l.name)))
        .collect();

    v.arguments = match (p.arguments, v.arguments.take()) {
        (Some(mut base), Some(child)) => {
            base.game.extend(child.game);
            base.jvm.extend(child.jvm);
            Some(base)
        }
        (a, b) => b.or(a),
    };
    v.minecraft_arguments = v.minecraft_arguments.or(p.minecraft_arguments);
    v.downloads = v.downloads.or(p.downloads);
    v.asset_index = v.asset_index.or(p.asset_index);
    v.assets = v.assets.or(p.assets);
    v.java_version = v.java_version.or(p.java_version);
    v.jar = v.jar.or(p.jar).or(Some(p.id));
    if v.main_class.is_empty() {
        v.main_class = p.main_class;
    }
    Ok(v)
}
