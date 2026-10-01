// Note et Docs pour de vrai : les daemons d'Azure tournent dans ce test,
// chaque app se presente avec son vrai manifeste, Docs sert `chercher` et
// `page`, Note les appelle (droits donnes par les manifestes). Les notes
// passent par le vrai stockage.
use azure_docs::{service, Docs};
use azure_foundation::app::{AppSockets, AzureApp};
use azure_foundation::flux::Value;
use azure_note::classeur::Classeur;
use azure_note::modele::{Affichage, Bloc, Definition, Filtre, Genre, GenreBloc, Operateur, Vue};
use azure_note::Carnet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn attendre(socket: &str) {
    let fin = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(socket).is_err() {
        assert!(Instant::now() < fin, "{socket} ne repond pas");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn demarrer() -> AppSockets {
    let pid = std::process::id();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("azure-note-apps");
    let _ = std::fs::remove_dir_all(&dir);
    let moi = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let sock = |nom: &str| format!("{}/note-{nom}-{pid}.sock", env!("CARGO_TARGET_TMPDIR"));
    let sockets = AppSockets { manager: sock("manager"), service: sock("service"), stockage: sock("stockage"), router: sock("router"), provider: false, sandbox: false };
    azure_service::managers::daemon::set_manager_socket(&sockets.manager);
    azure_stockage::managers::daemon::set_manager_socket(&sockets.manager);
    let (s, d, m) = (sockets.clone(), dir.clone(), moi.clone());
    std::thread::spawn(move || azure_service::managers::daemon::start_daemon_with(&s.service, &d.join("service"), Some(m)));
    let (s, d) = (sockets.clone(), dir.clone());
    std::thread::spawn(move || azure_stockage::managers::daemon::start_daemon_at(&s.stockage, &d.join("stockage")));
    attendre(&sockets.service);
    let config = azure_manager::managers::daemon::ManagerConfig {
        socket: sockets.manager.clone(),
        data_dir: dir.join("manager"),
        service_socket: sockets.service.clone(),
        provider_socket: "/tmp/aucun-provider.sock".into(),
        admins: vec![moi],
        provider_logs: dir.join("logs-provider"),
        app_logs: dir.join("logs-apps"),
    };
    std::thread::spawn(move || azure_manager::managers::daemon::run(config));
    attendre(&sockets.manager);
    attendre(&sockets.stockage);
    sockets
}

fn racine() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn note_cherche_dans_docs_et_garde_ses_notes() {
    let sockets = demarrer();
    let docs = AzureApp::from_manifest_at(racine().join("../azure-docs/app.azure"), sockets.clone()).unwrap();
    let note = AzureApp::from_manifest_at(racine().join("app.azure"), sockets.clone()).unwrap();

    // Docs fermee (rien de servi) : Note le sait tout de suite.
    let e = note.call("docs", "chercher", Value::map([("q", Value::from("flex"))])).unwrap_err();
    assert!(e.contains("n'est pas disponible"), "{e}");

    let contenu = racine().join("../azure-docs/contenu");
    let servir = |methode: &str, f: fn(&Docs, &Value) -> Result<Value, String>| {
        let contenu = contenu.clone();
        docs.serve(methode, move |req| f(&Docs::charger(&contenu)?, &req.args)).unwrap()
    };
    let _servies = [servir("chercher", service::chercher), servir("page", service::page)];

    let trouves = note.call_wait("docs", "chercher", Value::map([("q", Value::from("flexbox"))]), Duration::from_secs(5)).unwrap();
    let premier = &trouves.as_list().unwrap()[0];
    assert_eq!(premier.get("chemin").and_then(Value::as_str), Some("/doc/rsc/flex"));
    let page = note.call("docs", "page", Value::map([("exemple", Value::from("rsc.flex.2"))])).unwrap();
    assert_eq!(page.get("titre").and_then(Value::as_str), Some("Flexbox"));
    assert_eq!(page.get("exemple").and_then(Value::as_str), Some("rsc_flex_2"));
    assert!(page.get("blocs").and_then(Value::as_list).is_some_and(|b| b.len() > 5));
    let e = note.call("docs", "page", Value::map([("chemin", Value::from("/doc/rien/du-tout"))])).unwrap_err();
    assert!(e.contains("page introuvable"), "{e}");

    // Seule Note y a droit (`to = note` dans le manifeste de Docs).
    assert!(docs.call("docs", "chercher", Value::Null).unwrap_err().contains("Declarez [use chercher@docs]"));

    // Les notes survivent a un redemarrage de l'app.
    let carnet = Carnet::charger(note.stockage().unwrap()).unwrap();
    let a = carnet.nouvelle().unwrap();
    carnet.modifier(a, "Flex", "voir rsc.flex.2\nligne 2").unwrap();
    let b = carnet.nouvelle().unwrap();
    carnet.modifier(b, "Jetable", "x").unwrap();
    carnet.supprimer(b).unwrap();
    let relu = Carnet::charger(note.stockage().unwrap()).unwrap();
    assert_eq!(relu.notes(), carnet.notes());
    assert_eq!(relu.note(a).unwrap().texte, "voir rsc.flex.2\nligne 2");
    assert!(relu.note(b).is_none());
    assert!(!note.stockage().unwrap().has(&format!("note.{b}.texte")).unwrap(), "la note supprimee ne laisse rien");

    // v2 : les notes v1 deviennent des pages, une seule fois.
    let classeur = Classeur::ouvrir(note.stockage().unwrap()).unwrap();
    let pages = classeur.espace();
    assert_eq!(pages.pages.len(), 1);
    let flex = pages.enfants(None)[0].clone();
    assert_eq!(flex.titre, "Flex");
    assert_eq!(flex.blocs.iter().map(|b| b.contenu.as_str()).collect::<Vec<_>>(), vec!["voir rsc.flex.2", "ligne 2"]);

    // Tout ce que l'espace sait faire, relu a l'identique dans RsS.
    classeur
        .modifier(|e| {
            let base = e.creer_base(None, "Tâches", 1)?;
            e.definir(base, Definition { nom: "Avancement".into(), genre: Genre::Nombre }, true)?;
            e.definir(base, Definition { nom: "Moyenne".into(), genre: Genre::Formule("moyenne(enfants.avancement)".into()) }, true)?;
            let t = e.creer(Some(base), "Écrire \"RsS\" ; DROP TABLE pages", 1)?;
            e.changer(t, "avancement", "42,5")?;
            e.changer(t, "étiquettes", "a, b")?;
            e.changer(t, "statut", "En cours")?;
            let p = e.page_mut(base)?;
            p.vues.push(Vue { nom: "Mes filtres".into(), affichage: Affichage::Galerie, groupe: "statut".into(), filtres: vec![Filtre { propriete: "statut".into(), operateur: Operateur::Contient, valeur: "cours".into() }], tri: Some(("avancement".into(), true)) });
            let page = e.page_mut(flex.id)?;
            page.blocs.push(Bloc { id: 9, genre: GenreBloc::Code("rust".into()), contenu: "fn main() {}\n// fin".into() });
            page.blocs.push(Bloc { id: 10, genre: GenreBloc::Texte, contenu: "g\u{1f}#e06c75\u{1f}\u{1f}gras rouge\u{1e}\u{1f}\u{1f}\u{1f} normal".into() });
            page.blocs.push(Bloc { id: 11, genre: GenreBloc::Tache(true), contenu: "fait".into() });
            e.definir(flex.id, Definition { nom: "Priorité".into(), genre: Genre::Selection(vec!["Haute".into(), "Basse".into()]) }, false)?;
            e.changer(flex.id, "priorité", "Haute")
        })
        .unwrap();
    let relu = Classeur::ouvrir(note.stockage().unwrap()).unwrap();
    assert_eq!(relu.espace(), classeur.espace());
    assert_eq!(relu.espace().pages.len(), 3, "pas de 2e migration");
    // Une suppression efface aussi les lignes RsS.
    classeur.modifier(|e| Ok(e.supprimer(flex.id))).unwrap();
    assert_eq!(Classeur::ouvrir(note.stockage().unwrap()).unwrap().espace(), classeur.espace());
}
