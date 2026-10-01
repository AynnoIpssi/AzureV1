// Ou vivent les sockets des daemons d'Azure : un dossier PRIVE a
// l'utilisateur, jamais un nom fixe dans /tmp (qu'un autre compte pourrait
// occuper avant les daemons, et qu'une app enfermee pourrait remplacer).
//
//   $AZURE_RUNTIME_DIR                  si defini (tests, installations a part)
//   $XDG_RUNTIME_DIR/azure              cas normal (session graphique)
//   /tmp/azure-<uid>                    sans session (serveur, CI)
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub fn runtime_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("AZURE_RUNTIME_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir).join("azure");
    }
    // SAFETY : getuid n'echoue jamais.
    PathBuf::from(format!("/tmp/azure-{}", unsafe { libc::getuid() }))
}

/// Chemin du socket `name` (« router », « manager »...).
pub fn socket(name: &str) -> String {
    runtime_dir().join(format!("{name}.sock")).to_string_lossy().into_owned()
}

/// A appeler par un daemon avant `bind` : cree le dossier du socket (700) et
/// refuse un dossier qui n'est pas a nous ou que d'autres peuvent modifier.
pub fn prepare_socket_dir(socket: &str) -> Result<(), String> {
    let Some(dir) = Path::new(socket).parent() else { return Ok(()) };
    if dir.as_os_str().is_empty() || dir == Path::new("/tmp") {
        return Ok(());
    }
    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| format!("{} : {e}", dir.display()))?;
    }
    let meta = std::fs::symlink_metadata(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    // SAFETY : getuid n'echoue jamais.
    let uid = unsafe { libc::getuid() };
    if !meta.is_dir() || meta.uid() != uid {
        return Err(format!("{} n'appartient pas a cet utilisateur : refuse (usurpation ?)", dir.display()));
    }
    if meta.mode() & 0o022 != 0 {
        return Err(format!("{} est modifiable par d'autres comptes : refuse", dir.display()));
    }
    Ok(())
}
