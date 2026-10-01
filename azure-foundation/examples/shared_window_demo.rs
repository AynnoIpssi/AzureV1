// Demo : l'app B envoie une fenetre a ses abonnes, l'app A (abonnee a B) la
// recoit et l'ouvre toute seule. Les deux apps tournent dans ce process
// (chacune sa propre fenetre Wayland), avec un routeur azure-rooter prive
// sur sa propre socket pour ne pas toucher au vrai routeur.
//
// Lancer avec `cargo run --example shared_window_demo` depuis azure-foundation/.
use azure_core::models::window_model::{WindowKind, WindowScope, WindowSize, WindowSpec, WindowState};
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::navigation::managers::navigation_manager;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::window::models::shared_window::SharedWindow;
use azure_foundation::window::models::window::AzureWindow;
use std::time::Duration;

const APP_A: u32 = 1;
const APP_B: u32 = 2;
// Dans le dossier prive d'Azure, a cote des vrais sockets (nom a part).
static SOCKET: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| azure_core::paths::socket("router-shared-window-demo"));

const APP_A_RSH: &str = include_str!("shared_window/app_a.rsh");
const APP_B_RSH: &str = include_str!("shared_window/app_b.rsh");
const NOTE_RSH: &str = include_str!("shared_window/note.rsh");
const RSC: &str = include_str!("shared_window/demo.rsc");

fn ui(rsh: &str) -> Vec<UiNode> {
    let sheet = parse_rsc(tokenize_rsc(RSC)).expect("demo.rsc");
    let ast = parse_rsh(tokenize_rsh(rsh)).expect("rsH de la demo");
    build_ui(&ast, &StyleSource::Rsc(&sheet))
}

fn main() {
    std::thread::spawn(|| azure_rooter::managers::router::start_router_at(&SOCKET).expect("routeur de la demo"));
    std::thread::sleep(Duration::from_millis(100));

    let mut nav_a = navigation_manager::connect_at(&SOCKET, APP_A).expect("connexion app A");
    let nav_b = navigation_manager::connect_at(&SOCKET, APP_B).expect("connexion app B");
    std::thread::sleep(Duration::from_millis(50));
    navigation_manager::follow(&mut nav_a, APP_B).expect("A suit B");

    let app_a = AzureWindow::new("App A (abonnée à B)")
        .spec(WindowSpec::internal(APP_A, WindowSize::new(520, 260).unwrap()))
        .ui(ui(APP_A_RSH))
        .navigation(nav_a);
    let a = std::thread::spawn(move || app_a.run());

    AzureWindow::new("App B")
        .spec(WindowSpec::internal(APP_B, WindowSize::new(520, 260).unwrap()))
        .ui(ui(APP_B_RSH))
        .navigation(nav_b)
        .on_click(|ctx| {
            if ctx.clicked != Some("envoyer") {
                return;
            }
            let spec = WindowSpec::new(APP_B, WindowSize::new(460, 240).unwrap(), WindowState::Active, WindowScope::Followers, WindowKind::External).unwrap();
            let note = SharedWindow::new(spec, "Fenêtre envoyée par B", NOTE_RSH, RSC);
            if let Err(err) = ctx.send_window(note) {
                eprintln!("App B : envoi impossible : {err}");
            }
        })
        .run();

    let _ = a.join();
}
