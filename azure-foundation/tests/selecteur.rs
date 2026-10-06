// La boite « Ouvrir » (voir `azure_foundation::selecteur`) : elle parcourt
// de vrais dossiers, entre dans un dossier au clic, remonte, cache les
// fichiers caches, filtre par extension, et rend le chemin choisi. Le rendu
// est ecrit dans <target>/tmp/selecteur-*.ppm pour le regarder.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::selecteur::{abreger, Ouvert, Selecteur, Suite};
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use std::path::{Path, PathBuf};

const FENETRE: (u32, u32, u32, u32) = (0, 0, 1100, 700);

// Un dossier jetable : projets/{alpha,beta,.cache}, projets/alpha/{src,
// Cargo.toml, schema.merise, base.sql, notes.txt}.
fn dossier(nom: &str) -> PathBuf {
    let racine = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("selecteur-{nom}"));
    let _ = std::fs::remove_dir_all(&racine);
    for d in ["projets/alpha/src", "projets/beta", "projets/.cache"] {
        std::fs::create_dir_all(racine.join(d)).unwrap();
    }
    for f in ["Cargo.toml", "schema.merise", "base.SQL", "notes.txt"] {
        std::fs::write(racine.join("projets/alpha").join(f), "x").unwrap();
    }
    racine.canonicalize().unwrap()
}

fn ouvrir(demande: Selecteur, racine: &Path) -> Ouvert {
    Ouvert::avec_lieux(demande.depart(&racine.join("projets").to_string_lossy()), vec![racine.join("projets")])
}

fn noms(o: &Ouvert) -> Vec<&str> {
    o.entrees().iter().map(|e| e.nom.as_str()).collect()
}

// Le rang de l'entree `nom` : son bouton est `sel-entree-<rang>`.
fn entree(o: &Ouvert, nom: &str) -> String {
    format!("sel-entree-{}", o.entrees().iter().position(|e| e.nom == nom).unwrap_or_else(|| panic!("pas d'entree {nom}")))
}

fn ids(nodes: &[UiNode], out: &mut Vec<String>) {
    for n in nodes {
        match n {
            UiNode::Button(b) if !b.id.is_empty() => out.push(b.id.clone()),
            UiNode::Container(c) => ids(&c.children, out),
            _ => {}
        }
    }
}

fn image(o: &Ouvert, nom: &str) {
    let mut canvas = Canvas::new(FENETRE.2, FENETRE.3);
    draw_ui(&o.noeuds(), FENETRE, &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(format!("{}/selecteur-{nom}.ppm", env!("CARGO_TARGET_TMPDIR")), ppm).unwrap();
}

#[test]
fn choisir_un_dossier_en_le_parcourant() {
    let racine = dossier("dossier");
    let mut o = ouvrir(Selecteur::dossier().titre("Relier un projet"), &racine);
    assert_eq!(o.dossier(), racine.join("projets"));
    assert_eq!(noms(&o), ["alpha", "beta"], "dossiers par nom, le dossier cache n'est pas montre");

    assert_eq!(o.cliquer(&entree(&o, "alpha"), false), Suite::Liste);
    assert_eq!(o.dossier(), racine.join("projets/alpha"));
    assert_eq!(noms(&o), ["src", "base.SQL", "Cargo.toml", "notes.txt", "schema.merise"], "les dossiers d'abord, puis sans regarder la casse");
    // Un fichier ne se choisit pas quand on cherche un dossier.
    assert_eq!(o.cliquer(&entree(&o, "Cargo.toml"), true), Suite::Rien);
    image(&o, "dossier");

    assert_eq!(o.cliquer("sel-choisir", false), Suite::Choisi(racine.join("projets/alpha")));
}

#[test]
fn remonter_fil_et_lieux() {
    let racine = dossier("remonter");
    let mut o = ouvrir(Selecteur::dossier(), &racine);
    o.cliquer(&entree(&o, "alpha"), false);
    o.cliquer(&entree(&o, "src"), false);
    assert_eq!(o.dossier(), racine.join("projets/alpha/src"));

    assert_eq!(o.cliquer("sel-parent", false), Suite::Liste);
    assert_eq!(o.dossier(), racine.join("projets/alpha"));

    // Le seul lieu : projets.
    assert_eq!(o.cliquer("sel-lieu-0", false), Suite::Liste);
    assert_eq!(o.dossier(), racine.join("projets"));
    assert_eq!(o.cliquer("sel-lieu-7", false), Suite::Rien);

    // Le fil d'Ariane : son premier element est la racine du systeme (ou ~).
    o.cliquer(&entree(&o, "alpha"), false);
    let mut boutons = Vec::new();
    ids(&o.noeuds(), &mut boutons);
    let fil: Vec<&String> = boutons.iter().filter(|b| b.starts_with("sel-fil-")).collect();
    assert!(!fil.is_empty(), "les dossiers parents sont des boutons : {boutons:?}");
    // L'avant-dernier element du fil est le parent.
    assert_eq!(o.cliquer(fil.last().unwrap(), false), Suite::Liste);
    assert_eq!(o.dossier(), racine.join("projets"));
}

#[test]
fn fichiers_caches() {
    let racine = dossier("caches");
    let mut o = ouvrir(Selecteur::dossier(), &racine);
    assert_eq!(o.cliquer("sel-caches", false), Suite::Liste);
    assert_eq!(noms(&o), [".cache", "alpha", "beta"]);
    o.cliquer("sel-caches", false);
    assert_eq!(noms(&o), ["alpha", "beta"]);
}

#[test]
fn choisir_un_fichier() {
    let racine = dossier("fichier");
    let mut o = ouvrir(Selecteur::fichier().titre("Importer").extensions(&["merise", ".sql"]), &racine);
    // Rien n'est choisi : « Ouvrir » ne rend rien.
    assert_eq!(o.cliquer("sel-choisir", false), Suite::Rien);
    o.cliquer(&entree(&o, "alpha"), false);
    assert_eq!(noms(&o), ["src", "base.SQL", "schema.merise"], "seulement les extensions demandees, sans regarder la casse");

    // Un clic choisit, la liste ne bouge pas ; « Ouvrir » valide.
    let merise = entree(&o, "schema.merise");
    assert_eq!(o.cliquer(&merise, false), Suite::Garder);
    image(&o, "fichier");
    assert_eq!(o.cliquer("sel-choisir", false), Suite::Choisi(racine.join("projets/alpha/schema.merise")));

    // Le double-clic valide aussi : le premier clic choisit, le second ouvre.
    let sql = entree(&o, "base.SQL");
    assert_eq!(o.cliquer(&sql, false), Suite::Garder);
    assert_eq!(o.cliquer(&sql, true), Suite::Choisi(racine.join("projets/alpha/base.SQL")));

    // Changer de dossier oublie le choix.
    o.cliquer(&merise, false);
    o.cliquer("sel-parent", false);
    assert_eq!(o.cliquer("sel-choisir", false), Suite::Rien);
}

#[test]
fn depart_et_annulation() {
    let racine = dossier("depart");
    // Un fichier, ou un chemin qui n'existe pas : le dossier parent qui existe.
    for depart in ["projets/alpha/Cargo.toml", "projets/alpha/pas/encore/la"] {
        let o = Ouvert::avec_lieux(Selecteur::dossier().depart(&racine.join(depart).to_string_lossy()), Vec::new());
        assert_eq!(o.dossier(), racine.join("projets/alpha"), "{depart}");
    }
    // Rien de demande : le premier lieu.
    let mut o = Ouvert::avec_lieux(Selecteur::dossier().depart("  "), vec![racine.join("projets/beta")]);
    assert_eq!(o.dossier(), racine.join("projets/beta"));
    assert_eq!(noms(&o), [] as [&str; 0]);

    assert_eq!(o.cliquer("sel-annuler", false), Suite::Annuler);
    assert_eq!(o.cliquer("selecteur-fermer", false), Suite::Annuler, "Echap vise le bouton -fermer");
    assert_eq!(o.cliquer("autre-chose", false), Suite::Rien);
}

#[test]
fn dossier_illisible() {
    use std::os::unix::fs::PermissionsExt;
    let racine = dossier("illisible");
    let ferme = racine.join("projets/beta");
    std::fs::set_permissions(&ferme, std::fs::Permissions::from_mode(0o000)).unwrap();
    let mut o = ouvrir(Selecteur::dossier(), &racine);
    o.cliquer(&entree(&o, "beta"), false);
    let lisible = std::fs::read_dir(&ferme).is_ok();
    std::fs::set_permissions(&ferme, std::fs::Permissions::from_mode(0o755)).unwrap();
    // root lit tout : rien a verifier.
    if lisible {
        return;
    }
    assert!(o.erreur().contains("[permissions]"), "{}", o.erreur());
    assert_eq!(o.cliquer("sel-choisir", false), Suite::Rien, "un dossier illisible ne se choisit pas");
    image(&o, "illisible");
    // On en sort par le parent.
    o.cliquer("sel-parent", false);
    assert_eq!(noms(&o), ["alpha", "beta"]);
}

#[test]
fn chemin_abrege() {
    let maison = PathBuf::from(std::env::var_os("HOME").unwrap());
    assert_eq!(abreger(&maison.join("Dev/projet")), "~/Dev/projet");
    assert_eq!(abreger(&maison), "~");
    assert_eq!(abreger(Path::new("/usr/share")), "/usr/share");
}
