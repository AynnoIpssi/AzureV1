// Cote receveur de la demo (voir app-a/src/main.rs) : affiche un texte
// d'attente, puis le remplace des reception d'une navigation "/update-text"
// envoyee par App A - voir azure_foundation::navigation et
// AzureWindow::navigation/routes.
use azure_engine::rendering::models::color::Color;
use azure_foundation::layout::models::layout_props::LayoutProps;
use azure_foundation::navigation::managers::navigation_manager;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::window::models::window::AzureWindow;

const APP_B_ID: u32 = 2;

fn label_screen(text: &str) -> Vec<UiNode> {
    vec![UiNode::Label(Label::new(
        LayoutProps::new(10.0, 40.0, 80.0, 20.0, 0.0, 0.0),
        text.to_string(),
        Color::new(255, 255, 255, 255),
        24.0,
        400.0,
    ))]
}

fn main() {
    let nav = navigation_manager::connect(APP_B_ID)
        .expect("App B: enregistrement aupres du routeur (routeur_daemon est-il lance ?)");

    let routes = RouteTable::new().on("/update-text", label_screen);

    AzureWindow::new("App B")
        .size(800, 600)
        .ui(label_screen("En attente d'App A..."))
        .navigation(nav)
        .routes(routes)
        .run();
}
