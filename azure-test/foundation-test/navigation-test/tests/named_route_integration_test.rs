// Routes NOMMEES vers une autre app, via le VRAI routeur azure-rooter (socket
// Unix privee) : A ne connait que le NOM de la route de B, c'est la
// table de routes de B (meme `Router` que `RouteTable`, qui rend ici du texte) qui le traduit en chemin (voir `router::named_path`).
use azure_foundation::navigation::managers::navigation_manager;
use azure_foundation::navigation::models::incoming::Incoming;
use azure_foundation::navigation::models::navigation_client::NavigationClient;
use azure_foundation::navigation::models::router::Router;
use azure_foundation::window::models::window_context::WindowContext;
use std::time::{Duration, Instant};

const APP_A: u32 = 301;
const APP_B: u32 = 302;

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

#[test]
fn app_a_reaches_a_named_route_of_app_b() {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-router-named-route-test-{}.sock"), std::process::id());
    start_test_router(&socket);

    let mut nav_a = connect_with_retry(&socket, APP_A);
    let nav_b = connect_with_retry(&socket, APP_B);
    std::thread::sleep(Duration::from_millis(100));
    let from_b = navigation_manager::listen(&nav_b).unwrap();

    let routes_b = Router::new()
        .group("/profil", |g| g.route("/{id}", |r| format!("profil {} ({})", r.param("id").unwrap(), r.payload)).name("user.show"))
        .fallback(|r| format!("inconnue {}", r.path));

    let mut ctx = WindowContext { intra: None, nav: Some(&mut nav_a), windows: None, routes: None, stockage: None, app_id: Some(APP_A), clicked: None, values: None, scroll_request: None, effects: Default::default() };
    ctx.navigate_to_route(APP_B, "user.show", &[("id", "42")], "depuis A").unwrap();
    ctx.navigate_to_route(APP_B, "pas.la", &[], "").unwrap();

    for expected in ["profil 42 (depuis A)", "inconnue pas.la"] {
        match from_b.recv_timeout(Duration::from_secs(1)) {
            Ok(Incoming::Route(route)) => assert_eq!(routes_b.dispatch(&route.path, &route.payload).as_deref(), Some(expected)),
            Ok(Incoming::Window(_)) => panic!("fenetre inattendue"),
            Err(_) => panic!("B n'a rien recu (attendu : {expected})"),
        }
    }
    let _ = std::fs::remove_file(&socket);
}
