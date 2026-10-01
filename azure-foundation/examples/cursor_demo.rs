// Demonstration du curseur souris genere (voir `azure_foundation::cursor`) :
// survoler chaque zone de haut en bas.
// - le titre et le paragraphe (texte AFFICHE) : fleche ;
// - le bouton (cliquable) : main ;
// - la zone de saisie (texte EDITABLE) : barre de texte ;
// - les boutons de la barre d'en-tete : main.
//
// Lancer avec `cargo run --example cursor_demo` depuis azure-foundation/.
use azure_engine::rendering::models::color::Color;
use azure_foundation::layout::models::layout_props::{DisplayMode, FlexDirection, LayoutProps};
use azure_foundation::ui::models::button::Button;
use azure_foundation::ui::models::container::Container;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::textarea::TextArea;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::window::models::window::AzureWindow;

fn item(basis: f32) -> LayoutProps {
    let mut layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
    layout.flex_basis = Some(basis);
    layout.flex_shrink = 0.0;
    layout
}

fn main() {
    let text = Color::new(242, 242, 247, 255);
    let mut root = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 5.0);
    root.display = DisplayMode::Flex;
    root.flex_direction = FlexDirection::Column;
    root.gap = 4.0;

    let title = UiNode::Label(Label::new(item(10.0), "Texte affiche : fleche".to_string(), text, 26.0, 700.0));
    let paragraph = UiNode::Label(Label::new(item(8.0), "Ce paragraphe n'est pas editable, pas de barre de texte ici.".to_string(), text, 15.0, 400.0));
    let mut button = Button::new(item(14.0), Color::new(124, 156, 255, 255), false, "Bouton : main".to_string());
    button.text_color = Some(Color::new(20, 20, 31, 255));
    let input = TextArea::new(item(40.0), Color::new(16, 16, 26, 255), text, "Zone de saisie : barre de texte".to_string());

    let nodes = vec![UiNode::Container(Container::new(
        root,
        Color::new(20, 20, 31, 255),
        vec![title, paragraph, UiNode::Button(button), UiNode::TextArea(input)],
    ))];
    AzureWindow::new("Azure - curseur").size(760, 520).ui(nodes).run();
}
