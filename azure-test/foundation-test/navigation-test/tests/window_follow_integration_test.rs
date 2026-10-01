// Fenetres envoyees entre apps via le VRAI routeur azure-rooter (socket Unix
// privee, voir `router_socket_path`), sans compositeur : on verifie qui
// RECOIT la fenetre (`navigation_manager::listen`), pas son affichage.
//
// Scenario : A suit B, C ne suit personne. B envoie une fenetre a ses
// abonnes (seul A la recoit), puis une fenetre a toutes les apps (A et C la
// recoivent, jamais B lui-meme).
use azure_core::models::window_model::*;
use azure_foundation::navigation::managers::navigation_manager;
use azure_foundation::navigation::models::incoming::Incoming;
use azure_foundation::navigation::models::navigation_client::NavigationClient;
use azure_foundation::window::models::shared_window::SharedWindow;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

const APP_A: u32 = 201;
const APP_B: u32 = 202;
const APP_C: u32 = 203;

fn router_socket_path() -> String {
    format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-router-window-test-{}.sock"), std::process::id())
}

fn start_test_router(path: &str) {
    let _ = std::fs::remove_file(path);
    let path = path.to_string();
    std::thread::spawn(move || {
        azure_rooter::managers::router::start_router_at(&path).expect("le routeur de test aurait du demarrer");
    });
}

fn connect_with_retry(path: &str, app_id: u32) -> NavigationClient {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match navigation_manager::connect_at(path, app_id) {
            Ok(client) => return client,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Err(err) => panic!("connexion au routeur de test impossible: {err}"),
        }
    }
}

fn window_from_b(scope: WindowScope, title: &str) -> SharedWindow {
    let spec = WindowSpec::new(APP_B, WindowSize::new(300, 150).unwrap(), WindowState::Active, scope, WindowKind::External).unwrap();
    SharedWindow::new(spec, title, "<text>Salut<!text>\n", "")
}

fn received_title(incoming: &Receiver<Incoming>) -> Option<String> {
    match incoming.recv_timeout(Duration::from_millis(300)) {
        Ok(Incoming::Window(window)) => Some(window.title),
        Ok(Incoming::Route(route)) => panic!("route inattendue : {}", route.path),
        Err(_) => None,
    }
}

#[test]
fn a_window_reaches_followers_or_everyone_depending_on_its_scope() {
    let socket = router_socket_path();
    start_test_router(&socket);

    let mut nav_a = connect_with_retry(&socket, APP_A);
    let mut nav_b = connect_with_retry(&socket, APP_B);
    let nav_c = connect_with_retry(&socket, APP_C);
    // Pas d'accuse de reception dans le protocole : laisse le routeur
    // enregistrer les 3 apps avant le follow (voir le test de routes).
    std::thread::sleep(Duration::from_millis(100));

    navigation_manager::follow(&mut nav_a, APP_B).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let from_a = navigation_manager::listen(&nav_a).unwrap();
    let from_b = navigation_manager::listen(&nav_b).unwrap();
    let from_c = navigation_manager::listen(&nav_c).unwrap();

    navigation_manager::send_window(&mut nav_b, &window_from_b(WindowScope::Followers, "pour les abonnes")).unwrap();
    assert_eq!(received_title(&from_a).as_deref(), Some("pour les abonnes"));
    assert_eq!(received_title(&from_c), None, "C ne suit pas B");

    navigation_manager::send_window(&mut nav_b, &window_from_b(WindowScope::All, "pour tous")).unwrap();
    assert_eq!(received_title(&from_a).as_deref(), Some("pour tous"));
    assert_eq!(received_title(&from_c).as_deref(), Some("pour tous"));
    assert_eq!(received_title(&from_b), None, "l'expediteur ne recoit pas sa propre fenetre");

    // Apres unfollow, A ne recoit plus les fenetres des abonnes de B.
    navigation_manager::unfollow(&mut nav_a, APP_B).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    navigation_manager::send_window(&mut nav_b, &window_from_b(WindowScope::Followers, "apres unfollow")).unwrap();
    assert_eq!(received_title(&from_a), None);

    let _ = std::fs::remove_file(&socket);
}

#[test]
fn send_window_refuses_internal_windows_and_other_apps_windows() {
    let socket = format!("{}-refus", router_socket_path());
    start_test_router(&socket);
    let mut nav_a = connect_with_retry(&socket, APP_A);

    let internal = SharedWindow::new(WindowSpec::internal(APP_A, WindowSize::new(10, 10).unwrap()), "t", "", "");
    assert!(navigation_manager::send_window(&mut nav_a, &internal).is_err());
    // Fenetre de B envoyee par A : refusee.
    assert!(navigation_manager::send_window(&mut nav_a, &window_from_b(WindowScope::All, "t")).is_err());

    let _ = std::fs::remove_file(&socket);
}
