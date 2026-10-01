// letter-spacing (mesure et dessin coherents) et italique synthetique.
use azure_engine::rendering::managers::renderer::{char_positions_with, draw_text_with, measure_text_width, measure_text_width_with, TextOptions};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;

const FONT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/Sora-VariableFont_wght.ttf");

#[test]
fn letter_spacing_is_added_between_characters() {
    let plain = measure_text_width("Azure", FONT, 20.0, 400.0).unwrap();
    let spaced = measure_text_width_with("Azure", FONT, 20.0, 400.0, &TextOptions { letter_spacing: 3.0, ..Default::default() }).unwrap();
    assert!((spaced - plain - 12.0).abs() <= 1.0, "4 espaces de 3px : {plain} -> {spaced}");
    let positions = char_positions_with("Azure", FONT, 20.0, 400.0, &TextOptions { letter_spacing: 3.0, ..Default::default() }).unwrap();
    assert_eq!(*positions.last().unwrap(), spaced, "mesure et positions coherentes");
}

/// Colonne la plus a droite ou le texte a de l'encre, sur les lignes `rows`.
fn rightmost_ink(c: &Canvas, rows: std::ops::Range<u32>) -> u32 {
    let mut best = 0;
    for y in rows {
        for x in 0..c.width {
            if c.buffer[((y * c.width + x) * 4) as usize] > 60 {
                best = best.max(x);
            }
        }
    }
    best
}

#[test]
fn italic_leans_the_top_to_the_right() {
    let white = Color::new(255, 255, 255, 255);
    let (mut plain, mut italic) = (Canvas::new(120, 60), Canvas::new(120, 60));
    draw_text_with("I", FONT, 20, 5, 40.0, 400.0, &white, &TextOptions::default(), &mut plain).unwrap();
    draw_text_with("I", FONT, 20, 5, 40.0, 400.0, &white, &TextOptions { italic: true, ..Default::default() }, &mut italic).unwrap();
    // Haut du « I » decale a droite, pied (pres de la ligne de base) presque pas.
    // (Le « I » occupe les lignes 16 a 45 ; sa ligne de base est en bas.)
    assert!(rightmost_ink(&italic, 16..20) >= rightmost_ink(&plain, 16..20) + 4, "haut penche");
    assert!(rightmost_ink(&italic, 44..46) <= rightmost_ink(&plain, 44..46) + 1, "pied a sa place");
}
