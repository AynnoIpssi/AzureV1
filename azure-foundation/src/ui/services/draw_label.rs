use crate::ui::models::label::Label;
use crate::ui::models::text_style::TextStyle;
use crate::style::models::web_style::TextAlign;
use crate::ui::services::text_block;
use crate::ui::services::text_layout::char_slice;
use crate::ui::services::draw_ui::{pop_clip, push_clip};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::managers::renderer::{char_positions_with, draw_rect, draw_text, draw_text_with, TextOptions};
use azure_engine::rendering::models::color::Color;

// Ancre au dossier de CE crate (resolu a la COMPILATION, voir `CARGO_MANIFEST_DIR`)
// plutot qu'au dossier depuis lequel le binaire final est lance : un chemin
// relatif litteral ("../azure-engine/...") ne marche que si ce binaire est
// lance depuis `azure-foundation/` elle-meme, ce qui n'est vrai ni pour
// `app-a`/`app-b` (autres membres du workspace) ni pour `cargo run -p ...`
// invoque depuis la racine du workspace - voir `ui::services::draw_ui::FONT_PATH`
// et `window::services::draw_header::FONT_PATH`, qui avaient le meme souci.
const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../azure-engine/src/Sora-VariableFont_wght.ttf");

/// `own_box` est la boite DEJA resolue de ce `Label` (voir
/// `ui::services::draw_ui::draw_node`), pas une boite de parent a resoudre
/// nous-memes. `width`/`height` servent desormais a clipper le texte a sa
/// propre boite (un `Label` enfant d'un conteneur `Flex`/`Grid` peut se voir
/// attribuer une largeur/hauteur bien plus etroite que son texte) plutot que
/// d'etre simplement ignores comme avant : un texte trop long pour sa boite
/// est tronque visuellement, il ne deborde plus sur ses voisins.
pub fn draw_label(label: &Label, own_box: (u32, u32, u32, u32), canvas: &mut Canvas) {
    let (x, y, width, height) = own_box;
    let previous_clip = push_clip(canvas, (x, y, width, height));
    if label.selection.is_some() {
        draw_selection(label, own_box, canvas);
    }
    match &label.text_style {
        Some(style) => draw_paragraph(label, style, own_box, canvas),
        None => {
            draw_text(&label.text, FONT_PATH, x, y, label.font_size, label.weight, &label.color, canvas).expect("Failed to draw label");
        }
    }
    pop_clip(canvas, previous_clip);
}

/// Couleur du texte selectionne : un ton chaud et sourd, lisible sous un
/// texte clair comme sous un texte fonce (le canvas ne melange pas les
/// couleurs : le fond est plein).
pub const TEXT_SELECTION_COLOR: Color = Color::new(84, 71, 52, 255);

/// Une ligne de texte telle qu'elle est dessinee : caracteres
/// `[start, end)`, position de son premier caractere (`x`), haut et hauteur
/// de la ligne.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextLine {
    pub start: usize,
    pub end: usize,
    pub x: f32,
    pub top: f32,
    pub height: f32,
}

/// Les lignes de `label` dans `own_box`, exactement comme `draw_label` les
/// dessine : c'est ce que la selection a la souris utilise pour savoir
/// quel caractere est sous le pointeur.
pub fn label_lines(label: &Label, own_box: (i32, i32, u32, u32)) -> Vec<TextLine> {
    let (x, y, width, _) = own_box;
    let Some(style) = &label.text_style else {
        return vec![TextLine { start: 0, end: label.text.chars().count(), x: x as f32, top: y as f32, height: label.font_size * 1.2 }];
    };
    let [top, right, _, left] = label.layout.css.as_ref().map(|c| c.frame.get()).unwrap_or([0.0; 4]);
    let content_x = x as f32 + left;
    let content_w = (width as f32 - left - right).max(1.0);
    let content_y = y as f32 + top;
    text_block::lines(&label.text, style, label.font_size, label.weight, Some(content_w))
        .into_iter()
        .enumerate()
        .map(|(i, (start, end, line_width))| TextLine {
            start,
            end,
            x: match style.align {
                TextAlign::Left => content_x,
                TextAlign::Center => content_x + ((content_w - line_width) / 2.0).max(0.0),
                TextAlign::Right => content_x + (content_w - line_width).max(0.0),
            },
            top: content_y + i as f32 * style.line_height,
            height: style.line_height,
        })
        .collect()
}

/// Position (relative a `line.x`) de l'avant de chaque caractere de la
/// ligne, plus celle de la fin.
pub fn line_positions(label: &Label, line: &TextLine) -> Vec<f32> {
    let text = char_slice(&label.text, line.start, line.end);
    let (font, options) = match &label.text_style {
        Some(style) => (style.font, style.options),
        None => (FONT_PATH, TextOptions::default()),
    };
    char_positions_with(text, font, label.font_size, label.weight, &options).unwrap_or_else(|_| vec![0.0; text.chars().count() + 1])
}

// Surlignage de `label.selection`, derriere le texte.
fn draw_selection(label: &Label, own_box: (u32, u32, u32, u32), canvas: &mut Canvas) {
    let Some((sel_start, sel_end)) = label.selection else { return };
    for line in label_lines(label, (own_box.0 as i32, own_box.1 as i32, own_box.2, own_box.3)) {
        let (from, to) = (sel_start.max(line.start), sel_end.min(line.end));
        // Une ligne entierement selectionnee jusqu'a sa coupure : un petit
        // bout de plus montre que la selection continue a la ligne suivante.
        let continues = sel_end > line.end && line.end > line.start;
        if from >= to && !continues {
            continue;
        }
        let positions = line_positions(label, &line);
        let at = |i: usize| positions.get(i - line.start).or(positions.last()).copied().unwrap_or(0.0);
        let x0 = line.x + at(from.min(line.end));
        let x1 = line.x + at(to.max(from).min(line.end)) + if continues { label.font_size * 0.3 } else { 0.0 };
        if x1 > x0 {
            draw_rect(x0.round().max(0.0) as u32, line.top.round().max(0.0) as u32, (x1 - x0).round().max(1.0) as u32, line.height.round() as u32, &TEXT_SELECTION_COLOR, canvas);
        }
    }
}

// Texte web : les memes lignes que celles mesurees par la mise en page (voir
// `text_block::lines`), une par `line-height`, alignees selon `text-align`
// dans la boite de contenu (sans padding ni bordure). Le texte est centre
// verticalement dans sa ligne, comme un navigateur.
fn draw_paragraph(label: &Label, style: &TextStyle, own_box: (u32, u32, u32, u32), canvas: &mut Canvas) {
    let (x, y, width, height) = own_box;
    let [top, right, bottom, left] = label.layout.css.as_ref().map(|c| c.frame.get()).unwrap_or([0.0; 4]);
    let content_x = x as f32 + left;
    let content_w = (width as f32 - left - right).max(1.0);
    let content_y = y as f32 + top;
    let content_bottom = y as f32 + height as f32 - bottom;
    let lines = text_block::lines(&label.text, style, label.font_size, label.weight, Some(content_w));
    let size = label.font_size;
    // Ligne de base de `draw_text` a `y + size` : on place le corps du texte
    // (~1.2 x la taille) au milieu de la ligne.
    let offset = (style.line_height - size * 1.2) / 2.0 - size * 0.05;
    let (clip_y, clip_h) = (canvas.clip_bounds().1 as f32, canvas.clip_bounds().3 as f32);
    for (i, &(start, end, line_width)) in lines.iter().enumerate() {
        let line_top = content_y + i as f32 * style.line_height;
        if line_top >= content_bottom || line_top >= clip_y + clip_h {
            break;
        }
        if line_top + style.line_height < clip_y {
            continue;
        }
        let line_x = match style.align {
            TextAlign::Left => content_x,
            TextAlign::Center => content_x + ((content_w - line_width) / 2.0).max(0.0),
            TextAlign::Right => content_x + (content_w - line_width).max(0.0),
        };
        let text = char_slice(&label.text, start, end).trim_end_matches('\n');
        let text_y = (line_top + offset).max(0.0);
        let _ = draw_text_with(text, style.font, line_x.round() as u32, text_y.round() as u32, size, label.weight, &label.color, &style.options, canvas);
        crate::ui::services::draw_ui::draw_text_decoration(style.decoration, line_x, text_y, line_width, size, label.color, canvas);
    }
}
