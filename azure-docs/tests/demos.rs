// Les demonstrations : chaque bloc `> demo` mene a des fenetres qui se
// construisent, et le Recepteur affiche bien ce que l'Emetteur envoie.
// Les ecrans sont rendus dans target/tmp/azure-docs/demo-<nom>.ppm.
use azure_docs::contenu::Bloc;
use azure_docs::demos::{self, recepteur, Etat, DEMOS, TAILLES};
use azure_docs::Docs;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use std::path::{Path, PathBuf};

fn dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn fichier(nom: &str) -> String {
    dir().join("ui/demos").join(nom).to_string_lossy().into_owned()
}

fn ecran(rsh: &str, rsc: &str, contexte: impl Fn(&str) -> Context + Send + 'static) -> Vec<UiNode> {
    RouteTable::new().view_with("/", &fichier(rsh), &fichier(rsc), move |r| contexte(&r.payload)).resolve(&Route::new("/", "")).expect("ecran de demonstration")
}

fn textes(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => out.push(l.text.clone()),
            UiNode::Button(b) => out.push(format!("[{}]", b.text)),
            UiNode::Container(c) => textes(&c.children, out),
            _ => {}
        }
    }
}

fn tout(nodes: &[UiNode]) -> String {
    let mut out = Vec::new();
    textes(nodes, &mut out);
    out.join("\n")
}

fn capture(nodes: &[UiNode], nom: &str, (w, h): (u32, u32)) -> PathBuf {
    let mut canvas = Canvas::new(w, h);
    draw_ui(nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-docs");
    std::fs::create_dir_all(&out).unwrap();
    let chemin = out.join(format!("demo-{nom}.ppm"));
    std::fs::write(&chemin, ppm).unwrap();
    chemin
}

#[test]
fn chaque_bloc_demo_nomme_une_demonstration_connue_et_chacune_sert() {
    let docs = Docs::charger(&dir().join("contenu")).unwrap();
    let mut vues = Vec::new();
    for page in docs.pages() {
        for bloc in &page.blocs {
            if let Bloc::Demo { nom, texte } = bloc {
                assert!(DEMOS.contains(&nom.as_str()), "{} : démonstration inconnue « {nom} »", page.fichier.display());
                assert!(!texte.is_empty(), "{} : démonstration « {nom} » sans texte", page.fichier.display());
                vues.push(nom.clone());
            }
        }
    }
    for nom in DEMOS {
        assert!(vues.iter().any(|v| v == nom), "démonstration « {nom} » citée par aucune page");
    }
}

#[test]
fn chaque_demonstration_donne_de_vraies_fenetres() {
    let table = demos::fenetres(&dir().join("ui"), 7, None);
    let taille = |chemin: &str, payload: &str| {
        let w = table.resolve(chemin, payload).unwrap_or_else(|| panic!("aucune fenêtre pour {chemin}"));
        let spec = w.window_spec().expect("une fenêtre ouverte par open_window a une spec");
        assert_eq!(spec.owner_app_id(), 7);
        (spec.size().width(), spec.size().height())
    };
    taille("/demo/fenetre", "0");
    taille("/demo/partagee", "");
    taille("/demo/routeur/emetteur", "0");
    taille("/demo/routeur/recepteur", "0");
    for (i, (_, l, h)) in TAILLES.iter().enumerate() {
        assert_eq!(taille("/demo/taille", &i.to_string()), (*l, *h));
    }
}

#[test]
fn le_recepteur_affiche_ce_que_l_emetteur_envoie() {
    let vide = ecran("recepteur.rsh", "demos.rsc", recepteur);
    assert!(tout(&vide).contains("En attente de l'Émetteur"), "{}", tout(&vide));
    capture(&vide, "recepteur-attente", (500, 420));

    let mut etat = Etat::default();
    assert_eq!(etat.appliquer("envoyer", Some("   ")), None, "un message vide ne part pas");
    etat.appliquer("envoyer", Some("Salut Récepteur")).unwrap();
    etat.appliquer("plus", None).unwrap();
    etat.appliquer("plus", None).unwrap();
    let dernier = etat.appliquer("c-sauge", None).unwrap();
    assert_eq!(etat.appliquer("inconnu", None), None);
    let message = etat.message(&dernier);

    let recu = ecran("recepteur.rsh", "demos.rsc", move |_| recepteur(&message));
    let texte = tout(&recu);
    for attendu in ["Salut Récepteur", "2", "4", "couleur sauge"] {
        assert!(texte.lines().any(|l| l == attendu), "« {attendu} » absent de :\n{texte}");
    }
    capture(&recu, "recepteur", (500, 420));
    capture(&ecran("emetteur.rsh", "demos.rsc", |_| Context::new()), "emetteur", (500, 460));
}

#[test]
fn les_autres_ecrans_se_dessinent() {
    let n = ecran("fenetre.rsh", "demos.rsc", |_| Context::new().with_text("n", "3"));
    assert!(tout(&n).contains("[Cliquer]"));
    capture(&n, "fenetre", (540, 380));
    let (nom, l, h) = TAILLES[1];
    let t = ecran("taille.rsh", "demos.rsc", move |_| Context::new().with_text("nom", nom).with_text("largeur", &l.to_string()).with_text("hauteur", &h.to_string()));
    assert!(tout(&t).contains("560 × 380"), "{}", tout(&t));
    capture(&t, "taille", (l, h));
    capture(&ecran("note.rsh", "note.rsc", |_| Context::new()), "note", (460, 280));
}
