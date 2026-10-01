// Dessine un arbre de `UiNode` (celui construit par
// `compiler::services::interpreter::build_ui`, a partir d'un .rsh style par
// rsC ou par l'ancien systeme `.style`) directement sur un `Canvas` - c'est
// ce qui permet de voir le resultat dans une vraie fenetre plutot que de
// juste inspecter des valeurs dans un test.
use crate::layout::managers::web_layout::layout_roots;
use crate::layout::managers::layout_manager::{container_layout, intersect, to_rect, Rect};
use crate::ui::models::button::Button;
use crate::ui::models::container::Container;
use crate::ui::models::image::Image;
use crate::ui::models::textarea::TextArea;
use crate::ui::models::ui_node::UiNode;
use crate::ui::models::video::Video;
use crate::ui::services::draw_label::draw_label;
use crate::ui::services::text_layout;
use azure_engine::rendering::managers::renderer::{char_position, draw_box, draw_image as engine_draw_image, draw_rect, draw_text, load_image, measure_text_width};
use azure_engine::rendering::services::shapes::styled_box::blend_layer;
use crate::ui::models::decoration::Decoration;
use crate::style::models::web_style::TextAlign;
use azure_engine::rendering::models::paint::{BoxStyle, Fill};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::services::shapes::rect::draw_empty_rect;

// Meme police que `draw_label` - ancre au dossier de CE crate (resolu a la
// COMPILATION via `CARGO_MANIFEST_DIR`), pas au dossier depuis lequel le
// binaire final est lance (voir `ui::services::draw_label::FONT_PATH` pour
// le pourquoi). `pub(crate)` : reutilise par `ui::services::interact` pour
// le hit-testing souris (clic -> index de caractere), qui doit rester
// coherent avec ce qui est reellement dessine ici - voir
// `char_position`/`char_index_at`.
pub const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../azure-engine/src/Sora-VariableFont_wght.ttf");

// Taille/poids/marge interne du texte d'une textarea - repris a
// l'identique pour placer le curseur/la selection (voir `char_position`)
// et convertir un clic souris en index de caractere (voir
// `ui::services::interact`), sinon tout desynchronise.
pub const TEXTAREA_FONT_SIZE: f32 = 16.0;
pub const TEXTAREA_FONT_WEIGHT: f32 = 400.0;
pub const TEXTAREA_TEXT_PADDING: u32 = 6;

// Marge entre la position mesuree et le curseur de saisie affiche - une
// police variable + l'anti-collision par pixel de `draw_text` peuvent
// placer l'encre reelle un chouia au-dela de la position "propre" qu'on
// calcule (voir `char_position`) ; sans cette marge, le curseur semblait
// parfois empieter sur le caractere voisin plutot que d'etre nettement
// separe.
const CARET_GAP: u32 = 2;

// Le "mode souris" (voir ui::services::interact::HoverKind) n'est plus
// dessine ici par un contour de couleur : c'est le vrai curseur systeme qui
// change de forme (voir `crate::cursor`).
const FOCUS_COLOR: Color = Color::new(255, 255, 255, 255);
// Opaque (comme tout le reste de ce rendu, voir `rect::draw_rect`) : ce
// n'est pas un vrai blending semi-transparent - `draw_rect` ecrit les
// pixels directement, il ne les fusionne pas avec ce qu'il y avait avant.
// Une alpha < 255 ici laisserait voir a travers vers ce qu'il y a *derriere
// la fenetre* (le bureau), pas une teinte sur le fond de la textarea.
// La meme que celle du texte affiche (voir `draw_label::TEXT_SELECTION_COLOR`).
const SELECTION_COLOR: Color = crate::ui::services::draw_label::TEXT_SELECTION_COLOR;

// `Canvas::clip` n'est PAS une pile (voir `azure_engine::rendering::models::canvas::Canvas`) :
// un `set_clip`/`clear_clip` brut ecrase ou efface INCONDITIONNELLEMENT tout
// clip deja actif. Sans consequence tant qu'aucun appelant n'imbriquait de
// portees clippees - mais `draw_container` clippe desormais ses enfants a sa
// boite de contenu (fix d'audit : plus de debordement), et un enfant peut
// lui-meme etre une `Image`/`TextArea`/`Container` qui clippe - donc
// `push_clip`/`pop_clip` (au lieu d'appeler `Canvas` directement) : le
// nouveau clip est l'INTERSECTION avec celui deja actif (jamais un simple
// remplacement qui l'elargirait), et on restaure exactement le clip
// PRECEDENT en sortant de la portee (jamais `clear_clip`, qui effacerait
// aussi celui d'un ancêtre englobant).
pub(crate) fn push_clip(canvas: &mut Canvas, box_: (u32, u32, u32, u32)) -> (u32, u32, u32, u32) {
    let previous = canvas.clip_bounds();
    let (px, py, pw, ph) = previous;
    let (bx, by, bw, bh) = box_;
    let x = bx.max(px);
    let y = by.max(py);
    let w = (bx + bw).min(px + pw).saturating_sub(x);
    let h = (by + bh).min(py + ph).saturating_sub(y);
    canvas.set_clip(x, y, w, h);
    previous
}

pub(crate) fn pop_clip(canvas: &mut Canvas, previous: (u32, u32, u32, u32)) {
    canvas.set_clip(previous.0, previous.1, previous.2, previous.3);
}

/// `mouse_x`/`mouse_y` pilotent l'indicateur visuel de survol (voir
/// `ui::services::interact::HoverKind`) - le clic/focus/saisie reels sont
/// geres a part par `ui::services::interact`, appele depuis
/// `AzureWindow::run` avant le redessin. `caret_visible` pilote juste la
/// visibilite du curseur clignotant d'une textarea focalisee (l'etat
/// "clignotant" - le tic qui le fait basculer toutes les ~500ms - vit
/// aussi dans `AzureWindow::run`, pas ici : ce module ne fait que dessiner
/// l'etat qu'on lui donne).
/// `parent` est la boite de CONTENU (voir `layout::managers::layout_manager::content_box`)
/// dans laquelle `nodes` doit se positionner - `(0, 0, window_width, window_height)`
/// pour l'arbre racine, qui n'a pas de vrai parent. Chaque nœud RACINE se
/// resout ici independamment des autres contre `parent` (mode `Block`
/// implicite pour la liste racine - le flex/grid n'existe qu'au niveau d'un
/// `Container`, voir `draw_container`) avant que `draw_node` ne le dessine
/// avec sa boite deja calculee.
pub fn draw_ui(nodes: &[UiNode], parent: (u32, u32, u32, u32), canvas: &mut Canvas, mouse_x: i32, mouse_y: i32, caret_visible: bool) {
    let roots = layout_roots(nodes, parent);
    for (node, own_box) in nodes.iter().zip(roots) {
        if !crate::layout::managers::web_layout::is_fixed(node) {
            draw_node(node, own_box, canvas, mouse_x, mouse_y, caret_visible);
        }
    }
    // Couches `position: fixed` (modales, panneaux, notifications), du
    // dessous au dessus.
    for path in crate::ui::services::interact::layer_paths(nodes) {
        let node = crate::ui::services::interact::node_at(nodes, &path);
        draw_node(node, crate::layout::managers::web_layout::fixed_box(node, parent), canvas, mouse_x, mouse_y, caret_visible);
    }
    // Contours de focus clavier.
    crate::ui::services::interact::walk(nodes, parent, &mut |node, own_box| {
        let ring = match node {
            UiNode::Button(b) => b.focus_ring,
            UiNode::Control(c) => c.focus_ring,
            _ => false,
        };
        if ring {
            draw_focus_ring(own_box, canvas);
        }
    });
    // Liste deroulante ouverte : par-dessus tout le reste.
    crate::ui::services::interact::walk(nodes, parent, &mut |node, own_box| {
        if let UiNode::Control(control) = node
            && control.open {
                crate::ui::services::draw_control::draw_open_select(control, own_box, canvas, mouse_x, mouse_y);
            }
    });
}

const RING_COLOR: Color = Color::new(201, 168, 120, 230);

fn draw_focus_ring(own_box: Rect, canvas: &mut Canvas) {
    let r = (own_box.0 - 3, own_box.1 - 3, own_box.2 + 6, own_box.3 + 6);
    let mut style = BoxStyle::solid(Color::new(0, 0, 0, 0));
    style.radius = 8.0;
    style.border = azure_engine::rendering::models::paint::BorderWidths::uniform(2.0);
    style.border_color = RING_COLOR;
    if r.0 >= 0 && r.1 >= 0 {
        draw_box(r.0, r.1, r.2, r.3, &style, canvas);
    }
}

/// L'infobulle `text` pres de la souris (dessinee par-dessus tout, voir
/// `EventState::tooltip`), gardee dans `bounds`.
pub fn draw_tooltip(text: &str, mouse_x: i32, mouse_y: i32, bounds: (u32, u32, u32, u32), canvas: &mut Canvas) {
    let size = 12.5;
    let width = measure_text_width(text, FONT_PATH, size, 500.0).unwrap_or(0.0).ceil() as u32 + 20;
    let height = 26u32;
    let max_x = (bounds.0 + bounds.2).saturating_sub(width + 4) as i32;
    let x = (mouse_x + 12).min(max_x).max(bounds.0 as i32);
    let y = if mouse_y + 22 + height as i32 > (bounds.1 + bounds.3) as i32 { mouse_y - height as i32 - 8 } else { mouse_y + 22 };
    let mut style = BoxStyle::solid(Color::new(240, 236, 228, 245));
    style.radius = 6.0;
    style.shadow = Some(azure_engine::rendering::models::paint::Shadow { offset_x: 0.0, offset_y: 4.0, blur: 12.0, spread: 0.0, color: Color::new(0, 0, 0, 110), inset: false });
    draw_box(x, y.max(0), width, height, &style, canvas);
    let _ = draw_text(text, FONT_PATH, (x + 10) as u32, (y.max(0) + 6) as u32, size, 500.0, &Color::new(26, 23, 18, 255), canvas);
}

/// Le panneau de mise en forme du clic droit (voir `interact::FormatMenu`).
pub fn draw_format_menu(menu: &crate::ui::services::interact::FormatMenu, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32) {
    use azure_engine::rendering::managers::renderer::{draw_text_with, TextOptions};
    let r = menu.rect();
    let mut panel = BoxStyle::solid(Color::new(32, 30, 28, 252));
    panel.radius = 9.0;
    panel.border = azure_engine::rendering::models::paint::BorderWidths::uniform(1.0);
    panel.border_color = Color::new(70, 65, 58, 255);
    panel.shadow = Some(azure_engine::rendering::models::paint::Shadow { offset_x: 0.0, offset_y: 6.0, blur: 18.0, spread: 0.0, color: Color::new(0, 0, 0, 140), inset: false });
    draw_box(r.0, r.1, r.2, r.3, &panel, canvas);
    for item in menu.items() {
        let (x, y, w, h) = item.rect;
        if contains_point(item.rect, mouse_x, mouse_y) {
            let mut hover = BoxStyle::solid(Color::new(201, 168, 120, 40));
            hover.radius = 6.0;
            draw_box(x, y, w, h, &hover, canvas);
        }
        let size = if item.mark == "taille-" { 10.5 } else if item.mark.starts_with("taille-") { 12.0 } else { 13.0 };
        let (font, weight, options) = match item.mark {
            "gras" => (FONT_PATH, 800.0, TextOptions::default()),
            "italique" => (FONT_PATH, 500.0, TextOptions { italic: true, letter_spacing: 0.0 }),
            "code" => (crate::ui::services::fonts::font_for(Some("monospace")), 500.0, TextOptions::default()),
            _ => (FONT_PATH, if item.color.is_some() { 800.0 } else { 600.0 }, TextOptions::default()),
        };
        let color = item.color.unwrap_or(Color::new(228, 224, 216, 255));
        let tw = azure_engine::rendering::managers::renderer::measure_text_width_with(item.label, font, size, weight, &options).unwrap_or(0.0);
        let tx = (x as f32 + (w as f32 - tw) / 2.0).max(0.0);
        let ty = (y as f32 + (h as f32 - size * 1.2) / 2.0).max(0.0);
        let _ = draw_text_with(item.label, font, tx as u32, ty as u32, size, weight, &color, &options, canvas);
        let line_y = (ty + size * 1.2) as u32;
        match item.mark {
            "souligne" => draw_rect(tx as u32, line_y, tw.ceil() as u32, 1, &color, canvas),
            "barre" => draw_rect(tx as u32, (ty + size * 0.62) as u32, tw.ceil() as u32, 1, &color, canvas),
            _ => {}
        }
    }
}

/// Le menu `/` au clavier (voir `interact::CommandMenu`) : la recherche
/// tapee et sa completion en tete, puis les commandes, la choisie en
/// surbrillance.
pub fn draw_command_menu(menu: &crate::ui::services::interact::CommandMenu, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32) {
    use crate::ui::services::interact::command_menu_consts::{HEAD_H, PAD};
    let r = menu.rect();
    let mut panel = BoxStyle::solid(Color::new(40, 38, 35, 252));
    panel.radius = 9.0;
    panel.border = azure_engine::rendering::models::paint::BorderWidths::uniform(1.0);
    panel.border_color = Color::new(74, 69, 62, 255);
    panel.shadow = Some(azure_engine::rendering::models::paint::Shadow { offset_x: 0.0, offset_y: 8.0, blur: 22.0, spread: 0.0, color: Color::new(0, 0, 0, 150), inset: false });
    draw_box(r.0, r.1, r.2, r.3, &panel, canvas);
    let clip = push_clip(canvas, (r.0.max(0) as u32, r.1.max(0) as u32, r.2, r.3));

    // En tete : « /tit » puis la fin grisee (« re 1 »), ou « Commandes ».
    let (hx, hy) = (r.0 + 12, r.1 + PAD as i32 + 7);
    let dim = Color::new(140, 136, 128, 255);
    if menu.query.is_empty() {
        let _ = draw_text("Tape pour filtrer · Entrée pour choisir", FONT_PATH, hx.max(0) as u32, hy.max(0) as u32, 11.5, 500.0, &dim, canvas);
    } else {
        let typed = format!("/{}", menu.query);
        let _ = draw_text(&typed, FONT_PATH, hx.max(0) as u32, hy.max(0) as u32, 12.5, 600.0, &Color::new(236, 232, 224, 255), canvas);
        let w = measure_text_width(&typed, FONT_PATH, 12.5, 600.0).unwrap_or(0.0);
        let rest = menu.completion();
        if !rest.is_empty() {
            let _ = draw_text(&rest, FONT_PATH, (hx as f32 + w) as u32, hy.max(0) as u32, 12.5, 600.0, &Color::new(120, 116, 108, 255), canvas);
        }
    }
    draw_rect((r.0 + 1).max(0) as u32, (r.1 + (PAD + HEAD_H) as i32 - 3).max(0) as u32, r.2.saturating_sub(2), 1, &Color::new(60, 57, 52, 255), canvas);

    if menu.shown.is_empty() {
        let _ = draw_text("Aucune commande", FONT_PATH, (r.0 + 12).max(0) as u32, (r.1 + (PAD + HEAD_H) as i32 + 9).max(0) as u32, 12.5, 500.0, &dim, canvas);
    }
    for (i, (x, y, w, h)) in menu.rows() {
        let item = &menu.items[menu.shown[i]];
        let hovered = contains_point((x, y, w, h), mouse_x, mouse_y);
        if i == menu.selected || hovered {
            let mut hl = BoxStyle::solid(if i == menu.selected { Color::new(201, 168, 120, 46) } else { Color::new(255, 255, 255, 12) });
            hl.radius = 6.0;
            draw_box(x, y, w, h, &hl, canvas);
        }
        let _ = draw_text(&item.label, FONT_PATH, (x + 9).max(0) as u32, (y + 3).max(0) as u32, 13.0, 600.0, &Color::new(238, 234, 226, 255), canvas);
        if !item.desc.is_empty() {
            let _ = draw_text(&item.desc, FONT_PATH, (x + 9).max(0) as u32, (y + 19).max(0) as u32, 10.5, 500.0, &dim, canvas);
        }
    }
    pop_clip(canvas, clip);
}

/// Glisser-deposer en cours (voir `interact::drag`) : la zone survolee
/// entouree, un trait a l'emplacement ou l'element tomberait, et sa copie
/// translucide sous la souris.
pub fn draw_drag(nodes: &[UiNode], drag: &crate::ui::services::interact::Drag, mouse_x: i32, mouse_y: i32, canvas: &mut Canvas) {
    if !drag.active {
        return;
    }
    if let Some(target) = &drag.target {
        let z = target.zone_box;
        let mut ring = BoxStyle::solid(Color::new(0, 0, 0, 0));
        ring.radius = 8.0;
        ring.border = azure_engine::rendering::models::paint::BorderWidths::uniform(2.0);
        ring.border_color = RING_COLOR;
        draw_box(z.0, z.1, z.2, z.3, &ring, canvas);
        let mut line = BoxStyle::solid(RING_COLOR);
        line.radius = 1.5;
        let l = target.line;
        draw_box(l.0, l.1, l.2, l.3, &line, canvas);
    }
    let Some(node) = crate::ui::services::interact::node_at_path(nodes, &drag.path) else { return };
    let ghost = (mouse_x - drag.grab.0, mouse_y - drag.grab.1, drag.size.0, drag.size.1);
    let region = intersect(paint_bounds(node.decoration(), ghost), to_rect(canvas.clip_bounds()));
    if region.2 > 0 && region.3 > 0 {
        draw_with_opacity(node, ghost, region, 0.8, canvas, -1, -1, false);
    }
}

// `own_box` est la boite DEJA resolue de `node` (calculee par l'appelant :
// `draw_ui` pour un nœud racine, `draw_container` pour un enfant via
// `layout_container` - potentiellement un vrai calcul flex/grid, pas juste
// `resolve(&node.layout, parent)`). Ce n'est donc plus a `draw_node`/aux
// fonctions `draw_*` de re-resoudre quoi que ce soit : la boite qu'on leur
// donne est definitive.
//
// Un nœud entierement hors de la zone de decoupage courante (typiquement :
// defile hors de la partie visible d'un conteneur) n'est pas dessine du
// tout - ni texte mis en forme, ni sous-arbre parcouru. C'est ce qui rend le
// cout d'une image proportionnel a ce qui est VISIBLE, pas a la taille de
// la page.
fn draw_node(node: &UiNode, own_box: Rect, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32, caret_visible: bool) {
    let painted = intersect(paint_bounds(node.decoration(), own_box), to_rect(canvas.clip_bounds()));
    if painted.2 == 0 || painted.3 == 0 {
        return;
    }
    let decoration = node.decoration();
    let opacity = match decoration.group_hover_opacity {
        Some(o) if HOVER_GROUPS.with(|g| g.borrow().last().copied().unwrap_or(false)) => o,
        _ => decoration.opacity,
    };
    if opacity < 1.0 {
        draw_with_opacity(node, own_box, painted, opacity, canvas, mouse_x, mouse_y, caret_visible);
    } else {
        draw_opaque_node(node, own_box, canvas, mouse_x, mouse_y, caret_visible);
    }
}

// La zone qu'un nœud peut peindre : sa boite, plus son ombre portee.
fn paint_bounds(decoration: &Decoration, own_box: Rect) -> Rect {
    let Some(s) = decoration.shadow else { return own_box };
    let reach = (s.blur + s.spread.max(0.0) + 1.0).ceil() as i32;
    let left = own_box.0.min(own_box.0 + s.offset_x as i32 - reach);
    let top = own_box.1.min(own_box.1 + s.offset_y as i32 - reach);
    let right = (own_box.0 + own_box.2 as i32).max(own_box.0 + own_box.2 as i32 + s.offset_x as i32 + reach);
    let bottom = (own_box.1 + own_box.3 as i32).max(own_box.1 + own_box.3 as i32 + s.offset_y as i32 + reach);
    (left, top, (right - left) as u32, (bottom - top) as u32)
}

thread_local! {
    // Pendant le dessin : pour chaque groupe de survol traverse (voir
    // `Decoration::hover_group`), est-il sous la souris ?
    static HOVER_GROUPS: std::cell::RefCell<Vec<bool>> = const { std::cell::RefCell::new(Vec::new()) };
}

// `opacity` (rsC) s'applique au nœud ET a tout son contenu d'un bloc, comme
// en CSS : il est dessine a part, sur une copie de ce qu'il y a dessous,
// puis melange a `opacity` avec l'original. `region` est la partie visible
// de ce qu'il peint (deja decoupee).
#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
fn draw_with_opacity(node: &UiNode, own_box: Rect, region: Rect, opacity: f32, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32, caret_visible: bool) {
    if opacity <= 0.0 {
        return;
    }
    let (rx, ry, rw, rh) = (region.0 as u32, region.1 as u32, region.2, region.3);
    let mut layer = Canvas::new(rw, rh);
    for row in 0..rh {
        let src = (((ry + row) * canvas.width + rx) * 4) as usize;
        let dst = (row * rw * 4) as usize;
        layer.buffer[dst..dst + rw as usize * 4].copy_from_slice(&canvas.buffer[src..src + rw as usize * 4]);
    }
    let shifted = (own_box.0 - region.0, own_box.1 - region.1, own_box.2, own_box.3);
    draw_opaque_node(node, shifted, &mut layer, mouse_x - region.0, mouse_y - region.1, caret_visible);
    blend_layer(canvas, &layer, rx, ry, opacity);
}

fn draw_opaque_node(node: &UiNode, own_box: Rect, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32, caret_visible: bool) {
    let visible = intersect(own_box, to_rect(canvas.clip_bounds()));
    if let UiNode::Container(container) = node {
        draw_container(container, own_box, canvas, mouse_x, mouse_y, caret_visible);
        return;
    }
    // `visibility: hidden` : la place reste prise, rien n'est dessine.
    if !node.decoration().visible {
        return;
    }
    // Ombre, fond et bordure d'une feuille : `draw_box` accepte une
    // position signee, donc dessines ici pour les deux chemins ci-dessous.
    let decoration = node.decoration();
    match node {
        UiNode::Button(button) => {
            let hovered = contains_point(own_box, mouse_x, mouse_y);
            let fill = match &decoration.transition {
                // Transition : melange progressif entre l'etat de base et le survol.
                Some(t) => {
                    let p = decoration.anim.step(t, hovered);
                    crate::ui::models::transition::mix_fill(&button_fill(button, false), &button_fill(button, true), p)
                }
                None => button_fill(button, hovered),
            };
            draw_box(own_box.0, own_box.1, own_box.2, own_box.3, &decoration.box_style_with(fill), canvas);
        }
        UiNode::TextArea(area) => {
            let hovered = contains_point(own_box, mouse_x, mouse_y);
            let fill = match &decoration.transition {
                Some(t) => {
                    let active = hovered || area.focused;
                    let p = decoration.anim.step(t, active);
                    let target = if area.focused { textarea_fill(area, hovered) } else { textarea_fill(area, true) };
                    crate::ui::models::transition::mix_fill(&textarea_base_fill(area), &target, p)
                }
                None => textarea_fill(area, hovered),
            };
            draw_box(own_box.0, own_box.1, own_box.2, own_box.3, &decoration.box_style_with(fill), canvas);
        }
        _ if decoration.fill.is_some() || !decoration.is_plain() => {
            draw_box(own_box.0, own_box.1, own_box.2, own_box.3, &decoration.box_style(Color::new(0, 0, 0, 0)), canvas);
        }
        _ => {}
    }
    if decoration.background_image.is_some() {
        let corners = (decoration.radius > 0.0).then(|| save_corners(canvas, own_box, decoration.radius));
        draw_background_image(decoration, own_box, canvas);
        if let Some(saved) = corners {
            restore_corners(canvas, &saved, own_box, decoration.radius);
        }
    }
    if visible.2 == 0 || visible.3 == 0 {
        return;
    }
    if own_box.0 >= 0 && own_box.1 >= 0 {
        draw_leaf(node, (own_box.0 as u32, own_box.1 as u32, own_box.2, own_box.3), canvas, mouse_x, mouse_y, caret_visible);
    } else {
        draw_leaf_offscreen(node, own_box, visible, canvas, mouse_x, mouse_y, caret_visible);
    }
}

fn draw_leaf(node: &UiNode, own_box: (u32, u32, u32, u32), canvas: &mut Canvas, mouse_x: i32, mouse_y: i32, caret_visible: bool) {
    match node {
        UiNode::Container(_) => {}
        UiNode::Label(label) => draw_label(label, own_box, canvas),
        UiNode::Button(button) => draw_button(button, own_box, canvas, mouse_x, mouse_y),
        UiNode::Image(image) => draw_image(image, own_box, canvas),
        UiNode::Video(video) => draw_video(video, own_box, canvas),
        UiNode::TextArea(textarea) => draw_textarea(textarea, own_box, canvas, mouse_x, mouse_y, caret_visible),
        UiNode::Control(control) => crate::ui::services::draw_control::draw_control(control, (own_box.0 as i32, own_box.1 as i32, own_box.2, own_box.3), canvas, mouse_x, mouse_y),
    }
}

// Une feuille qui commence au-dessus (ou a gauche) du bord de la fenetre -
// en partie defilee hors de vue - ne peut pas etre dessinee directement :
// les primitives de azure-engine ne prennent que des positions positives.
// Elle est dessinee a (0, 0) dans un petit canvas a sa taille, prerempli
// avec ce qu'il y a deja dessous (le texte se melange au fond), puis seule
// sa partie `visible` est recopiee. Ne concerne que les quelques feuilles
// a cheval sur ce bord.
fn draw_leaf_offscreen(node: &UiNode, own_box: Rect, visible: Rect, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32, caret_visible: bool) {
    let (ox, oy, w, h) = own_box;
    let mut local = Canvas::new(w, h);
    let copy = |from: &Canvas, to: &mut Canvas, from_x: u32, from_y: u32, to_x: u32, to_y: u32| {
        let row_bytes = visible.2 as usize * 4;
        for row in 0..visible.3 {
            let src = (((from_y + row) * from.width + from_x) * 4) as usize;
            let dst = (((to_y + row) * to.width + to_x) * 4) as usize;
            to.buffer[dst..dst + row_bytes].copy_from_slice(&from.buffer[src..src + row_bytes]);
        }
    };
    // Position de la partie visible dans le canvas local et dans `canvas`.
    let (lx, ly) = ((visible.0 - ox) as u32, (visible.1 - oy) as u32);
    let (cx, cy) = (visible.0 as u32, visible.1 as u32);
    copy(canvas, &mut local, cx, cy, lx, ly);
    draw_leaf(node, (0, 0, w, h), &mut local, mouse_x - ox, mouse_y - oy, caret_visible);
    copy(&local, canvas, lx, ly, cx, cy);
}

pub fn draw_container(container: &Container, own_box: Rect, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32, caret_visible: bool) {
    let clip = to_rect(canvas.clip_bounds());
    // Cache (`visibility: hidden`) : pas son fond, mais ses enfants peuvent
    // redevenir visibles (`visibility: visible`), comme en CSS.
    if container.decoration.visible {
        draw_box(own_box.0, own_box.1, own_box.2, own_box.3, &container.decoration.box_style(container.background), canvas);
    }

    // Les enfants se positionnent dans la boite de CONTENU du conteneur
    // (sa propre boite retrecie par son padding), decalee par son
    // defilement - voir `layout_container`, le MEME calcul que celui fait par
    // `ui::services::interact` : rendu et hit-test ne peuvent pas diverger.
    let layout = container_layout(container, own_box);

    // Decoupage a la boite du conteneur (padding compris, comme en CSS) : un
    // enfant qui deborde (flex/grid mal dimensionne, contenu d'un conteneur
    // defilable) ne peint pas au-dela du conteneur, mais son ombre peut
    // s'etendre dans le padding.
    let content = intersect(layout.clip, clip);
    if content.2 == 0 || content.3 == 0 {
        return;
    }
    let previous_clip = push_clip(canvas, (content.0 as u32, content.1 as u32, content.2, content.3));
    // Coins arrondis : le contenu ne deborde pas dans l'arrondi. Ce qu'il y
    // avait sous les 4 coins est garde, puis remis hors de la forme arrondie
    // une fois les enfants dessines.
    let corners = (container.decoration.radius > 0.0).then(|| save_corners(canvas, own_box, container.decoration.radius));
    if container.decoration.visible {
        draw_background_image(&container.decoration, own_box, canvas);
    }
    // Les enfants `position: absolute` par-dessus les autres (par z-index).
    let group = container.decoration.hover_group;
    if group {
        HOVER_GROUPS.with(|g| g.borrow_mut().push(contains_point(own_box, mouse_x, mouse_y)));
    }
    for i in paint_order(&container.children) {
        draw_node(&container.children[i], layout.children[i], canvas, mouse_x, mouse_y, caret_visible);
    }
    if group {
        HOVER_GROUPS.with(|g| g.borrow_mut().pop());
    }
    let scrollbar = container.decoration.scrollbar_color.unwrap_or(SCROLLBAR_COLOR);
    if container.layout.scrollable() && layout.max_scroll > 0 {
        // Contre le bord droit du conteneur (dans son padding), sur la
        // hauteur de sa zone de contenu : la barre ne recouvre pas le contenu.
        let track = (layout.clip.0, layout.visible.1, layout.clip.2, layout.visible.3);
        draw_scrollbar(track, layout.content_height, container.scroll_offset.min(layout.max_scroll), scrollbar, canvas);
    }
    if container.layout.scrollable_x() && layout.max_scroll_x > 0 {
        let track = (layout.visible.0, layout.clip.1, layout.visible.2, layout.clip.3);
        draw_scrollbar_x(track, layout.content_width, container.scroll_x.min(layout.max_scroll_x), scrollbar, canvas);
    }
    if let Some(saved) = corners {
        restore_corners(canvas, &saved, own_box, container.decoration.radius);
    }
    pop_clip(canvas, previous_clip);
}

/// `background-image: url(...)` dans la boite (decoupe a elle).
fn draw_background_image(decoration: &Decoration, own_box: Rect, canvas: &mut Canvas) {
    let Some(bg) = &decoration.background_image else { return };
    let Ok(image) = load_image(&bg.src) else { return };
    let visible = intersect(own_box, to_rect(canvas.clip_bounds()));
    if visible.2 == 0 || visible.3 == 0 {
        return;
    }
    let (x, y, w, h) = bg.placement((image.width, image.height), own_box);
    let previous = push_clip(canvas, (visible.0 as u32, visible.1 as u32, visible.2, visible.3));
    azure_engine::rendering::managers::renderer::draw_image_at(&image, x, y, w, h, canvas);
    pop_clip(canvas, previous);
}

/// Ordre de dessin des enfants : ceux du flux (ordre du document), puis les
/// `position: absolute` du plus petit au plus grand z-index. L'ordre inverse
/// sert aux clics (le plus haut d'abord).
pub(crate) fn paint_order(children: &[UiNode]) -> Vec<usize> {
    use crate::layout::managers::web_layout::{is_absolute, z_index};
    let mut order: Vec<usize> = (0..children.len()).filter(|&i| !is_absolute(&children[i])).collect();
    let mut positioned: Vec<usize> = (0..children.len()).filter(|&i| is_absolute(&children[i])).collect();
    positioned.sort_by_key(|&i| z_index(&children[i]));
    order.extend(positioned);
    order
}

// Pixels sous les 4 coins (carres de `radius` de cote) de `own_box`.
struct SavedCorners {
    // (x, y, largeur, hauteur, pixels BGRA)
    parts: Vec<(i32, i32, u32, u32, Vec<u8>)>,
}

fn save_corners(canvas: &Canvas, own_box: Rect, radius: f32) -> SavedCorners {
    let r = radius.min(own_box.2 as f32 / 2.0).min(own_box.3 as f32 / 2.0).ceil() as u32;
    let (x, y, w, h) = own_box;
    let spots = [(x, y), (x + w as i32 - r as i32, y), (x, y + h as i32 - r as i32), (x + w as i32 - r as i32, y + h as i32 - r as i32)];
    let mut parts = Vec::new();
    for (sx, sy) in spots {
        let mut pixels = Vec::with_capacity((r * r * 4) as usize);
        for row in 0..r as i32 {
            for col in 0..r as i32 {
                let (px, py) = (sx + col, sy + row);
                if px >= 0 && py >= 0 && (px as u32) < canvas.width && (py as u32) < canvas.height {
                    let i = ((py as u32 * canvas.width + px as u32) * 4) as usize;
                    pixels.extend_from_slice(&canvas.buffer[i..i + 4]);
                } else {
                    pixels.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
        parts.push((sx, sy, r, r, pixels));
    }
    SavedCorners { parts }
}

// Remet ce qui etait sous les coins, hors de la forme arrondie (avec
// anti-aliasing sur le bord de l'arrondi).
fn restore_corners(canvas: &mut Canvas, saved: &SavedCorners, own_box: Rect, radius: f32) {
    let (bx, by, bw, bh) = (own_box.0 as f32, own_box.1 as f32, own_box.2 as f32, own_box.3 as f32);
    let r = radius.min(bw / 2.0).min(bh / 2.0);
    let (clip_x, clip_y, clip_w, clip_h) = canvas.clip_bounds();
    for (sx, sy, w, h, pixels) in &saved.parts {
        for row in 0..*h as i32 {
            for col in 0..*w as i32 {
                let (px, py) = (sx + col, sy + row);
                if px < clip_x as i32 || py < clip_y as i32 || px >= (clip_x + clip_w) as i32 || py >= (clip_y + clip_h) as i32 {
                    continue;
                }
                // Distance au centre du quart de cercle de ce coin.
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let cx = if fx < bx + bw / 2.0 { bx + r } else { bx + bw - r };
                let cy = if fy < by + bh / 2.0 { by + r } else { by + bh - r };
                let in_corner = (fx < bx + r || fx > bx + bw - r) && (fy < by + r || fy > by + bh - r);
                if !in_corner {
                    continue;
                }
                let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt() - r;
                let outside = (d + 0.5).clamp(0.0, 1.0);
                if outside <= 0.0 {
                    continue;
                }
                let i = ((py as u32 * canvas.width + px as u32) * 4) as usize;
                let s = (((row as u32) * w + col as u32) * 4) as usize;
                for c in 0..3 {
                    let now = canvas.buffer[i + c] as f32;
                    let before = pixels[s + c] as f32;
                    canvas.buffer[i + c] = (now + (before - now) * outside).round() as u8;
                }
            }
        }
    }
}

const SCROLLBAR_WIDTH: u32 = 6;
pub(crate) const SCROLLBAR_MARGIN: u32 = 3;
const SCROLLBAR_MIN_THUMB: u32 = 24;
// Poignee par defaut (voir `scrollbar-color` en rsC).
const SCROLLBAR_COLOR: Color = Color::new(78, 74, 68, 255);

// Indicateur de position dans un conteneur qui defile : une barre fine
// contre son bord droit, dont la hauteur est la part du contenu visible.
fn draw_scrollbar(visible: Rect, content_height: u32, scroll: u32, color: Color, canvas: &mut Canvas) {
    let (vx, vy, vw, vh) = visible;
    if vw < SCROLLBAR_WIDTH + 2 * SCROLLBAR_MARGIN || vh < 2 * SCROLLBAR_MARGIN {
        return;
    }
    let track = vh - 2 * SCROLLBAR_MARGIN;
    let thumb = ((track as u64 * vh as u64 / content_height.max(1) as u64) as u32).clamp(SCROLLBAR_MIN_THUMB.min(track), track);
    let max_scroll = content_height.saturating_sub(vh).max(1);
    let offset = ((track - thumb) as u64 * scroll as u64 / max_scroll as u64) as u32;
    let bar = (vx + (vw - SCROLLBAR_WIDTH - SCROLLBAR_MARGIN) as i32, vy + (SCROLLBAR_MARGIN + offset) as i32, SCROLLBAR_WIDTH, thumb);
    let bar = intersect(bar, to_rect(canvas.clip_bounds()));
    draw_rect(bar.0 as u32, bar.1 as u32, bar.2, bar.3, &color, canvas);
}

// Barre de defilement horizontale, contre le bas du conteneur.
fn draw_scrollbar_x(visible: Rect, content_width: u32, scroll: u32, color: Color, canvas: &mut Canvas) {
    let (vx, vy, vw, vh) = visible;
    if vh < SCROLLBAR_WIDTH + 2 * SCROLLBAR_MARGIN || vw < 2 * SCROLLBAR_MARGIN {
        return;
    }
    let track = vw - 2 * SCROLLBAR_MARGIN;
    let thumb = ((track as u64 * vw as u64 / content_width.max(1) as u64) as u32).clamp(SCROLLBAR_MIN_THUMB.min(track), track);
    let max_scroll = content_width.saturating_sub(vw).max(1);
    let offset = ((track - thumb) as u64 * scroll as u64 / max_scroll as u64) as u32;
    let bar = (vx + (SCROLLBAR_MARGIN + offset) as i32, vy + (vh - SCROLLBAR_WIDTH - SCROLLBAR_MARGIN) as i32, thumb, SCROLLBAR_WIDTH);
    let bar = intersect(bar, to_rect(canvas.clip_bounds()));
    draw_rect(bar.0 as u32, bar.1 as u32, bar.2, bar.3, &color, canvas);
}

/// Geometrie de la barre de defilement verticale d'un conteneur (piste et
/// poignee), la meme que celle dessinee - pour la saisir a la souris.
pub(crate) fn scrollbar_thumb(track: Rect, content_height: u32, scroll: u32) -> Option<(Rect, u32)> {
    let (vx, vy, vw, vh) = track;
    if vw < SCROLLBAR_WIDTH + 2 * SCROLLBAR_MARGIN || vh < 2 * SCROLLBAR_MARGIN {
        return None;
    }
    let length = vh - 2 * SCROLLBAR_MARGIN;
    let thumb = ((length as u64 * vh as u64 / content_height.max(1) as u64) as u32).clamp(SCROLLBAR_MIN_THUMB.min(length), length);
    let max_scroll = content_height.saturating_sub(vh).max(1);
    let offset = ((length - thumb) as u64 * scroll as u64 / max_scroll as u64) as u32;
    // Zone saisissable un peu plus large que la barre dessinee.
    let grab = (vx + (vw - SCROLLBAR_WIDTH - SCROLLBAR_MARGIN * 2) as i32, vy + (SCROLLBAR_MARGIN + offset) as i32, SCROLLBAR_WIDTH + SCROLLBAR_MARGIN * 2, thumb);
    Some((grab, length - thumb))
}

// Le fond du bouton (survol, bascule de clic, degrade, coins arrondis...)
// est dessine avant, par `draw_opaque_node` : ici, seulement son texte.
fn draw_button(button: &Button, own_box: (u32, u32, u32, u32), canvas: &mut Canvas, mouse_x: i32, mouse_y: i32) {
    let (x, y, width, height) = own_box;
    let hovered = contains_point(to_rect(own_box), mouse_x, mouse_y);
    // `text_color` (resolue depuis rsC, voir `ui::models::button::Button`) :
    // blanc fixe si aucune regle ne la definit ; `:hover { color }` au survol.
    let base_color = button.text_color.unwrap_or(Color::new(255, 255, 255, 255));
    let hover_color = button.hover_text_color.unwrap_or(base_color);
    let text_color = match &button.decoration.transition {
        _ if button.state && button.active_text_color.is_some() => button.active_text_color.unwrap_or(base_color),
        // L'avancement a deja ete calcule pour le fond, dans cette image.
        Some(t) => crate::ui::models::transition::mix_color(base_color, hover_color, button.decoration.anim.step(t, hovered)),
        None if hovered => hover_color,
        None => base_color,
    };
    let size = button.font_size;
    let font = button.text_style.as_ref().map(|s| s.font).unwrap_or(FONT_PATH);
    let options = button.text_style.as_ref().map(|s| s.options).unwrap_or_default();
    let text_width = azure_engine::rendering::managers::renderer::measure_text_width_with(&button.text, font, size, button.font_weight, &options).unwrap_or(0.0);

    // Zone du texte : la boite moins son padding et sa bordure (modele web),
    // toute la boite sinon. Texte centre verticalement ; horizontalement
    // selon `text-align` (centre par defaut, comme un bouton HTML).
    let [top, right, bottom, left] = button.layout.css.as_ref().map(|c| c.frame.get()).unwrap_or([0.0; 4]);
    let (cx, cw) = (x as f32 + left, (width as f32 - left - right).max(0.0));
    let (cy, ch) = (y as f32 + top, (height as f32 - top - bottom).max(0.0));
    let align = button.text_style.as_ref().map(|s| s.align).unwrap_or(TextAlign::Center);
    let text_x = match align {
        TextAlign::Left => cx,
        TextAlign::Center => cx + ((cw - text_width) / 2.0).max(0.0),
        TextAlign::Right => cx + (cw - text_width).max(0.0),
    };
    let text_y = (cy + ch / 2.0 - size * 0.64).max(y as f32);
    let previous_clip = push_clip(canvas, (x, y, width, height));
    let _ = azure_engine::rendering::managers::renderer::draw_text_with(&button.text, font, text_x.round() as u32, text_y.round() as u32, size, button.font_weight, &text_color, &options, canvas);
    if let Some(style) = &button.text_style {
        draw_text_decoration(style.decoration, text_x, text_y, text_width, size, text_color, canvas);
    }
    pop_clip(canvas, previous_clip);
}

/// Traits de `text-decoration` pour une ligne de texte dessinee en
/// `(x, y)` (haut de la ligne, comme `draw_text`) sur `width` pixels.
pub(crate) fn draw_text_decoration(decoration: crate::style::models::web_style::TextDecoration, x: f32, y: f32, width: f32, size: f32, color: Color, canvas: &mut Canvas) {
    if !decoration.any() || width <= 0.0 {
        return;
    }
    let thickness = (size / 14.0).round().max(1.0) as u32;
    let mut line = |offset: f32| {
        let top = (y + offset).round();
        if top >= 0.0 && x >= 0.0 {
            let (cx, cy, cw, ch) = canvas.clip_bounds();
            let (x0, y0) = (x.round() as u32, top as u32);
            let (x1, y1) = ((x + width).round() as u32, top as u32 + thickness);
            let (x0, y0, x1, y1) = (x0.max(cx), y0.max(cy), x1.min(cx + cw), y1.min(cy + ch));
            if x0 < x1 && y0 < y1 {
                draw_box(x0 as i32, y0 as i32, x1 - x0, y1 - y0, &BoxStyle::solid(color), canvas);
            }
        }
    };
    // La ligne de base de `draw_text` est a `y + size`.
    if decoration.underline {
        line(size + size * 0.12);
    }
    if decoration.line_through {
        line(size * 0.66);
    }
    if decoration.overline {
        line(size * 0.18);
    }
}

// Remplissage d'un bouton a cet instant. Survol : le degrade `:hover`, sinon
// la couleur `:hover`, sinon l'etat de base (degrade, sinon couleur).
// Pendant l'appui (`state`) : `:active` s'il y en a un, sinon la couleur
// unie legerement enfoncee (voir `pressed`) ; un degrade reste tel quel.
fn button_fill(button: &Button, hovered: bool) -> Fill {
    let d = &button.decoration;
    // `button:active` ecrit par l'app.
    if button.state {
        if let Some(fill) = &d.active_fill {
            return fill.clone();
        }
        if let Some(color) = button.active_color {
            return Fill::Solid(color);
        }
    }
    if hovered
        && let Some(fill) = &d.hover_fill {
            return fill.clone();
        }
    let color = if hovered { button.hover_color.unwrap_or(button.color) } else { button.color };
    match (&d.fill, hovered && button.hover_color.is_some()) {
        (Some(fill), false) => fill.clone(),
        _ => Fill::Solid(if button.state { pressed(&color) } else { color }),
    }
}

// Meme regle pour une zone de saisie : `:focus` l'emporte sur `:hover`.
fn textarea_fill(textarea: &TextArea, hovered: bool) -> Fill {
    let d = &textarea.decoration;
    let (fill, color) = if textarea.focused {
        (d.focus_fill.as_ref(), textarea.focus_background)
    } else if hovered {
        (d.hover_fill.as_ref(), textarea.hover_background)
    } else {
        (None, None)
    };
    if let Some(fill) = fill {
        return fill.clone();
    }
    if let Some(color) = color {
        return Fill::Solid(color);
    }
    d.fill.clone().unwrap_or(Fill::Solid(textarea.background))
}

// Fond d'une zone de saisie hors survol et hors focus.
fn textarea_base_fill(textarea: &TextArea) -> Fill {
    textarea.decoration.fill.clone().unwrap_or(Fill::Solid(textarea.background))
}

fn contains_point(r: Rect, x: i32, y: i32) -> bool {
    crate::layout::managers::layout_manager::contains(r, x, y)
}

/// Un bouton enfonce : la meme teinte, un peu plus sombre (un peu plus
/// claire si elle est deja presque noire), jamais une couleur inversee.
pub(crate) fn pressed(color: &Color) -> Color {
    let dark = color.r.max(color.g).max(color.b) < 48;
    let shade = |c: u8| if dark { c + (255 - c) / 12 } else { (c as u16 * 82 / 100) as u8 };
    Color::new(shade(color.r), shade(color.g), shade(color.b), color.a)
}

// Zone de texte interactive : fond + selection (derriere le texte) + texte
// tape - avec retour a la ligne automatique quand une ligne atteint le
// bord de la boite plutot que de deborder hors de la zone (voir
// `ui::services::text_layout::wrap_lines`), et defilement (molette ou
// auto-scroll pour garder le curseur visible, voir
// `ui::services::interact::scroll_at`/`ensure_cursor_visible`) quand le
// texte a plus de lignes que la boite n'en affiche - un contour blanc
// quand elle a effectivement le focus (le survol, lui, se voit au curseur
// en barre de texte, voir `crate::cursor`), et - focalisee
// uniquement - un curseur clignotant a la position reelle du curseur
// d'edition, sur la bonne ligne affichee (voir
// `TextArea::cursor`/`ui::services::interact`). Le fond suit
// `hover_background`/`focus_background` quand une feuille rsC en definit
// (voir `compiler::services::codegen::resolve_pseudo_background`),
// :focus l'emportant sur :hover si les deux s'appliquent - mais le
// contour blanc de focus reste un indicateur generique toujours dessine,
// independant de toute feuille de style. Pas de vraie propriete CSS
// `outline`/`caret-color`/`::selection`/`overflow` (pas encore evaluees).
fn draw_textarea(textarea: &TextArea, own_box: (u32, u32, u32, u32), canvas: &mut Canvas, mouse_x: i32, mouse_y: i32, caret_visible: bool) {
    let (x, y, width, height) = own_box;
    // Le fond (survol/focus compris) est deja dessine par `draw_opaque_node`.
    let _ = (mouse_x, mouse_y);
    // Contour de focus des champs ; pas pour le texte riche (un document,
    // le curseur suffit).
    if textarea.focused && textarea.decoration.border.is_zero() && textarea.rich.is_none() {
        draw_empty_rect(x, y, width, height, &FOCUS_COLOR, canvas);
    }
    if textarea.rich.is_some() {
        return draw_rich_textarea(textarea, own_box, canvas, caret_visible);
    }
    let text_x = x + TEXTAREA_TEXT_PADDING;
    let text_y = y + TEXTAREA_TEXT_PADDING;
    let line_height = (textarea.font_size * 1.1) as u32;
    let content_width = width.saturating_sub(2 * TEXTAREA_TEXT_PADDING).max(1);
    let content_height = height.saturating_sub(2 * TEXTAREA_TEXT_PADDING).max(1);

    // Le texte affiche (masque pour un mot de passe) ; un champ d'une ligne
    // ne revient pas a la ligne et defile horizontalement (voir
    // `text_layout::single_line_start`, le meme calcul qu'au clic).
    let shown = textarea.display_text();
    let chars = shown.chars().count();
    let (lines, first) = if textarea.single_line {
        let start = text_layout::single_line_start(&shown, textarea.cursor, content_width, textarea.focused, textarea.font_size, textarea.font_weight);
        (vec![(start, chars)], start)
    } else {
        (text_layout::wrap_lines(&shown, FONT_PATH, textarea.font_size, textarea.font_weight, content_width as f32), 0)
    };
    let _ = first;
    let visible_lines = if textarea.single_line { 1 } else { (content_height / line_height.max(1)).max(1) as usize };
    // Ne redessine que la fenetre de lignes actuellement visible, jamais
    // au-dela de ce que le texte contient reellement (le texte peut avoir
    // raccourci depuis le dernier defilement).
    let scroll_offset = if textarea.single_line { 0 } else { textarea.scroll_offset.min(lines.len().saturating_sub(1)) };
    // Un champ d'une ligne centre son texte verticalement.
    let text_y = if textarea.single_line { y + height.saturating_sub(line_height) / 2 } else { text_y };

    let previous_clip = push_clip(canvas, (text_x, y + TEXTAREA_TEXT_PADDING.min(height / 4), content_width, height.saturating_sub(2 * TEXTAREA_TEXT_PADDING.min(height / 4)).max(1)));

    // Champ vide : son texte indicatif, grise.
    if textarea.text.is_empty() && !textarea.placeholder.is_empty() {
        let dim = Color::new(textarea.text_color.r, textarea.text_color.g, textarea.text_color.b, 110);
        let _ = draw_text(&textarea.placeholder, FONT_PATH, text_x, text_y, textarea.font_size, textarea.font_weight, &dim, canvas);
    }

    // Surlignage de la selection, dessine AVANT le texte pour rester
    // derriere son encre.
    if let Some((sel_start, sel_end)) = textarea.selection_range() {
        for (i, &(ls, le)) in lines.iter().enumerate().skip(scroll_offset).take(visible_lines) {
            let clamp_start = sel_start.max(ls);
            let clamp_end = sel_end.min(le);
            if clamp_start >= clamp_end {
                continue;
            }
            let line_text = text_layout::char_slice(&shown, ls, le);
            let start_w = char_position(line_text, FONT_PATH, textarea.font_size, textarea.font_weight, clamp_start - ls).unwrap_or(0.0);
            let end_w = char_position(line_text, FONT_PATH, textarea.font_size, textarea.font_weight, clamp_end - ls).unwrap_or(0.0);
            let sel_x = text_x + start_w.round() as u32;
            let sel_w = ((end_w - start_w).round() as u32).max(1);
            draw_rect(sel_x, text_y + (i - scroll_offset) as u32 * line_height, sel_w, line_height, &SELECTION_COLOR, canvas);
        }
    }

    for (i, &(ls, le)) in lines.iter().enumerate().skip(scroll_offset).take(visible_lines) {
        let line_text = text_layout::char_slice(&shown, ls, le);
        let _ = draw_text(line_text, FONT_PATH, text_x, text_y + (i - scroll_offset) as u32 * line_height, textarea.font_size, textarea.font_weight, &textarea.text_color, canvas);
    }

    if textarea.focused && caret_visible {
        let line_idx = text_layout::line_containing(&lines, textarea.cursor);
        if line_idx >= scroll_offset && line_idx < scroll_offset + visible_lines {
            let (ls, le) = lines[line_idx];
            let line_text = text_layout::char_slice(&shown, ls, le);
            let cursor_x = char_position(line_text, FONT_PATH, textarea.font_size, textarea.font_weight, textarea.cursor.saturating_sub(ls)).unwrap_or(0.0);
            let caret_x = text_x + cursor_x.round() as u32 + CARET_GAP;
            draw_rect(caret_x, text_y + (line_idx - scroll_offset) as u32 * line_height, 2, line_height, &textarea.text_color, canvas);
        }
    }

    pop_clip(canvas, previous_clip);
}

// Lien dans un texte riche : l'accent d'Azure, souligne.
const LINK_COLOR: Color = Color::new(201, 168, 120, 255);
// Fond d'un `code` en ligne.
const CODE_BACKGROUND: Color = Color::new(128, 128, 128, 56);
// Hauteur du fond d'un `code` en ligne, en fois la taille de sa police.
const RICH_CODE_BG: f32 = 1.35;

// Texte riche (voir `ui::models::rich`, `ui::services::rich_layout`) :
// chaque morceau dans sa police et sa couleur, code sur fond, souligne,
// barre ; selection et curseur aux positions mesurees par `rich_layout`
// (les memes qu'au clic).
fn draw_rich_textarea(textarea: &TextArea, own_box: (u32, u32, u32, u32), canvas: &mut Canvas, caret_visible: bool) {
    use crate::ui::services::rich_layout;
    use azure_engine::rendering::managers::renderer::draw_text_with;
    let Some(rich) = &textarea.rich else { return };
    let size = textarea.font_size;
    let (x, y, width, height) = own_box;
    let text_x = x + TEXTAREA_TEXT_PADDING;
    let text_y = y + TEXTAREA_TEXT_PADDING;
    let line_height = crate::ui::services::interact::line_height(textarea).max(1);
    let content_width = width.saturating_sub(2 * TEXTAREA_TEXT_PADDING).max(1);
    let content_height = height.saturating_sub(2 * TEXTAREA_TEXT_PADDING).max(1);
    let (positions, lines) = rich_layout::layout(&textarea.text, &rich.styles, size, textarea.font_weight, content_width as f32);
    let heights = rich_layout::line_heights(&textarea.text, &rich.styles, size, &lines);
    let chars: Vec<char> = textarea.text.chars().collect();
    let scroll_offset = textarea.scroll_offset.min(lines.len().saturating_sub(1));
    // Haut de chaque ligne depuis la premiere affichee.
    let mut tops = Vec::with_capacity(lines.len());
    let mut top = 0u32;
    for (row, h) in heights.iter().enumerate() {
        tops.push(top);
        if row >= scroll_offset {
            top += h;
        }
    }
    // Le texte centre dans sa ligne.
    let glyph_top = ((line_height as f32 - size * 1.1) / 2.0).max(0.0) as u32;
    let previous_clip = push_clip(canvas, (text_x, text_y, content_width, content_height));

    if textarea.text.is_empty() && !textarea.placeholder.is_empty() {
        let dim = Color::new(textarea.text_color.r, textarea.text_color.g, textarea.text_color.b, 110);
        let _ = draw_text(&textarea.placeholder, FONT_PATH, text_x, text_y + glyph_top, size, textarea.font_weight, &dim, canvas);
    }

    let default = crate::ui::models::rich::RichStyle::default();
    let style = |i: usize| rich.styles.get(i).unwrap_or(&default);
    for (row, &(ls, le)) in lines.iter().enumerate().skip(scroll_offset) {
        if tops[row] >= content_height {
            break;
        }
        let line_y = text_y + tops[row];
        let line_h = heights[row];
        let line_max = rich_layout::line_size(&chars, &rich.styles, size, ls, le);
        let base = positions[ls];
        let px = |i: usize| text_x + (positions[i] - base).max(0.0).round() as u32;

        if let Some((start, end)) = textarea.selection_range() {
            let (a, b) = (start.max(ls), end.min(le));
            if a < b {
                draw_rect(px(a), line_y, (px(b) - px(a)).max(1), line_h, &SELECTION_COLOR, canvas);
            }
        }

        let mut i = ls;
        while i < le {
            let st = style(i);
            let mut j = i + 1;
            while j < le && style(j) == st {
                j += 1;
            }
            let run: String = chars[i..j].iter().filter(|c| **c != '\n').collect();
            let w = px(j) - px(i);
            let run_size = rich_layout::size_of(st, size);
            let run_y = line_y + rich_layout::glyph_top(line_h, line_max, run_size) as u32;
            let color = st.color.unwrap_or(if st.link.is_empty() { textarea.text_color } else { LINK_COLOR });
            if st.code && w > 0 {
                let mut bg = BoxStyle::solid(CODE_BACKGROUND);
                bg.radius = 3.0;
                let bg_h = (run_size * RICH_CODE_BG).round() as u32;
                draw_box(px(i) as i32 - 2, (run_y + (run_size * 1.1) as u32 / 2).saturating_sub(bg_h / 2) as i32, w + 4, bg_h, &bg, canvas);
            }
            let (font, weight, options) = rich_layout::font_of(st, textarea.font_weight);
            let _ = draw_text_with(&run, font, px(i), run_y, run_size, weight, &color, &options, canvas);
            if (st.underline || !st.link.is_empty()) && w > 0 {
                draw_rect(px(i), run_y + run_size as u32 + 2, w, 1, &color, canvas);
            }
            if st.strike && w > 0 {
                draw_rect(px(i), run_y + (run_size * 0.62) as u32, w, 1, &color, canvas);
            }
            i = j;
        }
    }

    if textarea.focused && caret_visible {
        let line_idx = text_layout::line_containing(&lines, textarea.cursor);
        if line_idx >= scroll_offset && tops[line_idx] < content_height {
            let (ls, le) = lines[line_idx];
            let base = positions[ls];
            let cursor = textarea.cursor.min(positions.len() - 1);
            let caret_x = text_x + (positions[cursor] - base).max(0.0).round() as u32;
            // La taille de ce qui sera tape : celle du caractere d'avant.
            let typed = rich.pending.as_ref().or_else(|| (cursor > ls).then(|| style(cursor - 1))).map_or(size, |s| rich_layout::size_of(s, size));
            let line_max = rich_layout::line_size(&chars, &rich.styles, size, ls, le).max(typed);
            let caret_h = (typed * 1.25) as u32;
            let glyph = rich_layout::glyph_top(heights[line_idx], line_max, typed);
            let caret_y = (text_y + tops[line_idx]) as f32 + glyph + typed * 0.55 - caret_h as f32 / 2.0;
            draw_rect(caret_x, caret_y.max(0.0) as u32, 2, caret_h, &textarea.text_color, canvas);
        }
    }
    pop_clip(canvas, previous_clip);
}

// `Image` decode et affiche un vrai PNG depuis son attribut `src` (voir
// `azure_engine::codec::png`) ; `Video` n'a toujours aucun decodage (une
// video, meme sans lib, est d'un tout autre ordre de grandeur qu'une image
// fixe - voir `ui::models::video::Video`) et reste un rectangle
// placeholder distinct. Couleur de repli pour `Image` : aucun `src`, ou un
// `src` qui ne pointe vers aucun PNG valide (fichier absent, format hors
// du perimetre de `codec::png` - voir sa doc) - garde l'element visible et
// identifiable plutot que de le faire disparaitre silencieusement.
pub const IMAGE_PLACEHOLDER_COLOR: Color = Color::new(74, 70, 64, 255);

pub fn draw_image(image: &Image, own_box: (u32, u32, u32, u32), canvas: &mut Canvas) {
    let (x, y, width, height) = own_box;

    if image.src.is_empty() {
        draw_rect(x, y, width, height, &IMAGE_PLACEHOLDER_COLOR, canvas);
        return;
    }

    match load_image(&image.src) {
        Ok(decoded) => {
            // Le fond de la boite reste visible autour de l'image tant
            // qu'elle ne la remplit pas entierement (pas de mise a
            // l'echelle - voir `azure_engine::rendering::services::image`,
            // l'equivalent d'un CSS `object-fit: none`) : peindre le fond
            // par-dessus l'ancien placeholder eviterait un flash visible
            // au premier redessin, donc on ne le dessine plus du tout ici,
            // seule l'image (decoupee a la boite) est composee.
            let previous_clip = push_clip(canvas, (x, y, width, height));
            engine_draw_image(&decoded, x, y, canvas);
            pop_clip(canvas, previous_clip);
        }
        Err(_) => draw_rect(x, y, width, height, &IMAGE_PLACEHOLDER_COLOR, canvas),
    }
}

fn draw_video(_video: &Video, own_box: (u32, u32, u32, u32), canvas: &mut Canvas) {
    let (x, y, width, height) = own_box;
    draw_rect(x, y, width, height, &Color::new(119, 85, 85, 255), canvas);
}
