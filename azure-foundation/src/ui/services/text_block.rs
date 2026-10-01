// Decoupage d'un texte web en lignes (retour a la ligne aux mots, `\n` en
// `white-space: pre`), partage par la mesure (`layout::managers::web_layout`)
// et le dessin (`ui::services::draw_label`) : les deux voient exactement
// les memes lignes.
use crate::style::models::web_style::WhiteSpace;
use crate::ui::models::text_style::TextStyle;
use crate::ui::services::text_layout::char_slice;
use azure_engine::rendering::managers::renderer::measure_text_width_with;

/// Lignes de `text` sous la forme `(debut, fin, largeur en px)` (indices en
/// caracteres). `max_width = None` : aucune coupure automatique (seulement
/// les `\n` en `pre`). Mis en cache dans `style` pour une largeur donnee.
pub fn lines(text: &str, style: &TextStyle, size: f32, weight: f32, max_width: Option<f32>) -> Vec<(usize, usize, f32)> {
    let wraps = style.white_space == WhiteSpace::Normal;
    let width_key = match (wraps, max_width) {
        (true, Some(w)) => (w * 10.0).round() as i32,
        _ => -1,
    };
    if let Some((key, cached)) = style.lines.borrow().as_ref()
        && *key == width_key {
            return cached.clone();
        }

    // En `pre`, les espaces de fin comptent (indentation, texte colle a un
    // autre) ; ailleurs, l'espace ou la ligne se coupe ne prend pas de place.
    let pre = style.white_space == WhiteSpace::Pre;
    let width_of = |start: usize, end: usize| {
        let slice = char_slice(text, start, end);
        measure_text_width_with(if pre { slice } else { slice.trim_end() }, style.font, size, weight, &style.options).unwrap_or(0.0)
    };
    let computed: Vec<(usize, usize, f32)> = if width_key >= 0 {
        crate::ui::services::text_layout::wrap_lines_with(text, style.font, size, weight, max_width.unwrap_or(f32::MAX), &style.options)
            .into_iter()
            .map(|(s, e)| (s, e, width_of(s, e)))
            .collect()
    } else {
        // Pas de coupure : une ligne par `\n` (il n'y en a qu'en `pre`, les
        // autres modes ont deja regroupe les espaces).
        let mut out = Vec::new();
        let mut start = 0;
        for (i, c) in text.chars().enumerate() {
            if c == '\n' {
                out.push((start, i, width_of(start, i)));
                start = i + 1;
            }
        }
        let total = text.chars().count();
        out.push((start, total, width_of(start, total)));
        out
    };
    *style.lines.borrow_mut() = Some((width_key, computed.clone()));
    computed
}

/// Largeur du plus long mot (ou de toute la ligne si le texte ne revient pas
/// a la ligne) : la largeur minimale sous laquelle le texte deborde.
pub fn min_content_width(text: &str, style: &TextStyle, size: f32, weight: f32) -> f32 {
    if style.white_space != WhiteSpace::Normal {
        return lines(text, style, size, weight, None).iter().map(|l| l.2).fold(0.0, f32::max);
    }
    text.split_whitespace()
        .map(|word| measure_text_width_with(word, style.font, size, weight, &style.options).unwrap_or(0.0))
        .fold(0.0, f32::max)
}
