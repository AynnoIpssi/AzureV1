// Premiere fenetre ouverte par `main` : un seul bouton, un seul effet de
// clic (`ctx.open_window` appele 3 fois d'affilee) - reste conforme a la
// contrainte documentee sur `AzureWindow::on_click` (une fenetre ne
// distingue pas quel widget est clique, donc au plus UN element cliquant
// avec effet de navigation par fenetre). Chaque `ctx.open_window` demarre
// sa propre connexion Wayland sur son propre thread (voir
// `window_context::WindowContext::open_window`) : le launcher reste
// reactif pendant que les 3 fenetres categorie s'ouvrent.
use crate::content::{rsc, rsh};
use crate::router_demo;
use crate::theme;
use crate::ui::category_window;
use azure_foundation::layout::models::layout_props::{AlignItems, DisplayMode, FlexDirection, JustifyContent, LayoutProps};
use azure_foundation::ui::models::button::Button;
use azure_foundation::ui::models::container::Container;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::window::models::window::AzureWindow;
use azure_foundation::window::models::window_table::WindowTable;

const PATH_RSH: &str = "/rsh";
const PATH_RSC: &str = "/rsc";
const PATH_ROUTER: &str = "/router";

pub fn window() -> AzureWindow {
    let windows = WindowTable::new()
        .on(PATH_RSH, |_| category_window_for("rsH", rsh::recipes()))
        .on(PATH_RSC, |_| category_window_for("rsC", rsc::recipes()))
        .on(PATH_ROUTER, |_| router_demo::window());

    AzureWindow::new("Azure Docs").size(720, 480).ui(screen()).windows(windows).on_click(|ctx| {
        ctx.open_window(PATH_RSH, "");
        ctx.open_window(PATH_RSC, "");
        ctx.open_window(PATH_ROUTER, "");
    })
}

fn category_window_for(title: &str, recipes: Vec<crate::content::Recipe>) -> AzureWindow {
    let nodes = category_window::page(title, &recipes);
    AzureWindow::new(&format!("Azure Docs - {title}")).size(1280, 860).ui(nodes)
}

fn screen() -> Vec<UiNode> {
    let mut root = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, theme::SPACE_LG);
    root.display = DisplayMode::Flex;
    root.flex_direction = FlexDirection::Column;
    root.justify_content = JustifyContent::Center;
    root.align_items = AlignItems::Center;
    root.gap = theme::SPACE_LG;

    let mut title_layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
    title_layout.flex_basis = Some(22.0);
    let title = UiNode::Label(Label::new(title_layout, "Azure Docs".to_string(), theme::TEXT_PRIMARY, theme::TITLE_SIZE + 6.0, theme::TITLE_WEIGHT));

    let mut subtitle_layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
    subtitle_layout.flex_basis = Some(30.0);
    let subtitle = UiNode::Label(Label::new(
        subtitle_layout,
        "rsH, rsC et le routeur, expliques avec du vrai code qui tourne.".to_string(),
        theme::TEXT_SECONDARY,
        theme::GOAL_SIZE + 1.0,
        theme::GOAL_WEIGHT,
    ));

    let mut button_layout = LayoutProps::new(0.0, 0.0, 70.0, 100.0, 0.0, 0.0);
    button_layout.flex_basis = Some(20.0);
    let mut button = Button::new(button_layout, theme::ACCENT, false, "Ouvrir les recettes".to_string());
    button.hover_color = Some(theme::ACCENT_HOVER);
    button.text_color = Some(theme::BACKGROUND);

    vec![UiNode::Container(Container::new(root, theme::BACKGROUND, vec![title, subtitle, UiNode::Button(button)]))]
}
