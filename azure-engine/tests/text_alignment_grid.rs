// Draws a red line through the center of each glyph's advance cell, on top
// of the actual rendered text, so cell width / centering can be checked by
// eye instead of guessed. Run with:
//   cargo test --test text_alignment_grid -- --nocapture
// Then open /tmp/azure_text_check/grid_*.ppm (or convert to PNG with
// `convert foo.ppm foo.png`).
//
// Note: Sora is a proportional (non-monospace) font — narrow letters like
// "i"/"l" and wide letters like "m"/"w" are *supposed* to have different
// cell widths. Equal visual gaps between every letter is not the goal;
// what matters is that each letter sits roughly centered under its own
// line, and that no letter's ink crosses into its neighbor's cell.

use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::managers::renderer::draw_text;
use azure_engine::rendering::services::text::glyph::exctract_glyph;
use azure_engine::rendering::services::text::kerning::get_advance;
use azure_engine::rendering::services::text::loader::load_font;
use azure_engine::rendering::services::shapes::line::draw_line_vertical;

fn dump_with_grid(name: &str, text: &str, size: f32, weight: f32) {
    let font_path = "src/Sora-VariableFont_wght.ttf";
    let start_x = 10u32;
    let start_y = 5u32;

    let mut canvas = Canvas::new(500, 40);
    canvas.buffer.chunks_mut(4).for_each(|p| { p[0]=50; p[1]=30; p[2]=30; p[3]=255; });

    let green = Color::new(0, 255, 0, 255);
    draw_text(text, font_path, start_x, start_y, size, weight, &green, &mut canvas).unwrap();

    // Same cursor math draw_text uses internally, so the lines line up with
    // what was actually drawn.
    let font_data = load_font(font_path).unwrap();
    let mut cursor_x = start_x as f32;
    let red = Color::new(255, 0, 0, 255);
    for ch in text.chars() {
        let glyph = exctract_glyph(&font_data, ch, size, weight).unwrap();
        let advance = get_advance(&glyph);
        let center = (cursor_x + advance / 2.0).round() as u32;
        draw_line_vertical(0, 40, center, &red, &mut canvas);
        cursor_x += advance;
    }

    let mut out = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) {
        out.push(px[2]);
        out.push(px[1]);
        out.push(px[0]);
    }
    std::fs::create_dir_all(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure_text_check")).unwrap();
    std::fs::write(format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure_text_check/grid_{}.ppm"), name), out).unwrap();
}

#[test]
fn dump_alignment_grid() {
    dump_with_grid("size12_weight100", "abcdefghijklmnopqrstuvwxyz", 12.0, 100.0);
    dump_with_grid("size12_weight800", "abcdefghijklmnopqrstuvwxyz", 12.0, 800.0);
    println!("Wrote grid PPMs to /tmp/azure_text_check/");
}
