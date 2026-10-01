// La vraie carte du clavier : celle que GNOME envoie (AZERTY, symboles en
// valeurs numeriques, deux dispositions) et une carte allemande ecrite avec
// des noms (`adiaeresis`), comme la produisent d'autres compositeurs.
use azure_core::rules::window_event::WindowEvent;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::{BTN_LEFT, KEY_LEFTCTRL, KEY_LEFTSHIFT};
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::form::{form_values, FieldValue};
use azure_foundation::ui::services::interact::{self, key_to_input_with, KeyInput, KeyboardLayout, Keymap, Modifiers};

// Codes evdev.
const KEY_1: u32 = 2;
const KEY_2: u32 = 3;
const KEY_Q: u32 = 16; // A en AZERTY
const KEY_W: u32 = 17; // Z en AZERTY
const KEY_E: u32 = 18;
const KEY_Y: u32 = 21; // Z en QWERTZ
const KEY_LEFTBRACE: u32 = 26; // ^ mort en AZERTY
const KEY_APOSTROPHE: u32 = 40; // ù en AZERTY, ä en QWERTZ
const KEY_C: u32 = 46;
const KEY_M: u32 = 50; // , en AZERTY
const KEY_RIGHTALT: u32 = 100;
const KEY_KP7: u32 = 71;
const KEY_102ND: u32 = 86; // < >
const KEY_SPACE: u32 = 57;

fn carte(nom: &str) -> KeyboardLayout {
    let texte = std::fs::read_to_string(format!("{}/tests/data/{nom}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    KeyboardLayout::Xkb(Keymap::parse(&texte).expect("carte lisible").leak())
}

fn touche(layout: KeyboardLayout, key: u32, mods: Modifiers) -> Option<KeyInput> {
    key_to_input_with(key, layout, mods)
}

fn car(layout: KeyboardLayout, key: u32, mods: Modifiers) -> char {
    match touche(layout, key, mods) {
        Some(KeyInput::Char(c)) => c,
        autre => panic!("touche {key} : {autre:?}"),
    }
}

const RIEN: Modifiers = Modifiers { shift: false, ctrl: false, altgr: false, caps_lock: false, num_lock: false, group: 0 };
const MAJ: Modifiers = Modifiers { shift: true, ..RIEN };
const ALTGR: Modifiers = Modifiers { altgr: true, ..RIEN };
const VERR_MAJ: Modifiers = Modifiers { caps_lock: true, ..RIEN };

#[test]
fn azerty_de_gnome() {
    let fr = carte("fr-gnome.xkb");
    assert_eq!(car(fr, KEY_1, RIEN), '&');
    assert_eq!(car(fr, KEY_1, MAJ), '1');
    assert_eq!(car(fr, KEY_2, RIEN), 'é');
    assert_eq!(car(fr, KEY_2, ALTGR), '~');
    assert_eq!(car(fr, KEY_Q, RIEN), 'a');
    assert_eq!(car(fr, KEY_W, RIEN), 'z');
    assert_eq!(car(fr, KEY_Q, MAJ), 'A');
    assert_eq!(car(fr, KEY_E, ALTGR), '€');
    assert_eq!(car(fr, KEY_M, RIEN), ',');
    assert_eq!(car(fr, KEY_APOSTROPHE, RIEN), 'ù');
    assert_eq!(car(fr, KEY_102ND, RIEN), '<');
    assert_eq!(car(fr, KEY_102ND, MAJ), '>');
    assert_eq!(car(fr, KEY_SPACE, MAJ), ' ');
    assert_eq!(touche(fr, KEY_LEFTBRACE, RIEN), Some(KeyInput::Dead('^')));
    assert_eq!(touche(fr, KEY_LEFTBRACE, MAJ), Some(KeyInput::Dead('¨')));
    assert!(fr.is_level3(KEY_RIGHTALT), "AltGr");
}

#[test]
fn verrouillages_et_raccourcis() {
    let fr = carte("fr-gnome.xkb");
    // Verr. Maj : les lettres en majuscule, pas la rangee des chiffres.
    assert_eq!(car(fr, KEY_Q, VERR_MAJ), 'A');
    assert_eq!(car(fr, KEY_Q, Modifiers { shift: true, ..VERR_MAJ }), 'a');
    assert_eq!(car(fr, KEY_2, VERR_MAJ), 'é');
    // Verr. Num : le pave donne des chiffres.
    assert_eq!(car(fr, KEY_KP7, Modifiers { num_lock: true, ..RIEN }), '7');
    // Ctrl+C a la place du C, Ctrl+A a la place du A (touche Q en QWERTY).
    assert_eq!(touche(fr, KEY_C, Modifiers { ctrl: true, ..RIEN }), Some(KeyInput::Copy));
    assert_eq!(touche(fr, KEY_Q, Modifiers { ctrl: true, ..RIEN }), Some(KeyInput::SelectAll));
    assert_eq!(touche(fr, KEY_W, Modifiers { ctrl: true, ..RIEN }), Some(KeyInput::Undo));
    // Deuxieme disposition de la carte (French alt.) : AltGr+R change.
    assert_eq!(car(fr, 19, ALTGR), '¶');
    assert_eq!(car(fr, 19, Modifiers { group: 1, ..ALTGR }), 'ê');
}

#[test]
fn qwertz_ecrit_avec_des_noms() {
    let de = carte("de-noms.xkb");
    assert_eq!(car(de, KEY_Y, RIEN), 'z');
    assert_eq!(car(de, 44, RIEN), 'y');
    assert_eq!(car(de, KEY_APOSTROPHE, RIEN), 'ä');
    assert_eq!(car(de, KEY_APOSTROPHE, MAJ), 'Ä');
    assert_eq!(car(de, KEY_1, MAJ), '!');
    assert_eq!(car(de, 16, ALTGR), '@');
    assert_eq!(car(de, KEY_E, ALTGR), '€');
}

#[test]
fn taper_dans_un_champ_avec_la_vraie_carte() {
    let fr = carte("fr-gnome.xkb");
    let d = format!("{}/tests/components", env!("CARGO_MANIFEST_DIR"));
    let mut s = EventState::new(RouteTable::new().view("/", &format!("{d}/formulaire.rsh"), &format!("{d}/formulaire.rsc")).resolve(&Route::new("/", "")).unwrap());
    let content = (0, 0, 800, 1000);
    let mut boite = None;
    interact::walk(&s.ui_nodes, content, &mut |n, b| {
        if let UiNode::TextArea(t) = n
            && t.id == "nom"
        {
            boite = Some(b);
        }
    });
    let (x, y, _, h) = boite.unwrap();
    let ev = |s: &mut EventState, e| handle_event(s, e, fr, content);
    ev(&mut s, WindowEvent::WindowMouseMove(x + 4, y + h as i32 / 2));
    ev(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, true));
    ev(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, false));
    let frappe = |s: &mut EventState, k| {
        ev(s, WindowEvent::WindowKeyPress(k, true));
        ev(s, WindowEvent::WindowKeyPress(k, false));
    };
    // « Été ê€ ^x » : Maj, touche morte, AltGr, accent sans lettre composable.
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_LEFTSHIFT, true));
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_LEFTBRACE, true));
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_LEFTBRACE, false));
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_LEFTSHIFT, false));
    frappe(&mut s, KEY_E); // ¨ + e = ë
    frappe(&mut s, KEY_2); // é
    frappe(&mut s, KEY_SPACE);
    frappe(&mut s, KEY_LEFTBRACE);
    frappe(&mut s, KEY_E); // ^ + e = ê
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_RIGHTALT, true));
    frappe(&mut s, KEY_E); // €
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_RIGHTALT, false));
    frappe(&mut s, KEY_LEFTBRACE);
    frappe(&mut s, 45); // ^ + x : rien a composer
    assert_eq!(form_values(&s.ui_nodes).get("nom"), Some(&FieldValue::Text("ëé ê€^x".into())));
    // Ctrl reste un raccourci.
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_LEFTCTRL, true));
    frappe(&mut s, KEY_Q);
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_LEFTCTRL, false));
    frappe(&mut s, KEY_W);
    assert_eq!(form_values(&s.ui_nodes).get("nom"), Some(&FieldValue::Text("z".into())), "Ctrl+A puis z remplace tout");
}
