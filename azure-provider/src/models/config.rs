// Fichier de config du provider (`~/.config/azure/provider.conf`) :
//
// ```text
// # Commentaire
// [stockage]                  <- un daemon d'Azure : ne change que ce qui est donne
// args = --root /mnt/disque/azure
//
// [rooter]
// enabled = false             <- ne plus le surveiller
//
// [notes-sync]                <- un nouveau service
// command = /usr/local/bin/notes-sync
// args = --fast "un argument avec espaces"
// health = /tmp/notes-sync.sock
// restart = always            <- always | on-failure | never
// autostart = true
// ```
use crate::models::service::{builtin, is_builtin, Restart, ServiceSpec};
use std::path::Path;

/// Les services d'Azure, modifies par `text`.
pub fn parse(text: &str) -> Result<Vec<ServiceSpec>, String> {
    parse_onto(text, builtin())
}

/// Les taches de fond d'une app installee (`<install>/services/<app>.conf`,
/// ecrit par `azure install`) : meme format, mais sans toucher aux daemons
/// d'Azure.
pub fn parse_app(text: &str) -> Result<Vec<ServiceSpec>, String> {
    let services = parse_onto(text, Vec::new())?;
    match services.iter().find(|spec| is_builtin(&spec.name)) {
        Some(spec) => Err(format!("'{}' est un daemon d'Azure : il se modifie dans provider.conf", spec.name)),
        None => Ok(services),
    }
}

/// Les taches de fond de toutes les apps installees (`dir/*.conf`), deja
/// connues du provider au demarrage. Un fichier illisible est ignore (et
/// signale), les autres comptent.
pub fn load_apps(dir: &Path) -> Vec<ServiceSpec> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "conf")).collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        match std::fs::read_to_string(&file).map_err(|e| e.to_string()).and_then(|text| parse_app(&text)) {
            Ok(specs) => out.extend(specs),
            Err(e) => eprintln!("azure-provider : {} ignore : {e}", file.display()),
        }
    }
    out
}

fn parse_onto(text: &str, mut services: Vec<ServiceSpec>) -> Result<Vec<ServiceSpec>, String> {
    // Services desactives (`enabled = false`), retires a la fin : une
    // section peut encore les modifier avant.
    let mut disabled: Vec<String> = Vec::new();
    let mut current: Option<usize> = None;

    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let at = |message: String| format!("provider.conf ligne {} : {message}", number + 1);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            let name = name.trim();
            current = Some(match services.iter().position(|spec| spec.name == name) {
                Some(index) => index,
                None => {
                    services.push(ServiceSpec::new(name, ""));
                    services.len() - 1
                }
            });
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| at(format!("'cle = valeur' attendu, trouve '{line}'")))?;
        let (key, value) = (key.trim(), value.trim());
        let index = current.ok_or_else(|| at("cle hors d'une section [service]".to_string()))?;
        let spec = &mut services[index];
        match key {
            "command" => spec.command = value.to_string(),
            "args" => spec.args = split_args(value).map_err(at)?,
            "health" => spec.health = (!value.is_empty()).then(|| value.to_string()),
            "restart" => spec.restart = Restart::from_name(value).ok_or_else(|| at(format!("restart inconnu '{value}' (always, on-failure, never)")))?,
            "autostart" => spec.autostart = parse_bool(value).map_err(at)?,
            "enabled" => {
                if parse_bool(value).map_err(at)? {
                    disabled.retain(|name| name != &spec.name);
                } else {
                    disabled.push(spec.name.clone());
                }
            }
            other => return Err(at(format!("cle inconnue '{other}'"))),
        }
    }

    services.retain(|spec| !disabled.contains(&spec.name));
    for spec in &services {
        spec.validate().map_err(|e| format!("provider.conf : {e}"))?;
    }
    Ok(services)
}

/// Lit le fichier ; absent = les services d'Azure tels quels.
pub fn load(path: Option<&Path>) -> Result<Vec<ServiceSpec>, String> {
    match path {
        Some(path) if path.exists() => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{} : {e}", path.display()))?;
            parse(&text)
        }
        _ => Ok(builtin()),
    }
}

pub fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" | "yes" | "oui" => Ok(true),
        "false" | "no" | "non" => Ok(false),
        _ => Err(format!("true ou false attendu, trouve '{value}'")),
    }
}

/// Decoupe aux espaces ; `"..."` garde un argument avec espaces.
pub fn split_args(value: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut has_arg = false;
    for c in value.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                has_arg = true;
            }
            c if c.is_whitespace() && !quoted => {
                if has_arg {
                    args.push(std::mem::take(&mut current));
                    has_arg = false;
                }
            }
            c => {
                current.push(c);
                has_arg = true;
            }
        }
    }
    if quoted {
        return Err("guillemet non ferme".to_string());
    }
    if has_arg {
        args.push(current);
    }
    Ok(args)
}
