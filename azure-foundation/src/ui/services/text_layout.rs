// Decoupage d'un texte en plusieurs lignes affichables - le retour a la
// ligne automatique d'une `TextArea` quand le texte atteint le bord de sa
// boite, plutot que de continuer a deborder hors de la zone (voir
// `ui::services::draw_ui::draw_textarea`).
use azure_engine::rendering::managers::renderer::{char_positions_with, TextOptions};

fn byte_index(text: &str, char_idx: usize) -> usize {
    text.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(text.len())
}

/// Le texte entre les positions en CARACTERES `start` et `end` (bornes
/// [start, end)) - le texte peut contenir de l'UTF-8 multi-octets, d'ou le
/// passage par un index de caracteres plutot qu'un decoupage direct par
/// octet.
pub fn char_slice(text: &str, start: usize, end: usize) -> &str {
    &text[byte_index(text, start)..byte_index(text, end)]
}

/// Decoupe `text` en lignes affichables dans une largeur de `max_width`
/// pixels : retour a la ligne automatique au niveau des mots (caractere
/// par caractere si un mot a lui seul depasse `max_width`), ET a chaque
/// retour a la ligne explicite (`\n`, insere par la touche Entree - voir
/// `ui::services::interact::KeyInput::Enter`). Chaque `(start, end)`
/// retourne est un intervalle de CARACTERES [start, end) dans `text`,
/// coherent avec `TextArea::cursor`. Toujours au moins une ligne, meme
/// pour un texte vide (`[(0, 0)]`).
pub fn wrap_lines(text: &str, font_path: &str, size: f32, weight: f32, max_width: f32) -> Vec<(usize, usize)> {
    wrap_lines_with(text, font_path, size, weight, max_width, &TextOptions::default())
}

/// Comme `wrap_lines`, avec italique / espacement des lettres.
pub fn wrap_lines_with(text: &str, font_path: &str, size: f32, weight: f32, max_width: f32, options: &TextOptions) -> Vec<(usize, usize)> {
    let total_chars = text.chars().count();
    if total_chars == 0 {
        return vec![(0, 0)];
    }

    // Une seule analyse de police pour tout le texte (voir
    // `char_positions`) : tester chaque largeur candidate separement
    // (une mesure par caractere) rechargerait/re-analyserait la police a
    // chaque fois, un cout multiplie par la longueur du texte a chaque
    // frame - exactement le probleme deja corrige ailleurs pour le
    // curseur/la selection.
    let positions = match char_positions_with(text, font_path, size, weight, options) {
        Ok(p) if p.len() == total_chars + 1 => p,
        // Police introuvable ou erreur de mesure : pas de retour a la
        // ligne automatique possible, mais on garde au moins les retours
        // explicites - mieux que de perdre completement le texte.
        _ => {
            let mut lines = Vec::new();
            let mut offset = 0usize;
            for segment in text.split('\n') {
                let start = offset;
                let end = start + segment.chars().count();
                lines.push((start, end));
                offset = end + 1;
            }
            return lines;
        }
    };

    let chars: Vec<char> = text.chars().collect();
    wrap_positions(&chars, &positions, max_width)
}

/// Le retour a la ligne, a partir des positions deja mesurees de chaque
/// caractere (`positions[i]` = abscisse du debut du caractere `i`,
/// `chars.len() + 1` valeurs) - partage avec le texte riche, dont chaque
/// morceau se mesure dans sa propre police (voir `rich_layout`).
pub fn wrap_positions(chars: &[char], positions: &[f32], max_width: f32) -> Vec<(usize, usize)> {
    let total_chars = chars.len();
    if total_chars == 0 {
        return vec![(0, 0)];
    }
    let mut lines = Vec::new();
    let mut line_start = 0usize;
    // Position juste apres le dernier espace rencontre sur la ligne
    // courante - le point de cassure "propre" prefere a une coupure en
    // plein milieu d'un mot.
    let mut last_space: Option<usize> = None;

    for i in 0..total_chars {
        if chars[i] == '\n' {
            lines.push((line_start, i));
            line_start = i + 1;
            last_space = None;
            continue;
        }

        let width_so_far = positions[i + 1] - positions[line_start];
        if width_so_far > max_width && i > line_start {
            match last_space {
                Some(bp) if bp > line_start => {
                    lines.push((line_start, bp));
                    line_start = bp;
                }
                // Pas d'espace sur cette ligne (un mot plus long que
                // `max_width` a lui seul) : on coupe au caractere.
                _ => {
                    lines.push((line_start, i));
                    line_start = i;
                }
            }
            last_space = None;
        }

        if chars[i] == ' ' {
            last_space = Some(i + 1);
        }
    }

    lines.push((line_start, total_chars));
    lines
}

/// L'index (dans `lines`, voir `wrap_lines`) de la ligne qui contient le
/// caractere numero `char_idx` - la derniere ligne si `char_idx` va
/// au-dela de la fin du texte (le curseur en toute fin de texte, par
/// exemple).
pub fn line_containing(lines: &[(usize, usize)], char_idx: usize) -> usize {
    for (i, &(_, end)) in lines.iter().enumerate() {
        if char_idx < end {
            return i;
        }
    }
    lines.len().saturating_sub(1)
}

/// Champ d'une ligne : premier caractere visible pour que le curseur reste
/// dans `width` pixels (le texte defile vers la gauche quand on tape au
/// dela du bord). Meme calcul au dessin et au clic.
pub fn single_line_start(text: &str, cursor: usize, width: u32, focused: bool, size: f32, weight: f32) -> usize {
    use crate::ui::services::draw_ui::FONT_PATH;
    if !focused || text.is_empty() {
        return 0;
    }
    let Ok(positions) = azure_engine::rendering::managers::renderer::char_positions(text, FONT_PATH, size, weight) else { return 0 };
    let cursor = cursor.min(positions.len().saturating_sub(1));
    let room = width.saturating_sub(6) as f32;
    let cursor_x = positions.get(cursor).copied().unwrap_or(0.0);
    (0..=cursor).find(|&start| cursor_x - positions[start] <= room).unwrap_or(cursor)
}
