// Outils communs : chercher dans un ecran, cliquer comme un utilisateur,
// garder une image de l'ecran (target/tmp/azure-note/<nom>.ppm).
#![allow(dead_code)]
use azure_core::rules::window_event::WindowEvent;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::interact::{self, animate_scroll, KeyboardLayout};
use std::path::{Path, PathBuf};

pub fn ui() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("ui")
}

/// Tous les textes (etiquettes, boutons, champs) dans l'ordre.
pub fn textes(nodes: &[UiNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn aller(nodes: &[UiNode], out: &mut Vec<String>) {
        for n in nodes {
            match n {
                UiNode::Label(l) => out.push(l.text.clone()),
                UiNode::Button(b) => out.push(b.text.clone()),
                UiNode::TextArea(t) => out.push(t.text.clone()),
                UiNode::Container(c) => aller(&c.children, out),
                _ => {}
            }
        }
    }
    aller(nodes, &mut out);
    out
}

pub fn boutons(nodes: &[UiNode], vue: (u32, u32, u32, u32)) -> Vec<String> {
    let mut out = Vec::new();
    interact::walk(nodes, vue, &mut |n, _| {
        if let UiNode::Button(b) = n
            && !b.id.is_empty()
        {
            out.push(b.id.clone());
        }
    });
    out
}

/// Le texte du champ `#id`.
pub fn champ(nodes: &[UiNode], id: &str) -> Option<String> {
    let mut found = None;
    interact::walk(nodes, (0, 0, 2000, 2000), &mut |n, _| {
        if let UiNode::TextArea(t) = n
            && t.id == id
        {
            found = Some(t.text.clone());
        }
    });
    found
}

pub fn boite(nodes: &[UiNode], id: &str, vue: (u32, u32, u32, u32)) -> Option<(i32, i32, u32, u32)> {
    let mut found = None;
    interact::walk(nodes, vue, &mut |n, b| {
        let touche = match n {
            UiNode::Button(x) => x.id == id,
            UiNode::TextArea(x) => x.id == id,
            UiNode::Control(x) => x.id == id,
            _ => false,
        };
        if touche {
            found = Some(b);
        }
    });
    found
}

/// Clique au centre de `#id` (bouton ou champ) ; rend l'id touche.
pub fn cliquer(state: &mut EventState, id: &str, vue: (u32, u32, u32, u32)) -> Option<String> {
    let visible = |b: (i32, i32, u32, u32)| b.1 >= 0 && b.1 + b.3 as i32 <= vue.3 as i32 && b.2 > 0 && b.3 > 0;
    if !boite(&state.ui_nodes, id, vue).is_some_and(visible) {
        interact::scroll_to_anchor(&mut state.ui_nodes, id, vue);
        while animate_scroll(&mut state.ui_nodes) {}
    }
    let b = boite(&state.ui_nodes, id, vue)?;
    let (x, y) = (b.0 + b.2 as i32 / 2, b.1 + b.3 as i32 / 2);
    state.clicked_id = None;
    handle_event(state, WindowEvent::WindowMouseMove(x, y), KeyboardLayout::Qwerty, vue);
    handle_event(state, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, vue);
    handle_event(state, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, vue);
    state.clicked_id.clone()
}

/// Une touche pressee puis relachee (code evdev).
pub fn touche(state: &mut EventState, code: u32, vue: (u32, u32, u32, u32)) {
    handle_event(state, WindowEvent::WindowKeyPress(code, true), KeyboardLayout::Qwerty, vue);
    handle_event(state, WindowEvent::WindowKeyPress(code, false), KeyboardLayout::Qwerty, vue);
}

pub fn capture(nodes: &[UiNode], taille: (u32, u32), nom: &str) {
    capture_souris(nodes, taille, (-1, -1), nom);
}

/// Comme `capture`, la souris en `(x, y)` (survol).
pub fn capture_souris(nodes: &[UiNode], (w, h): (u32, u32), (x, y): (i32, i32), nom: &str) {
    let mut canvas = Canvas::new(w, h);
    draw_ui(nodes, (0, 0, w, h), &mut canvas, x, y, false);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-note");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join(format!("{nom}.ppm")), ppm).unwrap();
}
