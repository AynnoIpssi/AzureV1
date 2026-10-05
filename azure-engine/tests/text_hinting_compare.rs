// Compare le rendu sans (gauche) et avec (droite) l'auto-hinting vertical,
// de 8 a 16px, pour les deux polices livrees. Lancer :
//   cargo test --test text_hinting_compare -- --nocapture
// puis regarder <target>/tmp/azure_text_check/hinting_compare.ppm.
use azure_engine::rendering::managers::renderer::{draw_rect, draw_text};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::services::text::glyph::set_hinting;

const FONTS: [&str; 2] = [
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/Sora-VariableFont_wght.ttf"),
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/Roboto-VariableFont_wdth,wght.ttf"),
];
const SAMPLE: &str = "Hamburgefonstiv xzeoc ETH 0123 - g/p/q";

#[test]
fn compare_hinting() {
    let (width, height) = (720u32, 420u32);
    let mut canvas = Canvas::new(width, height);
    draw_rect(0, 0, width, height, &Color::new(30, 30, 46, 255), &mut canvas);
    let fg = Color::new(235, 235, 235, 255);
    let mut y = 4u32;
    for font in FONTS {
        for size in 8..=16 {
            for (column, hinted) in [(0u32, false), (360, true)] {
                set_hinting(hinted);
                draw_text(SAMPLE, font, 4 + column, y, size as f32, 400.0, &fg, &mut canvas).unwrap();
            }
            y += size + 6;
        }
        y += 6;
    }
    set_hinting(true);

    let dir = format!("{}/azure_text_check", env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&dir).unwrap();
    let mut out = format!("P6\n{width} {height}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        out.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(format!("{dir}/hinting_compare.ppm"), out).unwrap();
}

#[test]
fn hinted_glyphs_share_the_baseline() {
    // Avec hinting, la boite de chaque glyphe est en pixels entiers : la
    // ligne de base (hauteur - descente) tombe sur un entier pour tous.
    use azure_engine::rendering::services::text::glyph::extract_glyph_from_face;
    use azure_engine::rendering::services::text::loader::load_font;
    for font in FONTS {
        let data = load_font(font).unwrap();
        let mut face = ttf_parser::Face::parse(&data, 0).unwrap();
        for size in [8.0, 11.0, 13.5] {
            for c in "Hxogp-'".chars() {
                let g = extract_glyph_from_face(&mut face, c, size, 400.0).unwrap();
                let baseline = g.height - g.descent;
                assert_eq!(baseline, baseline.round(), "{c:?} a {size}px : ligne de base {baseline}");
            }
            // Le pied du H et du x repose exactement sur la ligne de base.
            for c in ['H', 'x'] {
                assert_eq!(extract_glyph_from_face(&mut face, c, size, 400.0).unwrap().descent, 0.0, "{c:?} a {size}px");
            }
        }
    }
}
