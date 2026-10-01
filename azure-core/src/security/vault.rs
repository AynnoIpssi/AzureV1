// Chiffrement au repos des fichiers d'un daemon (ChaCha20-Poly1305, voir
// `crypto::aead`). La cle est un fichier `cle.bin` (droits 600) dans le
// dossier du daemon, que les apps enfermees ne peuvent pas lire (voir
// `sandbox`). Un fichier ecrit avant le chiffrement est encore lu (en
// clair) puis reecrit chiffre a la sauvegarde suivante.
use crate::crypto::aead::{open, seal};
use crate::crypto::random::random_bytes;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Debut d'un fichier chiffre.
pub const MAGIC: &[u8; 4] = b"AZS1";

#[derive(Clone)]
pub struct Vault {
    key: [u8; 32],
}

fn write_private(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data).map_err(|e| format!("{} : {e}", tmp.display()))?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).map_err(|e| format!("{} : {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{} : {e}", path.display()))
}

impl Vault {
    /// La cle de `dir` (creee au premier usage).
    pub fn open(dir: &Path) -> Result<Vault, String> {
        crate::security::hardening::private_dir(dir)?;
        let path = dir.join("cle.bin");
        let mut key = [0u8; 32];
        match std::fs::read(&path) {
            Ok(data) if data.len() == 32 => key.copy_from_slice(&data),
            Ok(_) => return Err(format!("{} : cle abimee", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                random_bytes(&mut key)?;
                write_private(&path, &key)?;
            }
            Err(e) => return Err(format!("{} : {e}", path.display())),
        }
        Ok(Vault { key })
    }

    /// Cle donnee (tests).
    pub fn with_key(key: [u8; 32]) -> Vault {
        Vault { key }
    }

    /// Ecrit `data` chiffre dans `path` (atomique, droits 600). `label`
    /// lie le contenu a son role : un fichier ne peut pas etre substitue a
    /// un autre.
    pub fn write(&self, path: &Path, label: &str, data: &[u8]) -> Result<(), String> {
        let sealed = seal(&self.key, label.as_bytes(), data)?;
        write_private(path, &[MAGIC.as_slice(), &sealed].concat())
    }

    /// Lit `path` (`None` s'il n'existe pas). Erreur si le fichier a ete
    /// modifie ou chiffre avec une autre cle.
    pub fn read(&self, path: &Path, label: &str) -> Result<Option<Vec<u8>>, String> {
        let data = match std::fs::read(path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{} : {e}", path.display())),
        };
        match data.strip_prefix(MAGIC.as_slice()) {
            Some(sealed) => open(&self.key, label.as_bytes(), sealed).map(Some).map_err(|e| format!("{} : {e}", path.display())),
            // Ecrit avant le chiffrement : lu tel quel.
            None => Ok(Some(data)),
        }
    }
}
