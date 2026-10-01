// Un bouton enfonce garde sa teinte (un peu plus sombre) : il ne devient
// jamais blanc, comme le faisait l'ancienne inversion des couleurs.
use azure_core::rules::window_event::WindowEvent;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::components::with_default_styles;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::interact::KeyboardLayout;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_engine::rendering::models::color::Color;

const ZONE: (u32, u32, u32, u32) = (0, 0, 200, 100);

// Couleur du pixel (5, 5), dans le fond du bouton, loin du texte.
fn pixel(s: &EventState) -> (u8, u8, u8) {
    let mut canvas = Canvas::new(200, 100);
    draw_ui(&s.ui_nodes, ZONE, &mut canvas, -1, -1, false);
    let i = (5 * 200 + 5) * 4;
    let b = &canvas.buffer[i..i + 4];
    (b[2], b[1], b[0])
}

fn bouton(rsc: &str) -> EventState {
    let rsc = with_default_styles(&format!("button {{ width: 200px; height: 100px; background-color: #262420; }} {rsc}"), None);
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    EventState::new(build_ui(&parse_rsh(tokenize_rsh("<button.b>Valider<!button>")).unwrap(), &StyleSource::Rsc(&sheet)))
}

fn appuyer(s: &mut EventState) {
    handle_event(s, WindowEvent::WindowMouseMove(100, 50), KeyboardLayout::Qwerty, ZONE);
    handle_event(s, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, ZONE);
}

#[test]
fn active_choisit_le_fond_et_le_texte_pendant_l_appui() {
    // :active passe devant :hover (declare apres, meme specificite).
    let mut s = bouton(".b:hover { background-color: #302c26; } .b:active { background-color: #c9a878; color: #1a1712; }");
    let UiNode::Button(b) = &s.ui_nodes[0] else { panic!() };
    assert_eq!(b.active_text_color, Some(Color::new(0x1a, 0x17, 0x12, 255)));
    appuyer(&mut s);
    assert_eq!(pixel(&s), (0xc9, 0xa8, 0x78));
}

#[test]
fn sans_active_le_bouton_est_juste_enfonce() {
    let s = bouton(".b:hover { background-color: #302c26; }");
    let UiNode::Button(b) = &s.ui_nodes[0] else { panic!() };
    assert_eq!((b.active_color, b.active_text_color), (None, None), "le survol seul n'invente pas de :active");
}

#[test]
fn un_bouton_sombre_reste_sombre_pendant_l_appui() {
    let mut s = bouton("");
    let repos = pixel(&s);
    assert_eq!(repos, (0x26, 0x24, 0x20));
    // Le dessin se fait sans survol (souris en -1, -1) : seul l'appui compte.
    appuyer(&mut s);
    let appui = pixel(&s);
    assert_ne!(appui, repos, "l'appui se voit");
    assert!(appui.0 < 0x40 && appui.1 < 0x40 && appui.2 < 0x40, "enfoncé mais toujours sombre : {appui:?}");
    handle_event(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, ZONE);
    assert_eq!(pixel(&s), repos, "relâché : retour à la normale");
}
