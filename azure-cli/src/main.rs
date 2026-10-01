// azure : installer et lancer Azure et ses apps.
//
//   azure setup [--from <dossier>]      installe les daemons (defaut : dossier de cette commande)
//   azure new <nom> [--titre <titre>] [--dans <dossier>] [--azure <sources>]
//   azure build <dossier | nom> [--installer]
//   azure install <dossier> [--bin <exe>]
//   azure uninstall <app>
//   azure list
//   azure run <app> [arguments...]
//   azure autostart on [--now] | off | status
use azure_cli::install::{self, ManagerAccess};
use azure_cli::paths::Paths;
use azure_cli::{autostart, setup};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("azure : {e}");
        std::process::exit(1);
    }
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

/// `--manager <socket>`, `--sans-manager`, sinon le daemon par defaut
/// (joint sans rien lancer pour `setup`, lance si besoin pour `install`).
fn manager_access(args: &[String], launch: bool) -> ManagerAccess {
    match option(args, "--manager") {
        Some(socket) => ManagerAccess::At(socket),
        None if args.iter().any(|a| a == "--sans-manager") => ManagerAccess::None,
        None if launch => ManagerAccess::Default,
        None => ManagerAccess::At(azure_manager::SOCKET_PATH.to_string()),
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let paths = Paths::from_env();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["setup", ..] => {
            let from = option(args, "--from").map(PathBuf::from).or_else(|| std::env::current_exe().ok().and_then(|e| e.parent().map(PathBuf::from))).ok_or("dossier des binaires inconnu")?;
            for note in setup::setup(&paths, &from, manager_access(args, false))? {
                println!("{note}");
            }
            println!("Ensuite : `azure autostart on` pour demarrer Azure a chaque connexion.");
            Ok(())
        }
        ["new", name, ..] => {
            let (title, parent, azure) = (option(args, "--titre"), option(args, "--dans").map(PathBuf::from), option(args, "--azure").map(PathBuf::from));
            let (_, notes) = azure_cli::new::new_app(&paths, name, title.as_deref(), parent.as_deref(), azure.as_deref())?;
            for note in notes {
                println!("{note}");
            }
            Ok(())
        }
        ["build", target, ..] => {
            let then_install = args.iter().any(|a| a == "--installer");
            for note in azure_cli::build::build_app(&paths, target, then_install, manager_access(args, true))? {
                println!("{note}");
            }
            Ok(())
        }
        ["install", source, ..] => {
            let manager = manager_access(args, true);
            let binary = option(args, "--bin").map(PathBuf::from);
            let (app, notes) = install::install(&paths, &PathBuf::from(source), binary.as_deref(), manager)?;
            println!("{} ({} {}) installee : {}", app.title, app.name, app.version, app.exe.display());
            for note in notes {
                println!("  {note}");
            }
            Ok(())
        }
        ["uninstall", name] => {
            install::uninstall(&paths, name)?;
            println!("{name} desinstallee (son stockage est garde)");
            Ok(())
        }
        ["list"] => {
            let apps = install::list(&paths);
            if apps.is_empty() {
                println!("Aucune app installee.");
            }
            for app in apps {
                println!("{:<20} {:<24} {:<8} {}", app.name, app.title, app.version, app.exe.display());
            }
            Ok(())
        }
        ["run", name, rest @ ..] => {
            let rest: Vec<String> = rest.iter().map(|s| s.to_string()).collect();
            let pid = install::run(&paths, name, &rest)?;
            println!("{name} lancee (pid {pid}, journal {})", paths.logs.join(format!("{name}.log")).display());
            Ok(())
        }
        ["autostart", "on", ..] => {
            for note in autostart::enable(&paths, args.iter().any(|a| a == "--now"))? {
                println!("{note}");
            }
            Ok(())
        }
        ["autostart", "off"] => {
            autostart::disable(&paths)?;
            println!("demarrage a la connexion desactive");
            Ok(())
        }
        ["autostart", "status"] | ["autostart"] => {
            println!("{}", autostart::status(&paths));
            Ok(())
        }
        _ => Err("usage : azure setup | new <nom> [--titre <titre>] [--dans <dossier>] | build <dossier|nom> [--installer] | install <dossier> [--bin <exe>] | uninstall <app> | list | run <app> | autostart on [--now]|off|status".to_string()),
    }
}
