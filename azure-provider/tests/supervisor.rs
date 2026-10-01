// Le superviseur avec de vrais processus. Le « faux daemon » a socket est le
// binaire du provider lui-meme (il ouvre un socket et y repond), lance avec
// une config qui desactive les daemons d'Azure.
use azure_provider::managers::supervisor::Supervisor;
use azure_provider::models::policy::RestartPolicy;
use azure_provider::models::service::{Restart, ServiceSpec, State};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn policy() -> RestartPolicy {
    RestartPolicy {
        base: Duration::from_millis(20),
        max: Duration::from_millis(50),
        max_crashes: 4,
        window: Duration::from_secs(60),
        start_timeout: Duration::from_millis(1500),
        health_interval: Duration::from_millis(20),
        health_failures: 2,
        stop_timeout: Duration::from_secs(1),
    }
}

fn dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("provider-{test}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn socket(test: &str) -> String {
    format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-provider-test-{test}-{}.sock"), std::process::id(), test = test)
}

/// Un faux daemon : repond sur `socket`.
fn fake_daemon(name: &str, socket: &str, dir: &std::path::Path) -> ServiceSpec {
    let config = dir.join("vide.conf");
    std::fs::write(&config, "[rooter]\nenabled = false\n[stockage]\nenabled = false\n[service]\nenabled = false\n[manager]\nenabled = false\n").unwrap();
    ServiceSpec::new(name, env!("CARGO_BIN_EXE_azure_provider"))
        .args(&["--socket", socket, "--config", config.to_str().unwrap(), "--logs", dir.join("logs-fake").to_str().unwrap()])
        .health_socket(socket)
}

fn sh(name: &str, script: &str) -> ServiceSpec {
    ServiceSpec::new(name, "sh").args(&["-c", script])
}

/// Fait tourner le superviseur jusqu'a `done` (ou echec apres `timeout`).
fn run_until(sup: &mut Supervisor, timeout: Duration, mut done: impl FnMut(&Supervisor) -> bool) {
    let deadline = Instant::now() + timeout;
    loop {
        sup.tick(Instant::now());
        if done(sup) {
            return;
        }
        assert!(Instant::now() < deadline, "delai depasse : {:?}", sup.status(Instant::now()));
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn pid(sup: &Supervisor, name: &str) -> Option<u32> {
    sup.status(Instant::now()).into_iter().find(|s| s.name == name).and_then(|s| s.pid)
}

fn alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

fn kill9(pid: u32) {
    Command::new("kill").args(["-9", &pid.to_string()]).status().unwrap();
}

#[test]
fn a_killed_service_is_relaunched() {
    let dir = dir("killed");
    let mut sup = Supervisor::new(vec![ServiceSpec::new("dort", "sleep").arg("60").autostart(true)], policy(), dir.join("logs"));
    sup.start_autostart(Instant::now());
    assert_eq!(sup.state("dort"), Some(State::Running));
    let first = pid(&sup, "dort").unwrap();

    kill9(first);
    run_until(&mut sup, Duration::from_secs(5), |s| pid(s, "dort").is_some_and(|p| p != first));
    let status = &sup.status(Instant::now())[0];
    assert_eq!(status.state, State::Running);
    assert_eq!(status.restarts, 1);
    assert_eq!(status.message, "tue par le signal 9");
}

#[test]
fn a_crash_loop_slows_down_then_gives_up() {
    let dir = dir("loop");
    let mut sup = Supervisor::new(vec![sh("plante", "echo 'boum' >&2; exit 3")], policy(), dir.join("logs"));
    let t0 = Instant::now();
    sup.start("plante", t0).unwrap();
    run_until(&mut sup, Duration::from_secs(10), |s| s.state("plante") == Some(State::Failed));

    let status = &sup.status(Instant::now())[0];
    assert_eq!(status.restarts, 3, "4 plantages = 3 relances puis abandon");
    assert_eq!(status.message, "sorti avec le code 3 ; 4 plantages en 60 s, abandon");
    // Delais 0 + 20 + 40 ms au minimum.
    assert!(t0.elapsed() >= Duration::from_millis(60));
    // Plus relance tout seul...
    for _ in 0..10 {
        sup.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(sup.state("plante"), Some(State::Failed));
    // ... et sa sortie est dans son journal.
    let log = std::fs::read_to_string(dir.join("logs/plante.log")).unwrap();
    assert_eq!(log.matches("boum").count(), 4);
    // `start` a la main : on repart de zero.
    sup.start("plante", Instant::now()).unwrap();
    assert_ne!(sup.state("plante"), Some(State::Failed));
}

#[test]
fn a_daemon_is_ready_only_when_its_socket_answers_and_is_relaunched_when_it_dies() {
    let dir = dir("socket");
    let socket = socket("socket");
    let mut sup = Supervisor::new(vec![fake_daemon("faux", &socket, &dir)], policy(), dir.join("logs"));
    sup.start("faux", Instant::now()).unwrap();
    assert_eq!(sup.state("faux"), Some(State::Starting));
    run_until(&mut sup, Duration::from_secs(5), |s| s.state("faux") == Some(State::Running));

    let first = pid(&sup, "faux").unwrap();
    kill9(first);
    run_until(&mut sup, Duration::from_secs(5), |s| s.state("faux") == Some(State::Running) && pid(s, "faux") != Some(first));
    assert_eq!(sup.status(Instant::now())[0].restarts, 1);
    sup.shutdown();
}

#[test]
fn a_process_that_never_answers_is_killed_and_relaunched() {
    let dir = dir("hung");
    let mut policy = policy();
    policy.start_timeout = Duration::from_millis(200);
    let spec = ServiceSpec::new("muet", "sleep").arg("60").health_socket("/nonexistent/azure-provider-test-personne.sock");
    let mut sup = Supervisor::new(vec![spec], policy, dir.join("logs"));
    sup.start("muet", Instant::now()).unwrap();
    let first = pid(&sup, "muet").unwrap();

    run_until(&mut sup, Duration::from_secs(5), |s| s.status(Instant::now())[0].restarts >= 1);
    assert!(sup.message("muet").unwrap().starts_with("ne repond pas sur /nonexistent/azure-provider-test-personne.sock"));
    assert!(!alive(first), "l'ancien processus muet doit etre tue");
}

#[test]
fn a_daemon_started_by_hand_is_watched_then_taken_over() {
    let dir = dir("external");
    let socket = socket("external");
    let spec = fake_daemon("faux", &socket, &dir);
    let mut by_hand = Command::new(&spec.command).args(&spec.args).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(&socket).is_err() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }

    let mut sup = Supervisor::new(vec![spec], policy(), dir.join("logs"));
    sup.start("faux", Instant::now()).unwrap();
    assert_eq!(sup.state("faux"), Some(State::External), "pas de second daemon");
    assert_eq!(pid(&sup, "faux"), None);

    // Le daemon lance a la main tombe : le provider prend le relais.
    by_hand.kill().unwrap();
    by_hand.wait().unwrap();
    run_until(&mut sup, Duration::from_secs(5), |s| s.state("faux") == Some(State::Running));
    assert!(pid(&sup, "faux").is_some());
    assert_eq!(sup.status(Instant::now())[0].restarts, 0, "pas un plantage du provider");
    sup.shutdown();
}

#[test]
fn restart_rules_for_tasks_that_end() {
    let dir = dir("rules");
    let services = vec![
        sh("ok-onfailure", "exit 0").restart(Restart::OnFailure),
        sh("ko-onfailure", "exit 1").restart(Restart::OnFailure),
        sh("ko-never", "exit 1").restart(Restart::Never),
    ];
    let mut sup = Supervisor::new(services, policy(), dir.join("logs"));
    for name in ["ok-onfailure", "ko-onfailure", "ko-never"] {
        sup.start(name, Instant::now()).unwrap();
    }
    run_until(&mut sup, Duration::from_secs(5), |s| {
        s.state("ok-onfailure") == Some(State::Exited) && s.state("ko-never") == Some(State::Exited) && s.state("ko-onfailure") == Some(State::Failed)
    });
    assert_eq!(sup.message("ok-onfailure").unwrap(), "sorti avec le code 0");
}

#[test]
fn stop_prevents_relaunch_and_drop_stops_everything() {
    let dir = dir("stop");
    let services = vec![ServiceSpec::new("a", "sleep").arg("60"), ServiceSpec::new("b", "sleep").arg("60")];
    let mut sup = Supervisor::new(services, policy(), dir.join("logs"));
    sup.start("a", Instant::now()).unwrap();
    sup.start("b", Instant::now()).unwrap();
    let (a, b) = (pid(&sup, "a").unwrap(), pid(&sup, "b").unwrap());

    sup.stop("a").unwrap();
    assert!(!alive(a));
    for _ in 0..10 {
        sup.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(sup.state("a"), Some(State::Stopped));
    assert_eq!(sup.message("a").unwrap(), "arrete a la demande");

    drop(sup);
    assert!(!alive(b), "le provider arrete ses services en partant");
}

#[test]
fn apps_register_their_own_services_but_cannot_touch_azure_daemons() {
    let dir = dir("register");
    let mut sup = Supervisor::new(azure_provider::models::service::builtin(), policy(), dir.join("logs"));
    let spec = ServiceSpec::new("tache", "sleep").arg("60");

    sup.register(spec.clone(), Instant::now()).unwrap();
    let first = pid(&sup, "tache").unwrap();
    // La meme demande deux fois (l'app relancee) : un seul processus.
    sup.register(spec.clone(), Instant::now()).unwrap();
    assert_eq!(pid(&sup, "tache"), Some(first));
    // Une definition differente remplace l'ancienne.
    sup.register(spec.arg("--x"), Instant::now()).unwrap();
    assert!(!alive(first));

    let hijack = ServiceSpec::new("stockage", "/tmp/faux-stockage");
    assert!(sup.register(hijack, Instant::now()).unwrap_err().contains("daemon d'Azure"));
    assert!(sup.unregister("rooter").unwrap_err().contains("daemon d'Azure"));
    assert!(sup.register(ServiceSpec::new("Pas Bon", "sleep"), Instant::now()).unwrap_err().contains("invalide"));

    sup.unregister("tache").unwrap();
    assert_eq!(sup.state("tache"), None);
}
