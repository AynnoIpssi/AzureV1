// Clavier (Tab, Entree, Espace, fleches, Echap) et couches `position:
// fixed` (modale, notifications, infobulle), via `dispatch` comme une
// vraie fenetre. Capture : target/tmp/couches.ppm.
use azure_core::rules::window_event::WindowEvent;
use azure_foundation::compiler::services::condition::{ConditionValue, Context};
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch::{handle_event, handle_tick};
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::form::{form_values, FieldValue};
use azure_foundation::ui::services::interact::{self, KeyboardLayout};

const VIEW: (u32, u32, u32, u32) = (0, 0, 900, 700);
const TAB: u32 = 15;
const ENTER: u32 = 28;
const SPACE: u32 = 57;
const ESC: u32 = 1;
const RIGHT: u32 = 106;
const DOWN: u32 = 108;
const SHIFT: u32 = 42;

fn page(modale: bool) -> EventState {
    let d = format!("{}/tests/components", env!("CARGO_MANIFEST_DIR"));
    let routes = RouteTable::new().view_with("/", &format!("{d}/couches.rsh"), &format!("{d}/couches.rsc"), move |_| {
        Context::new().with_bool("modale", modale).with_value("nombres", ConditionValue::List((1..=12).map(|n| ConditionValue::Number(n as f64)).collect()))
    });
    EventState::new(routes.resolve(&Route::new("/", "")).unwrap())
}

fn key(s: &mut EventState, code: u32) -> bool {
    let changed = handle_event(s, WindowEvent::WindowKeyPress(code, true), KeyboardLayout::Qwerty, VIEW);
    handle_event(s, WindowEvent::WindowKeyPress(code, false), KeyboardLayout::Qwerty, VIEW);
    changed
}

/// L'id du noeud focalise.
fn focused(s: &EventState) -> Option<String> {
    let mut out = None;
    interact::walk(&s.ui_nodes, VIEW, &mut |n, _| match n {
        UiNode::Button(b) if b.focused => out = Some(b.id.clone()),
        UiNode::Control(c) if c.focused => out = Some(c.id.clone()),
        UiNode::TextArea(t) if t.focused => out = Some(t.id.clone()),
        _ => {}
    });
    out
}

fn value(s: &EventState, id: &str) -> Option<FieldValue> {
    form_values(&s.ui_nodes).get(id).cloned()
}

fn box_of(s: &EventState, id: &str) -> (i32, i32, u32, u32) {
    let mut found = None;
    interact::walk(&s.ui_nodes, VIEW, &mut |n, b| {
        let nid = match n {
            UiNode::Button(x) => &x.id,
            UiNode::Control(x) => &x.id,
            UiNode::TextArea(x) => &x.id,
            _ => return,
        };
        if nid == id {
            found = Some(b);
        }
    });
    found.unwrap_or_else(|| panic!("{id}"))
}

fn click(s: &mut EventState, x: i32, y: i32) {
    handle_event(s, WindowEvent::WindowMouseMove(x, y), KeyboardLayout::Qwerty, VIEW);
    handle_event(s, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, VIEW);
    handle_event(s, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, VIEW);
}

#[test]
fn tab_goes_through_the_page_in_order_and_back() {
    let mut s = page(false);
    let mut order = Vec::new();
    for _ in 0..8 {
        key(&mut s, TAB);
        order.push(focused(&s).unwrap_or_default());
    }
    assert_eq!(order, ["nom", "cgu", "r1", "r2", "r3", "vol", "pays", "ok"]);
    handle_event(&mut s, WindowEvent::WindowKeyPress(SHIFT, true), KeyboardLayout::Qwerty, VIEW);
    key(&mut s, TAB);
    assert_eq!(focused(&s).as_deref(), Some("pays"), "Maj+Tab : en arriere");
    handle_event(&mut s, WindowEvent::WindowKeyPress(SHIFT, false), KeyboardLayout::Qwerty, VIEW);
    // Contour visible seulement au clavier.
    let mut ring = false;
    interact::walk(&s.ui_nodes, VIEW, &mut |n, _| if let UiNode::Control(c) = n { ring |= c.focus_ring });
    assert!(ring);
}

#[test]
fn keys_act_on_the_focused_field() {
    let mut s = page(false);
    key(&mut s, TAB);
    key(&mut s, TAB); // cgu
    assert!(key(&mut s, SPACE));
    assert!(s.take_activation(), "l'app est prevenue comme pour un clic");
    assert_eq!(s.clicked_id.as_deref(), Some("cgu"));
    assert_eq!(value(&s, "cgu"), Some(FieldValue::Bool(true)));

    key(&mut s, TAB); // r1
    key(&mut s, RIGHT);
    assert_eq!(value(&s, "t"), Some(FieldValue::Text("M".into())), "fleche : radio suivant du groupe");
    assert_eq!(focused(&s).as_deref(), Some("r2"));

    key(&mut s, TAB);
    key(&mut s, TAB); // vol
    key(&mut s, RIGHT);
    key(&mut s, RIGHT);
    assert_eq!(value(&s, "vol"), Some(FieldValue::Number(70.0)));

    key(&mut s, TAB); // pays
    key(&mut s, DOWN);
    assert_eq!(value(&s, "pays"), Some(FieldValue::Text("Belgique".into())));
    key(&mut s, ENTER);
    let open = |s: &EventState| {
        let mut o = false;
        interact::walk(&s.ui_nodes, VIEW, &mut |n, _| if let UiNode::Control(c) = n { o |= c.open });
        o
    };
    assert!(open(&s), "Entree ouvre la liste");
    key(&mut s, ESC);
    assert!(!open(&s), "Echap la ferme");

    key(&mut s, TAB); // ok
    s.take_activation();
    key(&mut s, ENTER);
    assert!(s.take_activation());
    assert_eq!(s.clicked_id.as_deref(), Some("ok"));
}

#[test]
fn tabbing_into_a_scrolled_list_scrolls_it() {
    let mut s = page(false);
    for _ in 0..(8 + 12) {
        key(&mut s, TAB);
    }
    assert_eq!(focused(&s).as_deref(), Some("ligne-12"));
    let (_, y, _, h) = box_of(&s, "ligne-12");
    let (_, ly, _, lh) = {
        let mut b = None;
        interact::walk(&s.ui_nodes, VIEW, &mut |n, r| if let UiNode::Container(c) = n && c.layout.scrollable() { b = Some(r) });
        b.unwrap()
    };
    assert!(y >= ly && y + h as i32 <= ly + lh as i32, "la ligne focalisee est visible");
}

#[test]
fn a_modal_blocks_the_page_keeps_the_keyboard_and_closes_with_escape() {
    let mut s = page(true);
    // Le bouton OK de la page est sous le fond de la modale : le clic ne
    // l'atteint pas.
    let (x, y, w, h) = box_of(&s, "ok");
    click(&mut s, x + w as i32 / 2, y + h as i32 / 2);
    assert_eq!(s.clicked_id, None);
    // Le champ de la modale, lui, se clique.
    let (x, y, _, h) = box_of(&s, "raison");
    click(&mut s, x + 10, y + h as i32 / 2);
    assert_eq!(focused(&s).as_deref(), Some("raison"));
    // Tab reste dans la modale.
    let mut seen = Vec::new();
    for _ in 0..5 {
        key(&mut s, TAB);
        seen.push(focused(&s).unwrap_or_default());
    }
    // Ordre du document dans la modale : fermer (x), raison, oui, annuler.
    assert_eq!(seen, ["m-oui", "m-annuler", "m-fermer", "raison", "m-oui"]);
    // Echap : le bouton de fermeture de la modale.
    s.take_activation();
    key(&mut s, ESC);
    assert!(s.take_activation());
    assert_eq!(s.clicked_id.as_deref(), Some("m-fermer"));
}

#[test]
fn toasts_sit_in_the_corner_and_tooltips_appear_after_a_pause() {
    let mut s = page(false);
    // Notifications en bas a droite (20 px du bord).
    let mut toast = None;
    interact::walk(&s.ui_nodes, VIEW, &mut |n, b| if let UiNode::Container(c) = n && c.children.len() == 1 && b.0 > 400 && b.1 > 500 { toast.get_or_insert(b); });
    let t = toast.expect("pile de notifications");
    assert_eq!((t.0 + t.2 as i32, t.1 + t.3 as i32), (880, 680));

    let (x, y, w, h) = box_of(&s, "ok");
    handle_event(&mut s, WindowEvent::WindowMouseMove(x + w as i32 / 2, y + h as i32 / 2), KeyboardLayout::Qwerty, VIEW);
    assert!(s.tooltip.is_none());
    s.still_since = std::time::Instant::now() - std::time::Duration::from_secs(1);
    assert!(handle_tick(&mut s, KeyboardLayout::Qwerty, VIEW));
    assert_eq!(s.tooltip.as_ref().map(|t| t.0.as_str()), Some("Enregistre le formulaire"));
    handle_event(&mut s, WindowEvent::WindowMouseMove(5, 5), KeyboardLayout::Qwerty, VIEW);
    assert!(s.tooltip.is_none(), "disparait des que la souris bouge");

    // Rien sous la souris : cherchee une fois par pause, pas a chaque tic
    // (la chercher refait la mise en page de tout l'ecran).
    s.still_since = std::time::Instant::now() - std::time::Duration::from_secs(1);
    assert!(!handle_tick(&mut s, KeyboardLayout::Qwerty, VIEW));
    assert!(s.tooltip_sought);
    (s.mouse_x, s.mouse_y) = (x + w as i32 / 2, y + h as i32 / 2);
    assert!(!handle_tick(&mut s, KeyboardLayout::Qwerty, VIEW) && s.tooltip.is_none(), "pas cherchee de nouveau tant que la souris n'a pas bouge");
    // La souris bouge : on cherche de nouveau a la pause suivante.
    handle_event(&mut s, WindowEvent::WindowMouseMove(x + w as i32 / 2, y + h as i32 / 2), KeyboardLayout::Qwerty, VIEW);
    s.still_since = std::time::Instant::now() - std::time::Duration::from_secs(1);
    assert!(handle_tick(&mut s, KeyboardLayout::Qwerty, VIEW));
    assert!(s.tooltip.is_some());
}

#[test]
fn screenshot_with_modal_focus_ring_and_tooltip() {
    use azure_engine::rendering::models::canvas::Canvas;
    for (modale, name) in [(true, "couches"), (false, "couches-page")] {
    let mut s = page(modale);
    key(&mut s, TAB);
    key(&mut s, TAB);
    let (w, h) = (VIEW.2, VIEW.3);
    let mut canvas = Canvas::new(w, h);
    azure_foundation::ui::services::draw_ui::draw_ui(&s.ui_nodes, VIEW, &mut canvas, -1, -1, true);
    azure_foundation::ui::services::draw_ui::draw_tooltip("Infobulle d'exemple", 600, 120, VIEW, &mut canvas);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.ppm")), ppm).unwrap();
    }
}
