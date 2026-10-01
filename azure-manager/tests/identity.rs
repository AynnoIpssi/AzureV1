// Identite de bout en bout : azure-stockage et azure-service demandent a
// azure-manager qui est l'app qui se presente (fichier a part : ils gardent
// le socket du manager pour tout le processus).
use azure_manager::models::manifest::Manifest;
use azure_manager::services::client::ManagerClient;
use azure_service::flux::Flux;
use azure_stockage::services::client::StockageClient;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn wait(socket: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(socket).is_err() {
        assert!(Instant::now() < deadline, "{socket}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn daemons_check_who_is_talking_with_the_manager() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("identity");
    let _ = std::fs::remove_dir_all(&dir);
    let pid = std::process::id();
    let (manager_socket, service, stockage) = (format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-id-manager-{pid}.sock"), pid = pid), format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-id-service-{pid}.sock"), pid = pid), format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-id-stockage-{pid}.sock"), pid = pid));
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    azure_service::managers::daemon::set_manager_socket(&manager_socket);
    azure_stockage::managers::daemon::set_manager_socket(&manager_socket);
    let (s, d) = (service.clone(), dir.join("service"));
    std::thread::spawn(move || azure_service::managers::daemon::start_daemon_with(&s, &d, None));
    let (s, d) = (stockage.clone(), dir.join("stockage"));
    std::thread::spawn(move || azure_stockage::managers::daemon::start_daemon_at(&s, &d));
    let mut manager = Command::new(env!("CARGO_BIN_EXE_manager_daemon"))
        .args(["--socket", &manager_socket, "--data", dir.join("manager").to_str().unwrap(), "--service-socket", &service, "--provider-socket", "/tmp/aucun.sock", "--admin", &me])
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    for socket in [&manager_socket, &service, &stockage] {
        wait(socket);
    }
    let summary = |text: &str| Manifest::parse(text, Path::new("/x")).unwrap().summary();

    // Une app de developpement (cet executable) : reconnue.
    let mut boutique = ManagerClient::connect_at(&manager_socket).unwrap();
    let b = boutique.register(&summary("[app]\nname = boutique")).unwrap();
    assert!(StockageClient::connect_at(&stockage, b).is_ok());
    assert!(Flux::connect_at(&service, b).is_ok());

    // Une app installee (autre executable, autre empreinte) : cet executable
    // ne peut pas prendre son id, ni au stockage ni aux flux.
    let mut admin = ManagerClient::connect_at(&manager_socket).unwrap();
    let c = admin.install(&summary("[app]\nname = caisse"), "/opt/caisse/caisse", &[9; 32]).unwrap();
    let error = StockageClient::connect_at(&stockage, c).err().unwrap();
    assert!(error.contains("'caisse'") && error.contains("n'est pas elle"), "{error}");
    assert!(Flux::connect_at(&service, c).err().unwrap().contains("n'est pas elle"));

    // Sans la permission stockage : refusee au stockage, pas aux flux.
    let mut sans = ManagerClient::connect_at(&manager_socket).unwrap();
    let s = sans.register(&summary("[app]\nname = sanscles\n[permissions]\nstockage = false")).unwrap();
    let error = StockageClient::connect_at(&stockage, s).err().unwrap();
    assert!(error.contains("permission stockage"), "{error}");
    assert!(Flux::connect_at(&service, s).is_ok());

    // Ids ecrits a la main (< 1000) : regle d'avant (premier executable).
    assert!(StockageClient::connect_at(&stockage, 7).is_ok());
    let _ = manager.kill();
    let _ = manager.wait();
}
