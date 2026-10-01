// La fenetre Doc, remplie par les memes fonctions qu'Azure Docs sert
// (azure_docs::service) sur le vrai contenu de la documentation.
use azure_docs::{service, Docs};
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::flux::Value;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_note::doc::{self, expliquer, Memoire, SourceDoc};
use std::path::Path;
use std::sync::{Arc, Mutex};

mod common;
use common::*;

const VUE: (u32, u32, u32, u32) = (0, 0, doc::TAILLE.0, doc::TAILLE.1);

struct Locale(Docs);

impl SourceDoc for Locale {
    fn appeler(&self, methode: &str, args: Value) -> Result<Value, String> {
        match methode {
            "chercher" => service::chercher(&self.0, &args),
            "page" => service::page(&self.0, &args),
            autre => Err(format!("méthode inconnue : {autre}")),
        }
    }

    fn ouvrir_dans_docs(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Azure Docs absente (pas installee, service en panne).
struct Absente;

impl SourceDoc for Absente {
    fn appeler(&self, methode: &str, _: Value) -> Result<Value, String> {
        Err(format!("'{methode}' de l'app 1003 n'est pas disponible (app arretee, ou methode pas servie)"))
    }

    fn ouvrir_dans_docs(&self, _: &str) -> Result<(), String> {
        Err("non".into())
    }
}

fn locale() -> Locale {
    Locale(Docs::charger(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../azure-docs/contenu")).unwrap())
}

fn table(source: Arc<dyn SourceDoc>, memoire: &Arc<Mutex<Memoire>>) -> RouteTable {
    let rsh = ui().join("doc.rsh").to_string_lossy().into_owned();
    let rsc = ui().join("doc.rsc").to_string_lossy().into_owned();
    let (s1, m1, s2, m2) = (source.clone(), memoire.clone(), source, memoire.clone());
    RouteTable::new().view_with("/", &rsh, &rsc, move |r| doc::resultats(&*s1, &m1, &r.payload)).view_with("/page", &rsh, &rsc, move |r| doc::page(&*s2, &m2, &r.payload))
}

#[test]
fn chercher_puis_ouvrir_une_page() {
    let memoire = Arc::new(Mutex::new(Memoire::default()));
    let t = table(Arc::new(locale()), &memoire);
    let e = EventState::new(t.resolve(&Route::new("/", "flexbox")).unwrap());
    let textes = common::textes(&e.ui_nodes);
    assert!(textes.iter().any(|t| t.contains("résultat(s) pour « flexbox »")), "{textes:?}");
    assert!(textes.contains(&"Flexbox".to_string()), "{textes:?}");
    assert_eq!(champ(&e.ui_nodes, "q").as_deref(), Some("flexbox"));
    capture(&e.ui_nodes, (VUE.2, VUE.3), "doc-resultats");

    // Le premier resultat est touchable et mene a sa page.
    let mut e = e;
    assert_eq!(cliquer(&mut e, "r-0", VUE).as_deref(), Some("r-0"));
    let cible = memoire.lock().unwrap().cibles[0].clone();
    assert_eq!(cible, "/doc/rsc/flex");
    let e = EventState::new(t.resolve(&Route::new("/page", &cible)).unwrap());
    let textes = common::textes(&e.ui_nodes);
    assert!(textes.contains(&"Flexbox".to_string()) && textes.contains(&"rsc.flex.1".to_string()), "{textes:?}");
    let ids = boutons(&e.ui_nodes, (0, 0, 760, 20000));
    for id in ["retour", "ouvrir-docs", "copier-rsc_flex_1"] {
        assert!(ids.contains(&id.to_string()), "{id} dans {ids:?}");
    }
    // Copier donne le code tel qu'ecrit dans la doc.
    let docs = locale().0;
    assert_eq!(doc::code(&memoire, "rsc_flex_1").as_deref(), Some(docs.exemple("rsc.flex.1").unwrap().1.code.trim_end()));
    capture(&e.ui_nodes, (VUE.2, VUE.3), "doc-page");
}

#[test]
fn un_exemple_cherche_par_son_identifiant_est_montre_en_tete() {
    let memoire = Arc::new(Mutex::new(Memoire::default()));
    let t = table(Arc::new(locale()), &memoire);
    t.resolve(&Route::new("/", "rsc.flex.2")).unwrap();
    let cible = memoire.lock().unwrap().cibles[0].clone();
    assert_eq!(cible, "exemple:rsc.flex.2");
    let e = EventState::new(t.resolve(&Route::new("/page", &cible)).unwrap());
    let textes = common::textes(&e.ui_nodes);
    let tete = textes.iter().position(|t| t == "L'EXEMPLE CHERCHÉ").expect("l'exemple en tete");
    assert_eq!(textes[tete + 3], "rsc.flex.2", "{textes:?}");
    assert!(boutons(&e.ui_nodes, (0, 0, 760, 20000)).contains(&"vu-copier-rsc_flex_2".to_string()));
    capture(&e.ui_nodes, (VUE.2, VUE.3), "doc-exemple");
}

#[test]
fn docs_absente_le_dit_clairement() {
    let memoire = Arc::new(Mutex::new(Memoire::default()));
    let t = table(Arc::new(Absente), &memoire);
    let e = EventState::new(t.resolve(&Route::new("/", "flex")).unwrap());
    let textes = common::textes(&e.ui_nodes);
    assert!(textes.contains(&"Azure Docs ne répond pas : est-elle installée ? (azure install azure-docs)".to_string()), "{textes:?}");
    assert!(boutons(&e.ui_nodes, VUE).contains(&"reessayer".to_string()));
    assert_eq!(memoire.lock().unwrap().derniere, ("/".to_string(), "flex".to_string()), "Réessayer refait la même recherche");
    assert!(expliquer("Acces refuse").starts_with("Azure Docs a répondu"));
    capture(&e.ui_nodes, (VUE.2, VUE.3), "doc-absente");
}
