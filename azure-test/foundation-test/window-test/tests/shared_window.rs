// Encodage / decodage d'une fenetre envoyee entre apps (voir
// `azure_foundation::window::models::shared_window`), sans routeur ni
// compositeur.
use azure_core::models::window_model::*;
use azure_foundation::navigation::models::incoming::Incoming;
use azure_foundation::window::models::shared_window::SharedWindow;

const RSH: &str = "<container.card>\n    <text>Salut depuis B<!text>\n<!container>\n";
const RSC: &str = ".card { background: #202040; padding: 8px; }\n";

fn external(scope: WindowScope) -> WindowSpec {
    WindowSpec::new(7, WindowSize::new(320, 200).unwrap(), WindowState::Active, scope, WindowKind::External).unwrap()
}

#[test]
fn encode_then_decode_gives_the_same_window() {
    let window = SharedWindow::new(external(WindowScope::Followers), "Note de B", RSH, RSC);
    let raw = window.encode().unwrap();
    assert!(SharedWindow::is_shared_window(&raw));
    assert_eq!(SharedWindow::decode(&raw).unwrap(), window);
}

#[test]
fn a_decoded_window_must_respect_the_core_rules() {
    // Fenetre externe avec le scope Owner, forgee a la main : refusee.
    let raw = "\u{1E}azure-window\u{1F}7\u{1F}320\u{1F}200\u{1F}0\u{1F}0\u{1F}1\u{1F}t\u{1F}\u{1F}";
    assert!(SharedWindow::decode(raw).is_err());
}

#[test]
fn a_separator_inside_the_source_is_refused() {
    let window = SharedWindow::new(external(WindowScope::All), "t", "<text>a\u{1F}b<!text>", "");
    assert!(window.encode().is_err());
}

#[test]
fn the_received_source_becomes_a_window_with_the_sender_spec() {
    let spec = external(WindowScope::All);
    let window = SharedWindow::new(spec, "Note de B", RSH, RSC).to_window().unwrap();
    assert_eq!(window.window_spec(), Some(&spec));
}

#[test]
fn incoming_tells_routes_and_windows_apart() {
    let raw = SharedWindow::new(external(WindowScope::All), "t", RSH, RSC).encode().unwrap();
    assert!(matches!(Incoming::decode(&raw), Some(Incoming::Window(_))));
    assert!(matches!(Incoming::decode("/settings\u{1F}x"), Some(Incoming::Route(_))));
    // Une fenetre abimee n'est jamais prise pour une route.
    assert!(Incoming::decode("\u{1E}azure-window\u{1F}garbage").is_none());
}
