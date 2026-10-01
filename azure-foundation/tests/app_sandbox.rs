// Une app enfermee (processus a part : c'est definitif) ne voit que son
// dossier, la police, les dossiers systeme et ses permissions.
use azure_foundation::app::sandbox_for;
use azure_manager::models::manifest::Manifest;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("app-sandbox-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn bundle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/app/boutique")
}

#[test]
fn an_app_only_reaches_what_its_manifest_allows() {
    if azure_core::security::sandbox::abi_version() < 1 {
        return;
    }
    let (documents, secret) = (tmp("documents"), tmp("secret"));
    std::fs::write(documents.join("lettre.txt"), "bonjour").unwrap();
    std::fs::write(secret.join("master.key"), "cle").unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "inner_enclosed_app", "--ignored", "--nocapture"])
        .env("DOCUMENTS", &documents)
        .env("SECRET", &secret)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("RESULTAT dossier-ok police-ok lecture-ok ecriture-refusee secret-refuse reseau-refuse"), "{text}");
}

#[test]
#[ignore]
fn inner_enclosed_app() {
    let documents = std::env::var("DOCUMENTS").unwrap();
    let secret = PathBuf::from(std::env::var("SECRET").unwrap());
    let manifest = Manifest::parse(&format!("[app]\nname = boutique\n[permissions]\nlecture = {documents}\n"), &bundle()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    sandbox_for(&manifest).unwrap().apply().unwrap();
    let ok = |cond: bool, yes: &'static str, no: &'static str| if cond { yes } else { no };
    let r = [
        "RESULTAT",
        ok(std::fs::read(bundle().join("app.azure")).is_ok(), "dossier-ok", "dossier-KO"),
        ok(std::fs::read(azure_foundation::ui::services::draw_ui::FONT_PATH).is_ok(), "police-ok", "police-KO"),
        ok(std::fs::read(Path::new(&documents).join("lettre.txt")).is_ok(), "lecture-ok", "lecture-KO"),
        ok(std::fs::write(Path::new(&documents).join("x.txt"), "x").is_err(), "ecriture-refusee", "ecriture-FAITE"),
        ok(std::fs::read(secret.join("master.key")).is_err(), "secret-refuse", "secret-LU"),
        ok(std::net::TcpStream::connect(("127.0.0.1", port)).is_err() || azure_core::security::sandbox::abi_version() < 4, "reseau-refuse", "reseau-OUVERT"),
    ];
    println!("{}", r.join(" "));
    // Enfermee, elle ne lance pas de daemon.
    assert!(azure_core::security::sandbox::is_sandboxed());
}

#[test]
fn permissions_cannot_open_azure_data() {
    let home = std::env::var("HOME").unwrap();
    for bad in ["~", "~/.local/share/azure", "~/.local", "~/.config/azure/provider.conf", "relatif"] {
        let text = format!("[app]\nname = a\n[permissions]\nlecture = {bad}\n");
        let error = Manifest::parse(&text, Path::new("/x")).unwrap_err();
        assert!(error.contains("[permissions]"), "{bad} : {error}");
    }
    let ok = Manifest::parse("[app]\nname = a\n[permissions]\nlecture = ~/Documents\necriture = /mnt/disque\nreseau = true\nstockage = false\n", Path::new("/x")).unwrap();
    assert_eq!(ok.permissions.read, [PathBuf::from(home).join("Documents")]);
    assert!(ok.permissions.network && !ok.permissions.storage);
    // Pas enfermee en developpement si demande ; toujours une fois installee.
    let dev = Manifest::parse("[app]\nname = a\n[permissions]\nbac_a_sable = false\n", Path::new("/x")).unwrap();
    assert!(sandbox_for(&dev).is_none());
    let installed = Manifest::parse("[app]\nname = a\n[permissions]\nbac_a_sable = false\n", &azure_provider::install_root().join("apps/a")).unwrap();
    assert!(sandbox_for(&installed).is_some());
}
