// Les champs comme un utilisateur : clics souris, glisser, saisie, via
// `event::services::dispatch` (le meme chemin qu'une vraie fenetre).
use azure_core::rules::window_event::WindowEvent;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::form::{form_values, FieldValue};
use azure_foundation::ui::services::interact::{self, KeyInput, KeyboardLayout};

const CONTENT: (u32, u32, u32, u32) = (0, 0, 800, 1000);

fn page() -> EventState {
    let d = format!("{}/tests/components", env!("CARGO_MANIFEST_DIR"));
    EventState::new(RouteTable::new().view("/", &format!("{d}/formulaire.rsh"), &format!("{d}/formulaire.rsc")).resolve(&Route::new("/", "")).unwrap())
}

/// Boite du champ `id`.
fn box_of(state: &EventState, id: &str) -> (i32, i32, u32, u32) {
    let mut found = None;
    interact::walk(&state.ui_nodes, CONTENT, &mut |node, b| {
        let node_id = match node {
            UiNode::Control(c) => &c.id,
            UiNode::TextArea(t) => &t.id,
            UiNode::Button(b) => &b.id,
            _ => return,
        };
        if node_id == id {
            found = Some(b);
        }
    });
    found.unwrap_or_else(|| panic!("{id} introuvable"))
}

fn click_at(state: &mut EventState, x: i32, y: i32) -> bool {
    handle_event(state, WindowEvent::WindowMouseMove(x, y), KeyboardLayout::Qwerty, CONTENT);
    let changed = handle_event(state, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, CONTENT);
    handle_event(state, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, CONTENT);
    changed
}

fn click(state: &mut EventState, id: &str) -> bool {
    let (x, y, _, h) = box_of(state, id);
    click_at(state, x + 4, y + h as i32 / 2)
}

fn value(state: &EventState, id: &str) -> Option<FieldValue> {
    form_values(&state.ui_nodes).get(id).cloned()
}

#[test]
fn checkbox_switch_and_radio_groups() {
    let mut s = page();
    assert!(click(&mut s, "cgu"));
    assert_eq!(s.clicked_id.as_deref(), Some("cgu"), "l'app voit quel champ a ete clique");
    assert_eq!(value(&s, "cgu"), Some(FieldValue::Bool(true)));
    click(&mut s, "cgu");
    assert_eq!(value(&s, "cgu"), Some(FieldValue::Bool(false)));
    click(&mut s, "sombre");
    assert_eq!(value(&s, "sombre"), Some(FieldValue::Bool(true)));

    assert_eq!(value(&s, "taille"), Some(FieldValue::Text("M".into())));
    click(&mut s, "r-l");
    assert_eq!(value(&s, "taille"), Some(FieldValue::Text("L".into())));
    assert_eq!(value(&s, "r-m"), Some(FieldValue::Bool(false)), "un seul coche par groupe");

    // Desactivee : rien ne change.
    click(&mut s, "off");
    assert_eq!(value(&s, "off"), Some(FieldValue::Bool(false)));
}

#[test]
fn slider_click_and_drag() {
    let mut s = page();
    let (x, y, w, h) = box_of(&s, "volume");
    let cy = y + h as i32 / 2;
    // Au milieu : 50 (par pas de 5).
    click_at(&mut s, x + w as i32 / 2, cy);
    assert_eq!(value(&s, "volume"), Some(FieldValue::Number(50.0)));
    // Glisser jusqu'au bout.
    handle_event(&mut s, WindowEvent::WindowMouseMove(x + 20, cy), KeyboardLayout::Qwerty, CONTENT);
    handle_event(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, CONTENT);
    handle_event(&mut s, WindowEvent::WindowMouseMove(x + w as i32 + 50, cy), KeyboardLayout::Qwerty, CONTENT);
    assert_eq!(value(&s, "volume"), Some(FieldValue::Number(100.0)));
    handle_event(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, CONTENT);
    // Relache : bouger ne change plus rien.
    handle_event(&mut s, WindowEvent::WindowMouseMove(x, cy), KeyboardLayout::Qwerty, CONTENT);
    assert_eq!(value(&s, "volume"), Some(FieldValue::Number(100.0)));
}

#[test]
fn select_opens_and_picks_an_option() {
    let mut s = page();
    assert_eq!(value(&s, "pays"), Some(FieldValue::Text("France".into())));
    let (x, y, _, h) = box_of(&s, "pays");
    click(&mut s, "pays");
    // La liste s'ouvre sous la boite : 3e ligne = Suisse (lignes de 30 px, 4 px sous la boite).
    let changed = click_at(&mut s, x + 20, y + h as i32 + 4 + 2 * 30 + 15);
    assert!(changed);
    assert_eq!(s.clicked_id.as_deref(), Some("pays"));
    assert_eq!(value(&s, "pays"), Some(FieldValue::Text("Suisse".into())));
    // Ouverte puis clic ailleurs : fermee, rien de choisi.
    click(&mut s, "pays");
    click_at(&mut s, 700, 990);
    assert_eq!(value(&s, "pays"), Some(FieldValue::Text("Suisse".into())));
}

#[test]
fn segmented_and_rating() {
    let mut s = page();
    let (x, y, w, h) = box_of(&s, "vue");
    click_at(&mut s, x + w as i32 - 10, y + h as i32 / 2);
    assert_eq!(value(&s, "vue"), Some(FieldValue::Text("Mois".into())));
    let (x, y, _, h) = box_of(&s, "note");
    // 4e pastille (18 px + 6 d'ecart).
    click_at(&mut s, x + 9 + 3 * 24, y + h as i32 / 2);
    assert_eq!(value(&s, "note"), Some(FieldValue::Number(4.0)));
}

#[test]
fn buttons_press_then_come_back_up() {
    let mut s = page();
    let (x, y, w, h) = box_of(&s, "ok");
    handle_event(&mut s, WindowEvent::WindowMouseMove(x + w as i32 / 2, y + h as i32 / 2), KeyboardLayout::Qwerty, CONTENT);
    handle_event(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, CONTENT);
    let pressed = |s: &EventState| {
        let mut p = false;
        interact::walk(&s.ui_nodes, CONTENT, &mut |n, _| {
            if let UiNode::Button(b) = n {
                p |= b.state;
            }
        });
        p
    };
    assert!(pressed(&s), "enfonce pendant l'appui");
    assert!(handle_event(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, CONTENT));
    assert!(!pressed(&s), "remonte au relachement (plus de bascule)");
}

#[test]
fn single_line_inputs() {
    let mut s = page();
    click(&mut s, "nom");
    for c in "Ada".chars() {
        interact::type_into_focused(&mut s.ui_nodes, KeyInput::Char(c), &mut String::new());
    }
    assert!(!interact::type_into_focused(&mut s.ui_nodes, KeyInput::Enter, &mut String::new()), "Entree n'ajoute pas de ligne");
    assert_eq!(value(&s, "nom"), Some(FieldValue::Text("Ada".into())));

    click(&mut s, "age");
    for c in "4x2".chars() {
        interact::type_into_focused(&mut s.ui_nodes, KeyInput::Char(c), &mut String::new());
    }
    assert_eq!(value(&s, "age"), Some(FieldValue::Text("42".into())), "chiffres seulement");

    click(&mut s, "mdp");
    let mut clipboard = "a\nb".to_string();
    interact::type_into_focused(&mut s.ui_nodes, KeyInput::Paste, &mut clipboard);
    assert_eq!(value(&s, "mdp"), Some(FieldValue::Text("ab".into())), "le retour a la ligne colle est retire");
    let masked = s.ui_nodes.iter().find_map(|n| match n {
        UiNode::Container(c) => c.children.iter().find_map(|n| match n {
            UiNode::TextArea(t) if t.id == "mdp" => Some(t.display_text().into_owned()),
            _ => None,
        }),
        _ => None,
    });
    assert_eq!(masked.as_deref(), Some("**"));
}
