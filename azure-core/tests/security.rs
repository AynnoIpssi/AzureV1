// Securite : bac a sable Landlock (dans un processus a part, puisqu'il est
// definitif), durcissement des daemons, coffre de chiffrement.
use azure_core::security::hardening::{harden_daemon, is_dumpable, private_dir};
use azure_core::security::sandbox::{abi_version, Enforcement, Sandbox};
use azure_core::security::vault::Vault;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("security-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Lance le test `inner` de ce fichier dans un processus neuf.
fn run_inner(inner: &str, env: &[(&str, String)]) -> (bool, String) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", inner, "--ignored", "--nocapture"]);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
}

#[test]
fn a_sandboxed_process_only_sees_what_it_is_given() {
    if abi_version() < 1 {
        eprintln!("Landlock absent : test saute");
        return;
    }
    let allowed = tmp("allowed");
    let secret = tmp("secret");
    std::fs::write(allowed.join("ok.txt"), "lisible").unwrap();
    std::fs::write(secret.join("master.key"), "cle secrete").unwrap();
    let (ok, out) = run_inner("inner_sandboxed", &[("ALLOWED", allowed.display().to_string()), ("SECRET", secret.display().to_string())]);
    assert!(ok, "{out}");
    assert!(out.contains("RESULTAT lecture-ok ecriture-ok secret-refuse ecriture-secret-refusee reseau-refuse"), "{out}");
}

#[test]
#[ignore]
fn inner_sandboxed() {
    let allowed = PathBuf::from(std::env::var("ALLOWED").unwrap());
    let secret = PathBuf::from(std::env::var("SECRET").unwrap());
    // Un serveur TCP ouvert AVANT d'etre enferme : s'y connecter apres doit echouer.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let enforcement = Sandbox::system().write(&allowed).apply().unwrap();
    let mut result = vec!["RESULTAT"];
    result.push(if std::fs::read_to_string(allowed.join("ok.txt")).is_ok() { "lecture-ok" } else { "lecture-KO" });
    result.push(if std::fs::write(allowed.join("nouveau.txt"), "x").is_ok() { "ecriture-ok" } else { "ecriture-KO" });
    result.push(if std::fs::read(secret.join("master.key")).is_err() { "secret-refuse" } else { "secret-LU" });
    result.push(if std::fs::write(secret.join("pirate.txt"), "x").is_err() { "ecriture-secret-refusee" } else { "ecriture-secret-FAITE" });
    let connect = std::net::TcpStream::connect(("127.0.0.1", port));
    result.push(match (enforcement, connect.is_err()) {
        (_, true) => "reseau-refuse",
        (Enforcement::FilesOnly, false) => "reseau-refuse", // noyau sans Landlock reseau : non verifiable
        _ => "reseau-OUVERT",
    });
    println!("{}", result.join(" "));
}

#[test]
fn a_sandboxed_process_cannot_signal_the_daemons() {
    if abi_version() < 6 {
        eprintln!("Landlock < 6 : pas de portee signaux, test saute");
        return;
    }
    // Un « daemon » hors de l'enclos.
    let mut daemon = Command::new("sleep").arg("30").spawn().unwrap();
    let (ok, out) = run_inner("inner_signals", &[("DAEMON", daemon.id().to_string())]);
    let alive = daemon.try_wait().unwrap().is_none();
    let _ = daemon.kill();
    let _ = daemon.wait();
    assert!(ok, "{out}");
    assert!(out.contains("RESULTAT signal-refuse soi-meme-ok"), "{out}");
    assert!(alive, "le daemon a survecu");
}

#[test]
#[ignore]
fn inner_signals() {
    let daemon: i32 = std::env::var("DAEMON").unwrap().parse().unwrap();
    Sandbox::system().apply().unwrap();
    // SAFETY : kill sans pointeur.
    let other = unsafe { libc::kill(daemon, libc::SIGTERM) };
    let own = unsafe { libc::kill(libc::getpid(), 0) };
    println!("RESULTAT {} {}", if other != 0 { "signal-refuse" } else { "signal-ENVOYE" }, if own == 0 { "soi-meme-ok" } else { "soi-meme-KO" });
}

#[test]
fn network_can_be_allowed() {
    if abi_version() < 4 {
        return;
    }
    let (ok, out) = run_inner("inner_network_allowed", &[]);
    assert!(ok && out.contains("CONNECTE"), "{out}");
}

#[test]
#[ignore]
fn inner_network_allowed() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    Sandbox::system().network(true).apply().unwrap();
    std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    println!("CONNECTE");
}

#[test]
fn daemons_are_hardened() {
    let (ok, out) = run_inner("inner_hardened", &[]);
    assert!(ok && out.contains("NON-INSPECTABLE"), "{out}");
    let dir = tmp("private").join("daemon");
    private_dir(&dir).unwrap();
    assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
}

#[test]
#[ignore]
fn inner_hardened() {
    assert!(is_dumpable());
    harden_daemon();
    if !is_dumpable() {
        println!("NON-INSPECTABLE");
    }
}

#[test]
fn the_vault_encrypts_and_detects_tampering() {
    let dir = tmp("vault");
    let vault = Vault::open(&dir).unwrap();
    assert_eq!(std::fs::metadata(dir.join("cle.bin")).unwrap().permissions().mode() & 0o777, 0o600);
    let file = dir.join("etat.bin");
    vault.write(&file, "etat", b"panier: 42").unwrap();
    let raw = std::fs::read(&file).unwrap();
    assert!(raw.starts_with(b"AZS1") && !raw.windows(10).any(|w| w == b"panier: 42"), "chiffre sur disque");
    assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(vault.read(&file, "etat").unwrap().unwrap(), b"panier: 42");
    // Meme cle au prochain lancement.
    assert_eq!(Vault::open(&dir).unwrap().read(&file, "etat").unwrap().unwrap(), b"panier: 42");
    // Un autre role, un octet modifie, une autre cle : refuses.
    assert!(vault.read(&file, "autre").is_err());
    let mut tampered = raw.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    std::fs::write(&file, &tampered).unwrap();
    assert!(vault.read(&file, "etat").is_err());
    assert!(Vault::with_key([7; 32]).read(&dir.join("absent"), "etat").unwrap().is_none());
    // Fichier d'avant le chiffrement : lu en clair.
    std::fs::write(&file, b"ancien format").unwrap();
    assert_eq!(vault.read(&file, "etat").unwrap().unwrap(), b"ancien format");
}

#[test]
fn fingerprints_follow_the_file_content() {
    use azure_core::managers::identity::{fingerprint_file, fingerprint_pid, hex};
    let dir = tmp("fingerprint");
    std::fs::write(dir.join("a"), "binaire").unwrap();
    std::fs::copy(dir.join("a"), dir.join("b")).unwrap();
    let a = fingerprint_file(&dir.join("a")).unwrap();
    assert_eq!(a, fingerprint_file(&dir.join("b")).unwrap(), "deplace / copie : meme empreinte");
    std::fs::write(dir.join("b"), "binaire modifie").unwrap();
    assert_ne!(a, fingerprint_file(&dir.join("b")).unwrap());
    assert_eq!(hex(&a).len(), 64);
    let me = fingerprint_pid(std::process::id() as i32).unwrap();
    assert_eq!(me, fingerprint_file(&std::env::current_exe().unwrap()).unwrap());
}

/// Sockets unix nommes (ABI 9) : une app enfermee joint les daemons (dossier
/// d'Azure) mais ni le bus de session, par lequel elle ferait lancer
/// n'importe quoi par systemd --user, ni un socket de /tmp (X11...).
#[test]
fn a_sandboxed_process_only_reaches_allowed_sockets() {
    if abi_version() < 9 {
        eprintln!("Landlock < 9 : sockets nommes non filtres (limite connue), test saute");
        return;
    }
    let dir = tmp("sockets");
    let outside = dir.join("bus");
    let _bus = std::os::unix::net::UnixListener::bind(&outside).unwrap();
    let runtime = dir.join("run");
    std::fs::create_dir_all(&runtime).unwrap();
    let daemon = runtime.join("stockage.sock");
    let _daemon = std::os::unix::net::UnixListener::bind(&daemon).unwrap();
    let x11 = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-security-test-x11-{}.sock"), std::process::id());
    let _ = std::fs::remove_file(&x11);
    let _x11 = std::os::unix::net::UnixListener::bind(&x11).unwrap();
    let (ok, out) = run_inner(
        "inner_sockets",
        &[("BUS", outside.display().to_string()), ("DAEMON", daemon.display().to_string()), ("X11", x11.clone()), ("AZURE_RUNTIME_DIR", runtime.display().to_string())],
    );
    let _ = std::fs::remove_file(&x11);
    assert!(ok, "{out}");
    assert!(out.contains("RESULTAT daemon-ok bus-refuse tmp-refuse"), "{out}");
}

#[test]
#[ignore]
fn inner_sockets() {
    let bus = std::env::var("BUS").unwrap();
    let daemon = std::env::var("DAEMON").unwrap();
    let x11 = std::env::var("X11").unwrap();
    // Le dossier du « bus » est meme accessible en ecriture : ca ne suffit pas.
    Sandbox::system().write(std::path::Path::new(&bus).parent().unwrap()).apply().unwrap();
    let d = std::os::unix::net::UnixStream::connect(&daemon).is_ok();
    let b = std::os::unix::net::UnixStream::connect(&bus).is_ok();
    let x = std::os::unix::net::UnixStream::connect(&x11).is_ok();
    println!(
        "RESULTAT {} {} {}",
        if d { "daemon-ok" } else { "daemon-KO" },
        if b { "bus-JOINT" } else { "bus-refuse" },
        if x { "tmp-JOINT" } else { "tmp-refuse" }
    );
}

/// Le vrai bus de session, s'il existe : constate qu'il est hors d'atteinte.
#[test]
fn the_session_bus_is_out_of_reach() {
    let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") else { return };
    if abi_version() < 9 || !std::path::Path::new(&runtime).join("bus").exists() {
        return;
    }
    let (ok, out) = run_inner("inner_real_bus", &[("BUS", format!("{runtime}/bus"))]);
    assert!(ok && out.contains("BUS refuse"), "{out}");
}

#[test]
#[ignore]
fn inner_real_bus() {
    let bus = std::env::var("BUS").unwrap();
    Sandbox::system().apply().unwrap();
    match std::os::unix::net::UnixStream::connect(&bus) {
        Ok(_) => println!("BUS joignable depuis l'enclos"),
        Err(e) => println!("BUS refuse ({e})"),
    }
}

/// Isolation par espaces de noms (lanceur, noyaux sans Landlock recent) :
/// SANS Landlock, le processus lance ne joint ni le bus de session ni rien
/// d'autre du dossier de session, sauf Wayland et les daemons d'Azure ; il
/// est le processus 1 de son espace et ne peut viser aucun autre processus.
#[test]
fn isolation_hides_the_session_and_the_other_processes() {
    use std::os::unix::process::CommandExt;
    let dir = tmp("isolation");
    let runtime = dir.join("run");
    std::fs::create_dir_all(runtime.join("azure")).unwrap();
    let _bus = std::os::unix::net::UnixListener::bind(runtime.join("bus")).unwrap();
    let _wayland = std::os::unix::net::UnixListener::bind(runtime.join("wayland-test")).unwrap();
    let _daemon = std::os::unix::net::UnixListener::bind(runtime.join("azure/stockage.sock")).unwrap();
    let mut victim = Command::new("sleep").arg("30").spawn().unwrap();

    let envs = [
        ("XDG_RUNTIME_DIR", runtime.display().to_string()),
        ("WAYLAND_DISPLAY", "wayland-test".to_string()),
        ("AZURE_RUNTIME_DIR", runtime.join("azure").display().to_string()),
        ("VICTIM", victim.id().to_string()),
    ];
    // L'isolation lit l'environnement : on le lui donne comme a l'app.
    for (k, v) in &envs {
        // SAFETY : test mono-thread a ce stade pour ces variables (lues juste apres).
        unsafe { std::env::set_var(k, v) };
    }
    let isolation = azure_core::security::isolation::Isolation::for_app(false);
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "inner_isolation", "--ignored", "--nocapture"]).envs(envs.iter().map(|(k, v)| (*k, v.clone())));
    // SAFETY : `enter` ne fait que des appels systeme.
    unsafe { cmd.pre_exec(move || isolation.enter()) };
    let out = cmd.output().unwrap();
    let alive = victim.try_wait().unwrap().is_none();
    let _ = victim.kill();
    let _ = victim.wait();
    let text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    if text.contains("NON-ISOLE") {
        eprintln!("espaces de noms indisponibles ici : test saute");
        return;
    }
    assert!(out.status.success(), "{text}");
    assert!(text.contains("RESULTAT pid=1 wayland-ok daemon-ok bus-refuse signal-refuse reseau-refuse"), "{text}");
    assert!(alive, "le processus vise a survecu");
}

#[test]
#[ignore]
fn inner_isolation() {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap();
    let victim: i32 = std::env::var("VICTIM").unwrap().parse().unwrap();
    // SAFETY : appels sans pointeur.
    let pid = unsafe { libc::getpid() };
    if pid != 1 {
        println!("NON-ISOLE");
        return;
    }
    let c = |p: String| std::os::unix::net::UnixStream::connect(p).is_ok();
    let w = c(format!("{runtime}/wayland-test"));
    let d = c(format!("{runtime}/azure/stockage.sock"));
    let b = c(format!("{runtime}/bus"));
    let s = unsafe { libc::kill(victim, libc::SIGTERM) } == 0;
    let n = std::net::TcpStream::connect("1.1.1.1:80").is_ok();
    println!(
        "RESULTAT pid={pid} {} {} {} {} {}",
        if w { "wayland-ok" } else { "wayland-KO" },
        if d { "daemon-ok" } else { "daemon-KO" },
        if b { "bus-JOINT" } else { "bus-refuse" },
        if s { "signal-ENVOYE" } else { "signal-refuse" },
        if n { "reseau-OUVERT" } else { "reseau-refuse" }
    );
}

/// Lance `inner` (test ignore de ce fichier) isole, comme `azure run` ;
/// attend qu'il soit pret, lui envoie SIGTERM (au lanceur : le seul pid
/// visible), et rend son code et sa sortie. `None` : pas d'espaces de noms.
fn kill_isolated(inner: &str) -> Option<(i32, String, std::time::Duration)> {
    use std::io::BufRead;
    use std::os::unix::process::CommandExt;
    let isolation = azure_core::security::isolation::Isolation::for_app(false);
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", inner, "--ignored", "--nocapture", "--test-threads", "1"]).stdout(std::process::Stdio::piped());
    // SAFETY : `enter` ne fait que des appels systeme.
    unsafe { cmd.pre_exec(move || isolation.enter()) };
    let mut child = cmd.spawn().unwrap();
    let mut out = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut text = String::new();
    loop {
        let mut line = String::new();
        assert!(out.read_line(&mut line).unwrap() > 0, "sorti avant d'etre pret : {text}");
        text.push_str(&line);
        if line.contains("NON-ISOLE") {
            let _ = child.wait();
            return None;
        }
        if line.contains("PRET") {
            break;
        }
    }
    let start = std::time::Instant::now();
    // SAFETY : signal a notre propre enfant.
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    let status = child.wait().unwrap();
    let elapsed = start.elapsed();
    let mut rest = String::new();
    std::io::Read::read_to_string(&mut out, &mut rest).unwrap();
    Some((status.code().unwrap_or(-1), text + &rest, elapsed))
}

#[test]
fn kill_stops_an_isolated_app_cleanly() {
    let Some((code, text, elapsed)) = kill_isolated("inner_termination_clean") else {
        eprintln!("espaces de noms indisponibles ici : test saute");
        return;
    };
    assert!(text.contains("ARRET PROPRE"), "{text}");
    assert_eq!(code, 0, "{text}");
    assert!(elapsed < std::time::Duration::from_secs(2), "{elapsed:?}");
}

#[test]
fn kill_stops_an_isolated_app_that_ignores_it() {
    let Some((code, text, elapsed)) = kill_isolated("inner_termination_stuck") else { return };
    // Pas de fermeture propre : sortie forcee apres GRACE, code 128 + SIGTERM.
    assert_eq!(code, 128 + libc::SIGTERM, "{text}");
    assert!(elapsed >= azure_core::security::termination::GRACE && elapsed < std::time::Duration::from_secs(10), "{elapsed:?}");
}

fn inner_ready() -> bool {
    // SAFETY : appel sans pointeur.
    if unsafe { libc::getpid() } != 1 {
        println!("NON-ISOLE");
        return false;
    }
    azure_core::security::termination::install();
    println!("PRET");
    true
}

#[test]
#[ignore]
fn inner_termination_clean() {
    if !inner_ready() {
        return;
    }
    while !azure_core::security::termination::requested() {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    println!("ARRET PROPRE");
}

#[test]
#[ignore]
fn inner_termination_stuck() {
    if inner_ready() {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
}
