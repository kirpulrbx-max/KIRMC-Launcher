use md5::{Digest, Md5};

use crate::error::{Error, Result};

/// UUID v3 idéntico al de Java `UUID.nameUUIDFromBytes("OfflinePlayer:<name>")`,
/// el mismo que calcula un servidor en `online-mode=false`, así inventarios y permisos coinciden.
pub fn offline_uuid(name: &str) -> String {
    let mut h: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;
    hex::encode(h)
}

pub fn validate(name: &str) -> Result<()> {
    let ok = (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ok {
        Ok(())
    } else {
        Err(Error::Msg("El nombre debe tener 3-16 caracteres (letras, números o _)".into()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_deterministic_uuid_v3() {
        let a = super::offline_uuid("Kirmc_Player");
        assert_eq!(a, super::offline_uuid("Kirmc_Player"));
        assert_eq!(a.len(), 32);
        assert_eq!(&a[12..13], "3"); // nibble de versión
    }
}
