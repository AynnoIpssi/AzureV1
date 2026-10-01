// Durcissement des daemons d'Azure.
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// A appeler au demarrage de chaque daemon : un autre processus du meme
/// utilisateur ne peut plus lire sa memoire (ptrace, /proc/<pid>/mem) ni
/// obtenir un vidage memoire, et ce qu'il cree n'est lisible que par
/// l'utilisateur.
pub fn harden_daemon() {
    // SAFETY : appels systeme simples, sans pointeur.
    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
        libc::umask(0o077);
    }
}

/// Cree `dir` (et ses parents) et le reserve a l'utilisateur (droits 700).
pub fn private_dir(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| format!("{} : {e}", dir.display()))
}

/// Ce processus peut-il etre inspecte par un autre ? (`false` apres
/// `harden_daemon`.)
pub fn is_dumpable() -> bool {
    // SAFETY : lecture d'un attribut du processus.
    unsafe { libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) == 1 }
}
