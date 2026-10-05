// `azure setup` : installe les daemons d'Azure (et la commande `azure`)
// depuis un dossier de build, et le tableau de bord comme app.
use crate::install::{install, ManagerAccess};
use crate::install_binary;
use crate::paths::Paths;
use std::path::Path;

/// Ce qu'Azure fait tourner en arriere-plan, plus ses commandes.
pub const SYSTEM_BINARIES: &[&str] = &["azure", "azure_provider", "routeur_daemon", "stockage_daemon", "service_daemon", "manager_daemon", "azure_manager"];

/// Copie les binaires d'Azure de `from` (ex. `target/release`) vers
/// `bin/`, fait le lien `~/.local/bin/azure`, et installe le tableau de
/// bord s'il est trouve (`<dossier des apps>/azure-dashboard`, compile).
/// Retourne les remarques a afficher.
pub fn setup(paths: &Paths, from: &Path, manager: ManagerAccess) -> Result<Vec<String>, String> {
    let missing: Vec<&str> = SYSTEM_BINARIES.iter().copied().filter(|name| !from.join(name).is_file()).collect();
    if !missing.is_empty() {
        return Err(format!("{} : manquent {} (cargo build --release d'abord)", from.display(), missing.join(", ")));
    }
    let bin = paths.bin();
    std::fs::create_dir_all(&bin).map_err(|e| format!("{} : {e}", bin.display()))?;
    for name in SYSTEM_BINARIES {
        install_binary(&from.join(name), &bin.join(name))?;
    }
    let mut notes = vec![format!("{} binaires installes dans {}", SYSTEM_BINARIES.len(), bin.display())];

    std::fs::create_dir_all(&paths.local_bin).map_err(|e| format!("{} : {e}", paths.local_bin.display()))?;
    let link = paths.local_bin.join("azure");
    if link.symlink_metadata().is_ok() {
        std::fs::remove_file(&link).map_err(|e| format!("{} : {e}", link.display()))?;
    }
    std::os::unix::fs::symlink(bin.join("azure"), &link).map_err(|e| format!("{} : {e}", link.display()))?;
    notes.push(format!("{} -> {}", link.display(), bin.join("azure").display()));

    // Le projet : `from` etant `<projet>/target/<profil>`. Note pour
    // `azure new`, s'il contient les sources d'Azure.
    let project = from.canonicalize().ok().and_then(|f| f.ancestors().nth(2).map(Path::to_path_buf));
    if let Some(project) = project.as_ref().filter(|p| p.join("azure-foundation/Cargo.toml").is_file()) {
        let file = crate::new::source_file(paths);
        std::fs::write(&file, project.to_string_lossy().as_bytes()).map_err(|e| format!("{} : {e}", file.display()))?;
        notes.push(format!("sources d'Azure notees : {}", project.display()));
    }
    // Le tableau de bord : une app comme les autres, dans le dossier des
    // apps. Son binaire : celui de sa compilation (`target-dir`), sinon
    // `from`.
    let dashboard = crate::new::apps_dir(paths).ok().map(|apps| apps.join("azure-dashboard")).filter(|d| d.join("app.azure").is_file());
    let binary = dashboard.as_ref().and_then(|dir| crate::install::find_app_binary(dir, "azure_dashboard")).or_else(|| Some(from.join("azure_dashboard")).filter(|b| b.is_file()));
    match (dashboard, binary) {
        (Some(dir), Some(binary)) => {
            let (app, more) = install(paths, &dir, Some(&binary), manager)?;
            notes.push(format!("tableau de bord installe : {}", app.exe.display()));
            notes.extend(more);
        }
        (Some(_), None) => notes.push("tableau de bord pas encore compile : azure build dashboard --installer".to_string()),
        (None, _) => notes.push("tableau de bord non trouve (azure dossier <dossier des apps>, puis azure build dashboard --installer)".to_string()),
    }
    Ok(notes)
}
