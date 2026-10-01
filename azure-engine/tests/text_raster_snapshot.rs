// Sauvegarde / compare le rendu brut d'un texte (octets BGRA) avec une
// capture de reference : TEXT_SNAPSHOT=save pour l'ecrire, sinon compare
// si le fichier existe. Sert a verifier qu'une optimisation du rendu du
// texte ne change pas les pixels.
use azure_engine::rendering::managers::renderer::{draw_rect, draw_text};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;

const FONT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/Sora-VariableFont_wght.ttf");

fn render() -> Canvas {
    let mut canvas = Canvas::new(640, 120);
    draw_rect(0, 0, 640, 120, &Color::new(30, 30, 46, 255), &mut canvas);
    draw_text("Le rapide renard brun - saute 'par-dessus' g/p/q", FONT, 4, 4, 16.0, 400.0, &Color::new(240, 240, 240, 255), &mut canvas).unwrap();
    draw_text("Titre gras 24px *", FONT, 4, 40, 24.0, 700.0, &Color::new(124, 156, 255, 255), &mut canvas).unwrap();
    draw_text("Code 13px <container.carte>", FONT, 4, 90, 13.0, 400.0, &Color::new(200, 211, 255, 200), &mut canvas).unwrap();
    canvas
}

#[test]
fn text_pixels_match_snapshot() {
    let path = std::env::var("TEXT_SNAPSHOT_PATH").unwrap_or_else(|_| format!("{}/text_snapshot.bin", env!("CARGO_TARGET_TMPDIR")));
    let canvas = render();
    if std::env::var("TEXT_SNAPSHOT").as_deref() == Ok("save") {
        std::fs::write(&path, &canvas.buffer).unwrap();
        return;
    }
    let Ok(reference) = std::fs::read(&path) else { return };
    let mut max_diff = 0u8;
    let mut differing = 0usize;
    for (a, b) in canvas.buffer.iter().zip(&reference) {
        let d = a.abs_diff(*b);
        if d > 0 { differing += 1; }
        max_diff = max_diff.max(d);
    }
    println!("octets differents : {differing}, ecart max : {max_diff}");
    assert!(max_diff <= 1, "ecart max {max_diff}");
}
