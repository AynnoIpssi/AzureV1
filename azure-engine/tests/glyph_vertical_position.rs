// Regression : les glyphes dont l'encre commence AU-DESSUS de la ligne de
// base (- ' " *) etaient dessines poses sur la ligne de base : "-" ressemblait
// a "_", "'" a ".", '"' a "„" (vu dans Azure Docs : "ci_dessous", "l.ecran").
use azure_engine::rendering::services::text::glyph::extract_glyph_from_face;
use azure_engine::rendering::services::text::loader::load_font;

const FONT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/Sora-VariableFont_wght.ttf");
const SIZE: f32 = 16.0;

// Distance (en pixels, vers le haut) entre la ligne de base et le BAS de
// l'encre, telle que `draw_text` la place : bas = baseline + descent.
fn ink_bottom_above_baseline(c: char) -> f32 {
    let data = load_font(FONT).unwrap();
    let mut face = ttf_parser::Face::parse(&data, 0).unwrap();
    -extract_glyph_from_face(&mut face, c, SIZE, 400.0).unwrap().descent
}

#[test]
fn floating_glyphs_stay_above_baseline() {
    for c in ['-', '\'', '"', '*'] {
        let lift = ink_bottom_above_baseline(c);
        println!("{c:?}: bas de l'encre a {lift:.2}px au-dessus de la ligne de base");
        assert!(lift > 2.0, "{c:?} est colle a la ligne de base ({lift:.2}px)");
    }
}

#[test]
fn baseline_and_descender_glyphs_unchanged() {
    assert!(ink_bottom_above_baseline('x').abs() < 0.5, "x doit reposer sur la ligne de base");
    assert!(ink_bottom_above_baseline('g') < -2.0, "g doit descendre sous la ligne de base");
}
