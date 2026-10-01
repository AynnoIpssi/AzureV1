// Verification de bout en bout (voir scripts/verification-e2e.sh) : une
// app sans fenetre qui partage un flux persistant, sert une methode,
// utilise son stockage, recoit les messages envoyes pendant son absence,
// et verifie qu'elle est bien enfermee. Chaque resultat : une ligne `E2E`.
use azure_foundation::app::AzureApp;
use azure_foundation::flux::Value;
use azure_foundation::navigation::managers::navigation_manager;
use std::time::Duration;

fn main() {
    let app = match AzureApp::find(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/e2e/boutique")) {
        Ok(app) => app,
        Err(e) => {
            println!("E2E boutique ERREUR demarrage : {e}");
            std::process::exit(1);
        }
    };
    // La tache de fond `reveil` : lancee par Azure au premier appel de
    // `taxe`, sans que personne n'ouvre la boutique.
    if std::env::args().any(|a| a == "--reveil") {
        let _taxe = app.serve("taxe", |_| Ok(Value::Int(20)));
        println!("E2E boutique reveillee sert-taxe={} enfermee={}", _taxe.is_ok(), azure_core_sandboxed());
        loop {
            std::thread::park();
        }
    }
    println!("E2E boutique id={} installee={}", app.id(), azure_foundation::app::is_installed(app.manifest()));
    println!("E2E boutique enfermee={}", azure_core_sandboxed());

    // Messages envoyes pendant son absence (boite aux lettres du routeur).
    match navigation_manager::connect(app.id()) {
        Ok(nav) => {
            std::thread::spawn(move || {
                let mut nav = nav;
                while let Ok(message) = navigation_manager::receive(&mut nav) {
                    println!("E2E boutique message={message}");
                }
            });
        }
        Err(e) => println!("E2E boutique ERREUR routeur : {e}"),
    }

    match app.stockage().and_then(|s| s.update("lancements", 0i64, |n: i64| n + 1)) {
        Ok(n) => println!("E2E boutique stockage lancements={n}"),
        Err(e) => println!("E2E boutique ERREUR stockage : {e}"),
    }

    let _panier = match app.share("panier") {
        Ok(mut panier) => {
            let _ = panier.set("total", 42);
            let _ = panier.push("items", "pomme");
            println!("E2E boutique flux panier seq={}", panier.seq());
            Some(panier)
        }
        Err(e) => {
            println!("E2E boutique ERREUR flux : {e}");
            None
        }
    };
    let _prix = app.serve("prix", |d| d.args.get("produit").and_then(Value::as_i64).map(|p| Value::Int(p * 3)).ok_or_else(|| "produit attendu".to_string()));
    println!("E2E boutique sert prix={}", _prix.is_ok());

    // Le bac a sable : la cle maitre et les fichiers d'Azure sont hors de portee.
    let root = azure_provider::install_root();
    let cle = std::fs::read(root.join("stockage/master.key"));
    println!("E2E boutique cle-maitre-lisible={}", cle.is_ok());
    let config = std::fs::write(root.join("../../config/azure/provider.conf"), "[pirate]\ncommand = /bin/sh\n");
    println!("E2E boutique config-modifiable={}", config.is_ok());
    let reseau = std::net::TcpStream::connect_timeout(&"1.1.1.1:80".parse().unwrap(), Duration::from_secs(2));
    println!("E2E boutique reseau={}", reseau.is_ok());
    // Le bus de session (systemd --user lancerait n'importe quoi) : hors d'atteinte.
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        let bus = std::path::Path::new(&runtime).join("bus");
        println!("E2E boutique bus-joignable={}", std::os::unix::net::UnixStream::connect(&bus).is_ok());
    }
    // X11 (XWayland) : hors d'atteinte aussi.
    let x11 = ["/tmp/.X11-unix/X0", "/tmp/.X11-unix/X1"].iter().find(|p| std::path::Path::new(p).exists() || std::fs::symlink_metadata(p).is_ok());
    println!("E2E boutique x11-joignable={}", x11.is_some_and(|p| std::os::unix::net::UnixStream::connect(p).is_ok()));
    // Lancee par `azure run` : processus 1 de ses propres espaces de noms.
    // SAFETY : getpid n'echoue jamais.
    println!("E2E boutique pid-isole={}", unsafe { libc_getpid() } == 1);
    // Le provider : l'etat oui, lui faire lancer une commande non.
    let pirate = azure_provider::Service::new("pirate", "/bin/sh").register();
    println!("E2E boutique provider-pilotable={}", pirate.is_ok());
    // Les daemons : pas de signal (meme 0, qui ne fait que tester).
    let pids: Vec<u32> = azure_provider::Provider::status().map(|s| s.iter().filter_map(|s| s.pid).collect()).unwrap_or_default();
    let signal = pids.iter().any(|pid| std::process::Command::new("kill").args(["-0", &pid.to_string()]).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success()));
    println!("E2E boutique daemons={} signal-daemon={signal}", pids.len());

    app.info("boutique prete");
    std::thread::sleep(Duration::from_secs(std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10)));
    println!("E2E boutique fin");
}

unsafe extern "C" {
    #[link_name = "getpid"]
    fn libc_getpid() -> i32;
}

fn azure_core_sandboxed() -> bool {
    azure_core::security::sandbox::is_sandboxed()
}
