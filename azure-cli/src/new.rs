// `azure new <nom>` : cree une app vide, prete a compiler puis installer.
//
// ```text
// <projet>/azure-<nom>/
//   Cargo.toml        depend d'azure-foundation (chemin vers les sources d'Azure)
//   app.azure         manifeste : nom, fenetre, une page
//   src/main.rs       azure_app!() -> fenetre
//   ui/accueil.rsh    la page d'accueil
//   ui/app.rsc        son style (theme d'Azure)
// ```
//
// Les sources d'Azure (le workspace qui contient azure-foundation) sont
// notees par `azure setup` dans `<racine>/source`. Creee dans ce workspace,
// l'app y est ajoutee (`members`).
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

/// `meteo-locale` -> `Meteo locale`.
pub fn default_title(name: &str) -> String {
    let text = name.replace('-', " ");
    let mut chars = text.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// Cree l'app `name` dans `parent` (sinon dans les sources d'Azure).
/// Retourne son dossier et les remarques a afficher.
pub fn new_app(paths: &Paths, name: &str, title: Option<&str>, parent: Option<&Path>, azure: Option<&Path>) -> Result<(PathBuf, Vec<String>), String> {
    check_app_name(name)?;
    let source = azure_source(paths, azure)?;
    let parent = match parent {
        Some(dir) => dir.canonicalize().map_err(|e| format!("{} : {e}", dir.display()))?,
        None => source.clone(),
    };
    let crate_name = format!("azure-{}", name.strip_prefix("azure-").unwrap_or(name));
    let dir = parent.join(&crate_name);
    if dir.exists() {
        return Err(format!("{} existe deja", dir.display()));
    }
    let title = title.map(str::to_string).unwrap_or_else(|| default_title(name));
    let exec = crate_name.replace('-', "_");
    let in_workspace = parent == source;
    let foundation = if in_workspace { "../azure-foundation".to_string() } else { source.join("azure-foundation").to_string_lossy().into_owned() };

    let files: [(&str, String); 5] = [
        ("Cargo.toml", format!("[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"{exec}\"\npath = \"src/main.rs\"\n\n[dependencies]\nazure-foundation = {{ path = \"{foundation}\" }}\n")),
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
    if in_workspace {
        add_member(&source.join("Cargo.toml"), &crate_name)?;
        notes.push(format!("ajoutee au workspace ({})", source.join("Cargo.toml").display()));
        notes.push(format!("ensuite : azure build {name} --installer"));
    } else {
        notes.push(format!("ensuite : azure build {} --installer", dir.display()));
    }
    Ok((dir, notes))
}

/// Ajoute `member` a `members = [...]` du Cargo.toml du workspace.
fn add_member(cargo: &Path, member: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(cargo).map_err(|e| format!("{} : {e}", cargo.display()))?;
    let start = text.find("members = [").ok_or_else(|| format!("{} : pas de `members = [`", cargo.display()))?;
    let end = start + text[start..].find(']').ok_or_else(|| format!("{} : `members` non ferme", cargo.display()))?;
    if text[start..end].contains(&format!("\"{member}\"")) {
        return Ok(());
    }
    let text = format!("{}    \"{member}\",\n{}", &text[..end], &text[end..]);
    std::fs::write(cargo, text).map_err(|e| format!("{} : {e}", cargo.display()))
}
