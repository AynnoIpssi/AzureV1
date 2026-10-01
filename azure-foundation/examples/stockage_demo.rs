// Demo : un compteur dont la valeur est gardee par azure-stockage (chiffree,
// sur le disque). Ferme la fenetre, relance : le compte est toujours la.
//
// Le daemon tourne dans ce process, sur un socket et un dossier a lui
// (/tmp/azure-stockage-demo*) : le vrai stockage n'est jamais touche.
//
// Lancer avec `cargo run --example stockage_demo` depuis azure-foundation/.
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::navigation::managers::intra_navigation_manager;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::storage::Stockage;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::window::models::window::AzureWindow;
use azure_rooter::managers::intra_router::IntraRouter;
use std::time::{Duration, Instant};

const APP: u32 = 1;
static SOCKET: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| azure_core::paths::socket("stockage-demo"));
static ROOT: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| azure_core::paths::runtime_dir().join("stockage-demo"));
const RSH: &str = include_str!("stockage/compteur.rsh");
const RSC: &str = include_str!("stockage/compteur.rsc");

/// La page, avec les valeurs du stockage a la place des `{{...}}`.
fn page(store: &Stockage) -> Vec<UiNode> {
    let rsh = RSH
        .replace("{{app}}", &store.app_id().to_string())
        .replace("{{clics}}", &store.get_or("clics", 0i64).to_string())
        .replace("{{ouvertures}}", &store.get_or("ouvertures", 0i64).to_string())
        .replace("{{dernier}}", &store.get_or("dernier-clic", "jamais".to_string()));
    let sheet = parse_rsc(tokenize_rsc(RSC)).expect("compteur.rsc");
    let ast = parse_rsh(tokenize_rsh(&rsh)).expect("compteur.rsh");
    build_ui(&ast, &StyleSource::Rsc(&sheet))
}

fn heure() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{:02}:{:02}:{:02} UTC", secs / 3600 % 24, secs / 60 % 60, secs % 60)
}

fn connect() -> Stockage {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match Stockage::connect_at(&SOCKET, APP) {
            Ok(store) => return store,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("daemon de stockage injoignable : {e}"),
        }
    }
}

fn main() {
    std::thread::spawn(|| azure_stockage::managers::daemon::start_daemon_at(&SOCKET, &ROOT).expect("daemon de la demo"));
    let store = connect();
    let ouvertures = store.update("ouvertures", 0, |n: i64| n + 1).expect("stockage");
    println!("Demo ouverte {ouvertures} fois (donnees dans {}, chiffrees)", ROOT.display());

    // Chaque clic change le stockage, puis redemande la page "/" : la route
    // la reconstruit avec les nouvelles valeurs.
    let router = IntraRouter::new();
    let routes = RouteTable::new().route("/", {
        let store = store.clone();
        move |_| page(&store)
    });

    AzureWindow::new("Compteur (azure-stockage)")
        .size(560, 560)
        .ui(page(&store))
        .intra(intra_navigation_manager::connect(&router, APP))
        .routes(routes)
        .stockage(store)
        .on_click(|ctx| {
            let Some(store) = ctx.stockage() else { return };
            let changed = match ctx.clicked {
                Some("plus") => store.update("clics", 0, |n: i64| n + 1).map(|_| ()),
                Some("zero") => store.set("clics", 0),
                _ => return,
            };
            if let Err(err) = changed.and_then(|_| store.set("dernier-clic", heure())) {
                eprintln!("Compteur : {err}");
            }
            ctx.goto("/", "");
        })
        .run();
}
