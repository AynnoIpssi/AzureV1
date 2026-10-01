// Demo visuelle du systeme de routes (voir azure_foundation::navigation),
// distinguant ses deux mecanismes des qu'on clique le bouton :
//   - INTRA-app, via l'`IntraRouter` d'azure-rooter (100% en memoire) :
//     `ctx.goto` bascule App A elle-meme sur son propre ecran "/sent".
//   - INTER-app, via azure-rooter par socket Unix : `ctx.navigate_to` envoie
//     en plus une navigation "/update-text" a App B (voir app-b/src/main.rs).
// Lancer `cargo run --bin routeur_daemon` avant ces deux binaires.
use azure_engine::rendering::models::color::Color;
use azure_foundation::layout::models::layout_props::LayoutProps;
use azure_foundation::navigation::managers::{intra_navigation_manager, navigation_manager};
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::button::Button;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::window::models::window::AzureWindow;
use azure_rooter::managers::intra_router::IntraRouter;

const APP_A_ID: u32 = 1;
const APP_B_ID: u32 = 2;
// Id de vue local a App A (namespace separe des `app_id` inter-app
// ci-dessus - voir `IntraRouter`, qui ne tourne que dans CE process).
const APP_A_VIEW_ID: u32 = 1;

fn home_screen() -> Vec<UiNode> {
    vec![UiNode::Button(Button::new(
        LayoutProps::new(20.0, 40.0, 60.0, 20.0, 0.0, 0.0),
        Color::new(70, 120, 220, 255),
        false,
        "Modifier le texte de App B".to_string(),
    ))]
}

fn sent_screen(_payload: &str) -> Vec<UiNode> {
    vec![UiNode::Label(Label::new(
        LayoutProps::new(10.0, 40.0, 80.0, 20.0, 0.0, 0.0),
        "Envoye a App B !".to_string(),
        Color::new(255, 255, 255, 255),
        24.0,
        400.0,
    ))]
}

fn main() {
    let nav = navigation_manager::connect(APP_A_ID)
        .expect("App A: enregistrement aupres du routeur (routeur_daemon est-il lance ?)");

    let intra_router = IntraRouter::new();
    let intra = intra_navigation_manager::connect(&intra_router, APP_A_VIEW_ID);

    let routes = RouteTable::new().on("/sent", sent_screen);

    AzureWindow::new("App A")
        .size(800, 600)
        .ui(home_screen())
        .navigation(nav)
        .intra(intra)
        .routes(routes)
        .on_click(|ctx| {
            // Intra-app : App A s'envoie a elle-meme une navigation vers
            // "/sent" via l'IntraRouter - aucun appel reseau, appliquee au
            // prochain tic.
            ctx.goto("/sent", "");

            // Inter-app : App A demande a App B (autre process) de changer
            // le SIEN, via azure-rooter.
            let result = ctx.navigate_to(APP_B_ID, "/update-text", "Texte modifie par App A !");
            if let Err(err) = result {
                eprintln!("App A: echec de la navigation vers App B: {err}");
            }
        })
        .run();
}
