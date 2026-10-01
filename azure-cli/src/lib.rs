// La commande `azure` : installer Azure et ses apps, les lancer, demarrer
// Azure a l'ouverture de session.
//
// ```text
// ~/.local/share/azure/bin/                daemons d'Azure + `azure` (azure setup)
// ~/.local/share/azure/apps/<nom>/         une app : binaire, app.azure, ui/...
// ~/.local/share/applications/azure-<nom>.desktop   son lanceur (menu, dock)
// ~/.local/state/azure/apps/<nom>.log      sa sortie quand `azure run` la lance
// ~/.config/systemd/user/azure-provider.service     demarrage a la connexion
// ~/.local/bin/azure                       lien vers bin/azure (dans le PATH)
// ~/.local/share/azure/source              sources d'Azure (pour `azure new`)
// ```
pub mod autostart;
pub mod build;
pub mod install;
pub mod new;
pub mod paths;
pub mod setup;

/// Copie un fichier ou un dossier (recursif), en gardant les droits.
pub fn copy_all(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    let error = |e: std::io::Error| format!("{} -> {} : {e}", from.display(), to.display());
    if from.is_dir() {
        std::fs::create_dir_all(to).map_err(error)?;
        for entry in std::fs::read_dir(from).map_err(error)? {
            let entry = entry.map_err(error)?;
            copy_all(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(error)?;
        }
        std::fs::copy(from, to).map(|_| ()).map_err(error)
    }
}

/// Copie un executable en le remplacant d'un coup (possible meme s'il
/// tourne : l'ancien reste utilise par le processus en cours).
pub fn install_binary(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let tmp = to.with_extension("azure-tmp");
    copy_all(from, &tmp)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("{} : {e}", tmp.display()))?;
    std::fs::rename(&tmp, to).map_err(|e| format!("{} : {e}", to.display()))
}
