// Test d'integration "bout en bout" du systeme de routes (voir
// `azure_foundation::navigation`) : DEUX apps simulees (A et B) qui parlent
// au VRAI routeur `azure-rooter` (un vrai socket Unix, pas de mock), sans
// jamais ouvrir de fenetre Wayland - contrairement a
// `window_integration_test.rs`, rien ici n'a besoin d'un compositeur, donc
// ce test tourne dans un `cargo test` normal (pas `--ignored`).
//
// Scenario : App A affiche un bouton. Un clic dessus (simule via
// `event::services::dispatch::handle_event`, exactement le code qu'utilise
// une vraie `AzureWindow`) declenche `navigation_manager::navigate` vers
// App B, lui demandant d'afficher un nouveau texte. App B ecoute le
// routeur (`navigation_manager::listen`) et resout la route recue via sa
// `RouteTable`, exactement comme le fait `AzureWindow::run` a chaque tic -
// voir `window::models::window`, le bras `on_tick`.
use azure_core::rules::window_event::WindowEvent;
use azure_engine::rendering::models::color::Color;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch;
use azure_foundation::layout::models::layout_props::LayoutProps;
use azure_foundation::navigation::managers::navigation_manager;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::button::Button;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::ui_node::UiNode;
use std::time::Duration;

// SURTOUT PAS `azure_rooter::SOCKET_PATH` (le chemin reel utilise par
// `routeur_daemon`/les apps de demo) : un premier essai de ce test
// partageait ce chemin, et supprimait/re-creait le fichier de socket a
// chaque execution - cassant silencieusement n'importe quel routeur reel
// lance a la main en parallele (voir `start_router_at`/`connect_at`, ajoutes
// pour permettre cette isolation). Suffixe par le PID pour que deux
// executions de `cargo test` concurrentes (ou un run precedent tue sans
// nettoyer) ne se marchent pas dessus non plus.
fn router_socket_path() -> String {
    format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-router-test-{}.sock"), std::process::id())
}
const APP_A_ID: u32 = 101;
const APP_B_ID: u32 = 102;

/// Demarre un routeur `azure-rooter` DEDIE a ce test, sur un chemin isole
/// (voir `router_socket_path`), dans un thread a part (comme le fait
/// `routeur_daemon`) pour toute la duree du test. Supprime d'abord un socket
/// laisse par une precedente execution interrompue (`start_router_at`
/// echouerait sinon avec "address already in use") - sans effet s'il
/// n'existe pas.
fn start_test_router(path: &str) {
    let _ = std::fs::remove_file(path);
    let path = path.to_string();
    std::thread::spawn(move || {
        azure_rooter::managers::router::start_router_at(&path).expect("le routeur de test aurait du demarrer");
    });
}

/// `navigation_manager::connect_at` echoue si appele avant que le thread du
/// routeur ait fini son `bind` (voir `start_test_router`) - reessaie plutot
/// que d'exiger un `sleep` fixe avant le premier appel.
fn connect_with_retry(path: &str, app_id: u32) -> azure_foundation::navigation::models::navigation_client::NavigationClient {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        match navigation_manager::connect_at(path, app_id) {
            Ok(client) => return client,
            Err(err) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
                let _ = err;
            }
            Err(err) => panic!("connexion au routeur de test impossible: {err}"),
        }
    }
}

fn full_box(w: u32, h: u32) -> (u32, u32, u32, u32) {
    (0, 0, w, h)
}

#[test]
fn clicking_app_a_s_button_makes_app_b_update_its_text_via_the_router() {
    let socket = router_socket_path();
    start_test_router(&socket);

    let mut nav_a = connect_with_retry(&socket, APP_A_ID);
    let nav_b = connect_with_retry(&socket, APP_B_ID);

    // `register` (cote client, voir `azure_rooter::services::client`)
    // retourne des que l'ECRITURE reussit, pas une fois que le thread du
    // routeur a reellement insere la connexion dans sa table (aucun accuse
    // de reception dans ce protocole, voir `azure_rooter::managers::router`) -
    // sans ce court delai, le `navigate` d'App A plus bas peut arriver avant
    // que le routeur connaisse App B et se faire ignorer silencieusement
    // ("Destinataire introuvable").
    std::thread::sleep(Duration::from_millis(100));

    // App B commence sur un ecran d'attente, et branche sa table de routes
    // AVANT de se mettre a ecouter - exactement l'ordre que suit
    // `AzureWindow::run` (voir `navigation`/`routes` puis le listener
    // demarre dans `run`).
    let mut app_b_state = EventState::new(vec![UiNode::Label(Label::new(
        LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0),
        "En attente...".to_string(),
        Color::new(255, 255, 255, 255),
        16.0,
        400.0,
    ))]);
    let app_b_routes = RouteTable::new().on("/update-text", |payload| {
        vec![UiNode::Label(Label::new(
            LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0),
            payload.to_string(),
            Color::new(255, 255, 255, 255),
            16.0,
            400.0,
        ))]
    });
    let incoming_routes = navigation_manager::listen(&nav_b).expect("App B aurait du pouvoir ecouter le routeur");

    // App A : un unique bouton plein cadre, comme dans les tests de
    // `event::services::dispatch`.
    let mut app_a_state = EventState::new(vec![UiNode::Button(Button::new(
        LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0),
        Color::new(0, 0, 0, 255),
        false,
        "Envoyer a App B".to_string(),
    ))]);
    let content = full_box(200, 100);
    app_a_state.mouse_x = 10;
    app_a_state.mouse_y = 10;

    // Le clic lui-meme : le meme `dispatch::handle_event` qu'appelle une
    // vraie `AzureWindow` dans son callback `on_event` (voir
    // `window::models::window::run`).
    let changed = dispatch::handle_event(
        &mut app_a_state,
        WindowEvent::WindowMouseButton(BTN_LEFT, true),
        azure_foundation::ui::services::interact::KeyboardLayout::Qwerty,
        content,
    );
    assert!(changed, "le clic aurait du toggler le bouton d'App A");
    match &app_a_state.ui_nodes[0] {
        UiNode::Button(button) => assert!(button.state, "le bouton d'App A aurait du passer a l'etat presse"),
        _ => unreachable!(),
    }

    // C'est ICI, dans le code d'App A (pas dans le framework - voir la
    // discussion dans `navigation/mod.rs`), que le clic se traduit en
    // navigation vers App B.
    navigation_manager::navigate(&mut nav_a, APP_B_ID, "/update-text", "Salut depuis App A !")
        .expect("App A aurait du pouvoir envoyer sa route a App B");

    // Cote App B : exactement ce que fait `AzureWindow::run` a chaque tic
    // (voir le bras `on_tick`) - drainer le canal, resoudre via la table de
    // routes, remplacer `ui_nodes`.
    let route = match incoming_routes
        .recv_timeout(Duration::from_secs(2))
        .expect("App B aurait du recevoir la route envoyee par App A")
    {
        azure_foundation::navigation::models::incoming::Incoming::Route(route) => route,
        azure_foundation::navigation::models::incoming::Incoming::Window(_) => panic!("App B attendait une route, pas une fenetre"),
    };
    let resolved = app_b_routes.resolve(&route).expect("'/update-text' est enregistree dans app_b_routes");
    app_b_state.ui_nodes = resolved;

    match &app_b_state.ui_nodes[0] {
        UiNode::Label(label) => assert_eq!(label.text, "Salut depuis App A !"),
        _ => unreachable!("App B aurait du afficher un Label apres la navigation"),
    }
}

// Pendant INTRA-app du test ci-dessus : meme scenario (un clic sur App A
// change un ecran), mais transporte par `IntraRouter` (100% en memoire,
// voir `azure_rooter::managers::intra_router`) plutot que par le routeur
// socket - aucun thread, aucun `/tmp/...sock`, resultat visible tout de
// suite apres un seul `drain`.
#[test]
fn clicking_app_a_s_button_locally_switches_its_own_screen_via_the_intra_router() {
    use azure_foundation::navigation::managers::intra_navigation_manager;
    use azure_rooter::managers::intra_router::IntraRouter;

    const APP_A_VIEW_ID: u32 = 1;

    let shared_router = IntraRouter::new();
    let intra_a = intra_navigation_manager::connect(&shared_router, APP_A_VIEW_ID);

    let mut app_a_state = EventState::new(vec![UiNode::Button(Button::new(
        LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0),
        Color::new(0, 0, 0, 255),
        false,
        "Confirmer".to_string(),
    ))]);
    let app_a_routes = RouteTable::new().on("/confirmed", |_payload| {
        vec![UiNode::Label(Label::new(
            LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0),
            "Confirme !".to_string(),
            Color::new(255, 255, 255, 255),
            16.0,
            400.0,
        ))]
    });
    let content = full_box(200, 100);
    app_a_state.mouse_x = 10;
    app_a_state.mouse_y = 10;

    let changed = dispatch::handle_event(
        &mut app_a_state,
        WindowEvent::WindowMouseButton(BTN_LEFT, true),
        azure_foundation::ui::services::interact::KeyboardLayout::Qwerty,
        content,
    );
    assert!(changed, "le clic aurait du toggler le bouton d'App A");

    // C'est ICI (pas dans le framework) que le clic se traduit en
    // navigation intra-app - exactement ce que fait `WindowContext::goto`
    // depuis `AzureWindow::on_click`.
    intra_navigation_manager::navigate(&intra_a, intra_a.view_id, "/confirmed", "");

    // Exactement ce que fait `AzureWindow::run` a chaque tic (voir le bras
    // `on_tick`) : sonder l'IntraRouter, resoudre via la table de routes.
    let resolved = intra_navigation_manager::drain(&intra_a, &app_a_routes)
        .expect("le message auto-adresse aurait du resoudre '/confirmed'");
    app_a_state.ui_nodes = resolved;

    match &app_a_state.ui_nodes[0] {
        UiNode::Label(label) => assert_eq!(label.text, "Confirme !"),
        _ => unreachable!("App A aurait du afficher un Label apres la navigation intra-app"),
    }
}
