// Le provider lance comme un vrai process, pilote par son socket, comme
// le font les apps et la ligne de commande.
use azure_provider::{Provider, ProviderClient, Service, State};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_azure_provider");

fn dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("provider-daemon-{test}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn socket(name: &str) -> String {
    format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-provider-dtest-{name}-{}.sock"), std::process::id(), name = name)
}

/// Config : daemons d'Azure desactives, un « faux daemon » a socket (le
/// binaire du provider lui-meme) lance a la demande.
fn write_config(dir: &Path, fake_socket: &str) -> PathBuf {
    let empty = dir.join("vide.conf");
    std::fs::write(&empty, "[rooter]\nenabled = false\n[stockage]\nenabled = false\n[service]\nenabled = false\n[manager]\nenabled = false\n").unwrap();
    let config = dir.join("provider.conf");
    std::fs::write(
        &config,
        format!(
            "[rooter]\nenabled = false\n[stockage]\nenabled = false\n[service]\nenabled = false\n[manager]\nenabled = false\n\n[faux]\ncommand = {BIN}\nargs = --socket {fake_socket} --config {} --logs {}\nhealth = {fake_socket}\n",
            empty.display(),
            dir.join("logs-faux").display()
        ),
    )
    .unwrap();
    config
}

fn wait_socket(socket: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(socket).is_err() {
        assert!(Instant::now() < deadline, "{socket} ne repond pas");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn start_provider(dir: &Path, socket: &str, fake_socket: &str) -> Child {
    let config = write_config(dir, fake_socket);
    let child = Command::new(BIN)
        .args(["--socket", socket, "--config", config.to_str().unwrap(), "--logs", dir.join("logs").to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    wait_socket(socket);
    child
}

fn state(client: &mut ProviderClient, name: &str) -> Option<State> {
    client.status().unwrap().into_iter().find(|s| s.name == name).map(|s| s.state)
}

#[test]
fn ensure_register_stop_and_shutdown_through_the_socket() {
    let dir = dir("pilot");
    let (socket, fake) = (socket("pilot"), socket("pilot-faux"));
    let mut provider = start_provider(&dir, &socket, &fake);
    let mut client = ProviderClient::connect_at(&socket).unwrap();

    // A la demande : pas lance tant que personne n'en a besoin.
    assert_eq!(state(&mut client, "faux"), Some(State::Stopped));
    client.ensure("faux", Duration::from_secs(5)).unwrap();
    assert!(UnixStream::connect(&fake).is_ok(), "pret = son socket repond");
    assert_eq!(state(&mut client, "faux"), Some(State::Running));

    // Une app confie une tache de fond.
    Service::new("tache", "sleep").arg("60").register_at(&socket).unwrap();
    let tache = client.status().unwrap().into_iter().find(|s| s.name == "tache").unwrap();
    assert_eq!(tache.state, State::Running);
    let tache_pid = tache.pid.unwrap();

    client.restart("tache").unwrap();
    let new_pid = client.status().unwrap().into_iter().find(|s| s.name == "tache").unwrap().pid.unwrap();
    assert_ne!(new_pid, tache_pid);

    client.stop("faux").unwrap();
    assert_eq!(state(&mut client, "faux"), Some(State::Stopped));
    assert!(client.ensure("inconnu", Duration::from_secs(1)).unwrap_err().contains("Service inconnu"));

    // SHUTDOWN : le provider sort, arrete ses services et retire son socket.
    client.shutdown().unwrap();
    let status = provider.wait().unwrap();
    assert!(status.success());
    assert!(!Path::new(&format!("/proc/{new_pid}")).exists());
    assert!(!Path::new(&socket).exists());
}

#[test]
fn an_app_launches_the_provider_when_it_is_not_running() {
    let dir = dir("launch");
    let (socket, fake) = (socket("launch"), socket("launch-faux"));
    // Le provider lance par l'app lit la config par defaut : on lui en donne
    // une via XDG_CONFIG_HOME, dans un process a part pour ne pas toucher a
    // l'environnement des autres tests.
    let config = write_config(&dir, &fake);
    std::fs::create_dir_all(dir.join("xdg/azure")).unwrap();
    std::fs::copy(&config, dir.join("xdg/azure/provider.conf")).unwrap();
    assert!(UnixStream::connect(&socket).is_err());

    let out = Command::new(BIN)
        .args(["start", "faux", "--socket", &socket])
        .output()
        .unwrap();
    assert!(!out.status.success(), "la ligne de commande ne lance pas le provider");
    assert!(String::from_utf8_lossy(&out.stderr).contains("provider injoignable"));

    // Comme une app : `Provider::ensure_at` lance le binaire trouve a cote
    // de l'executable (target/debug/azure_provider).
    let app = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "app_side_ensure", "--ignored", "--nocapture"])
        .env("XDG_CONFIG_HOME", dir.join("xdg"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("PROVIDER_TEST_SOCKET", &socket)
        .output()
        .unwrap();
    assert!(app.status.success(), "{}", String::from_utf8_lossy(&app.stdout));
    assert!(UnixStream::connect(&fake).is_ok());

    // Le provider lance par l'app lui survit ; la ligne de commande le voit.
    let out = Command::new(BIN).args(["status", "--socket", &socket]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("SERVICE") && text.lines().any(|l| l.starts_with("faux") && l.contains("actif")), "{text}");
    assert!(dir.join("state/azure/provider/faux.log").exists(), "journal au bon endroit");

    let out = Command::new(BIN).args(["shutdown", "--socket", &socket]).output().unwrap();
    assert!(out.status.success());
    let deadline = Instant::now() + Duration::from_secs(5);
    while Path::new(&socket).exists() {
        assert!(Instant::now() < deadline, "le provider ne s'arrete pas");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(UnixStream::connect(&fake).is_err(), "ses services partent avec lui");
}

/// Joue le role de l'app (lance par le test ci-dessus, dans son propre
/// environnement).
#[test]
#[ignore]
fn app_side_ensure() {
    let socket = std::env::var("PROVIDER_TEST_SOCKET").unwrap();
    Provider::ensure_at(&socket, "faux").unwrap();
}

#[test]
fn a_second_provider_refuses_to_start() {
    let dir = dir("twice");
    let (socket, fake) = (socket("twice"), socket("twice-faux"));
    let mut first = start_provider(&dir, &socket, &fake);
    let out = Command::new(BIN).args(["--socket", &socket, "--config", dir.join("provider.conf").to_str().unwrap()]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("un provider tourne deja"));

    // SIGTERM arrete proprement le premier.
    Command::new("kill").arg(first.id().to_string()).status().unwrap();
    assert!(first.wait().unwrap().success());
    assert!(!Path::new(&socket).exists());
}

#[test]
fn installed_app_tasks_are_known_at_start_and_survive_the_connection_that_woke_them() {
    let dir = dir("apps");
    let (socket, fake) = (socket("apps"), socket("apps-faux"));
    // Ecrit par `azure install` : <XDG_DATA_HOME>/azure/services/<app>.conf.
    let services = dir.join("share/azure/services");
    std::fs::create_dir_all(&services).unwrap();
    std::fs::write(services.join("docs.conf"), "[docs-methodes]\ncommand = sleep\nargs = 60\nrestart = on-failure\nautostart = false\n").unwrap();
    std::fs::write(services.join("pirate.conf"), "[stockage]\ncommand = /bin/sh\n").unwrap();
    let config = write_config(&dir, &fake);
    let mut provider = Command::new(BIN)
        .args(["--socket", &socket, "--config", config.to_str().unwrap(), "--logs", dir.join("logs").to_str().unwrap()])
        .env("XDG_DATA_HOME", dir.join("share"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    wait_socket(&socket);

    let mut client = ProviderClient::connect_at(&socket).unwrap();
    assert_eq!(state(&mut client, "docs-methodes"), Some(State::Stopped), "connue, pas lancee");
    assert!(client.status().unwrap().iter().filter(|s| s.name == "stockage").count() == 0, "un fichier d'app ne remplace pas un daemon d'Azure");

    // Reveillee par une connexion qui se ferme aussitot (comme le manager).
    ProviderClient::connect_at(&socket).unwrap().ensure("docs-methodes", Duration::from_secs(5)).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    let s = client.status().unwrap().into_iter().find(|s| s.name == "docs-methodes").unwrap();
    assert_eq!((s.state, s.restarts), (State::Running, 0), "{s:?}");
    assert!(Path::new(&format!("/proc/{}", s.pid.unwrap())).exists());

    // DECLARE : une nouvelle definition ne tue pas le processus en cours.
    let pid = s.pid.unwrap();
    client.declare(&Service::new("docs-methodes", "sleep").arg("61").restart(azure_provider::Restart::OnFailure)).unwrap();
    client.declare(&Service::new("note-rien", "sleep").arg("60")).unwrap();
    assert_eq!(state(&mut client, "note-rien"), Some(State::Stopped));
    assert_eq!(client.status().unwrap().into_iter().find(|s| s.name == "docs-methodes").unwrap().pid, Some(pid));

    client.shutdown().unwrap();
    provider.wait().unwrap();
}
