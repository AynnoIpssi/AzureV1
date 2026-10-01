// La fenetre "Routeur" : les recettes statiques de `content::router` +
// UN bloc interactif en bandeau, qui fait un VRAI aller-retour sur le
// socket reel du routeur (auto-adresse : cette fenetre s'envoie a
// elle-meme une navigation "/demo-result" via azure-rooter, exactement
// comme app-a le fait vers app-b dans azure-test) plutot que de le simuler.
//
// Le bouton vit dans un bandeau FIXE en haut de la fenetre, hors de tout
// `Container` scrollable (voir `ui::category_window`) : le hit-test d'un
// widget a l'interieur d'une zone defilable reste correct aux offsets
// habituels, mais volontairement pas mis a l'epreuve ici pour l'unique
// bouton interactif de cette app - le garder hors zone scrollable evite
// toute question a ce sujet plutot que d'avoir a la trancher.
use crate::content::router;
use crate::theme;
use crate::ui::category_window;
use azure_foundation::layout::models::layout_props::{AlignItems, DisplayMode, FlexDirection, LayoutProps};
use azure_foundation::navigation::managers::navigation_manager;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::button::Button;
use azure_foundation::ui::models::container::Container;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::window::models::window::AzureWindow;

pub const DOCS_ROUTER_APP_ID: u32 = 900;
const DEMO_PATH: &str = "/demo-result";
const IDLE_MESSAGE: &str = "Cliquez sur le bouton pour lancer un vrai aller-retour sur le routeur.";
const DEMO_PAYLOAD: &str = "Bonjour depuis le bouton !";
const OFFLINE_MESSAGE: &str = "routeur_daemon n'est pas lance - demarrez-le (voir la premiere recette ci-dessous) puis relancez cette fenetre pour activer la demo.";

pub fn window() -> AzureWindow {
    match navigation_manager::connect(DOCS_ROUTER_APP_ID) {
        Ok(nav) => {
            let routes = RouteTable::new().on(DEMO_PATH, |payload| screen(&format!("Recu en direct via le routeur : \"{payload}\""), true));
            AzureWindow::new("Azure Docs - Routeur")
                .size(1280, 860)
                .ui(screen(IDLE_MESSAGE, true))
                .navigation(nav)
                .routes(routes)
                .on_click(|ctx| {
                    if let Err(err) = ctx.navigate_to(DOCS_ROUTER_APP_ID, DEMO_PATH, DEMO_PAYLOAD) {
                        eprintln!("Azure Docs (routeur): echec de la demo live: {err}");
                    }
                })
        }
        Err(_) => AzureWindow::new("Azure Docs - Routeur").size(1280, 860).ui(screen(OFFLINE_MESSAGE, false)),
    }
}

fn screen(status: &str, online: bool) -> Vec<UiNode> {
    let mut header_layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
    header_layout.flex_basis = Some(12.0);
    header_layout.flex_shrink = 0.0;

    let mut row_layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
    row_layout.display = DisplayMode::Flex;
    row_layout.flex_direction = FlexDirection::Row;
    row_layout.gap = theme::SPACE_SM;
    row_layout.align_items = AlignItems::Stretch;
    row_layout.flex_grow = 1.0;
    row_layout.flex_basis = Some(88.0);

    let recipes = router::recipes();
    let row = UiNode::Container(Container::new(
        row_layout,
        theme::BACKGROUND,
        vec![category_window::sidebar("Routeur", &recipes), category_window::content_pane(&recipes)],
    ));

    let mut root = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, theme::SPACE_SM);
    root.display = DisplayMode::Flex;
    root.flex_direction = FlexDirection::Column;
    root.gap = theme::SPACE_SM;

    vec![UiNode::Container(Container::new(root, theme::BACKGROUND, vec![header(header_layout, status, online), row]))]
}

fn header(mut layout: LayoutProps, status: &str, online: bool) -> UiNode {
    layout.display = DisplayMode::Flex;
    layout.flex_direction = FlexDirection::Row;
    layout.align_items = AlignItems::Center;
    layout.gap = theme::SPACE_SM;
    layout.padding = theme::SPACE_SM;

    let mut label_layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
    label_layout.flex_grow = 1.0;
    let label_color = if online { theme::TEXT_PRIMARY } else { theme::WARNING };
    let status_label = UiNode::Label(Label::new(label_layout, status.to_string(), label_color, theme::GOAL_SIZE + 2.0, 500.0));

    let mut children = vec![status_label];
    if online {
        let mut button_layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
        button_layout.flex_basis = Some(26.0);
        button_layout.flex_shrink = 0.0;
        let mut button = Button::new(button_layout, theme::ACCENT, false, "Lancer la demo live".to_string());
        button.hover_color = Some(theme::ACCENT_HOVER);
        button.text_color = Some(theme::BACKGROUND);
        children.push(UiNode::Button(button));
    }

    UiNode::Container(Container::new(layout, theme::SURFACE, children))
}
