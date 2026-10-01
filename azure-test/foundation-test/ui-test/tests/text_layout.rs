#[cfg(test)]
mod tests {
    
    use azure_foundation::ui::services::text_layout::*;

    const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../azure-engine/src/Sora-VariableFont_wght.ttf");

    #[test]
    fn empty_text_is_a_single_empty_line() {
        assert_eq!(wrap_lines("", FONT_PATH, 16.0, 400.0, 1000.0), vec![(0, 0)]);
    }

    #[test]
    fn short_text_that_fits_stays_on_one_line() {
        let lines = wrap_lines("hi", FONT_PATH, 16.0, 400.0, 1000.0);
        assert_eq!(lines, vec![(0, 2)]);
    }

    #[test]
    fn explicit_newline_always_breaks_the_line() {
        let lines = wrap_lines("hi\nthere", FONT_PATH, 16.0, 400.0, 1000.0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (0, 2));
        // Le '\n' (index 2) est consomme par la rupture, la 2e ligne
        // commence juste apres.
        assert_eq!(lines[1], (3, 8));
    }

    #[test]
    fn long_text_wraps_at_a_word_boundary_not_mid_word() {
        let text = "hello world this wraps";
        // Largeur choisie pour ne laisser passer qu'une partie des mots -
        // la coupure doit tomber apres un espace, jamais au milieu d'un mot.
        let lines = wrap_lines(text, FONT_PATH, 16.0, 400.0, 80.0);
        assert!(lines.len() > 1, "le texte devrait etre coupe sur plusieurs lignes");
        for &(start, end) in &lines {
            let segment = char_slice(text, start, end);
            // Un segment ne doit jamais commencer par un espace (consomme
            // par la ligne precedente) et ne contient jamais de mot coupe
            // en plein milieu par une simple troncature de largeur, sauf
            // s'il n'y avait aucun espace disponible.
            assert!(!segment.starts_with(' '), "segment ne doit pas commencer par un espace: {segment:?}");
        }
    }

    #[test]
    fn a_single_word_longer_than_the_line_gets_character_wrapped() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let lines = wrap_lines(text, FONT_PATH, 16.0, 400.0, 40.0);
        assert!(lines.len() > 1, "un mot plus long que la ligne doit quand meme etre coupe");
        // Toutes les lignes bout a bout redonnent le texte original.
        let rejoined: String = lines.iter().map(|&(s, e)| char_slice(text, s, e)).collect();
        assert_eq!(rejoined, text);
    }

    #[test]
    fn lines_are_contiguous_and_cover_the_whole_text() {
        let text = "the quick brown fox jumps over the lazy dog";
        let lines = wrap_lines(text, FONT_PATH, 16.0, 400.0, 100.0);
        assert_eq!(lines[0].0, 0);
        assert_eq!(lines.last().unwrap().1, text.chars().count());
        for pair in lines.windows(2) {
            assert_eq!(pair[0].1, pair[1].0, "les lignes doivent s'enchainer sans trou ni chevauchement");
        }
    }

    #[test]
    fn line_containing_finds_the_right_line_including_at_the_very_end() {
        let lines = vec![(0, 5), (5, 10), (10, 12)];
        assert_eq!(line_containing(&lines, 0), 0);
        assert_eq!(line_containing(&lines, 4), 0);
        assert_eq!(line_containing(&lines, 5), 1);
        assert_eq!(line_containing(&lines, 9), 1);
        assert_eq!(line_containing(&lines, 12), 2); // fin de tout le texte
    }

    #[test]
    fn char_slice_handles_multi_byte_utf8_characters() {
        let text = "café monde";
        // 'é' est un caractere, mais 2 octets - char_slice doit compter en
        // caracteres, pas en octets.
        assert_eq!(char_slice(text, 0, 4), "café");
        assert_eq!(char_slice(text, 5, 10), "monde");
    }
}
