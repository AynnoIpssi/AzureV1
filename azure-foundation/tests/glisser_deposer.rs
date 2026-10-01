// Glisser-deposer : une carte `<draggable>` passe d'une colonne
// `<dropzone>` a l'autre ; un simple clic reste un clic.
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
use azure_foundation::ui::services::draw_ui::{draw_drag, draw_ui};
use azure_foundation::ui::services::interact::{Dropped, KeyboardLayout};

const ZONE: (u32, u32, u32, u32) = (0, 0, 400, 300);
const K: KeyboardLayout = KeyboardLayout::Qwerty;

// Deux colonnes de 200px ; cartes de 180x40, 10px d'ecart.
const RSC: &str = "
.board { display: flex; width: 400px; height: 300px; }
.col { width: 200px; height: 300px; padding: 10px; display: flex; flex-direction: column; gap: 10px; }
.card { width: 180px; height: 40px; background-color: #333333; }
";

const RSH: &str = "<container.board>\
<dropzone id=\"a\"><container.col>\
<draggable id=\"c1\"><container.card><button#ouvrir-1>Un<!button><!container><!draggable>\
<draggable id=\"c2\"><container.card><text>Deux<!text><!container><!draggable>\
<!container><!dropzone>\
<dropzone id=\"b\"><container.col>\
<draggable id=\"c3\"><container.card><text>Trois<!text><!container><!draggable>\
<!container><!dropzone>\
<!container>";

fn page() -> EventState {
    let rsc = with_default_styles(RSC, None);
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    EventState::new(build_ui(&parse_rsh(tokenize_rsh(RSH)).unwrap(), &StyleSource::Rsc(&sheet)))
}

fn go(s: &mut EventState, x: i32, y: i32) -> bool {
    handle_event(s, WindowEvent::WindowMouseMove(x, y), K, ZONE)
}

fn down(s: &mut EventState) {
    handle_event(s, WindowEvent::WindowMouseButton(BTN_LEFT, true), K, ZONE);
}

fn up(s: &mut EventState) {
    handle_event(s, WindowEvent::WindowMouseButton(BTN_LEFT, false), K, ZONE);
}

#[test]
fn une_carte_change_de_colonne() {
    let mut s = page();
    // Saisit « Deux » (colonne a, 2e carte : y 60..100).
    go(&mut s, 150, 80);
    down(&mut s);
    assert_eq!(s.drag.as_ref().map(|d| d.source.as_str()), Some("c2"));
    // Moins que le seuil : pas encore un glisser.
    go(&mut s, 152, 82);
    assert!(!s.drag_active());
    // Au-dessus de la colonne b, sous « Trois » (y 10..50).
    assert!(go(&mut s, 300, 200));
    assert!(s.drag_active());
    let target = s.drag.as_ref().and_then(|d| d.target.clone()).unwrap();
    assert_eq!((target.zone.as_str(), target.position), ("b", 1));
    // La copie et la zone se dessinent sans paniquer.
    let mut canvas = Canvas::new(400, 300);
    draw_ui(&s.ui_nodes, ZONE, &mut canvas, 300, 200, false);
    draw_drag(&s.ui_nodes, s.drag.as_ref().unwrap(), 300, 200, &mut canvas);
    up(&mut s);
    assert_eq!(s.take_dropped(), Some(Dropped { source: "c2".into(), target: "b".into(), position: 1 }));
    assert!(s.drag.is_none());
}

#[test]
fn au_dessus_de_la_premiere_carte_rang_zero() {
    let mut s = page();
    go(&mut s, 300, 30);
    down(&mut s);
    go(&mut s, 100, 15);
    let target = s.drag.as_ref().and_then(|d| d.target.clone()).unwrap();
    assert_eq!((target.zone.as_str(), target.position), ("a", 0));
    up(&mut s);
    assert_eq!(s.take_dropped().map(|d| (d.source, d.position)), Some(("c3".into(), 0)));
}

#[test]
fn un_clic_sans_glisser_reste_un_clic() {
    let mut s = page();
    go(&mut s, 15, 15);
    down(&mut s);
    // Le clic du bouton attend le relachement.
    assert_eq!(s.clicked_id, None);
    up(&mut s);
    assert!(s.take_activation());
    assert_eq!(s.clicked_id.as_deref(), Some("ouvrir-1"));
    assert_eq!(s.take_dropped(), None);
}

#[test]
fn lache_hors_zone_ou_echap_rien() {
    let mut s = page();
    go(&mut s, 150, 80);
    down(&mut s);
    go(&mut s, 150, 400);
    up(&mut s);
    assert_eq!(s.take_dropped(), None);
    assert!(!s.take_activation());
}

