// Dessin de l'inspecteur : l'element survole/choisi entoure sur la page
// (marge, bordure, padding, contenu, aux couleurs de Chrome), puis le
// panneau (en-tete, arbre, details).
use super::enregistreur::Etape;
use super::{Action, Inspector, NodeInfo, Row, Zone, HEADER_H, ROW_H};
use crate::layout::managers::layout_manager::Rect;
use crate::layout::models::css_box::{CssBox, Length, Sides};
use crate::ui::models::ui_node::UiNode;
use crate::ui::services::draw_ui::FONT_PATH;
use crate::ui::services::interact::node_at_path;
use azure_engine::rendering::managers::renderer::{draw_box, draw_rect, draw_text, measure_text_width};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::BoxStyle;

const BG: Color = Color::new(22, 21, 20, 255);
const HEADER_BG: Color = Color::new(30, 29, 27, 255);
const LINE: Color = Color::new(52, 50, 46, 255);
const TEXT: Color = Color::new(231, 229, 225, 255);
const DIM: Color = Color::new(138, 135, 127, 255);
const TAG: Color = Color::new(201, 168, 120, 255);
const CLASS: Color = Color::new(160, 196, 140, 255);
const ID: Color = Color::new(222, 150, 150, 255);
const VALUE: Color = Color::new(214, 190, 150, 255);
const ROW_SELECTED: Color = Color::new(52, 46, 36, 255);
const ROW_HOVER: Color = Color::new(36, 34, 31, 255);
// Enregistrement en cours (terre cuite).
const REC: Color = Color::new(217, 135, 106, 255);

// Couleurs de Chrome pour la boite d'un element.
const MARGIN: Color = Color::new(246, 178, 107, 150);
const BORDER: Color = Color::new(255, 229, 153, 150);
const PADDING: Color = Color::new(147, 196, 125, 140);
const CONTENT: Color = Color::new(111, 168, 220, 150);

const FONT_SIZE: f32 = 12.0;

fn mono() -> &'static str {
    crate::ui::services::fonts::font_for(Some("monospace"))
}

/// L'arbre et les details, sous l'en-tete du panneau.
pub(crate) fn panel_parts(panel: Zone) -> (Zone, Zone) {
    let (x, y, w, h) = panel;
    let body = h.saturating_sub(HEADER_H);
    let tree_h = body * 45 / 100;
    ((x, y + HEADER_H, w, tree_h), (x, y + HEADER_H + tree_h + 1, w, body.saturating_sub(tree_h + 1)))
}

/// Les boutons « Choisir » et « Fermer » de l'en-tete.
/// Les boutons « Enregistrer », « Choisir » et « Fermer » de l'en-tete.
pub(crate) fn header_buttons(panel: Zone) -> (Zone, Zone, Zone) {
    let (x, y, w, _) = panel;
    let close = (x + w - 70, y + 6, 62, HEADER_H - 12);
    let pick = (close.0 - 82, y + 6, 76, HEADER_H - 12);
    let rec = (pick.0 - 96, y + 6, 90, HEADER_H - 12);
    (rec, pick, close)
}

/// Decalage d'une ligne de l'arbre a la profondeur `depth` (1 = racine).
pub(crate) fn indent(depth: usize) -> u32 {
    8 + (depth.saturating_sub(1) as u32) * 14
}

#[allow(clippy::too_many_arguments)] // texte, police, position, taille, graisse, couleur
fn text(canvas: &mut Canvas, s: &str, font: &str, x: i32, y: i32, size: f32, weight: f32, color: Color) -> i32 {
    if x < 0 || y < 0 {
        return x;
    }
    let _ = draw_text(s, font, x as u32, y as u32, size, weight, &color, canvas);
    // La mesure ne compte pas les espaces de fin : ajoutes a la main.
    let trailing = s.len() - s.trim_end_matches(' ').len();
    let space = measure_text_width("x x", font, size, weight).unwrap_or(0.0) - measure_text_width("xx", font, size, weight).unwrap_or(0.0);
    x + measure_text_width(s, font, size, weight).unwrap_or(0.0) as i32 + (trailing as f32 * space.max(3.0)) as i32
}

fn fill(canvas: &mut Canvas, r: (i32, i32, u32, u32), color: Color) {
    if r.2 > 0 && r.3 > 0 {
        draw_box(r.0, r.1, r.2, r.3, &BoxStyle::solid(color), canvas);
    }
}

// L'anneau entre `outer` et `inner` (contenu dans `outer`).
fn ring(canvas: &mut Canvas, outer: Rect, inner: Rect, color: Color) {
    let (ox, oy, ow, oh) = (outer.0, outer.1, outer.2 as i32, outer.3 as i32);
    let (ix, iy, iw, ih) = (inner.0, inner.1, inner.2 as i32, inner.3 as i32);
    let h = |v: i32| v.max(0) as u32;
    fill(canvas, (ox, oy, h(ow), h(iy - oy)), color);
    fill(canvas, (ox, iy + ih, h(ow), h(oy + oh - iy - ih)), color);
    fill(canvas, (ox, iy, h(ix - ox), h(ih)), color);
    fill(canvas, (ix + iw, iy, h(ox + ow - ix - iw), h(ih)), color);
}

fn px(l: Length) -> f32 {
    match l {
        Length::Px(v) => v,
        _ => 0.0,
    }
}

fn length_text(l: Length) -> String {
    match l {
        Length::Auto => "auto".into(),
        Length::Px(v) if v.fract() == 0.0 => format!("{}", v as i64),
        Length::Px(v) => format!("{v:.1}"),
        Length::Percent(v) => format!("{v}%"),
    }
}

// Retrecit `r` de `sides` (en px).
fn shrink(r: Rect, top: f32, right: f32, bottom: f32, left: f32) -> Rect {
    let w = (r.2 as f32 - left - right).max(0.0) as u32;
    let h = (r.3 as f32 - top - bottom).max(0.0) as u32;
    (r.0 + left as i32, r.1 + top as i32, w, h)
}

fn css_of(node: &UiNode) -> Option<&CssBox> {
    node.layout().css.as_deref()
}

/// Le bouton « Inspecter » de la barre de titre (sable quand le panneau est
/// ouvert).
pub fn draw_header_button(b: Zone, open: bool, hover: bool, recording: bool, canvas: &mut Canvas) {
    let background = if recording { REC } else if open { TAG } else if hover { Color::new(58, 55, 50, 255) } else { Color::new(44, 42, 38, 255) };
    let mut style = BoxStyle::solid(background);
    style.radius = 5.0;
    draw_box(b.0 as i32, b.1 as i32, b.2, b.3, &style, canvas);
    let label = if recording { "Enregistre…" } else { "Inspecter" };
    let open = open || recording;
    let tw = measure_text_width(label, FONT_PATH, 11.5, 500.0).unwrap_or(0.0) as u32;
    let color = if open { Color::new(26, 23, 18, 255) } else { TEXT };
    text(canvas, label, FONT_PATH, (b.0 + b.2.saturating_sub(tw) / 2) as i32, b.1 as i32 + 3, 11.5, 500.0, color);
}

/// Dessine l'inspecteur (s'il est ouvert) par-dessus la fenetre.
pub fn draw(inspector: &mut Inspector, nodes: &[UiNode], content: Zone, canvas: &mut Canvas) {
    if !inspector.open {
        return;
    }
    inspector.validate(nodes);
    let (page, panel) = super::split(content);
    let rows = inspector.shown_rows(nodes, page);
    let all = inspector.rows(nodes, page);

    canvas.set_clip(page.0, page.1, page.2, page.3);
    if let Some(selected) = &inspector.selected
        && inspector.hovered.as_ref() != Some(selected)
        && let Some(row) = all.iter().find(|r| &r.path == selected)
    {
        let b = row.own_box;
        ring(canvas, (b.0 - 1, b.1 - 1, b.2 + 2, b.3 + 2), b, TAG);
    }
    if let Some(target) = inspector.hovered.as_ref().or(inspector.selected.as_ref())
        && let Some(row) = all.iter().find(|r| &r.path == target)
        && let Some(node) = node_at_path(nodes, target)
    {
        highlight(canvas, node, row, page);
    }
    canvas.clear_clip();

    draw_panel(inspector, nodes, &rows, &all, page, panel, canvas);
}

// La boite d'un element sur la page, et son nom dans une etiquette.
fn highlight(canvas: &mut Canvas, node: &UiNode, row: &Row, page: Zone) {
    let b = row.own_box;
    let css = css_of(node);
    let (m, p) = css.map(|c| (c.margin, c.padding)).unwrap_or_default();
    let border = css.map(|c| c.border).unwrap_or_default();
    let margin_box = shrink(b, -px(m.top), -px(m.right), -px(m.bottom), -px(m.left));
    let padding_box = shrink(b, border.top, border.right, border.bottom, border.left);
    let content_box = shrink(padding_box, px(p.top), px(p.right), px(p.bottom), px(p.left));
    ring(canvas, margin_box, b, MARGIN);
    ring(canvas, b, padding_box, BORDER);
    ring(canvas, padding_box, content_box, PADDING);
    fill(canvas, content_box, CONTENT);

    // Etiquette : sous l'element, ou au-dessus s'il touche le bas.
    let name = node.decoration().inspect.as_ref().map(|i| i.selector()).unwrap_or_else(|| kind(node).to_string());
    let size = format!("{} × {}", b.2, b.3);
    let w = measure_text_width(&name, mono(), FONT_SIZE, 600.0).unwrap_or(0.0) as u32 + measure_text_width(&size, mono(), FONT_SIZE, 400.0).unwrap_or(0.0) as u32 + 26;
    let below = margin_box.1 + margin_box.3 as i32 + 4;
    let y = if below + 24 < (page.1 + page.3) as i32 { below } else { (margin_box.1 - 28).max(page.1 as i32) };
    let x = b.0.clamp(page.0 as i32, ((page.0 + page.2) as i32 - w as i32).max(page.0 as i32));
    let mut style = BoxStyle::solid(Color::new(32, 30, 28, 245));
    style.radius = 4.0;
    draw_box(x, y, w, 24, &style, canvas);
    let after = text(canvas, &name, mono(), x + 8, y + 5, FONT_SIZE, 600.0, TAG);
    text(canvas, &size, mono(), after + 10, y + 5, FONT_SIZE, 400.0, DIM);
}

fn kind(node: &UiNode) -> &'static str {
    match node {
        UiNode::Container(_) => "container",
        UiNode::Label(_) => "text",
        UiNode::Button(_) => "button",
        UiNode::Image(_) => "image",
        UiNode::Video(_) => "video",
        UiNode::TextArea(_) => "textarea",
        UiNode::Control(_) => "control",
    }
}

// Ce que montre un element a cote de sa balise dans l'arbre.
fn preview(node: &UiNode) -> String {
    let s = match node {
        UiNode::Label(l) => l.text.clone(),
        UiNode::Button(b) => b.text.clone(),
        UiNode::Image(i) => i.src.clone(),
        UiNode::TextArea(t) => t.text.clone(),
        _ => String::new(),
    };
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() > 40 { format!("{}…", s.chars().take(40).collect::<String>()) } else { s }
}

// `<tag.classe#id>` en couleurs ; retourne la fin.
fn selector_segments(canvas: &mut Canvas, info: Option<&NodeInfo>, fallback: &str, x: i32, y: i32, weight: f32) -> i32 {
    let font = mono();
    let Some(info) = info else {
        return text(canvas, fallback, font, x, y, FONT_SIZE, weight, TAG);
    };
    let mut x = text(canvas, &info.tag, font, x, y, FONT_SIZE, weight, TAG);
    for class in info.class.split_whitespace() {
        x = text(canvas, &format!(".{class}"), font, x, y, FONT_SIZE, weight, CLASS);
    }
    if !info.id.is_empty() {
        x = text(canvas, &format!("#{}", info.id), font, x, y, FONT_SIZE, weight, ID);
    }
    x
}

fn draw_panel(inspector: &mut Inspector, nodes: &[UiNode], rows: &[Row], all: &[Row], page: Zone, panel: Zone, canvas: &mut Canvas) {
    let (x, y, w, h) = panel;
    draw_rect(x, y, w, h, &BG, canvas);
    draw_rect(x, y, 1, h, &LINE, canvas);

    // En-tete.
    draw_rect(x, y, w, HEADER_H, &HEADER_BG, canvas);
    draw_rect(x, y + HEADER_H - 1, w, 1, &LINE, canvas);
    text(canvas, "Inspecteur", FONT_PATH, x as i32 + 12, y as i32 + 9, 13.0, 600.0, TEXT);
    let (rec, pick, close) = header_buttons(panel);
    let enregistre = inspector.enregistrement.is_some();
    for (b, label, on, couleur) in [(rec, if enregistre { "Arrêter" } else { "Enregistrer" }, enregistre, REC), (pick, "Choisir", inspector.picking, TAG), (close, "Fermer", false, TAG)] {
        let mut style = BoxStyle::solid(if on { couleur } else { Color::new(44, 42, 38, 255) });
        style.radius = 5.0;
        draw_box(b.0 as i32, b.1 as i32, b.2, b.3, &style, canvas);
        let tw = measure_text_width(label, FONT_PATH, 12.0, 500.0).unwrap_or(0.0) as u32;
        text(canvas, label, FONT_PATH, (b.0 + b.2.saturating_sub(tw) / 2) as i32, b.1 as i32 + 5, 12.0, 500.0, if on { Color::new(26, 23, 18, 255) } else { TEXT });
    }

    let (tree, details) = panel_parts(panel);
    draw_tree(inspector, nodes, rows, tree, canvas);
    draw_rect(x, details.1 - 1, w, 1, &LINE, canvas);
    draw_details(inspector, nodes, all, page, details, canvas);
}

fn draw_tree(inspector: &mut Inspector, nodes: &[UiNode], rows: &[Row], tree: Zone, canvas: &mut Canvas) {
    let (x, y, w, h) = tree;
    let total = rows.len() as f32 * ROW_H as f32;
    if std::mem::take(&mut inspector.reveal)
        && let Some(i) = inspector.selected.as_ref().and_then(|s| rows.iter().position(|r| &r.path == s))
    {
        let top = i as f32 * ROW_H as f32;
        if top < inspector.tree_scroll || top + ROW_H as f32 > inspector.tree_scroll + h as f32 {
            inspector.tree_scroll = top - h as f32 / 2.0;
        }
    }
    inspector.tree_scroll = inspector.tree_scroll.clamp(0.0, (total - h as f32).max(0.0));

    canvas.set_clip(x, y, w, h);
    let first = (inspector.tree_scroll / ROW_H as f32) as usize;
    for (i, row) in rows.iter().enumerate().skip(first) {
        let ry = y as i32 + (i as f32 * ROW_H as f32 - inspector.tree_scroll) as i32;
        if ry > (y + h) as i32 {
            break;
        }
        let Some(node) = node_at_path(nodes, &row.path) else { continue };
        if inspector.selected.as_ref() == Some(&row.path) {
            fill(canvas, (x as i32, ry, w, ROW_H), ROW_SELECTED);
        } else if inspector.hovered.as_ref() == Some(&row.path) {
            fill(canvas, (x as i32, ry, w, ROW_H), ROW_HOVER);
        }
        let mut tx = x as i32 + indent(row.path.len()) as i32;
        if row.has_children {
            let arrow = if inspector.collapsed.contains(&row.path) { "▸" } else { "▾" };
            text(canvas, arrow, mono(), tx, ry + 3, FONT_SIZE, 400.0, DIM);
        }
        tx += 14;
        let hidden = !node.decoration().visible;
        tx = text(canvas, "<", mono(), tx, ry + 3, FONT_SIZE, 400.0, DIM);
        tx = selector_segments(canvas, node.decoration().inspect.as_deref(), kind(node), tx, ry + 3, 400.0);
        tx = text(canvas, ">", mono(), tx, ry + 3, FONT_SIZE, 400.0, DIM);
        let mut note = preview(node);
        if hidden {
            note = format!("(cache) {note}");
        }
        if !note.is_empty() {
            text(canvas, &note, FONT_PATH, tx + 8, ry + 3, FONT_SIZE, 400.0, DIM);
        }
    }
    canvas.clear_clip();
    if rows.is_empty() {
        text(canvas, "Page vide", FONT_PATH, x as i32 + 12, y as i32 + 10, FONT_SIZE, 400.0, DIM);
    }
}

// Une ligne des details : des morceaux de texte colores.
struct Line {
    parts: Vec<(String, Color, bool)>,
    indent: u32,
    strike: bool,
    height: u32,
    box_model: bool,
    /// Un bouton (son libelle : le premier morceau).
    bouton: Option<Action>,
}

impl Line {
    fn new(parts: Vec<(String, Color, bool)>) -> Line {
        Line { parts, indent: 0, strike: false, height: ROW_H, box_model: false, bouton: None }
    }

    fn bouton(libelle: &str, action: Action) -> Line {
        Line { height: 30, bouton: Some(action), ..Line::new(vec![part(libelle, TEXT)]) }
    }
}

// Une etape du scenario telle que le test l'ecrira.
fn etape_texte(e: &Etape) -> String {
    match e {
        Etape::Clic(id) => format!("clic({id:?})"),
        Etape::ClicSansId(x, y) => format!("clic sans #id a ({x}, {y}) : non rejouable"),
        Etape::Remplir(id, v) => format!("remplir({id:?}, {v:?})"),
        Etape::Touche(t) => format!("touche({t:?})"),
        Etape::Molette(id, dy) => format!("molette({id:?}, {dy:.0})"),
        Etape::Verifier(t) => format!("attendre_texte({t:?})"),
    }
}

// La section « Scenario » des details : l'enregistrement en cours, ou le
// test genere au dernier arret.
fn scenario_lines(inspector: &Inspector, nodes: &[UiNode]) -> Vec<Line> {
    let mut l = Vec::new();
    if let Some(r) = &inspector.enregistrement {
        l.push(Line::new(vec![part("Enregistrement", REC), part(format!("  {} étape(s) · utilisez l'app, puis « Arrêter »", r.etapes.len()), DIM)]));
        let debut = r.etapes.len().saturating_sub(6);
        for e in &r.etapes[debut..] {
            l.push(Line { indent: 12, ..Line::new(vec![(etape_texte(e), if matches!(e, Etape::ClicSansId(..)) { DIM } else { TEXT }, true)]) });
        }
        let a_texte = inspector.selected.as_ref().and_then(|p| node_at_path(nodes, p)).and_then(crate::window::models::pilote::texte_de).is_some();
        if a_texte {
            l.push(Line::bouton("Vérifier ce texte", Action::Verifier));
        } else {
            l.push(Line::new(vec![part("« Choisir » un élément de la page pour vérifier son texte.", DIM)]));
        }
        l.push(Line { height: 10, ..Line::new(Vec::new()) });
    } else if let Some(code) = &inspector.code {
        l.push(Line::new(vec![part("Test généré", TEXT), part("  copié : collez-le dans un fichier tests/ de l'app", DIM)]));
        for ligne in code.lines() {
            l.push(Line { indent: 4, height: 17, ..Line::new(vec![(ligne.to_string(), VALUE, true)]) });
        }
        l.push(Line::bouton("Fermer le test", Action::FermerCode));
        l.push(Line { height: 10, ..Line::new(Vec::new()) });
    }
    l
}

fn part(s: impl Into<String>, color: Color) -> (String, Color, bool) {
    (s.into(), color, false)
}

const BOX_MODEL_H: u32 = 190;

fn detail_lines(node: &UiNode, row: &Row, page: Zone) -> Vec<Line> {
    let mut lines = Vec::new();
    let info = node.decoration().inspect.as_deref();
    let b = row.own_box;
    lines.push(Line::new(vec![
        part("Taille ", DIM),
        part(format!("{} × {}", b.2, b.3), TEXT),
        part("    Position ", DIM),
        part(format!("{}, {}", b.0 - page.0 as i32, b.1 - page.1 as i32), TEXT),
    ]));
    if css_of(node).is_some() {
        lines.push(Line { box_model: true, height: BOX_MODEL_H, ..Line::new(Vec::new()) });
    }
    if let Some(css) = css_of(node) {
        let mut parts = vec![part("display ", DIM), part(format!("{:?}", css.display).to_lowercase(), VALUE)];
        if format!("{:?}", css.display).contains("Flex") {
            parts.push(part("  direction ", DIM));
            parts.push(part(format!("{:?}", css.flex_direction).to_lowercase(), VALUE));
            parts.push(part("  justify ", DIM));
            parts.push(part(format!("{:?}", css.justify_content).to_lowercase(), VALUE));
            parts.push(part("  align ", DIM));
            parts.push(part(format!("{:?}", css.align_items).to_lowercase(), VALUE));
        }
        lines.push(Line::new(parts));
        if css.row_gap != Length::Px(0.0) || css.column_gap != Length::Px(0.0) {
            lines.push(Line::new(vec![part("gap ", DIM), part(format!("{} {}", length_text(css.row_gap), length_text(css.column_gap)), VALUE)]));
        }
        lines.push(Line::new(vec![
            part("width ", DIM),
            part(length_text(css.width), VALUE),
            part("  height ", DIM),
            part(length_text(css.height), VALUE),
            part("  flex-grow ", DIM),
            part(format!("{}", css.flex_grow), VALUE),
        ]));
    }
    let font = match node {
        UiNode::Label(l) => Some((l.font_size, l.weight, l.color)),
        UiNode::Button(bt) => Some((bt.font_size, bt.font_weight, bt.text_color.unwrap_or(TEXT))),
        UiNode::TextArea(t) => Some((t.font_size, t.font_weight, t.text_color)),
        _ => None,
    };
    if let Some((size, weight, color)) = font {
        lines.push(Line::new(vec![
            part("texte ", DIM),
            part(format!("{size}px"), VALUE),
            part(" · ", DIM),
            part(format!("{weight}"), VALUE),
            part(" · ", DIM),
            part(format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b), VALUE),
        ]));
    }

    lines.push(Line { height: 10, ..Line::new(Vec::new()) });
    lines.push(Line::new(vec![part("Styles", TEXT)]));
    match info {
        None => lines.push(Line::new(vec![part("Element construit sans .rsh : pas de styles a montrer.", DIM)])),
        Some(info) if info.rules.is_empty() => lines.push(Line::new(vec![part("Aucune regle rsC ne vise cet element.", DIM)])),
        Some(info) => {
            for rule in info.rules.iter() {
                let mut head = vec![(rule.selector.clone(), TAG, true)];
                if !rule.state.is_empty() {
                    head.push(part(format!("  (au {})", &rule.state[1..]), DIM));
                }
                head.push(part(" {", DIM));
                lines.push(Line::new(head));
                for (name, value, overridden) in &rule.declarations {
                    let color = if *overridden { DIM } else { TEXT };
                    lines.push(Line {
                        indent: 16,
                        strike: *overridden,
                        ..Line::new(vec![(name.clone(), if *overridden { DIM } else { CLASS }, true), part(": ", DIM), (value.clone(), color, true), part(";", DIM)])
                    });
                }
                lines.push(Line::new(vec![part("}", DIM)]));
            }
        }
    }
    lines
}

fn draw_details(inspector: &mut Inspector, nodes: &[UiNode], all: &[Row], page: Zone, details: Zone, canvas: &mut Canvas) {
    let (x, y, w, h) = details;
    inspector.boutons.clear();
    let mut lines = scenario_lines(inspector, nodes);
    let choisi = inspector.selected.clone().and_then(|p| Some((node_at_path(nodes, &p)?, all.iter().find(|r| r.path == p)?)));
    let mut body = (x, y + 8, w, h.saturating_sub(8));
    match choisi {
        Some((node, row)) => {
            if lines.is_empty() {
                // Titre fixe : le selecteur de l'element.
                selector_segments(canvas, node.decoration().inspect.as_deref(), kind(node), x as i32 + 12, y as i32 + 10, 600.0);
                body = (x, y + 32, w, h.saturating_sub(32));
            } else {
                let titre = node.decoration().inspect.as_ref().map(|i| i.selector()).unwrap_or_else(|| kind(node).to_string());
                lines.push(Line::new(vec![(titre, TAG, true)]));
            }
            lines.extend(detail_lines(node, row, page));
        }
        None if lines.is_empty() => {
            text(canvas, "Survolez la page puis cliquez sur un element,", FONT_PATH, x as i32 + 12, y as i32 + 12, FONT_SIZE, 400.0, DIM);
            text(canvas, "ou choisissez une ligne de l'arbre (fleches : naviguer).", FONT_PATH, x as i32 + 12, y as i32 + 30, FONT_SIZE, 400.0, DIM);
            return;
        }
        None => {}
    }
    let total: u32 = lines.iter().map(|l| l.height).sum();
    inspector.detail_scroll = inspector.detail_scroll.clamp(0.0, (total as f32 - body.3 as f32 + 12.0).max(0.0));

    canvas.set_clip(body.0, body.1, body.2, body.3);
    let mut ly = body.1 as i32 - inspector.detail_scroll as i32;
    for line in &lines {
        if ly + line.height as i32 >= body.1 as i32 && ly < (body.1 + body.3) as i32 {
            if line.box_model {
                if let Some((node, row)) = choisi
                    && let Some(css) = css_of(node)
                {
                    box_model(canvas, css, row.own_box, (x as i32 + 12, ly + 4, w - 24, BOX_MODEL_H - 12));
                }
            } else if let Some(action) = line.bouton {
                let libelle = line.parts.first().map(|p| p.0.as_str()).unwrap_or("");
                let lw = measure_text_width(libelle, FONT_PATH, 12.0, 500.0).unwrap_or(0.0) as u32 + 24;
                let b = (x + 12, (ly + 3).max(0) as u32, lw, 24);
                let mut style = BoxStyle::solid(Color::new(44, 42, 38, 255));
                style.radius = 5.0;
                draw_box(b.0 as i32, ly + 3, b.2, b.3, &style, canvas);
                text(canvas, libelle, FONT_PATH, b.0 as i32 + 12, ly + 8, 12.0, 500.0, TEXT);
                if ly + 3 >= body.1 as i32 {
                    inspector.boutons.push((b, action));
                }
            } else {
                let mut tx = x as i32 + 12 + line.indent as i32;
                let start = tx;
                for (s, color, monospace) in &line.parts {
                    let font = if *monospace { mono() } else { FONT_PATH };
                    tx = text(canvas, s, font, tx, ly + 3, FONT_SIZE, 400.0, *color);
                }
                if line.strike {
                    fill(canvas, (start, ly + 10, (tx - start).max(0) as u32, 1), DIM);
                }
            }
        }
        ly += line.height as i32;
    }
    canvas.clear_clip();
}

// Le schema de la boite (comme Chrome) : marge, bordure, padding, contenu.
fn box_model(canvas: &mut Canvas, css: &CssBox, own: Rect, area: (i32, i32, u32, u32)) {
    let layers: [(&str, Color, Sides); 3] = [
        ("marge", MARGIN, css.margin),
        ("bordure", BORDER, Sides { top: Length::Px(css.border.top), right: Length::Px(css.border.right), bottom: Length::Px(css.border.bottom), left: Length::Px(css.border.left) }),
        ("padding", PADDING, css.padding),
    ];
    let mut r = area;
    let step_x = (area.2 / 9) as i32;
    let step_y = 26;
    let small = 11.0;
    for (name, color, sides) in layers {
        fill(canvas, (r.0, r.1, r.2, r.3), Color::new(color.r, color.g, color.b, 70));
        draw_box(r.0, r.1, r.2, r.3, &{ let mut s = BoxStyle::solid(Color::new(0, 0, 0, 0)); s.border = azure_engine::rendering::models::paint::BorderWidths::uniform(1.0); s.border_color = color; s }, canvas);
        text(canvas, name, FONT_PATH, r.0 + 4, r.1 + 2, small, 400.0, DIM);
        let mid_x = r.0 + r.2 as i32 / 2;
        let mid_y = r.1 + r.3 as i32 / 2;
        let center = |canvas: &mut Canvas, s: &str, cx: i32, cy: i32| {
            let tw = measure_text_width(s, mono(), small, 400.0).unwrap_or(0.0) as i32;
            text(canvas, s, mono(), cx - tw / 2, cy - 7, small, 400.0, TEXT);
        };
        center(canvas, &length_text(sides.top), mid_x, r.1 + step_y / 2 + 3);
        center(canvas, &length_text(sides.bottom), mid_x, r.1 + r.3 as i32 - step_y / 2 + 3);
        center(canvas, &length_text(sides.left), r.0 + step_x / 2, mid_y);
        center(canvas, &length_text(sides.right), r.0 + r.2 as i32 - step_x / 2, mid_y);
        r = (r.0 + step_x, r.1 + step_y, (r.2 as i32 - 2 * step_x).max(0) as u32, (r.3 as i32 - 2 * step_y).max(0) as u32);
    }
    // Contenu : la taille reelle (boite moins bordure et padding).
    fill(canvas, (r.0, r.1, r.2, r.3), Color::new(CONTENT.r, CONTENT.g, CONTENT.b, 90));
    let cw = own.2 as f32 - css.border.left - css.border.right - px(css.padding.left) - px(css.padding.right);
    let ch = own.3 as f32 - css.border.top - css.border.bottom - px(css.padding.top) - px(css.padding.bottom);
    let s = format!("{} × {}", cw.max(0.0).round(), ch.max(0.0).round());
    let tw = measure_text_width(&s, mono(), small, 400.0).unwrap_or(0.0) as i32;
    text(canvas, &s, mono(), r.0 + r.2 as i32 / 2 - tw / 2, r.1 + r.3 as i32 / 2 - 7, small, 400.0, TEXT);
}
