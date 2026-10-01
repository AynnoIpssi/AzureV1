// L'app est enfermee (Landlock) avec les permissions de son manifeste :
// `cargo test` doit y marcher sur un projet de ~/Dev (lire la chaine
// d'outils, ecrire target/ et ~/.cargo). Le processus lance est enferme
// avec les memes regles que l'app.
use azure_manager::models::manifest::Manifest;
use azure_testeur::langage::{rust::Rust, Etape, Langage};
use std::os::unix::process::CommandExt;
use std::path::Path;

#[test]
fn cargo_test_tourne_dans_le_bac_a_sable() {
    let manifeste = Manifest::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("app.azure")).unwrap();
    let Some(bac) = azure_foundation::app::sandbox_for(&manifeste) else { panic!("l'app doit être enfermée") };
    // Un projet sous ~/Dev (le dossier target d'Azure y est).
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-testeur-bac");
    let home = std::env::var("HOME").unwrap();
    if !d.starts_with(format!("{home}/Dev")) {
        eprintln!("projet hors de ~/Dev : test sans objet");
        return;
    }
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::write(d.join("Cargo.toml"), "[package]\nname = \"bac\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n").unwrap();
    std::fs::write(d.join("src/lib.rs"), "#[test]\nfn un() { assert_eq!(1, 1); }\n").unwrap();
    let mut cmd = Rust.commande(&d, &Etape { paquet: String::new(), ..Etape::default() }).unwrap();
    let pret = bac.prepare().unwrap();
    // SAFETY : `enforce` ne fait que des appels systeme.
    unsafe { cmd.pre_exec(move || pret.enforce()) };
    let sortie = cmd.output().unwrap();
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    assert!(sortie.status.success() && texte.contains("test un ... ok"), "{texte}");
}
