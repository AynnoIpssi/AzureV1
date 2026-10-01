// Selectionner du texte affiche a la souris et le copier, comme dans un
// navigateur, via `event::services::dispatch` (le chemin d'une vraie
// fenetre).
use azure_core::rules::window_event::WindowEvent;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::{BTN_LEFT, KEY_LEFTCTRL};
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::interact::{self, HoverKind, KeyboardLayout};

const CONTENT: (u32, u32, u32, u32) = (0, 0, 800, 600);
const KEY_A: u32 = 30;
const KEY_C: u32 = 46;

fn page() -> EventState {
    let d = format!("{}/tests/selection", env!("CARGO_MANIFEST_DIR"));
    EventState::new(RouteTable::new().view("/", &format!("{d}/page.rsh"), &format!("{d}/page.rsc")).resolve(&Route::new("/", "")).unwrap())
}

fn ev(s: &mut EventState, e: WindowEvent) -> bool {
    handle_event(s, e, KeyboardLayout::Qwerty, CONTENT)
}

/// Boite du texte qui commence par `debut`.
fn boite(s: &EventState, debut: &str) -> (i32, i32, u32, u32) {
    let mut found = None;
    interact::walk(&s.ui_nodes, CONTENT, &mut |n, b| {
        if let UiNode::Label(l) = n
            && l.text.starts_with(debut)
            && found.is_none()
        {
            found = Some(b);
        }
    });
    found.unwrap_or_else(|| panic!("« {debut} » introuvable"))
}

fn glisser(s: &mut EventState, de: (i32, i32), a: (i32, i32)) {
    ev(s, WindowEvent::WindowMouseMove(de.0, de.1));
    ev(s, WindowEvent::WindowMouseButton(BTN_LEFT, true));
    ev(s, WindowEvent::WindowMouseMove((de.0 + a.0) / 2, (de.1 + a.1) / 2));
    ev(s, WindowEvent::WindowMouseMove(a.0, a.1));
    ev(s, WindowEvent::WindowMouseButton(BTN_LEFT, false));
}

fn clic(s: &mut EventState, x: i32, y: i32) -> bool {
    ev(s, WindowEvent::WindowMouseMove(x, y));
    let changed = ev(s, WindowEvent::WindowMouseButton(BTN_LEFT, true));
    ev(s, WindowEvent::WindowMouseButton(BTN_LEFT, false));
    changed
}

fn ctrl(s: &mut EventState, key: u32) -> bool {
    ev(s, WindowEvent::WindowKeyPress(KEY_LEFTCTRL, true));
    let changed = ev(s, WindowEvent::WindowKeyPress(key, true));
    ev(s, WindowEvent::WindowKeyPress(key, false));
    ev(s, WindowEvent::WindowKeyPress(KEY_LEFTCTRL, false));
    changed
}

fn selection(s: &EventState) -> Option<String> {
    interact::selected_text(&s.ui_nodes, CONTENT)
}

#[test]
fn glisser_selectionne_a_travers_plusieurs_textes_puis_ctrl_c_copie() {
    let mut s = page();
    let t = boite(&s, "Bonjour");
    let p = boite(&s, "Premier");
    // Du debut du titre jusqu'au milieu du paragraphe.
    glisser(&mut s, (t.0 + 1, t.1 + t.3 as i32 / 2), (p.0 + 60, p.1 + p.3 as i32 / 2));
    let texte = selection(&s).expect("une selection");
    assert!(texte.starts_with("Bonjour tout le monde\nPrem"), "{texte:?}");
    assert!(!texte.contains("page."), "{texte:?}");

    assert!(!s.clipboard_changed);
    ctrl(&mut s, KEY_C);
    assert_eq!(s.clipboard, texte);
    assert!(s.take_clipboard_change(), "la copie part vers le presse-papiers du systeme");
}

#[test]
fn double_clic_un_mot_triple_clic_tout_le_texte() {
    let mut s = page();
    let t = boite(&s, "Bonjour");
    // « tout » : un peu apres « Bonjour ».
    let (x, y) = (t.0 + 150, t.1 + t.3 as i32 / 2);
    clic(&mut s, x, y);
    clic(&mut s, x, y);
    assert_eq!(selection(&s).as_deref(), Some("tout"));
    clic(&mut s, x, y);
    assert_eq!(selection(&s).as_deref(), Some("Bonjour tout le monde"));
}

#[test]
fn ctrl_a_selectionne_la_page_et_les_morceaux_colles_restent_colles() {
    let mut s = page();
    assert!(ctrl(&mut s, KEY_A));
    let texte = selection(&s).unwrap();
    assert!(texte.contains("Bonjour tout le monde\nPremier paragraphe de la page.\nfn main()"), "{texte:?}");
}

#[test]
fn un_clic_ailleurs_efface_la_selection() {
    let mut s = page();
    ctrl(&mut s, KEY_A);
    assert!(clic(&mut s, 700, 580), "effacer la selection redessine");
    assert_eq!(selection(&s), None);
}

#[test]
fn un_bouton_reste_un_bouton() {
    let mut s = page();
    let mut b = None;
    interact::walk(&s.ui_nodes, CONTENT, &mut |n, r| {
        if matches!(n, UiNode::Button(_)) {
            b = Some(r);
        }
    });
    let b = b.unwrap();
    glisser(&mut s, (b.0 + 4, b.1 + 4), (b.0 + 60, b.1 + 4));
    assert_eq!(s.clicked_id.as_deref(), Some("aller"));
    assert_eq!(selection(&s), None);
}

#[test]
fn le_texte_montre_le_curseur_de_texte() {
    let mut s = page();
    let p = boite(&s, "Premier");
    ev(&mut s, WindowEvent::WindowMouseMove(p.0 + 5, p.1 + 5));
    assert_eq!(s.hover, HoverKind::Text);
}
