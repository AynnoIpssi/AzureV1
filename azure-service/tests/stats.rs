// Les compteurs vus par azure-manager : appels (reussis, erreurs, delais)
// et flux (modifications, ecoutes).
use azure_service::call::DEFAULT_TIMEOUT;
use azure_service::flux::{Access, Flux, Value};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn calls_and_flux_are_counted() {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-service-stats-{}.sock"), std::process::id());
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("service-stats");
    let _ = std::fs::remove_dir_all(&data);
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let s = socket.clone();
    std::thread::spawn(move || azure_service::managers::daemon::start_daemon_with(&s, &data, Some(me)));
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(&socket).is_err() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }

    let b = Flux::connect_at(&socket, 2).unwrap();
    let _prix = b.serve("prix", Access::Public, |d| d.args.as_i64().map(|n| Value::Int(n * 2)).ok_or("nombre attendu".to_string())).unwrap();
    let _lent = b.serve("lent", Access::Public, |_| {
        std::thread::sleep(Duration::from_millis(300));
        Ok(Value::Null)
    }).unwrap();
    let mut panier = b.share("panier").public().persist(true).open().unwrap();
    panier.set("n", 1).unwrap();
    panier.set("n", 2).unwrap();

    let a = Flux::connect_at(&socket, 1).unwrap();
    let _ecoute = a.listen(2, "panier").start().unwrap();
    a.call(2, "prix", 21i64, DEFAULT_TIMEOUT).unwrap();
    a.call(2, "prix", 4i64, DEFAULT_TIMEOUT).unwrap();
    assert!(a.call(2, "prix", "x", DEFAULT_TIMEOUT).is_err());
    assert!(a.call(2, "lent", Value::Null, Duration::from_millis(50)).is_err());
    assert!(a.call(2, "absente", Value::Null, DEFAULT_TIMEOUT).is_err());

    let (flux, methods) = a.stats().unwrap();
    let p = flux.iter().find(|f| f.name == "panier").unwrap();
    assert_eq!((p.changes, p.listeners, p.persist), (2, 1, true));
    let prix = methods.iter().find(|m| m.method == "prix").unwrap();
    assert_eq!((prix.calls, prix.errors, prix.timeouts, prix.served), (3, 1, 0, true));
    assert_eq!(prix.last_error, "nombre attendu");
    let lent = methods.iter().find(|m| m.method == "lent").unwrap();
    assert_eq!((lent.calls, lent.errors, lent.timeouts), (1, 1, 1));
    let absente = methods.iter().find(|m| m.method == "absente").unwrap();
    assert!(!absente.served && absente.errors == 1);
}
