// Le flux vu d'une fenetre foundation : l'app B partage l'etat d'une
// commande, l'app A change d'ecran a chaque modification. On rejoue ici ce
// que fait `AzureWindow::flux` a chaque tic (sans Wayland) : `poll`, puis le
// rappel avec un `WindowContext`, puis le sondage intra-app.
use azure_foundation::flux::{Flux, FluxEvent, Listener, Value};
use azure_foundation::navigation::managers::intra_navigation_manager;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::window::models::window_context::WindowContext;
use azure_rooter::managers::intra_router::IntraRouter;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const A: u32 = 1;
const B: u32 = 2;

fn texts(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => out.push(l.text.clone()),
            UiNode::Container(c) => texts(&c.children, out),
            _ => {}
        }
    }
}

fn start_daemon() -> String {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-foundation-flux-{}.sock"), std::process::id());
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("foundation-flux");
    let _ = std::fs::remove_dir_all(&data);
    let (s, d) = (socket.clone(), data.clone());
    std::thread::spawn(move || azure_service::managers::daemon::start_daemon_at(&s, &d));
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(&socket).is_err() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    socket
}

/// Le rappel que l'app A donnerait a `AzureWindow::flux`.
fn follow_order(ctx: &mut WindowContext, event: &FluxEvent, etat: &Value) {
    if event.touches("etat") {
        let etat = etat.get("etat").and_then(Value::as_str).unwrap_or("attente");
        ctx.goto_route("commande", &[("etat", etat)], "").unwrap();
    }
}

/// Un tic de fenetre : retourne les textes du nouvel ecran, s'il a change.
fn tick(listener: &mut Listener, intra: &azure_foundation::navigation::models::intra_client::IntraClient, routes: &RouteTable) -> Option<Vec<String>> {
    for event in listener.poll() {
        let mut ctx = WindowContext { intra: Some(intra), routes: Some(routes), ..Default::default() };
        follow_order(&mut ctx, &event, listener.state());
    }
    intra_navigation_manager::drain(intra, routes).map(|nodes| {
        let mut out = Vec::new();
        texts(&nodes, &mut out);
        out
    })
}

fn next_screen(listener: &mut Listener, intra: &azure_foundation::navigation::models::intra_client::IntraClient, routes: &RouteTable) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(screen) = tick(listener, intra, routes) {
            return screen;
        }
        assert!(Instant::now() < deadline, "l'ecran n'a pas change");
        std::thread::sleep(Duration::from_millis(16));
    }
}

#[test]
fn the_screen_of_app_a_follows_the_stream_of_app_b() {
    let socket = start_daemon();
    let dir = env!("CARGO_MANIFEST_DIR");
    let routes = RouteTable::new().view("/commande/{etat}", &format!("{dir}/tests/flux/commande.rsh"), &format!("{dir}/tests/flux/commande.rsc")).name("commande");
    let router = IntraRouter::new();
    let intra = intra_navigation_manager::connect(&router, 1);

    let mut commande = Flux::connect_at(&socket, B).unwrap().share("commande").to(&[A]).open().unwrap();
    commande.set("etat", "preparation").unwrap();
    commande.set("client", "Ana").unwrap();

    let mut ecoute = Flux::connect_at(&socket, A).unwrap().listen(B, "commande").path("etat").start().unwrap();
    assert_eq!(next_screen(&mut ecoute, &intra, &routes), ["Commande", "En preparation"], "etat recu au branchement");

    commande.set("client", "Bo").unwrap(); // pas ecoute : l'ecran ne bouge pas
    commande.set("etat", "livree").unwrap();
    assert_eq!(next_screen(&mut ecoute, &intra, &routes), ["Commande", "Livree !"]);
    assert_eq!(ecoute.get("client"), None);
}
