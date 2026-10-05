// Dessin et geometrie des champs `Control` (case a cocher, radio,
// interrupteur, curseur, progression, liste deroulante, choix segmente,
// note). Les positions (segments, lignes de la liste ouverte, pastilles) sont
// calculees ICI et reutilisees par les clics (`ui::services::interact`) :
// ce qu'on voit est exactement ce qu'on touche.
use crate::layout::managers::layout_manager::Rect;
use crate::ui::models::control::{Control, ControlKind};
use crate::ui::services::draw_ui::FONT_PATH;
use azure_engine::rendering::managers::renderer::{draw_box, draw_text, measure_text_width};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::{BorderWidths, BoxStyle};
use azure_engine::rendering::services::shapes::line::draw_line;

const BOX: f32 = 18.0;
const GAP: f32 = 8.0;
const SWITCH_W: f32 = 36.0;
const SWITCH_H: f32 = 20.0;
const KNOB: f32 = 16.0;
const SELECT_H: f32 = 34.0;
const OPTION_H: f32 = 30.0;
const SEGMENT_PAD: f32 = 14.0;
const DOT: f32 = 18.0;
const DOT_GAP: f32 = 6.0;

const WHITE: Color = Color::new(255, 255, 255, 255);

fn text_width(text: &str, size: f32) -> f32 {
    measure_text_width(text, FONT_PATH, size, 500.0).unwrap_or(0.0).ceil()
}

fn line_height(control: &Control) -> f32 {
    (control.font_size * 1.35).ceil()
}

/// Largeur du contenu quand rien ne l'impose en rsC.
pub fn natural_width(control: &Control) -> f32 {
    let label = if control.label.is_empty() { 0.0 } else { GAP + text_width(&control.label, control.font_size) };
    match control.kind {
        ControlKind::Checkbox | ControlKind::Radio => BOX + label,
        ControlKind::Switch => SWITCH_W + label,
        ControlKind::Slider => 220.0,
        ControlKind::Progress => 220.0,
        ControlKind::Select => control.options.iter().map(|(_, l)| text_width(l, control.font_size)).fold(120.0, f32::max) + 44.0,
        ControlKind::Segmented => control.options.iter().map(|(_, l)| text_width(l, control.font_size) + 2.0 * SEGMENT_PAD).sum::<f32>() + 4.0,
        ControlKind::Rating => rating_count(control) as f32 * (DOT + DOT_GAP) - DOT_GAP + label,
        ControlKind::Toile => 480.0,
    }
}

pub fn natural_height(control: &Control) -> f32 {
    match control.kind {
        ControlKind::Checkbox | ControlKind::Radio | ControlKind::Rating => BOX.max(line_height(control)),
        ControlKind::Switch => SWITCH_H.max(line_height(control)),
        ControlKind::Slider => 22.0,
        ControlKind::Progress => 8.0,
        ControlKind::Select => SELECT_H,
        ControlKind::Segmented => 32.0,
        ControlKind::Toile => 320.0,
    }
}

fn rating_count(control: &Control) -> usize {
    (control.max.round() as usize).clamp(1, 10)
}

#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
fn fill_box(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color, border: Option<Color>, canvas: &mut Canvas) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let mut style = BoxStyle::solid(color);
    style.radius = radius;
    if let Some(border) = border {
        style.border = BorderWidths::uniform(1.0);
        style.border_color = border;
    }
    draw_box(x.round() as i32, y.round() as i32, w.round() as u32, h.round() as u32, &style, canvas);
}

fn dot(cx: f32, cy: f32, r: f32, color: Color, canvas: &mut Canvas) {
    fill_box(cx - r, cy - r, 2.0 * r, 2.0 * r, r, color, None, canvas);
}

fn label_at(control: &Control, x: f32, own_box: Rect, canvas: &mut Canvas) {
    if control.label.is_empty() {
        return;
    }
    let lh = line_height(control);
    let y = own_box.1 as f32 + (own_box.3 as f32 - lh) / 2.0 + (lh - control.font_size * 1.2) / 2.0;
    if x >= 0.0 && y >= 0.0 {
        let _ = draw_text(&control.label, FONT_PATH, x as u32, y as u32, control.font_size, 450.0, &control.text_color, canvas);
    }
}

fn thick_line(x1: f32, y1: f32, x2: f32, y2: f32, color: Color, canvas: &mut Canvas) {
    for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
        draw_line(x1 as i32 + dx, y1 as i32 + dy, x2 as i32 + dx, y2 as i32 + dy, &color, canvas);
    }
}

fn dim(color: Color, alpha: u8) -> Color {
    Color::new(color.r, color.g, color.b, alpha)
}

/// Les segments d'un choix segmente : (x, largeur), relatifs a la boite.
pub fn segments(control: &Control, width: u32) -> Vec<(f32, f32)> {
    let natural: Vec<f32> = control.options.iter().map(|(_, l)| text_width(l, control.font_size) + 2.0 * SEGMENT_PAD).collect();
    let total: f32 = natural.iter().sum::<f32>().max(1.0);
    // La place en plus (largeur imposee en rsC) est repartie.
    let scale = ((width as f32 - 4.0) / total).max(1.0);
    let mut x = 2.0;
    natural
        .iter()
        .map(|w| {
            let seg = (x, w * scale);
            x += w * scale;
            seg
        })
        .collect()
}

/// Les lignes de la liste deroulante ouverte (sous la boite), en
/// coordonnees fenetre.
pub fn option_rows(control: &Control, own_box: Rect) -> Vec<Rect> {
    let top = own_box.1 + own_box.3 as i32 + 4;
    (0..control.options.len()).map(|i| (own_box.0, top + (i as f32 * OPTION_H) as i32, own_box.2.max(160), OPTION_H as u32)).collect()
}

/// Centre x de la pastille `i` d'une note.
pub fn rating_dot_x(own_box: Rect, i: usize) -> f32 {
    own_box.0 as f32 + DOT / 2.0 + i as f32 * (DOT + DOT_GAP)
}

/// Valeur du curseur pour un clic a `x`.
pub fn slider_value_at(control: &Control, own_box: Rect, x: i32) -> f64 {
    let (start, width) = (own_box.0 as f32 + KNOB / 2.0, (own_box.2 as f32 - KNOB).max(1.0));
    let fraction = ((x as f32 - start) / width).clamp(0.0, 1.0) as f64;
    control.min + fraction * (control.max - control.min)
}

pub fn draw_control(control: &Control, own_box: Rect, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32) {
    let (x, y, w, h) = (own_box.0 as f32, own_box.1 as f32, own_box.2 as f32, own_box.3 as f32);
    let cy = y + h / 2.0;
    let hovered = mouse_x >= own_box.0 && mouse_y >= own_box.1 && mouse_x < own_box.0 + own_box.2 as i32 && mouse_y < own_box.1 + own_box.3 as i32;
    let accent = if control.disabled { dim(control.accent, 110) } else { control.accent };
    if let (ControlKind::Toile, Some(toile)) = (control.kind, control.toile.as_deref()) {
        crate::ui::services::draw_toile::draw_toile(toile, own_box, canvas);
        return;
    }
    match control.kind {
        // Dessinee plus haut (sans dessin : rien).
        ControlKind::Toile => {}
        ControlKind::Checkbox => {
            let (bx, by) = (x, cy - BOX / 2.0);
            if control.checked {
                fill_box(bx, by, BOX, BOX, 4.0, accent, None, canvas);
                thick_line(bx + 4.0, by + 9.0, bx + 7.5, by + 12.5, WHITE, canvas);
                thick_line(bx + 7.5, by + 12.5, bx + 13.5, by + 5.5, WHITE, canvas);
            } else {
                let border = if hovered { accent } else { dim(control.text_color, 90) };
                fill_box(bx, by, BOX, BOX, 4.0, control.track, Some(border), canvas);
            }
            label_at(control, x + BOX + GAP, own_box, canvas);
        }
        ControlKind::Radio => {
            let (rx, r) = (x + BOX / 2.0, BOX / 2.0);
            if control.checked {
                dot(rx, cy, r, accent, canvas);
                dot(rx, cy, 4.0, WHITE, canvas);
            } else {
                let border = if hovered { accent } else { dim(control.text_color, 90) };
                fill_box(rx - r, cy - r, BOX, BOX, r, control.track, Some(border), canvas);
            }
            label_at(control, x + BOX + GAP, own_box, canvas);
        }
        ControlKind::Switch => {
            let (sx, sy) = (x, cy - SWITCH_H / 2.0);
            fill_box(sx, sy, SWITCH_W, SWITCH_H, SWITCH_H / 2.0, if control.checked { accent } else { control.track }, None, canvas);
            let knob_x = if control.checked { sx + SWITCH_W - 2.0 - KNOB / 2.0 } else { sx + 2.0 + KNOB / 2.0 };
            dot(knob_x, cy, KNOB / 2.0, WHITE, canvas);
            label_at(control, x + SWITCH_W + GAP, own_box, canvas);
        }
        ControlKind::Slider => {
            let (start, width) = (x + KNOB / 2.0, (w - KNOB).max(1.0));
            fill_box(start, cy - 2.0, width, 4.0, 2.0, control.track, None, canvas);
            let filled = width * control.fraction() as f32;
            fill_box(start, cy - 2.0, filled, 4.0, 2.0, accent, None, canvas);
            let knob = if control.dragging || hovered { KNOB / 2.0 + 1.0 } else { KNOB / 2.0 };
            dot(start + filled, cy, knob, accent, canvas);
            dot(start + filled, cy, knob - 3.0, WHITE, canvas);
        }
        ControlKind::Progress => {
            fill_box(x, y, w, h, h / 2.0, control.track, None, canvas);
            fill_box(x, y, w * control.fraction() as f32, h, h / 2.0, accent, None, canvas);
        }
        ControlKind::Select => {
            // Fond transparent en rsC : liste « a plat », comme du texte ; le
            // fond et le chevron n'apparaissent qu'au survol.
            let flat = control.track.a == 0;
            if flat {
                if control.open || hovered {
                    fill_box(x, y, w, h, 5.0, Color::new(255, 255, 255, 14), None, canvas);
                }
            } else {
                let border = if control.open || hovered { accent } else { dim(control.text_color, 60) };
                fill_box(x, y, w, h, 6.0, control.track, Some(border), canvas);
            }
            let ty = y + (h - control.font_size * 1.2) / 2.0;
            let _ = draw_text(control.selected_label(), FONT_PATH, (x + if flat { 6.0 } else { 12.0 }) as u32, ty.max(0.0) as u32, control.font_size, 450.0, &control.text_color, canvas);
            // Petit chevron (sans glyphe : toutes les polices ne l'ont pas).
            if !flat || control.open || hovered {
                let (ax, ay) = (x + w - 20.0, cy - 2.0);
                thick_line(ax, ay, ax + 4.0, ay + 4.0, control.text_color, canvas);
                thick_line(ax + 4.0, ay + 4.0, ax + 8.0, ay, control.text_color, canvas);
            }
        }
        ControlKind::Segmented => {
            fill_box(x, y, w, h, 8.0, control.track, None, canvas);
            for (i, ((sx, sw), (_, label))) in segments(control, own_box.2).into_iter().zip(&control.options).enumerate() {
                let selected = i == control.selected;
                if selected {
                    fill_box(x + sx, y + 2.0, sw, h - 4.0, 6.0, accent, None, canvas);
                }
                let tw = text_width(label, control.font_size);
                let tx = x + sx + (sw - tw) / 2.0;
                let ty = y + (h - control.font_size * 1.2) / 2.0;
                let color = if selected { WHITE } else { control.text_color };
                let _ = draw_text(label, FONT_PATH, tx.max(0.0) as u32, ty.max(0.0) as u32, control.font_size, if selected { 600.0 } else { 450.0 }, &color, canvas);
            }
        }
        ControlKind::Rating => {
            for i in 0..rating_count(control) {
                let on = (i as f64) < control.value.round();
                dot(rating_dot_x(own_box, i), cy, DOT / 2.0, if on { accent } else { control.track }, canvas);
            }
            label_at(control, x + rating_count(control) as f32 * (DOT + DOT_GAP), own_box, canvas);
        }
    }
}

/// La liste d'une liste deroulante ouverte, dessinee par-dessus tout le
/// reste (voir `draw_ui`).
pub fn draw_open_select(control: &Control, own_box: Rect, canvas: &mut Canvas, mouse_x: i32, mouse_y: i32) {
    let rows = option_rows(control, own_box);
    let Some(first) = rows.first() else { return };
    let height = rows.len() as u32 * OPTION_H as u32 + 8;
    let mut style = BoxStyle::solid(Color::new(28, 27, 25, 255));
    style.radius = 8.0;
    style.border = BorderWidths::uniform(1.0);
    style.border_color = dim(control.text_color, 50);
    draw_box(first.0, first.1 - 4, first.2, height, &style, canvas);
    for (i, (row, (_, label))) in rows.iter().zip(&control.options).enumerate() {
        let hovered = mouse_x >= row.0 && mouse_y >= row.1 && mouse_x < row.0 + row.2 as i32 && mouse_y < row.1 + row.3 as i32;
        if i == control.selected || hovered {
            let color = if i == control.selected { dim(control.accent, 70) } else { Color::new(255, 255, 255, 18) };
            fill_box(row.0 as f32 + 4.0, row.1 as f32, row.2 as f32 - 8.0, row.3 as f32, 5.0, color, None, canvas);
        }
        let ty = row.1 as f32 + (OPTION_H - control.font_size * 1.2) / 2.0;
        if row.0 >= 0 && ty >= 0.0 {
            let _ = draw_text(label, FONT_PATH, (row.0 + 12) as u32, ty as u32, control.font_size, 450.0, &control.text_color, canvas);
        }
    }
}
