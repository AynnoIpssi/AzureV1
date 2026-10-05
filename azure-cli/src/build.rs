// `azure build <dossier | nom> [--installer]` : compile une app (cargo
// build --release) et, avec `--installer`, l'installe.
use crate::install::{install, ManagerAccess};
use crate::paths::Paths;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Le dossier de l'app : `target` s'il contient app.azure, sinon
/// `azure-<nom>` dans le dossier des apps (voir `azure dossier`).
pub fn app_dir(paths: &Paths, target: &str) -> Result<PathBuf, String> {
    let dir = PathBuf::from(target);
    if dir.join("app.azure").is_file() {
        return Ok(dir);
    }
    if target.contains('/') {
        return Err(format!("{target} : pas de app.azure ici"));
    }
    let apps = crate::new::apps_dir(paths)?;
    let dir = apps.join(format!("azure-{}", target.strip_prefix("azure-").unwrap_or(target)));
    if dir.join("app.azure").is_file() { Ok(dir) } else { Err(format!("'{target}' : ni un dossier d'app, ni {}", dir.display())) }
}

/// `cargo` : `$CARGO`, le `PATH`, puis ~/.cargo/bin (un daemon n'a pas
/// toujours le `PATH` du terminal).
pub fn cargo() -> Result<PathBuf, String> {
    if let Some(cargo) = std::env::var_os("CARGO").map(PathBuf::from).filter(|p| p.is_file()) {
        return Ok(cargo);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let home = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo/bin"));
    std::env::split_paths(&path).chain(home).map(|d| d.join("cargo")).find(|p| p.is_file()).ok_or_else(|| "cargo introuvable (installez Rust : rustup.rs)".to_string())
}

/// Compile l'app de `dir` : `cargo build --release` dans son dossier (dans
/// un workspace, cargo ne compile que ce paquet). Sa sortie passe telle
/// quelle.
pub fn build(dir: &Path) -> Result<(), String> {
    let status = Command::new(cargo()?)
        .args(["build", "--release", "--color", "never"])
        .current_dir(dir)
        // Tout sur la sortie standard : dans l'ordre, pour le terminal du
        // tableau de bord (qui lit la sortie puis les erreurs).
        .stderr(std::process::Stdio::from(std::io::stdout()))
        .status()
        .map_err(|e| format!("cargo : {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("compilation echouee (code {})", status.code().unwrap_or(-1))) }
}

/// `azure build` : compile, puis installe si `then_install`. Retourne les
/// remarques a afficher.
pub fn build_app(paths: &Paths, target: &str, then_install: bool, manager: ManagerAccess) -> Result<Vec<String>, String> {
    let dir = app_dir(paths, target)?;
    build(&dir)?;
    let mut notes = vec![format!("{} compilee", dir.display())];
    if then_install {
        let (app, more) = install(paths, &dir, None, manager)?;
        notes.push(format!("{} ({}) installee : {}", app.title, app.name, app.exe.display()));
        notes.extend(more);
        notes.push(format!("lancez-la : azure run {}", app.name));
    } else {
        notes.push(format!("ensuite : azure install {}", dir.display()));
    }
    Ok(notes)
}
