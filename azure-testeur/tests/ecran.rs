// L'ecran, comme dans l'app : relier un vrai petit crate, voir ses tests,
// les lancer (vrai `cargo test`), puis ecrire un test dans l'atelier et le
// lancer. Images dans target/tmp/azure-testeur/<nom>.ppm.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::form::{form_values, FieldValue};
use azure_foundation::ui::services::interact;
use azure_testeur::clics::{cliquer, Lecture};
use azure_testeur::ecran::{routes, Testeur};
use azure_testeur::projet::{self, EnMemoire, Projets};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const VUE: (u32, u32) = (1280, 820);

struct Valeurs(BTreeMap<String, FieldValue>);

impl Lecture for Valeurs {
    fn valeur(&self, id: &str) -> Option<String> {
        self.0.get(id).map(FieldValue::as_text)
    }

    fn coche(&self, id: &str) -> bool {
        matches!(self.0.get(id), Some(FieldValue::Bool(true)))
    }
}

struct Appli {
    t: Arc<Testeur>,
    nodes: Vec<UiNode>,
}

fn ui() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("ui")
}

fn textes(nodes: &[UiNode], out: &mut Vec<String>) {
    for n in nodes {
        match n {
            UiNode::Label(l) => out.push(l.text.clone()),
            UiNode::Button(b) => out.push(b.text.clone()),
            UiNode::TextArea(t) => out.push(t.text.clone()),
            UiNode::Container(c) => textes(&c.children, out),
            _ => {}
        }
    }
}

impl Appli {
    fn new(env: Option<projet::Projet>) -> Appli {
        let t = Testeur::new(Projets::new(env, Box::new(EnMemoire::default())));
        let mut a = Appli { t, nodes: Vec::new() };
        a.redessiner();
        a
    }

    fn redessiner(&mut self) {
        self.nodes = routes(&ui(), &self.t).resolve(&Route::new("/", "")).expect("rsH ou rsC invalide");
    }

    fn ids(&self) -> Vec<String> {
        let mut out = Vec::new();
        interact::walk(&self.nodes, (0, 0, VUE.0, 100_000), &mut |n, _| match n {
            UiNode::Button(b) if !b.id.is_empty() => out.push(b.id.clone()),
            UiNode::TextArea(t) if !t.id.is_empty() => out.push(t.id.clone()),
            UiNode::Control(c) if !c.id.is_empty() => out.push(c.id.clone()),
            _ => {}
        });
        out
    }

    fn textes(&self) -> String {
        let mut out = Vec::new();
        textes(&self.nodes, &mut out);
        out.join("\n")
    }

    fn clic(&mut self, id: &str) {
        assert!(self.ids().contains(&id.to_string()), "pas de #{id} à l'écran : {:?}", self.ids());
        let valeurs = Valeurs(form_values(&self.nodes));
        cliquer(&self.t, id, &valeurs);
        self.redessiner();
    }

    fn remplir(&mut self, id: &str, valeur: &str) {
        fn aller(nodes: &mut [UiNode], id: &str, valeur: &str) -> bool {
            for n in nodes {
                match n {
                    UiNode::TextArea(t) if t.id == id => {
                        t.text = valeur.to_string();
                        if let Some(r) = &mut t.rich {
                            r.styles = vec![Default::default(); valeur.chars().count()];
                        }
                        return true;
                    }
                    UiNode::Container(c) => {
                        if aller(&mut c.children, id, valeur) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }
        assert!(aller(&mut self.nodes, id, valeur), "pas de champ #{id}");
    }

    /// Attend la fin de l'execution lancee.
    fn attendre(&mut self) {
        let fin = Instant::now() + Duration::from_secs(300);
        while self.t.lanceur.lire().en_cours {
            assert!(Instant::now() < fin, "l'execution ne finit pas");
            std::thread::sleep(Duration::from_millis(100));
        }
        self.redessiner();
    }

    fn capture(&self, nom: &str) {
        let (w, h) = VUE;
        let mut canvas = Canvas::new(w, h);
        draw_ui(&self.nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        for px in canvas.buffer.chunks(4) {
            ppm.extend_from_slice(&[px[2], px[1], px[0]]);
        }
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-testeur");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join(format!("{nom}.ppm")), ppm).unwrap();
    }
}

/// Un petit crate avec un test qui passe et un qui echoue.
fn crate_demo(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("azure-testeur-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::write(d.join("Cargo.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n").unwrap();
    std::fs::write(
        d.join("src/lib.rs"),
        "pub fn double(x: i32) -> i32 {\n    x * 2\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn double_ok() {\n        assert_eq!(double(2), 4);\n    }\n\n    #[test]\n    fn double_faux() {\n        assert_eq!(double(2), 5, \"2 x 2 ne fait pas 5\");\n    }\n}\n",
    )
    .unwrap();
    d
}

#[test]
fn relier_lancer_puis_ecrire_un_test() {
    let demo = crate_demo("ecran");
    let mut a = Appli::new(None);
    assert!(a.textes().contains("Aucun projet"));

    // Relier le projet.
    a.remplir("chemin", &demo.to_string_lossy());
    a.clic("lier");
    let texte = a.textes();
    assert!(texte.contains("tests::double_ok") && texte.contains("tests::double_faux"), "{texte}");
    assert!(texte.contains("2 test(s) trouvé(s)"));

    // Tout lancer (vrai cargo test).
    a.clic("tout-lancer");
    a.attendre();
    let (ok, ko) = {
        let e = a.t.etat();
        let exec = a.t.lanceur.lire();
        let statut = |nom: &str| exec.statut(&e.analyse.tests.iter().find(|x| x.nom == nom).unwrap().cle());
        (statut("double_ok"), statut("double_faux"))
    };
    assert_eq!((ok.code(), ko.code()), ("reussi", "echoue"), "sortie : {:?}", a.t.lanceur.lire().lignes);
    let i = a.t.etat().analyse.tests.iter().position(|x| x.nom == "double_faux").unwrap();
    a.clic(&format!("voir-{i}"));
    assert!(a.textes().contains("2 x 2 ne fait pas 5"), "la sortie de l'échec est montrée");
    a.clic("console");
    a.capture("tests");
    a.clic("console");

    // Atelier : nouveau fichier de tests, son code generique, un test.
    a.clic("onglet-atelier");
    a.clic("nouveau-fichier");
    a.remplir("nf-nom", "calculs");
    a.clic("nf-creer");
    let fichier = demo.join("tests/calculs.rs");
    assert!(std::fs::read_to_string(&fichier).unwrap().contains("use demo::*;"));
    a.clic("ouvrir-nouveau");
    a.remplir("nouveau-nom", "double_de_trois");
    a.remplir("nouveau-code", "let x = trois();\nassert_eq!(double(x), 6);");
    a.clic("ouvrir-generique");
    a.remplir("generique", "use demo::*;\n\nfn trois() -> i32 {\n    3\n}");
    a.clic("generique-enregistrer");
    // Le nouveau test tape avant d'ouvrir le code generique est garde.
    a.clic("ouvrir-nouveau");
    a.clic("creer-test");
    assert_eq!(
        std::fs::read_to_string(&fichier).unwrap(),
        "use demo::*;\n\nfn trois() -> i32 {\n    3\n}\n\n#[test]\nfn double_de_trois() {\n    let x = trois();\n    assert_eq!(double(x), 6);\n}\n"
    );
    assert!(a.textes().contains("Test « double_de_trois » créé"));
    a.clic("lancer-at-0");
    a.attendre();
    assert!(a.textes().contains("réussi"), "{:?}", a.t.lanceur.lire().lignes);
    a.capture("atelier");

    // Le test cree est ouvert ; les autres restent une simple liste.
    assert!(a.ids().contains(&"code-0".to_string()));
    // Modifier (renommer + corps + option), puis supprimer.
    a.remplir("nom-0", "double_de_trois_ignore");
    a.remplir("code-0", "assert_eq!(double(trois()), 6);");
    a.clic("enregistrer-0");
    let texte = std::fs::read_to_string(&fichier).unwrap();
    assert!(texte.contains("fn double_de_trois_ignore() {\n    assert_eq!(double(trois()), 6);\n}"), "{texte}");
    a.clic("supprimer-0");
    a.clic("supprimer-oui-0");
    assert!(!std::fs::read_to_string(&fichier).unwrap().contains("#[test]"));
    let _ = std::fs::remove_dir_all(&demo);
}

/// L'environnement Azure (s'il est installe ici) : analyse sans lancer.
#[test]
fn environnement_azure() {
    let Some(env) = projet::environnement() else { return };
    let mut a = Appli::new(Some(env));
    let nb = a.t.etat().analyse.tests.len();
    assert!(nb > 300, "{nb} tests trouvés dans Azure");
    a.capture("environnement");
    a.clic("ouvrir-tout");
    a.clic("fermer-tout");
    a.clic("groupe-0");
    a.clic("voir-0");
    a.capture("environnement-detail");
    a.clic("onglet-atelier");
    // Liste legere : aucun editeur de code tant qu'on n'a rien ouvert.
    assert!(!a.ids().iter().any(|i| i.starts_with("code-") || i == "generique"), "{:?}", a.ids());
    let dessin = |a: &Appli| {
        let debut = Instant::now();
        let mut canvas = Canvas::new(VUE.0, VUE.1);
        draw_ui(&a.nodes, (0, 0, VUE.0, VUE.1), &mut canvas, -1, -1, false);
        debut.elapsed()
    };
    let d = dessin(&a);
    assert!(d < Duration::from_millis(80), "l'atelier se dessine en {d:?}");
    a.capture("environnement-atelier");
    a.clic("ouvrir-0");
    assert!(a.ids().contains(&"code-0".to_string()));
    eprintln!("atelier : {d:?} ; un test ouvert : {:?}", dessin(&a));
    a.capture("environnement-atelier-ouvert");
}

/// Les positions de defilement de l'ecran (conteneurs qui ont defile).
fn defilement(nodes: &[UiNode]) -> Vec<u32> {
    let mut out = Vec::new();
    fn aller(nodes: &[UiNode], out: &mut Vec<u32>) {
        for n in nodes {
            if let UiNode::Container(c) = n {
                if c.scroll_target > 0 {
                    out.push(c.scroll_target);
                }
                aller(&c.children, out);
            }
        }
    }
    aller(nodes, &mut out);
    out
}

/// L'atelier ne remonte pas en haut apres une action (un message apparait
/// en haut : l'ecran change de forme, le defilement doit suivre).
#[test]
fn atelier_garde_le_defilement() {
    let demo = crate_demo("defilement");
    let mut a = Appli::new(None);
    a.remplir("chemin", &demo.to_string_lossy());
    a.clic("lier");
    a.clic("fermer-message");
    a.clic("onglet-atelier");
    a.clic("ouvrir-generique");
    a.clic("ouvrir-0");
    // Fenetre basse : la page du petit crate doit defiler.
    let vue = (0, 0, VUE.0, 380);
    for _ in 0..8 {
        interact::scroll_at(&mut a.nodes, 900, 300, 60.0, vue);
    }
    let avant = defilement(&a.nodes);
    assert!(!avant.is_empty(), "la page défile");
    // Comme la fenetre : clic, puis nouvel ecran qui garde le defilement.
    for id in ["enregistrer-0", "fermer-message", "supprimer-0", "supprimer-non"] {
        let valeurs = Valeurs(form_values(&a.nodes));
        cliquer(&a.t, id, &valeurs);
        let vieux = std::mem::take(&mut a.nodes);
        a.redessiner();
        interact::carry_scroll(&vieux, &mut a.nodes);
        assert_eq!(defilement(&a.nodes), avant, "après #{id}");
    }
    let _ = std::fs::remove_dir_all(&demo);
}

/// Bibliotheque de composants : un stockage, des dossiers dans des
/// dossiers, un composant avec son code ; puis ce composant ajoute a un
/// nouveau test de l'atelier, ecrit dans le vrai crate.
#[test]
fn composants_en_dossiers_puis_dans_un_nouveau_test() {
    let demo = crate_demo("composants");
    let mut a = Appli::new(None);
    a.clic("comp-nouveau-stockage");
    assert!(a.ids().contains(&"comp-nom".to_string()), "la page du stockage s'ouvre");
    a.remplir("comp-nom", "Mes modèles");
    a.clic("comp-renommer");
    a.clic("comp-dossier");
    a.remplir("comp-nom", "Calculs");
    a.clic("comp-renommer");
    a.clic("comp-dossier");
    a.remplir("comp-nom", "Doubles");
    a.clic("comp-renommer");
    a.clic("comp-composant");
    a.remplir("comp-nom", "double_de_deux");
    a.clic("comp-renommer");
    a.remplir("comp-code", "let x = double(2);\nassert_eq!(x, 4);");
    a.clic("comp-enregistrer");
    let texte = a.textes();
    for nom in ["Mes modèles", "Calculs", "Doubles", "double_de_deux"] {
        assert!(texte.contains(nom), "{nom} dans l'arbre : {texte}");
    }
    let chemin = {
        let b = a.t.bibliotheque();
        let c = b.noeuds.iter().find(|n| n.nom == "double_de_deux").unwrap();
        assert_eq!(c.code, "let x = double(2);\nassert_eq!(x, 4);");
        b.chemin_texte(c.id)
    };
    assert_eq!(chemin, "Mes modèles › Calculs › Doubles › double_de_deux");
    a.capture("composants");

    // Plier le stockage cache tout ce qu'il contient.
    let stockage = a.t.bibliotheque().noeuds[0].id;
    a.clic(&format!("plier-{stockage}"));
    assert!(!a.ids().contains(&"noeud-2".to_string()), "{:?}", a.ids());
    a.clic(&format!("plier-{stockage}"));

    // Dans un nouveau test : le composant s'ajoute au code.
    a.remplir("chemin", &demo.to_string_lossy());
    a.clic("lier");
    a.clic("onglet-atelier");
    a.clic("nouveau-fichier");
    a.remplir("nf-nom", "avec_composant");
    a.clic("nf-creer");
    a.clic("ouvrir-nouveau");
    a.remplir("nouveau-nom", "depuis_un_composant");
    a.clic("inserer-composant");
    a.capture("composant-insere");
    a.clic("creer-test");
    let src = std::fs::read_to_string(demo.join("tests/avec_composant.rs")).unwrap();
    assert!(src.contains("fn depuis_un_composant()") && src.contains("let x = double(2);\n    assert_eq!(x, 4);"), "{src}");

    // Supprimer un dossier supprime ce qu'il contient.
    a.clic("onglet-composants");
    let calculs = a.t.bibliotheque().noeuds.iter().find(|n| n.nom == "Calculs").unwrap().id;
    a.clic(&format!("noeud-{calculs}"));
    a.clic("comp-supprimer");
    a.clic("comp-supprimer-oui");
    assert_eq!(a.t.bibliotheque().noeuds.len(), 1, "il reste le stockage");
    let _ = std::fs::remove_dir_all(demo);
}
