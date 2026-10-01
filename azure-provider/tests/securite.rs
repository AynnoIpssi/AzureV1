// Une app enfermee (NoNewPrivs, comme sous Landlock) ne pilote pas le
// provider : sinon elle lui ferait lancer, hors de son enfermement, la
// commande de son choix. Fichier a part : NoNewPrivs est irreversible et
// vaut pour tout ce processus de test.
use azure_provider::{ProviderClient, Service};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Tue le provider meme si une assertion echoue.
struct Guard(Child);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_azure_provider");

#[test]
fn une_app_enfermee_ne_pilote_pas_le_provider() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("provider-securite");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("provider.conf");
    std::fs::write(&config, "[rooter]\nenabled = false\n[stockage]\nenabled = false\n[service]\nenabled = false\n[manager]\nenabled = false\n").unwrap();
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-provider-secu-{}.sock"), std::process::id());
    let _child = Guard(Command::new(BIN)
        .args(["--socket", &socket, "--config", config.to_str().unwrap(), "--logs", dir.join("logs").to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(&socket).is_err() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }

    // SAFETY : prctl sans pointeur.
    assert_eq!(unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) }, 0);

    let mut client = ProviderClient::connect_at(&socket).unwrap();
    assert!(client.status().is_ok(), "l'etat reste lisible");
    let err = client.register(&Service::new("evasion", "/bin/sh")).unwrap_err();
    assert!(err.contains("enferme"), "{err}");
    assert!(client.stop("rooter").is_err());
    assert!(client.shutdown().is_err());
    assert!(client.status().unwrap().iter().all(|s| s.name != "evasion"));
}
