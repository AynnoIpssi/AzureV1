// Le tableau de bord hors fenetre : un etat realiste (fabrique par le vrai
// `Manager`), chaque page dessinee dans target/tmp/dashboard-<page>.ppm, et
// les clics traduits en actions.
use azure_dashboard::{action, routes, Action, Dashboard, Kind};
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use std::path::Path;

mod common;
use common::{dashboard, state};

fn collect(nodes: &[UiNode], texts: &mut Vec<String>, buttons: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => texts.push(l.text.clone()),
            UiNode::Button(b) => buttons.push(b.id.clone()),
            UiNode::Container(c) => collect(&c.children, texts, buttons),
            _ => {}
        }
    }
}

fn render(dashboard: &Dashboard, path: &str, file: &str) -> (Vec<String>, Vec<String>) {
    let table = routes(&Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"), dashboard);
    let nodes = table.resolve(&Route::new(path, "")).unwrap();
    assert!(!nodes.is_empty(), "page {path} vide (rsH ou rsC invalide ?)");
    let (w, h) = (1180, 780);
    let mut canvas = Canvas::new(w, h);
    draw_ui(&nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("dashboard-{file}.ppm")), ppm).unwrap();
    let (mut texts, mut buttons) = (Vec::new(), Vec::new());
    collect(&nodes, &mut texts, &mut buttons);
    (texts, buttons)
}

#[test]
fn every_page_renders_with_the_manager_state() {
    let d = dashboard();
    let (texts, buttons) = render(&d, "/", "apps");
    assert!(texts.contains(&"3 / 4 apps actives".to_string()), "{texts:?}");
    assert!(texts.contains(&"partage « panier » avec caisse".to_string()) && texts.contains(&"partage « annonces » avec tout le monde".to_string()), "{texts:?}");
    assert!(texts.contains(&"arrêtée".to_string()));
    assert!(buttons.contains(&"app-boutique".to_string()) && buttons.contains(&"app-stats".to_string()));
    assert_eq!(d.current(), "/");

    let (texts, buttons) = render(&d, "/app/boutique", "app");
    assert!(texts.contains(&"Boutique".to_string()) && texts.contains(&"active (pid 4101)".to_string()), "{texts:?}");
    assert!(texts.contains(&"Déclarent l'écouter : caisse, stats".to_string()), "{texts:?}");
    assert!(buttons.contains(&"retirer-0-0".to_string()), "retirer caisse du panier : {buttons:?}");
    assert!(buttons.contains(&"autoriser-0-0".to_string()) && buttons.contains(&"prive-1".to_string()), "{buttons:?}");
    assert!(!buttons.iter().any(|b| b.starts_with("autoriser-1-")), "pas de liste d'apps pour un flux public");
    assert!(texts.contains(&"Prix d'un produit (en centimes)".to_string()) && texts.contains(&"Déclarent l'appeler : caisse, stats".to_string()), "{texts:?}");
    assert!(buttons.contains(&"aretirer-0-0".to_string()) && buttons.contains(&"aautoriser-0-0".to_string()), "{buttons:?}");
    assert_eq!(d.current(), "/app/boutique");

    assert!(texts.contains(&"128 modification(s) · 2 écoute(s) en cours".to_string()) && texts.contains(&"persistant".to_string()), "{texts:?}");
    assert!(texts.contains(&"57 appel(s) · 3 erreur(s) · 1 délai(s) dépassé(s) · 4 ms en moyenne".to_string()), "{texts:?}");
    assert!(texts.contains(&"Dernière erreur : produit inconnu".to_string()) && texts.contains(&"paiement refuse (carte expiree)".to_string()), "{texts:?}");
    assert!(buttons.contains(&"journal-boutique".to_string()));
    assert!(texts.contains(&"Permissions : sans reseau · stockage".to_string()) && texts.contains(&"enfermée".to_string()), "{texts:?}");

    let (texts, _) = render(&d, "/evenements", "evenements");
    assert!(texts.contains(&"caisse ouverte".to_string()) && texts.contains(&"erreur".to_string()), "{texts:?}");
    let (texts, buttons) = render(&d, "/journaux", "journaux");
    assert!(buttons.contains(&"journal-stockage".to_string()) && buttons.contains(&"journal-caisse".to_string()), "{buttons:?} {texts:?}");
    let (texts, _) = render(&d, "/journaux/stockage", "journal");
    assert!(texts.contains(&"connexion de l'app 1000".to_string()), "{texts:?}");
    let (texts, _) = render(&d, "/journaux/caisse", "journal-vide");
    assert!(texts.iter().any(|t| t.contains("caisse.log introuvable")), "{texts:?}");

    let (texts, _) = render(&d, "/liens", "liens");
    assert!(texts.contains(&"écoute « panier » de".to_string()) && texts.contains(&"refusé".to_string()), "{texts:?}");
    assert!(texts.contains(&"appelle « prix » de".to_string()), "{texts:?}");

    let (texts, buttons) = render(&d, "/services", "services");
    assert!(texts.contains(&"en echec".to_string()), "{texts:?}");
    assert!(buttons.contains(&"relancer-boutique-sync".to_string()));

    let (texts, buttons) = render(&d, "/terminal", "terminal-vide");
    assert!(texts.iter().any(|t| t.starts_with("Tapez « install »")), "{texts:?}");
    assert!(buttons.contains(&"terminal-executer".to_string()));
    let mut d = d;
    d.runner = Some(std::sync::Arc::new(|_: &[String]| Ok((1, vec!["azure : binaire 'azure_note' introuvable (compilez l'app : cargo build --release, ou donnez --bin <chemin>)".to_string()]))));
    d.execute("aide");
    d.execute("install ~/Dev/Azure/azure-note");
    let (texts, _) = render(&d, "/terminal", "terminal");
    assert!(texts.contains(&"$ azure install ~/Dev/Azure/azure-note".to_string()), "{texts:?}");
    assert!(texts.contains(&"échec (code 1)".to_string()), "{texts:?}");
}

#[test]
fn clicks_become_manager_actions() {
    let state = state();
    let s = |v: &str| v.to_string();
    assert_eq!(action("nav-liens", "/", &state), Some(Action::Goto(s("/liens"))));
    assert_eq!(action("app-caisse", "/", &state), Some(Action::Goto(s("/app/caisse"))));
    assert_eq!(action("retirer-0-0", "/app/boutique", &state), Some(Action::Revoke { kind: Kind::Flux, owner: s("boutique"), name: s("panier"), app: s("caisse") }));
    // `autres` du panier : stats, tableau-de-bord (ordre alphabetique).
    assert_eq!(action("autoriser-0-0", "/app/boutique", &state), Some(Action::Grant { kind: Kind::Flux, owner: s("boutique"), name: s("panier"), app: s("stats") }));
    assert_eq!(action("prive-1", "/app/boutique", &state), Some(Action::SetPublic { kind: Kind::Flux, owner: s("boutique"), name: s("annonces"), public: false }));
    // Les memes boutons pour une methode, prefixes par `a`.
    assert_eq!(action("aautoriser-0-0", "/app/boutique", &state), Some(Action::Grant { kind: Kind::Call, owner: s("boutique"), name: s("prix"), app: s("stats") }));
    assert_eq!(action("aretirer-0-0", "/app/boutique", &state), Some(Action::Revoke { kind: Kind::Call, owner: s("boutique"), name: s("prix"), app: s("caisse") }));
    assert_eq!(action("apublic-0", "/app/boutique", &state), Some(Action::SetPublic { kind: Kind::Call, owner: s("boutique"), name: s("prix"), public: true }));
    assert_eq!(action("relancer-boutique-sync", "/services", &state), Some(Action::Restart(s("boutique-sync"))));
    assert_eq!(action("retirer-9-0", "/app/boutique", &state), None, "ligne inexistante");
    assert_eq!(action("journal-stockage", "/journaux", &state), Some(Action::Goto(s("/journaux/stockage"))));
    assert_eq!(action("journal-actualiser", "/journaux/stockage", &state), Some(Action::Goto(s("/journaux/stockage"))));
    assert_eq!(action("nav-evenements", "/", &state), Some(Action::Goto(s("/evenements"))));
    assert_eq!(action("retirer-0-0", "/liens", &state), None, "pas sur la page d'une app");
}
