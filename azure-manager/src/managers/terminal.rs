// Le terminal du tableau de bord : le manager lance la commande `azure` a
// sa place. Le tableau de bord est enferme (pas d'acces au dossier
// personnel, ni a ~/.local/share/azure) ; le manager, lui, ne l'est pas,
// et la commande lancee est celle installee a cote de lui : une seule
// installation d'Azure.
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Ce que le terminal peut demander a `azure` (pas `setup` : remplacer les
/// daemons depuis l'un d'eux).
pub const ALLOWED: [&str; 7] = ["new", "build", "install", "uninstall", "list", "run", "autostart"];

/// Au-dela, la commande est arretee.
pub const TIMEOUT: Duration = Duration::from_secs(120);
/// `build` : une premiere compilation en release peut etre longue.
pub const BUILD_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Le delai de `azure <sub>`.
pub fn timeout_for(sub: &str) -> Duration {
    if sub == "build" { BUILD_TIMEOUT } else { TIMEOUT }
}

/// La commande `azure` : a cote de l'executable du manager, sinon dans
/// l'installation d'Azure.
pub fn azure_binary() -> PathBuf {
    let beside = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join("azure")));
    beside.filter(|p| p.is_file()).unwrap_or_else(|| azure_provider::install_root().join("bin/azure"))
}

/// `~` et `~/...` -> dossier personnel (pas de shell pour le faire).
pub fn expand_home(arg: &str, home: Option<&Path>) -> String {
    match (arg, home) {
        ("~", Some(home)) => home.to_string_lossy().into_owned(),
        (a, Some(home)) if a.starts_with("~/") => home.join(&a[2..]).to_string_lossy().into_owned(),
        (a, _) => a.to_string(),
    }
}

/// Verifie la sous-commande, puis lance `azure <args>` ; rend son code de
/// sortie et ses lignes (sortie et erreurs, dans l'ordre de lecture).
pub fn run(azure: &Path, args: &[String], timeout: Duration) -> Result<(i32, Vec<String>), String> {
    let sub = args.first().ok_or("commande vide")?;
    if !ALLOWED.contains(&sub.as_str()) {
        return Err(format!("'{sub}' : non disponible ici (possibles : {})", ALLOWED.join(", ")));
    }
    if !azure.is_file() {
        return Err(format!("{} introuvable (azure setup)", azure.display()));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let args: Vec<String> = args.iter().map(|a| expand_home(a, home.as_deref())).collect();
    let spawn = || Command::new(azure).args(&args).current_dir(home.as_deref().unwrap_or(Path::new("/"))).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    // « Text file busy » : l'executable vient d'etre ecrit (installation) et
    // un autre processus, entre fork et exec, le tient encore ouvert.
    let mut attempt = 0;
    let mut child = loop {
        match spawn() {
            Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) && attempt < 20 => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(10));
            }
            other => break other.map_err(|e| format!("{} : {e}", azure.display()))?,
        }
    };
    // Lues a cote : un tuyau plein bloquerait la commande.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut pipe) = pipe {
                let mut bytes = Vec::new();
                let _ = pipe.read_to_end(&mut bytes);
                text = String::from_utf8_lossy(&bytes).into_owned();
            }
            text
        })
    };
    let out = drain(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let err = drain(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break Some(status),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let mut lines: Vec<String> = [out, err].into_iter().flat_map(|t| t.join().unwrap_or_default().lines().map(str::to_string).collect::<Vec<_>>()).collect();
    let code = match status {
        Some(status) => status.code().unwrap_or(-1),
        None => {
            lines.push(format!("arretee apres {} s", timeout.as_secs()));
            -1
        }
    };
    Ok((code, lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join("azure");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("azure-terminal-{name}-{}", std::process::id()))
    }

    #[test]
    fn tilde() {
        let home = Path::new("/home/ada");
        assert_eq!(expand_home("~", Some(home)), "/home/ada");
        assert_eq!(expand_home("~/Dev/note", Some(home)), "/home/ada/Dev/note");
        assert_eq!(expand_home("a~/b", Some(home)), "a~/b");
        assert_eq!(expand_home("~/x", None), "~/x");
    }

    #[test]
    fn runs_and_captures() {
        let azure = script(&dir("ok"), "echo \"args: $*\"; echo oups >&2; exit 3");
        let (code, lines) = run(&azure, &["list".into()], TIMEOUT).unwrap();
        assert_eq!(code, 3);
        assert_eq!(lines, vec!["args: list".to_string(), "oups".to_string()]);
    }

    #[test]
    fn refuses_other_commands() {
        let azure = script(&dir("refus"), "echo non");
        assert!(run(&azure, &["setup".into()], TIMEOUT).unwrap_err().contains("setup"));
        assert!(run(&azure, &[], TIMEOUT).is_err());
    }

    #[test]
    fn stops_after_timeout() {
        let azure = script(&dir("lent"), "exec sleep 5");
        let (code, lines) = run(&azure, &["list".into()], Duration::from_millis(200)).unwrap();
        assert_eq!(code, -1);
        assert!(lines.last().unwrap().contains("arretee"));
    }
}
