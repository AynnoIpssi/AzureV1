use crate::rendering::models::canvas::Canvas;
use crate::rendering::models::color::Color;
use crate::rendering::models::glyph::Glyph;
use crate::rendering::services::shapes::rect;
use crate::rendering::services::text::loader::load_font;
use crate::rendering::services::text::glyph::extract_glyph_for_font;
use crate::rendering::services::text::renderer::{draw_glyph, draw_glyph_slanted};
use crate::rendering::services::text::kerning::get_advance;
use crate::rendering::services::image;
use crate::codec::png::DecodedImage;
use std::rc::Rc;

/// Boite decoree (degrade, transparence, coins arrondis, bordure, ombre) -
/// voir `services::shapes::styled_box`.
pub fn draw_box(x: i32, y: i32, width: u32, height: u32, style: &crate::rendering::models::paint::BoxStyle, canvas: &mut Canvas) {
    crate::rendering::services::shapes::styled_box::draw_box(x, y, width, height, style, canvas);
}

pub fn draw_rect(x: u32, y: u32, width: u32, height: u32, color: &Color, canvas: &mut Canvas) {

    rect::draw_rect(x, y, width, height, color, canvas);
}

// Guarantee at least this many whole background pixel columns between one
// glyph's rendered ink and the next. This has to be enforced in *rounded
// pixel space*, not on the float advance/bearing math: at small sizes a
// sub-pixel float gap (e.g. 0.5px) still reliably rounds away to 0 real
// pixels once both glyphs' positions are independently rounded, so glyphs
// end up touching no matter how the float gap is computed beforehand.
const MIN_PIXEL_GAP: i64 = 1;

// Un caractere deja positionne par `layout_line` : `pen_x` est la position
// du "crayon" juste avant ce caractere (ce qu'un curseur d'edition doit
// utiliser), `glyph_x` celle ou son encre est effectivement dessinee
// (`pen_x` + son propre lsb, puis eventuellement repoussee par
// `MIN_PIXEL_GAP` pour ne pas toucher le caractere precedent - les deux
// divergent legerement, d'ou l'existence des deux champs).
struct PositionedGlyph {
    pen_x: i64,
    glyph_x: i64,
    glyph: Glyph,
}

// Calcule la position de chaque caractere de `text` pour `draw_text` -
// c'est la SEULE implementation de cette logique (anti-collision par
// pixel comprise) : `draw_text`, `measure_text_width` et `char_index_at`
// l'appellent tous plutot que de la reimplementer chacun a leur maniere,
// ce qui les faisait auparavant deriver l'un de l'autre (un curseur
// d'edition mesure par une logique differente de celle qui dessine le
// texte fini legerement, mais visiblement, mal place - pire apres
// certaines sequences de caracteres, ex: autour des espaces, qui
// declenchent ou non l'anti-collision differemment selon ce qui les
// entoure).
fn layout_line(text: &str, face: &mut ttf_parser::Face, font_key: usize, size: f32, weight: f32, letter_spacing: f32) -> Result<(Vec<PositionedGlyph>, i64), String> {
    let count = text.chars().count();
    let mut result = Vec::with_capacity(text.chars().count());
    let mut cursor_x = 0.0f32;
    let mut prev_right_edge: Option<i64> = None;

    for character in text.chars() {
        if character == '\n' {
            // La plupart des polices n'ont pas de glyphe imprimable pour un
            // retour a la ligne - `extract_glyph_from_face` echouerait
            // (`Glyph not found`), ce qui faisait auparavant echouer TOUT
            // `draw_text`/`measure_text_width` des qu'un `\n` apparaissait
            // dans le texte (silencieusement, via `let _ = draw_text(...)`
            // cote appelant : le texte entier devenait invisible dès qu'on
            // appuyait sur Entree). On le traite comme un caractere
            // invisible de largeur nulle : le retour a la ligne visuel
            // lui-meme est gere plus haut, par decoupage en plusieurs
            // lignes avant d'appeler ces fonctions (voir, cote
            // azure_foundation, `ui::services::text_layout::wrap_lines`).
            let pen_x = cursor_x.round() as i64;
            result.push(PositionedGlyph { pen_x, glyph_x: pen_x, glyph: Glyph::new(0.0, 0.0, 0.0, 0.0, 0.0, Vec::new()) });
            continue;
        }

        // Caractere absent de la police (`→`, un emoji...) : un `?` a sa
        // place. Avant, l'erreur faisait disparaitre TOUT le texte.
        let glyph = match extract_glyph_for_font(face, font_key, character, size, weight) {
            Ok(glyph) => glyph,
            Err(_) => extract_glyph_for_font(face, font_key, '?', size, weight)?,
        };
        let advance = get_advance(&glyph);

        // The ink doesn't start exactly at the pen position — lsb accounts for
        // each glyph's own left side bearing, otherwise spacing looks uneven.
        let mut glyph_x = (cursor_x + glyph.lsb).round() as i64;

        // Enforce the minimum gap against the previous glyph's actual rightmost
        // drawn column, after rounding, so it can't be erased by rounding.
        if let Some(prev_edge) = prev_right_edge {
            let min_x = prev_edge + MIN_PIXEL_GAP;
            if glyph_x < min_x {
                cursor_x += (min_x - glyph_x) as f32;
                glyph_x = min_x;
            }
        }

        // Position du crayon juste avant ce caractere, APRES ce
        // repoussement eventuel : c'est la ou un curseur d'edition doit
        // s'afficher pour cet index, pas `glyph_x` (qui inclut en plus le
        // lsb propre a ce caractere).
        let pen_x = cursor_x.round() as i64;

        // draw_glyph's coverage buffer is width.ceil()+1 wide (room for the AA
        // edge column), so the rightmost column it can actually paint is at
        // local x = width.ceil(), not width.ceil() - 1.
        prev_right_edge = Some(glyph_x + glyph.width.ceil() as i64);
        cursor_x += advance;
        // `letter-spacing` : entre deux caracteres (pas apres le dernier).
        if result.len() + 1 < count {
            cursor_x += letter_spacing;
        }

        result.push(PositionedGlyph { pen_x, glyph_x, glyph });
    }

    Ok((result, cursor_x.round() as i64))
}

/// Italique et espacement des lettres (`font-style`, `letter-spacing`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextOptions {
    /// Oblique synthetique (la police n'a pas de variante italique).
    pub italic: bool,
    /// Espace ajoute entre deux caracteres, en pixels (peut etre negatif).
    pub letter_spacing: f32,
}

/// Pente de l'oblique synthetique (environ 12 degres).
const ITALIC_SLANT: f32 = 0.21;

#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
pub fn draw_text(text: &str, font_path: &str, x: u32, y: u32, size: f32, weight: f32, color: &Color, canvas: &mut Canvas) -> Result<(), String> {
    draw_text_with(text, font_path, x, y, size, weight, color, &TextOptions::default(), canvas)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_with(text: &str, font_path: &str, x: u32, y: u32, size: f32, weight: f32, color: &Color, options: &TextOptions, canvas: &mut Canvas) -> Result<(), String> {
    let font_data = load_font(font_path)?;
    // Parse une seule fois pour toute la chaine plutot qu'une fois par
    // caractere (c'etait le cas avant, via `exctract_glyph`) : pour une
    // police variable comme celle-ci, `Face::parse` + l'interpolation des
    // tables de variation ne sont pas negligeables, et ce cout etait donc
    // multiplie par la longueur du texte a chaque frame - la cause du delai
    // "enorme" en tapant dans une textarea, qui empire avec le texte tape.
    let mut face = ttf_parser::Face::parse(&font_data, 0).map_err(|e| e.to_string())?;
    let (positioned, _) = layout_line(text, &mut face, Rc::as_ptr(&font_data) as usize, size, weight, options.letter_spacing)?;

    for p in &positioned {
        // All glyphs share one baseline; descenders (g/p/q/j/y) hang below it
        // instead of having their bounding box aligned like every other letter.
        let glyph_y = (y as f32 + size - p.glyph.height + p.glyph.descent).round() as u32;
        if options.italic {
            let baseline = p.glyph.height - p.glyph.descent;
            draw_glyph_slanted(&p.glyph, (x as i64 + p.glyph_x) as u32, glyph_y, baseline, ITALIC_SLANT, color, canvas);
        } else {
            draw_glyph(&p.glyph, (x as i64 + p.glyph_x) as u32, glyph_y, color, canvas);
        }
    }

    Ok(())
}

/// Largeur totale (en pixels) qu'occuperait `text` dessine par `draw_text`
/// aux memes taille/poids, sans rien dessiner - meme logique exacte que
/// `draw_text` (voir `layout_line`), donc toujours coherente avec ce qui
/// est effectivement affiche. Utilise pour positionner un curseur de
/// saisie clignotant juste apres le texte deja tape (voir, cote
/// azure_foundation, `ui::services::draw_ui::draw_textarea`).
pub fn measure_text_width(text: &str, font_path: &str, size: f32, weight: f32) -> Result<f32, String> {
    measure_text_width_with(text, font_path, size, weight, &TextOptions::default())
}

pub fn measure_text_width_with(text: &str, font_path: &str, size: f32, weight: f32, options: &TextOptions) -> Result<f32, String> {
    let font_data = load_font(font_path)?;
    let mut face = ttf_parser::Face::parse(&font_data, 0).map_err(|e| e.to_string())?;
    let (_, end) = layout_line(text, &mut face, Rc::as_ptr(&font_data) as usize, size, weight, options.letter_spacing)?;
    Ok(end as f32)
}

/// Position en pixels (relative au point de depart de `text`) ou se
/// trouverait le curseur d'edition juste avant le caractere numero
/// `char_idx` (ou juste apres le dernier caractere si `char_idx` depasse
/// sa longueur) - meme logique exacte que `draw_text` (voir `layout_line`).
/// C'est ce qui permet au curseur clignotant affiche de ne jamais deriver
/// de l'endroit ou le texte est reellement dessine.
pub fn char_position(text: &str, font_path: &str, size: f32, weight: f32, char_idx: usize) -> Result<f32, String> {
    let options = TextOptions::default();
    let font_data = load_font(font_path)?;
    let mut face = ttf_parser::Face::parse(&font_data, 0).map_err(|e| e.to_string())?;
    let (positioned, end) = layout_line(text, &mut face, Rc::as_ptr(&font_data) as usize, size, weight, options.letter_spacing)?;
    Ok(match positioned.get(char_idx) {
        Some(p) => p.pen_x as f32,
        None => end as f32,
    })
}

/// Position en pixels du curseur d'edition juste avant CHAQUE caractere de
/// `text` (indices `0..=text.chars().count()`, le dernier etant la
/// position juste apres le dernier caractere) - une seule analyse de
/// police au lieu d'une par index demande (voir `char_position`). Utilise
/// par le retour a la ligne automatique (voir `ui::services::text_layout`,
/// cote azure_foundation), qui a besoin de tester beaucoup de positions
/// d'un coup et ne peut pas se permettre de re-analyser la police a
/// chaque fois.
pub fn char_positions(text: &str, font_path: &str, size: f32, weight: f32) -> Result<Vec<f32>, String> {
    char_positions_with(text, font_path, size, weight, &TextOptions::default())
}

pub fn char_positions_with(text: &str, font_path: &str, size: f32, weight: f32, options: &TextOptions) -> Result<Vec<f32>, String> {
    let font_data = load_font(font_path)?;
    let mut face = ttf_parser::Face::parse(&font_data, 0).map_err(|e| e.to_string())?;
    let (positioned, end) = layout_line(text, &mut face, Rc::as_ptr(&font_data) as usize, size, weight, options.letter_spacing)?;
    let mut positions: Vec<f32> = positioned.iter().map(|p| p.pen_x as f32).collect();
    positions.push(end as f32);
    Ok(positions)
}

/// L'inverse de `char_position` : a quel index de caractere correspond la
/// position en pixels `pixel_x` (relative au meme point de depart) - pour
/// qu'un clic souris place le curseur d'edition exactement sous le
/// pointeur (voir `ui::services::interact`, cote azure_foundation).
/// Arrondit au caractere dont le CENTRE est le plus proche de `pixel_x`,
/// pas systematiquement vers le premier ou le dernier.
pub fn char_index_at(text: &str, font_path: &str, size: f32, weight: f32, pixel_x: f32) -> Result<usize, String> {
    let options = TextOptions::default();
    let font_data = load_font(font_path)?;
    let mut face = ttf_parser::Face::parse(&font_data, 0).map_err(|e| e.to_string())?;
    let (positioned, end) = layout_line(text, &mut face, Rc::as_ptr(&font_data) as usize, size, weight, options.letter_spacing)?;

    if positioned.is_empty() || pixel_x <= 0.0 {
        return Ok(0);
    }

    for i in 0..positioned.len() {
        let start = positioned[i].pen_x as f32;
        let stop = positioned.get(i + 1).map(|p| p.pen_x as f32).unwrap_or(end as f32);
        let mid = (start + stop) / 2.0;
        if pixel_x < mid {
            return Ok(i);
        }
    }
    Ok(positioned.len())
}

/// Charge (avec cache, voir `services::image::load_png`) et decode le PNG
/// a `path` en pixels RGBA. `path` est relatif au repertoire de travail,
/// comme un chemin de police (voir `load_font`).
pub fn load_image(path: &str) -> Result<Rc<DecodedImage>, String> {
    image::load_png(path)
}

/// Compose une image deja decodee sur `canvas` a `(x, y)`, a sa taille
/// naturelle - voir `services::image::draw_image` pour le detail (alpha
/// blending, decoupage).
pub fn draw_image(decoded: &DecodedImage, x: u32, y: u32, canvas: &mut Canvas) {
    image::draw_image(&decoded.pixels, decoded.width, decoded.height, x, y, canvas);
}

/// Comme `draw_image`, mais reduite/agrandie a `(dst_width, dst_height)` -
/// voir `services::image::draw_image_scaled` (plus proche voisin). Utilise
/// pour une icone d'application dessinee plus petite que l'image source
/// (voir `window::services::draw_header`, cote azure_foundation).
/// Image a une position signee, a la taille donnee (voir
/// `services::image::draw_image_at`).
pub fn draw_image_at(decoded: &DecodedImage, x: i32, y: i32, width: u32, height: u32, canvas: &mut Canvas) {
    image::draw_image_at(&decoded.pixels, decoded.width, decoded.height, x, y, width, height, canvas);
}

pub fn draw_image_scaled(decoded: &DecodedImage, x: u32, y: u32, dst_width: u32, dst_height: u32, canvas: &mut Canvas) {
    image::draw_image_scaled(&decoded.pixels, decoded.width, decoded.height, x, y, dst_width, dst_height, canvas);
}
