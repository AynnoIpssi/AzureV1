use azure_engine::rendering::services::text::glyph::exctract_glyph;
use azure_engine::rendering::services::text::loader::load_font;
use azure_engine::rendering::services::text::kerning::get_advance;

const MIN_PIXEL_GAP: i64 = 1;

#[test]
fn trace_size8() {
    let font_data = load_font("src/Sora-VariableFont_wght.ttf").unwrap();
    let size = 8.0f32;
    let weight = 100.0f32;
    let mut cursor_x: f32 = 10.0;
    let mut prev_right_edge: Option<i64> = None;

    for ch in "abcdefghijklmnopqrstuvwxyz".chars() {
        let glyph = exctract_glyph(&font_data, ch, size, weight).unwrap();
        let advance = get_advance(&glyph);
        let mut glyph_x = (cursor_x + glyph.lsb).round() as i64;
        if let Some(prev_edge) = prev_right_edge {
            let min_x = prev_edge + MIN_PIXEL_GAP;
            if glyph_x < min_x {
                cursor_x += (min_x - glyph_x) as f32;
                glyph_x = min_x;
            }
        }
        let right_edge = glyph_x + glyph.width.ceil() as i64 - 1;
        println!("{:?}: lsb={:.2} width={:.2} advance={:.2} glyph_x={} right_edge={}", ch, glyph.lsb, glyph.width, advance, glyph_x, right_edge);
        prev_right_edge = Some(right_edge);
        cursor_x += advance;
    }
}
