// Le tableau de bord comme un utilisateur : chaque bouton de chaque page est
// clique a sa vraie position (defilement compris), via la meme chaine
// d'evenements qu'une fenetre ; le bouton touche doit etre le bon, et son
// action celle attendue. Puis un parcours : liste -> app -> autoriser.
use azure_core::rules::window_event::WindowEvent;
use azure_dashboard::{action, routes, Action, Dashboard, Kind};
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::interact::{self, animate_scroll, KeyboardLayout};
use std::path::Path;

mod common;
use common::dashboard;

const VIEW: (u32, u32, u32, u32) = (0, 0, 1180, 780);

fn open(d: &Dashboard, path: &str) -> EventState {
    let table = routes(&Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"), d);
    EventState::new(table.resolve(&Route::new(path, "")).unwrap())
}

fn buttons(nodes: &[UiNode]) -> Vec<String> {
    let mut out = Vec::new();
    interact::walk(nodes, VIEW, &mut |n, _| {
        if let UiNode::Button(b) = n
            && !b.id.is_empty() {
                out.push(b.id.clone());
            }
    });
    out
}

fn box_of(nodes: &[UiNode], id: &str) -> Option<(i32, i32, u32, u32)> {
    let mut found = None;
    interact::walk(nodes, VIEW, &mut |n, b| {
        if matches!(n, UiNode::Button(x) if x.id == id) {
            found = Some(b);
        }
    });
    found
}

/// Clique au centre du bouton `id` (en le faisant defiler dans la vue s'il
/// le faut) ; rend l'id que la fenetre a vraiment touche.
fn click(state: &mut EventState, id: &str) -> Option<String> {
    let visible = |b: (i32, i32, u32, u32)| b.1 >= 0 && b.1 + b.3 as i32 <= VIEW.3 as i32 && b.2 > 0 && b.3 > 0;
    if !box_of(&state.ui_nodes, id).is_some_and(visible) {
        interact::scroll_to_anchor(&mut state.ui_nodes, id, VIEW);
        while animate_scroll(&mut state.ui_nodes) {}
    }
    let b = box_of(&state.ui_nodes, id)?;
    let (x, y) = (b.0 + b.2 as i32 / 2, b.1 + b.3 as i32 / 2);
    state.clicked_id = None;
    handle_event(state, WindowEvent::WindowMouseMove(x, y), KeyboardLayout::Qwerty, VIEW);
    handle_event(state, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, VIEW);
    handle_event(state, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, VIEW);
    state.clicked_id.clone()
}

#[test]
fn every_button_of_every_page_is_reachable_and_does_something() {
    let d = dashboard();
    let pages = ["/", "/liens", "/services", "/evenements", "/journaux", "/journaux/stockage", "/app/boutique", "/app/caisse"];
    let mut total = 0;
    for page in pages {
        let mut state = open(&d, page);
        let ids = buttons(&state.ui_nodes);
        assert!(!ids.is_empty(), "{page} : aucun bouton");
        for id in ids {
            // Page fraiche a chaque fois (un clic peut faire defiler).
            let mut fresh = open(&d, page);
            let hit = click(&mut fresh, &id);
            assert_eq!(hit.as_deref(), Some(id.as_str()), "{page} : le clic sur #{id} touche autre chose");
            assert!(action(&id, page, &d.state()).is_some(), "{page} : #{id} ne fait rien");
            total += 1;
        }
        let _ = &mut state;
    }
    assert!(total > 30, "{total} boutons verifies");
}

#[test]
fn a_user_goes_from_the_list_to_an_app_and_grants_access() {
    let d = dashboard();
    let mut state = open(&d, "/");
    let hit = click(&mut state, "app-boutique").expect("carte de la boutique");
    let Some(Action::Goto(path)) = action(&hit, "/", &d.state()) else { panic!("{hit}") };
    assert_eq!(path, "/app/boutique");

    let mut state = open(&d, &path);
    assert_eq!(d.current(), "/app/boutique", "page ouverte retenue");
    let hit = click(&mut state, "autoriser-0-0").expect("bouton autoriser");
    assert_eq!(
        action(&hit, &d.current(), &d.state()),
        Some(Action::Grant { kind: Kind::Flux, owner: "boutique".into(), name: "panier".into(), app: "stats".into() })
    );
    let hit = click(&mut state, "nav-services").expect("menu");
    let Some(Action::Goto(path)) = action(&hit, &d.current(), &d.state()) else { panic!() };
    let mut state = open(&d, &path);
    let hit = click(&mut state, "relancer-boutique-sync").expect("bouton relancer");
    assert_eq!(action(&hit, &d.current(), &d.state()), Some(Action::Restart("boutique-sync".into())));
}

#[test]
fn oublier_une_app_demande_confirmation_et_une_erreur_s_affiche() {
    let d = dashboard();
    let state = d.state();
    let page = "/app/stats";
    assert_eq!(action("oublier", page, &state), Some(Action::AskForget("stats".into())));
    assert_eq!(action("oublier-oui", page, &state), Some(Action::Forget("stats".into())));
    assert_eq!(action("oublier-non", page, &state), Some(Action::Goto(page.into())));
    assert_eq!(action("erreur-fermer", page, &state), Some(Action::Goto(page.into())));

    let table = routes(&Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"), &d);
    let texte = |payload: &str| {
        let nodes = table.resolve(&Route::new(page, payload)).unwrap();
        (buttons(&nodes), nodes)
    };
    let (avant, _) = texte("");
    assert!(avant.contains(&"oublier".to_string()) && !avant.contains(&"oublier-oui".to_string()));
    let (confirmation, _) = texte("oublier");
    assert!(confirmation.contains(&"oublier-oui".to_string()) && confirmation.contains(&"oublier-non".to_string()), "{confirmation:?}");

    // Une action refusee s'affiche en haut de la page, avec de quoi la fermer.
    d.set_error("Action refusée : 'stats' tourne encore");
    let (boutons, _) = texte("");
    assert!(boutons.contains(&"erreur-fermer".to_string()));
    d.set_error("");
    assert!(!texte("").0.contains(&"erreur-fermer".to_string()));
}

#[test]
fn terminal() {
    use azure_dashboard::split_command;
    assert_eq!(split_command(r#"install "~/Mes apps/note" --bin a\ b"#).unwrap(), vec!["install", "~/Mes apps/note", "--bin", "a b"]);
    assert_eq!(split_command("run  ''  x").unwrap(), vec!["run", "", "x"]);
    assert!(split_command("install \"x").is_err());

    let mut d = dashboard();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = std::sync::Arc::clone(&seen);
    d.runner = Some(std::sync::Arc::new(move |args: &[String]| {
        s.lock().unwrap().push(args.to_vec());
        match args[0].as_str() {
            "list" => Ok((0, vec!["note   Azure Note".to_string()])),
            _ => Ok((1, vec!["azure : binaire introuvable".to_string()])),
        }
    }));
    // Le mot `azure` devant est facultatif.
    d.execute("azure list");
    d.execute("install ~/x");
    assert_eq!(*seen.lock().unwrap(), vec![vec!["list".to_string()], vec!["install".to_string(), "~/x".to_string()]]);
    let genres: Vec<String> = d.terminal().iter().map(|l| l.get("genre").unwrap().as_str().unwrap().to_string()).collect();
    assert_eq!(genres, ["cmd", "out", "ok", "cmd", "err", "err"]);
    d.execute("effacer");
    assert!(d.terminal().is_empty());
    d.execute("aide");
    assert!(d.terminal().len() > 3);

    // La page, ses boutons, et Entree dans le champ.
    let mut state = open(&d, "/terminal");
    for id in ["terminal-list", "terminal-aide", "terminal-effacer", "terminal-executer"] {
        assert_eq!(click(&mut state, id).as_deref(), Some(id));
    }
    let st = d.state();
    assert_eq!(action("commande", "/terminal", &st), Some(Action::Execute));
    assert_eq!(action("terminal-list", "/terminal", &st), Some(Action::Command("list".into())));
    assert_eq!(action("nav-terminal", "/", &st), Some(Action::Goto("/terminal".into())));
    // Depuis la liste des apps.
    let mut apps = open(&d, "/");
    assert_eq!(click(&mut apps, "nav-terminal").as_deref(), Some("nav-terminal"));
}

#[test]
fn terminal_runs_long_commands_in_the_background() {
    let mut d = dashboard();
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let rx = std::sync::Mutex::new(rx);
    d.runner = Some(std::sync::Arc::new(move |_: &[String]| {
        rx.lock().unwrap().recv().unwrap();
        Ok((0, vec!["meteo compilee".to_string()]))
    }));
    d.start("build meteo --installer");
    assert_eq!(d.running(), "azure build meteo --installer");
    assert!(d.take_refresh());
    // Une seule a la fois.
    d.start("list");
    assert!(d.terminal().last().unwrap().get("texte").unwrap().as_str().unwrap().contains("tourne encore"));
    tx.send(()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !d.running().is_empty() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(d.take_refresh());
    let texts: Vec<String> = d.terminal().iter().map(|l| l.get("texte").unwrap().as_str().unwrap().to_string()).collect();
    assert!(texts.ends_with(&["meteo compilee".to_string(), "terminé".to_string()]), "{texts:?}");
}

#[test]
fn an_app_is_built_from_its_page() {
    let mut d = dashboard();
    assert_eq!(action("compiler", "/app/boutique", &d.state()), Some(Action::Build("boutique".to_string())));
    assert_eq!(action("compiler", "/", &d.state()), None, "seulement sur la page d'une app");
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let rx = std::sync::Mutex::new(rx);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let args_seen = seen.clone();
    d.runner = Some(std::sync::Arc::new(move |args: &[String]| {
        *args_seen.lock().unwrap() = args.to_vec();
        rx.lock().unwrap().recv().unwrap();
        let mut lines: Vec<String> = (0..20).map(|n| format!("ligne {n}")).collect();
        lines.push("boutique compilee".to_string());
        Ok((0, lines))
    }));
    d.start_build("boutique").unwrap();
    assert_eq!(d.build_of("boutique").unwrap().state, "en_cours");
    assert!(d.build_of("caisse").is_none());
    assert!(d.start_build("caisse").unwrap_err().contains("tourne encore"), "une seule commande a la fois");
    tx.send(()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !d.running().is_empty() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(*seen.lock().unwrap(), ["build", "boutique", "--installer"]);
    let build = d.build_of("boutique").unwrap();
    assert_eq!(build.state, "ok");
    assert_eq!(build.lines.len(), azure_dashboard::BUILD_LINES, "seulement la fin");
    assert_eq!(build.lines.last().unwrap(), "boutique compilee");
    // Toute la sortie est aussi dans le terminal.
    assert!(d.terminal().iter().any(|l| l.get("texte").unwrap().as_str() == Some("ligne 0")));
    // La page de l'app montre le resultat.
    let table = routes(&Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"), &d);
    let nodes = table.resolve(&Route::new("/app/boutique", "")).unwrap();
    let mut texts = Vec::new();
    fn walk(nodes: &[UiNode], out: &mut Vec<String>) {
        for n in nodes {
            match n {
                UiNode::Label(l) => out.push(l.text.clone()),
                UiNode::Container(c) => walk(&c.children, out),
                _ => {}
            }
        }
    }
    walk(&nodes, &mut texts);
    assert!(texts.contains(&"compilée et installée".to_string()) && texts.contains(&"boutique compilee".to_string()), "{texts:?}");
}
