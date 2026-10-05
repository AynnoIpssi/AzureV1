// `azure new <nom>` : cree une app vide, prete a compiler puis installer.
//
// ```text
// <dossier des apps>/azure-<nom>/
//   Cargo.toml          depend d'azure-foundation (chemin absolu vers les sources d'Azure)
//   .cargo/config.toml  compile dans le dossier partage (~/.cache/azure/target)
//   app.azure           manifeste : nom, fenetre, une page
//   src/main.rs         azure_app!() -> fenetre
//   ui/accueil.rsh      la page d'accueil
//   ui/app.rsc          son style (theme d'Azure)
// ```
//
// Les apps ne vivent JAMAIS dans les sources d'Azure : elles vont dans le
// dossier des apps (`azure dossier <chemin>`, note dans `<racine>/dossier-apps`)
// ou la ou `--dans` le dit. Les sources d'Azure (le workspace qui contient
// azure-foundation) sont notees par `azure setup` dans `<racine>/source`.
use crate::paths::Paths;
use azure_manager::models::manifest::check_app_name;
use std::path::{Path, PathBuf};

/// Le fichier ou `azure setup` note les sources d'Azure.
pub fn source_file(paths: &Paths) -> PathBuf {
    paths.root.join("source")
}

/// Le workspace d'Azure : `--azure` s'il est donne, sinon celui note par
/// `azure setup`.
pub fn azure_source(paths: &Paths, given: Option<&Path>) -> Result<PathBuf, String> {
    let dir = match given {
        Some(dir) => dir.to_path_buf(),
        None => {
            let text = std::fs::read_to_string(source_file(paths)).map_err(|_| "sources d'Azure inconnues : donnez --azure <dossier du projet Azure> (ou relancez azure setup depuis son target/release)".to_string())?;
            PathBuf::from(text.trim())
        }
    };
    if !dir.join("azure-foundation/Cargo.toml").is_file() {
        return Err(format!("{} : pas de azure-foundation ici (ce n'est pas le projet Azure)", dir.display()));
    }
    dir.canonicalize().map_err(|e| format!("{} : {e}", dir.display()))
}

/// Le fichier ou `azure dossier` note le dossier des apps.
pub fn apps_dir_file(paths: &Paths) -> PathBuf {
    paths.root.join("dossier-apps")
}

/// Le dossier des apps (`azure dossier`).
pub fn apps_dir(paths: &Paths) -> Result<PathBuf, String> {
    let text = std::fs::read_to_string(apps_dir_file(paths)).map_err(|_| "dossier des apps inconnu : choisissez-le avec `azure dossier <chemin>` (ou donnez --dans <dossier>)".to_string())?;
    Ok(PathBuf::from(text.trim()))
}

/// Refuse un dossier d'apps dans les sources d'Azure (s'il les connait).
fn outside_azure(paths: &Paths, dir: &Path) -> Result<(), String> {
    if let Ok(source) = azure_source(paths, None)
        && dir.starts_with(&source)
    {
        return Err(format!("{} : dans les sources d'Azure ({}) ; les apps se rangent ailleurs", dir.display(), source.display()));
    }
    Ok(())
}

/// `azure dossier <chemin>` : note le dossier des apps (cree s'il manque).
pub fn set_apps_dir(paths: &Paths, dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    let dir = dir.canonicalize().map_err(|e| format!("{} : {e}", dir.display()))?;
    outside_azure(paths, &dir)?;
    std::fs::create_dir_all(&paths.root).map_err(|e| format!("{} : {e}", paths.root.display()))?;
    let file = apps_dir_file(paths);
    std::fs::write(&file, dir.to_string_lossy().as_bytes()).map_err(|e| format!("{} : {e}", file.display()))?;
    Ok(dir)
}

/// Reglages d'optimisation d'Azure, repris dans le Cargo.toml de chaque app
/// (hors du workspace d'Azure, ceux du workspace ne s'appliquent plus) : le
/// rendu pixel par pixel est ~10x plus lent sans optimisation, meme en
/// `cargo run`.
pub const OPT_PROFILES: &str = "\n# Le moteur d'Azure est optimise meme en `cargo run` (rendu ~10x plus\n# lent sinon) ; le code de l'app reste compile normalement.\n[profile.dev.package.azure-engine]\nopt-level = 3\n\n[profile.dev.package.azure-foundation]\nopt-level = 3\n\n[profile.dev.package.ttf-parser]\nopt-level = 3\n\n[profile.dev.package.azure-stockage]\nopt-level = 3\n\n[profile.dev.package.azure-core]\nopt-level = 3\n";

/// `.cargo/config.toml` d'une app : compilation dans le dossier partage.
pub fn cargo_config(paths: &Paths) -> String {
    format!("# Compilation partagee par toutes les apps Azure (azure build la retrouve\n# la) : sans elle, chaque app recompilerait tout le moteur d'Azure.\n[build]\ntarget-dir = \"{}\"\n", paths.shared_target().display())
}

/// `meteo-locale` -> `Meteo locale`.
pub fn default_title(name: &str) -> String {
    let text = name.replace('-', " ");
    let mut chars = text.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// Cree l'app `name` dans `parent` (sinon dans le dossier des apps).
/// Retourne son dossier et les remarques a afficher.
pub fn new_app(paths: &Paths, name: &str, title: Option<&str>, parent: Option<&Path>, azure: Option<&Path>) -> Result<(PathBuf, Vec<String>), String> {
    check_app_name(name)?;
    let source = azure_source(paths, azure)?;
    let parent = match parent {
        Some(dir) => dir.canonicalize().map_err(|e| format!("{} : {e}", dir.display()))?,
        None => apps_dir(paths)?,
    };
    outside_azure(paths, &parent)?;
    if parent.starts_with(&source) {
        return Err(format!("{} : dans les sources d'Azure ; les apps se rangent ailleurs", parent.display()));
    }
    let crate_name = format!("azure-{}", name.strip_prefix("azure-").unwrap_or(name));
    let dir = parent.join(&crate_name);
    if dir.exists() {
        return Err(format!("{} existe deja", dir.display()));
    }
    let title = title.map(str::to_string).unwrap_or_else(|| default_title(name));
    let exec = crate_name.replace('-', "_");
    let foundation = source.join("azure-foundation").to_string_lossy().into_owned();

    let files: [(&str, String); 6] = [
        ("Cargo.toml", format!("[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"{exec}\"\npath = \"src/main.rs\"\n\n[dependencies]\nazure-foundation = {{ path = \"{foundation}\" }}\n{OPT_PROFILES}")),
        (".cargo/config.toml", cargo_config(paths)),
        ("app.azure", format!("# {title} : app Azure (creee par `azure new`).\n[app]\nname = {name}\ntitle = {title}\nversion = 0.1\nexec = {exec}\nfiles = ui\n\n[window]\ntitle = {title}\nwidth = 900\nheight = 600\nstart = /\nstyle = ui/app.rsc\n\n[route /]\nview = ui/accueil.rsh\n")),
        ("src/main.rs", format!("// {title} : app Azure. Manifeste : app.azure ; pages : ui/.\nuse std::time::Duration;\n\nfn main() {{\n    if let Err(e) = run() {{\n        eprintln!(\"{name} : {{e}}\");\n        std::process::exit(1);\n    }}\n}}\n\nfn run() -> Result<(), String> {{\n    // Le manifeste : a cote de l'executable une fois installee, sinon ce dossier.\n    let app = azure_foundation::azure_app!()?;\n    app.window()?\n        .on_click(|ctx| {{\n            if ctx.clicked == Some(\"bonjour\") {{\n                ctx.flash(\"bonjour\", \"Bonjour !\", Duration::from_millis(1200));\n            }}\n        }})\n        .run();\n    Ok(())\n}}\n")),
        ("ui/accueil.rsh", format!("<!-- Page d'accueil de {title}. -->\n<container.app>\n    <title2.h>{title}<!title2>\n    <text.lead>Une app Azure toute neuve. Modifiez ui/accueil.rsh et ui/app.rsc, puis src/main.rs pour les clics.<!text>\n    <button.btn#bonjour>Dire bonjour<!button>\n<!container>\n")),
        ("ui/app.rsc", "/* Theme d'Azure : gris chauds, accent sable. */\n.app { display: flex; flex-direction: column; gap: 12px; height: 100%; padding: 32px 40px; background-color: #121212; color: #c9c6bf; font-size: 14px; }\n.h { color: #f0eeea; font-size: 26px; font-weight: 700; }\n.lead { color: #8a877f; font-size: 14px; }\n.btn { align-self: flex-start; padding: 8px 16px; background-color: #1e1e1e; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; color: #e7e5e1; font-size: 13px; font-weight: 500; }\n.btn:hover { background-color: #26241f; }\n.btn:active { background-color: #c9a878; color: #1a1712; }\n".to_string()),
    ];
    for (file, text) in &files {
        let path = dir.join(file);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
        }
        std::fs::write(&path, text).map_err(|e| format!("{} : {e}", path.display()))?;
    }

    let mut notes = vec![format!("{title} ({name}) creee : {}", dir.display())];
    let by_name = apps_dir(paths).is_ok_and(|apps| apps == parent);
    notes.push(format!("ensuite : azure build {} --installer", if by_name { name.to_string() } else { dir.display().to_string() }));
    Ok((dir, notes))
}
