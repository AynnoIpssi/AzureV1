// Un caractere absent de la police ne doit pas faire disparaitre tout le
// texte : il est remplace par un `?` (meme largeur).
use azure_engine::rendering::managers::renderer::{draw_text, measure_text_width};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;

const FONT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/Sora-VariableFont_wght.ttf");

#[test]
fn a_missing_character_is_drawn_as_a_question_mark() {
    // U+2192 (fleche) n'existe pas dans Sora.
    let with_arrow = measure_text_width("panier \u{2192} caisse", FONT, 14.0, 400.0).unwrap();
    let with_mark = measure_text_width("panier ? caisse", FONT, 14.0, 400.0).unwrap();
    assert_eq!(with_arrow, with_mark);

    let mut canvas = Canvas::new(200, 30);
    draw_text("panier \u{2192} caisse", FONT, 2, 4, 14.0, 400.0, &Color::new(255, 255, 255, 255), &mut canvas).unwrap();
    assert!(canvas.buffer.chunks(4).any(|px| px[0] > 128), "le texte doit etre dessine");
}
