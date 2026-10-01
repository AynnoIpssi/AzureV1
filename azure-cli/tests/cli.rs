// La commande `azure`, lancee pour de vrai avec des dossiers XDG
// temporaires a la place de ~/.local et ~/.config.
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

struct Home {
    dir: PathBuf,
}

impl Home {
    fn new(test: &str) -> Home {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("azure-cli-{test}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Home { dir }
    }

    fn azure(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_azure"))
            .args(args)
            .env("HOME", &self.dir)
            .env("XDG_DATA_HOME", self.dir.join("share"))
            .env("XDG_CONFIG_HOME", self.dir.join("config"))
            .env("XDG_STATE_HOME", self.dir.join("state"))
            // Jamais les daemons qui tourneraient sur la machine.
            .env("AZURE_RUNTIME_DIR", self.dir.join("run"))
            .env("AZURE_SYSTEMCTL", "/usr/bin/true")
            .output()
            .unwrap()
    }

    fn ok(&self, args: &[&str]) -> String {
        let out = self.azure(args);
        assert!(out.status.success(), "azure {args:?} : {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn err(&self, args: &[&str]) -> String {
        let out = self.azure(args);
        assert!(!out.status.success(), "azure {args:?} aurait du echouer");
        String::from_utf8_lossy(&out.stderr).into_owned()
    }

    fn share(&self) -> PathBuf {
        self.dir.join("share")
    }

    /// Un faux binaire d'app : ecrit `lancee.txt` dans son dossier.
    fn fake_binary(&self, name: &str) -> PathBuf {
        let path = self.dir.join(name);
        std::fs::write(&path, "#!/bin/sh\necho \"lancee $*\" > lancee.txt\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

fn notes_source() -> String {
    format!("{}/tests/notes", env!("CARGO_MANIFEST_DIR"))
}

fn wait_file(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(path) {
            return text;
        }
        assert!(Instant::now() < deadline, "{} jamais cree", path.display());
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn install_run_list_and_uninstall_an_app() {
    let home = Home::new("app");
    let binary = home.fake_binary("notes_app");
    let out = home.ok(&["install", &notes_source(), "--bin", binary.to_str().unwrap(), "--sans-manager"]);
    assert!(out.contains("Mes notes (notes 2.0) installee"), "{out}");

    // Le dossier de l'app : binaire executable, manifeste, pages, icone.
    let app = home.share().join("azure/apps/notes");
    let exe = app.join("notes_app");
    assert!(exe.metadata().unwrap().permissions().mode() & 0o111 != 0);
    for file in ["app.azure", "ui/accueil.rsh", "ui/app.rsc", "icon.png"] {
        assert!(app.join(file).is_file(), "{file} manquant");
    }

    // Son lanceur (menu, dock).
    let desktop = std::fs::read_to_string(home.share().join("applications/azure-notes.desktop")).unwrap();
    assert!(desktop.contains("Name=Mes notes"));
    assert!(desktop.contains(&format!("Exec=\"{}\"", exe.display())));
    assert!(desktop.contains(&format!("Icon={}", app.join("icon.png").display())));
    assert!(desktop.contains("StartupWMClass=azure-notes"), "meme id que la fenetre (AzureApp::window)");

    let list = home.ok(&["list"]);
    assert!(list.starts_with("notes") && list.contains("Mes notes") && list.contains("2.0"), "{list}");

    // Lancee detachee, dans son dossier, avec ses arguments ; journal cree.
    let out = home.ok(&["run", "notes", "--rapide"]);
    assert!(out.contains("notes lancee (pid"), "{out}");
    assert_eq!(wait_file(&app.join("lancee.txt")).trim(), "lancee --rapide");
    assert!(home.dir.join("state/azure/apps/notes.log").exists());

    // Reinstaller remplace tout (l'ancien fichier `lancee.txt` disparait).
    home.ok(&["install", &notes_source(), "--bin", binary.to_str().unwrap(), "--sans-manager"]);
    assert!(!app.join("lancee.txt").exists());

    let out = home.ok(&["uninstall", "notes"]);
    assert!(out.contains("son stockage est garde"));
    assert!(!app.exists() && !home.share().join("applications/azure-notes.desktop").exists());
    assert!(home.ok(&["list"]).contains("Aucune app installee"));
    assert!(home.err(&["run", "notes"]).contains("n'est pas installee"));
}

#[test]
fn install_explains_what_is_missing() {
    let home = Home::new("errors");
    let source = home.dir.join("sans-exec");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("app.azure"), "[app]\nname = vide\n").unwrap();
    assert!(home.err(&["install", source.to_str().unwrap()]).contains("ajoutez `exec = <binaire>`"));

    std::fs::write(source.join("app.azure"), "[app]\nname = vide\nexec = introuvable_xyz\n").unwrap();
    assert!(home.err(&["install", source.to_str().unwrap(), "--sans-manager"]).contains("binaire 'introuvable_xyz' introuvable"));

    std::fs::write(source.join("app.azure"), "[app]\nname = vide\nexec = x\nicon = absente.png\n").unwrap();
    let binary = home.fake_binary("x");
    assert!(home.err(&["install", source.to_str().unwrap(), "--bin", binary.to_str().unwrap()]).contains("absente.png : fichier declare dans app.azure introuvable"));
    assert!(home.err(&["uninstall", "personne"]).contains("n'est pas installee"));
}

#[test]
fn setup_installs_azure_and_the_dashboard_then_autostart() {
    let home = Home::new("setup");
    // Un faux projet : target/release avec les binaires, et le vrai dossier
    // du tableau de bord.
    let project = home.dir.join("projet");
    let release = project.join("target/release");
    std::fs::create_dir_all(&release).unwrap();
    for name in azure_cli::setup::SYSTEM_BINARIES.iter().chain(&["azure_dashboard"]) {
        std::fs::write(release.join(name), "#!/bin/sh\n").unwrap();
    }
    azure_cli::copy_all(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../azure-dashboard/app.azure"), &project.join("azure-dashboard/app.azure")).unwrap();
    azure_cli::copy_all(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../azure-dashboard/ui"), &project.join("azure-dashboard/ui")).unwrap();
    azure_cli::copy_all(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../azure-dashboard/icone.png"), &project.join("azure-dashboard/icone.png")).unwrap();

    assert!(home.err(&["autostart", "on"]).contains("lancez `azure setup` d'abord"));
    let out = home.ok(&["setup", "--from", release.to_str().unwrap(), "--sans-manager"]);
    assert!(out.contains("7 binaires installes"), "{out}");
    assert!(out.contains("tableau de bord installe"), "{out}");
    let bin = home.share().join("azure/bin");
    for name in azure_cli::setup::SYSTEM_BINARIES {
        assert!(bin.join(name).metadata().unwrap().permissions().mode() & 0o111 != 0, "{name}");
    }
    assert_eq!(std::fs::read_link(home.dir.join(".local/bin/azure")).unwrap(), bin.join("azure"));
    let dashboard = home.share().join("azure/apps/dashboard");
    assert!(dashboard.join("azure_dashboard").is_file() && dashboard.join("ui/tableau.rsh").is_file());
    assert!(std::fs::read_to_string(home.share().join("applications/azure-dashboard.desktop")).unwrap().contains("Name=Azure Dashboard"));

    // Demarrage a la connexion : un service systemd utilisateur.
    let out = home.ok(&["autostart", "on"]);
    assert!(out.contains("prochaine connexion"), "{out}");
    let unit = std::fs::read_to_string(home.dir.join("config/systemd/user/azure-provider.service")).unwrap();
    assert!(unit.contains(&format!("ExecStart={}", bin.join("azure_provider").display())));
    assert!(unit.contains("Restart=on-failure") && unit.contains("WantedBy=default.target"));
    assert!(home.ok(&["autostart", "status"]).starts_with("installe"));
    home.ok(&["autostart", "off"]);
    assert!(!home.dir.join("config/systemd/user/azure-provider.service").exists());
    assert_eq!(home.ok(&["autostart", "status"]).trim(), "desactive");
}

#[test]
fn installing_registers_the_identity_and_keeps_the_id() {
    let home = Home::new("identity");
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-cli-test-manager-{}.sock"), std::process::id());
    let config = azure_manager::managers::daemon::ManagerConfig {
        socket: socket.clone(),
        data_dir: home.dir.join("manager"),
        service_socket: "/tmp/aucun-service.sock".into(),
        provider_socket: "/tmp/aucun-provider.sock".into(),
        admins: vec![env!("CARGO_BIN_EXE_azure").to_string()],
        provider_logs: home.dir.join("logs-provider"),
        app_logs: home.dir.join("logs-apps"),
    };
    std::thread::spawn(move || azure_manager::managers::daemon::run(config));
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(&socket).is_err() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    // `notes` a deja tourne en developpement (ce test) : elle a un id.
    let summary = azure_manager::models::manifest::Manifest::load(&Path::new(&notes_source()).join("app.azure")).unwrap().summary();
    let mut dev = azure_manager::services::client::ManagerClient::connect_at(&socket).unwrap();
    let old_id = dev.register(&summary).unwrap();
    drop(dev);

    let binary = home.fake_binary("notes_app");
    let out = home.ok(&["install", &notes_source(), "--bin", binary.to_str().unwrap(), "--manager", &socket]);
    assert!(out.contains(&format!("identite enregistree : id {old_id}")), "meme id, donc memes donnees : {out}");

    // L'app installee a pour identite son empreinte : un autre executable
    // (ce test) ne peut plus se faire passer pour elle.
    let mut imposteur = azure_manager::services::client::ManagerClient::connect_at(&socket).unwrap();
    let error = imposteur.register(&summary).unwrap_err();
    assert!(error.contains("est installee") && error.contains("empreinte"), "{error}");
    // Reinstaller (nouvelle version) : toujours le meme id.
    std::fs::write(&binary, "#!/bin/sh\necho v2\n").unwrap();
    let out = home.ok(&["install", &notes_source(), "--bin", binary.to_str().unwrap(), "--manager", &socket]);
    assert!(out.contains(&format!("id {old_id}")), "{out}");
}

#[test]
fn new_creates_an_empty_app_in_the_azure_workspace() {
    let home = Home::new("new");
    let source = home.dir.join("Azure");
    std::fs::create_dir_all(source.join("azure-foundation")).unwrap();
    std::fs::write(source.join("azure-foundation/Cargo.toml"), "[package]\n").unwrap();
    std::fs::write(source.join("Cargo.toml"), "[workspace]\nmembers = [\n    \"azure-foundation\",\n]\n").unwrap();

    assert!(home.err(&["new", "meteo"]).contains("sources d'Azure inconnues"));
    let out = home.ok(&["new", "meteo-locale", "--azure", source.to_str().unwrap()]);
    assert!(out.contains("Meteo locale (meteo-locale) creee"), "{out}");
    let app = source.join("azure-meteo-locale");
    let manifest = std::fs::read_to_string(app.join("app.azure")).unwrap();
    assert!(manifest.contains("name = meteo-locale") && manifest.contains("exec = azure_meteo_locale"), "{manifest}");
    assert!(std::fs::read_to_string(app.join("Cargo.toml")).unwrap().contains("path = \"../azure-foundation\""));
    assert!(app.join("src/main.rs").is_file() && app.join("ui/accueil.rsh").is_file() && app.join("ui/app.rsc").is_file());
    let workspace = std::fs::read_to_string(source.join("Cargo.toml")).unwrap();
    assert!(workspace.contains("    \"azure-foundation\",\n    \"azure-meteo-locale\",\n]"), "{workspace}");
    // Le manifeste genere est valide.
    azure_manager::models::manifest::Manifest::load(&app.join("app.azure")).unwrap();

    assert!(home.err(&["new", "meteo-locale", "--azure", source.to_str().unwrap()]).contains("existe deja"));
    assert!(home.err(&["new", "Meteo", "--azure", source.to_str().unwrap()]).contains("Nom d'app invalide"));

    // Ailleurs : chemin absolu vers azure-foundation, workspace inchange.
    std::fs::create_dir_all(home.dir.join("ailleurs")).unwrap();
    home.ok(&["new", "jeu", "--titre", "Mon jeu", "--dans", home.dir.join("ailleurs").to_str().unwrap(), "--azure", source.to_str().unwrap()]);
    let cargo = std::fs::read_to_string(home.dir.join("ailleurs/azure-jeu/Cargo.toml")).unwrap();
    assert!(cargo.contains(&format!("path = \"{}\"", source.canonicalize().unwrap().join("azure-foundation").display())), "{cargo}");
    assert!(std::fs::read_to_string(home.dir.join("ailleurs/azure-jeu/app.azure")).unwrap().contains("title = Mon jeu"));
    assert!(!std::fs::read_to_string(source.join("Cargo.toml")).unwrap().contains("azure-jeu"));
}

#[test]
fn build_compiles_then_installs() {
    let home = Home::new("build");
    let source = home.dir.join("Azure");
    std::fs::create_dir_all(source.join("azure-foundation")).unwrap();
    std::fs::write(source.join("azure-foundation/Cargo.toml"), "[package]\n").unwrap();
    std::fs::write(source.join("Cargo.toml"), "[workspace]\nmembers = [\n]\n").unwrap();
    home.ok(&["new", "meteo", "--azure", source.to_str().unwrap()]);
    std::fs::create_dir_all(home.share().join("azure")).unwrap();
    std::fs::write(home.share().join("azure/source"), source.to_str().unwrap()).unwrap();

    // Un faux cargo : "compile" en ecrivant le binaire la ou cargo le mettrait.
    let cargo = home.dir.join("cargo");
    std::fs::write(&cargo, "#!/bin/sh\necho \"cargo $* dans $(basename $PWD)\"\necho '   Compiling azure-meteo' >&2\nmkdir -p ../target/release\nprintf '#!/bin/sh\\n' > ../target/release/azure_meteo\nchmod +x ../target/release/azure_meteo\n").unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
    let run = |args: &[&str]| home_cmd(&home, &cargo, args);

    let out = run(&["build", "meteo"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("cargo build --release --color never dans azure-meteo"), "{text}");
    // La sortie d'erreur de cargo arrive dans l'ordre, sur la sortie standard.
    assert!(text.contains("Compiling azure-meteo") && text.contains("ensuite : azure install"), "{text}");
    assert!(!home.share().join("azure/apps/meteo").exists());

    let out = run(&["build", source.join("azure-meteo").to_str().unwrap(), "--installer", "--sans-manager"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Meteo (meteo) installee"));
    assert!(home.share().join("azure/apps/meteo/azure_meteo").is_file());

    let out = run(&["build", "inconnue"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("ni un dossier d'app"));
    // Echec de cargo : pas d'installation, code d'erreur.
    std::fs::write(&cargo, "#!/bin/sh\necho 'error[E0425]: oups' >&2\nexit 101\n").unwrap();
    let out = run(&["build", "meteo", "--installer", "--sans-manager"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("error[E0425]: oups"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("compilation echouee (code 101)"));
}

fn home_cmd(home: &Home, cargo: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_azure"))
        .args(args)
        .env("HOME", &home.dir)
        .env("XDG_DATA_HOME", home.share())
        .env("XDG_CONFIG_HOME", home.dir.join("config"))
        .env("XDG_STATE_HOME", home.dir.join("state"))
        .env("AZURE_RUNTIME_DIR", home.dir.join("run"))
        .env("AZURE_SYSTEMCTL", "/usr/bin/true")
        .env("CARGO", cargo)
        .output()
        .unwrap()
}
