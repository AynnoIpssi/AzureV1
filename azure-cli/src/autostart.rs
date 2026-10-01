// Demarrer Azure a l'ouverture de session : un service systemd utilisateur
// qui lance azure-provider (qui lance a son tour tous les daemons).
use crate::paths::Paths;
use std::process::Command;

pub const UNIT: &str = "azure-provider.service";

/// `systemctl` (ou `$AZURE_SYSTEMCTL`, pour les tests).
fn systemctl(args: &[&str]) -> Result<String, String> {
    let program = std::env::var("AZURE_SYSTEMCTL").unwrap_or_else(|_| "systemctl".to_string());
    let out = Command::new(&program).arg("--user").args(args).output().map_err(|e| format!("{program} : {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() { Ok(text) } else { Err(format!("systemctl --user {} : {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim())) }
}

pub fn unit_text(paths: &Paths) -> String {
    format!(
        "[Unit]\nDescription=Azure : processus d'arriere-plan (azure-provider)\n\n[Service]\nExecStart={}\nRestart=on-failure\nRestartSec=2\n\n[Install]\nWantedBy=default.target\n",
        paths.bin().join("azure_provider").display()
    )
}

/// Active le demarrage a la connexion. `now` : bascule tout de suite (le
/// provider en cours, lance par une app, est arrete puis relance par
/// systemd).
pub fn enable(paths: &Paths, now: bool) -> Result<Vec<String>, String> {
    if !paths.bin().join("azure_provider").is_file() {
        return Err(format!("{} absent : lancez `azure setup` d'abord", paths.bin().join("azure_provider").display()));
    }
    std::fs::create_dir_all(&paths.systemd).map_err(|e| format!("{} : {e}", paths.systemd.display()))?;
    let unit = paths.systemd.join(UNIT);
    std::fs::write(&unit, unit_text(paths)).map_err(|e| format!("{} : {e}", unit.display()))?;
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", UNIT])?;
    let mut notes = vec![format!("{} active : Azure demarrera a la prochaine connexion", unit.display())];
    if now {
        if let Ok(mut provider) = azure_provider::ProviderClient::connect() {
            let _ = provider.shutdown();
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        systemctl(&["start", UNIT])?;
        notes.push("demarre maintenant par systemd".to_string());
    }
    Ok(notes)
}

pub fn disable(paths: &Paths) -> Result<(), String> {
    let unit = paths.systemd.join(UNIT);
    if unit.exists() {
        let _ = systemctl(&["disable", UNIT]);
        std::fs::remove_file(&unit).map_err(|e| format!("{} : {e}", unit.display()))?;
        systemctl(&["daemon-reload"])?;
    }
    Ok(())
}

/// "active", "desactive"...
pub fn status(paths: &Paths) -> String {
    if !paths.systemd.join(UNIT).exists() {
        return "desactive".to_string();
    }
    let enabled = systemctl(&["is-enabled", UNIT]).unwrap_or_else(|_| "inconnu".into());
    let active = systemctl(&["is-active", UNIT]).unwrap_or_else(|_| "inactif".into());
    format!("installe ({enabled}, {active})")
}
