// Mise en page du texte riche (`TextArea::rich`) : chaque morceau de meme
// style se mesure dans sa police (gras = graisse 700, italique = oblique
// synthetique, code = police a chasse fixe), ce qui donne l'abscisse de
// chaque caractere. Retour a la ligne, clic, curseur et selection partent
// tous de ces memes positions : le curseur tombe toujours la ou le texte
// est dessine.
use crate::ui::models::rich::RichStyle;
use crate::ui::services::draw_ui::FONT_PATH;
use crate::ui::services::fonts::font_for;
use crate::ui::services::text_layout;
use azure_engine::rendering::managers::renderer::{char_positions_with, TextOptions};

/// Interligne du texte riche (en fois la taille), plus aere qu'un champ.
pub const RICH_LINE_SPACING: f32 = 1.5;

/// Police, graisse et options d'un style (`base` : graisse du texte sans
/// gras, celle de la zone).
pub fn font_of(style: &RichStyle, base: f32) -> (&'static str, f32, TextOptions) {
    let path = if style.code { font_for(Some("monospace")) } else { FONT_PATH };
    let weight = if style.bold { (base + 300.0).min(900.0) } else { base };
    (path, weight, TextOptions { italic: style.italic, letter_spacing: 0.0 })
}

/// Taille de police d'un style (`base` : celle de la zone).
pub fn size_of(style: &RichStyle, base: f32) -> f32 {
    style.size.unwrap_or(base)
}

/// La plus grande taille de police de la ligne `[start, end)` (celle de la
/// zone pour une ligne vide).
pub fn line_size(chars: &[char], styles: &[RichStyle], size: f32, start: usize, end: usize) -> f32 {
    (start..end).filter(|i| chars.get(*i) != Some(&'\n')).map(|i| styles.get(i).map_or(size, |s| size_of(s, size))).fold(None, |m: Option<f32>, t| Some(m.map_or(t, |m| m.max(t)))).unwrap_or(size)
}

/// Hauteur de chaque ligne : sa plus grande police fois l'interligne.
pub fn line_heights(text: &str, styles: &[RichStyle], size: f32, lines: &[(usize, usize)]) -> Vec<u32> {
    let chars: Vec<char> = text.chars().collect();
    lines.iter().map(|&(a, b)| ((line_size(&chars, styles, size, a, b) * RICH_LINE_SPACING) as u32).max(1)).collect()
}

/// Haut d'un morceau de taille `run` dans une ligne de hauteur `height`
/// dont la plus grande police est `max` : les lignes de base s'alignent.
pub fn glyph_top(height: u32, max: f32, run: f32) -> f32 {
    ((height as f32 - max * 1.1) / 2.0).max(0.0) + (max - run) * 0.8
}

/// L'abscisse du debut de chaque caractere depuis le debut du texte
/// (`chars + 1` valeurs). Un `\n` ne prend pas de place.
///
/// Mesurer (mettre en forme chaque morceau dans sa police) est cher et se
/// refait a chaque mise en page : a chaque mouvement de souris, chaque
/// image, chaque clic. Le resultat ne depend que du texte, de la police de
/// chaque caractere et des tailles : il est garde (`POSITIONS`).
pub fn positions(text: &str, styles: &[RichStyle], size: f32, weight: f32) -> Vec<f32> {
    let key = positions_key(text, styles, size, weight);
    if let Some(p) = POSITIONS.with(|c| c.borrow().get(&key).cloned()) {
        return p;
    }
    let p = measure_positions(text, styles, size, weight);
    POSITIONS.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= POSITIONS_MAX {
            c.clear();
        }
        c.insert(key, p.clone());
    });
    p
}

/// Positions deja mesurees (par thread : chaque fenetre a le sien).
const POSITIONS_MAX: usize = 256;

thread_local! {
    static POSITIONS: std::cell::RefCell<std::collections::HashMap<u64, Vec<f32>>> = Default::default();
}

/// Ce dont dependent les positions : le texte, et pour chaque caractere ce
/// qui change sa largeur (gras, italique, code, taille) - pas la couleur.
fn positions_key(text: &str, styles: &[RichStyle], size: f32, weight: f32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    size.to_bits().hash(&mut h);
    weight.to_bits().hash(&mut h);
    for s in styles.iter().take(text.chars().count()) {
        (s.bold as u8 | (s.italic as u8) << 1 | (s.code as u8) << 2).hash(&mut h);
        s.size.map(f32::to_bits).hash(&mut h);
    }
    h.finish()
}

fn measure_positions(text: &str, styles: &[RichStyle], size: f32, weight: f32) -> Vec<f32> {
    let chars: Vec<char> = text.chars().collect();
    let default = RichStyle::default();
    let style = |i: usize| styles.get(i).unwrap_or(&default);
    let mut widths = vec![0.0f32; chars.len()];
    let mut start = 0;
    while start < chars.len() {
        if chars[start] == '\n' {
            start += 1;
            continue;
        }
        let key = (font_of(style(start), weight), size_of(style(start), size));
        let mut end = start + 1;
        while end < chars.len() && chars[end] != '\n' && (font_of(style(end), weight), size_of(style(end), size)) == key {
            end += 1;
        }
        let run: String = chars[start..end].iter().collect();
        let ((path, w, options), run_size) = key;
        match char_positions_with(&run, path, run_size, w, &options) {
            Ok(p) if p.len() == end - start + 1 => {
                for k in 0..end - start {
                    widths[start + k] = p[k + 1] - p[k];
                }
            }
            // Police illisible : une largeur moyenne, le texte reste utilisable.
            _ => widths[start..end].iter_mut().for_each(|w| *w = run_size * 0.55),
        }
        start = end;
    }
    let mut out = Vec::with_capacity(chars.len() + 1);
    let mut x = 0.0;
    out.push(x);
    for w in widths {
        x += w;
        out.push(x);
    }
    out
}

/// Positions et lignes (intervalles de caracteres) dans `max_width`.
pub fn layout(text: &str, styles: &[RichStyle], size: f32, weight: f32, max_width: f32) -> (Vec<f32>, Vec<(usize, usize)>) {
    let chars: Vec<char> = text.chars().collect();
    let positions = positions(text, styles, size, weight);
    let lines = text_layout::wrap_positions(&chars, &positions, max_width);
    (positions, lines)
}

/// Le caractere de la ligne `[start, end)` le plus proche de `x` (relatif
/// au debut de la ligne).
pub fn index_at(positions: &[f32], start: usize, end: usize, x: f32) -> usize {
    let base = positions[start];
    (start..=end).min_by(|a, b| {
        let da = (positions[*a] - base - x).abs();
        let db = (positions[*b] - base - x).abs();
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    }).unwrap_or(start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gras_plus_large_et_retour_a_la_ligne() {
        let bold = RichStyle { bold: true, ..Default::default() };
        let plain = positions("mmmm", &[], 16.0, 400.0);
        let heavy = positions("mmmm", &vec![bold; 4], 16.0, 400.0);
        assert!(heavy[4] > plain[4], "{heavy:?} / {plain:?}");
        let p = positions("ab\ncd", &[], 16.0, 400.0);
        assert_eq!(p[2], p[3]);
        let (_, lines) = layout("un deux trois quatre", &[], 16.0, 400.0, 60.0);
        assert!(lines.len() > 1);
        assert_eq!(index_at(&plain, 0, 4, 0.0), 0);
        assert_eq!(index_at(&plain, 0, 4, 1000.0), 4);
    }

    #[test]
    fn taille_par_caractere() {
        let big = RichStyle { size: Some(32.0), ..Default::default() };
        let plain = positions("mmmm", &[], 16.0, 400.0);
        let large = positions("mmmm", &vec![big.clone(); 4], 16.0, 400.0);
        assert!(large[4] > plain[4] * 1.5, "{large:?} / {plain:?}");
        let styles = vec![RichStyle::default(), RichStyle::default(), RichStyle::default(), big];
        let h = line_heights("ab\ncd", &styles, 16.0, &[(0, 3), (3, 5)]);
        assert_eq!(h, vec![24, 48]);
    }
}
