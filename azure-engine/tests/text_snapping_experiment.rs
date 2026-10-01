// Experiment only — NOT wired into the main renderer. Compares normal
// rendering against a crude "snap every contour point to the pixel grid"
// version, to see if it actually helps crispness or just distorts curves.
// Run with:
//   cargo test --test text_snapping_experiment -- --nocapture
// Then look at /tmp/azure_text_check/snap_compare.ppm — top half is
// unsnapped (current renderer), bottom half is the snapped experiment.

use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::glyph::Glyph;
use azure_engine::rendering::managers::renderer::draw_text;
use azure_engine::rendering::services::text::glyph::exctract_glyph;
use azure_engine::rendering::services::text::kerning::get_advance;
use azure_engine::rendering::services::text::loader::load_font;
use azure_engine::rendering::services::text::renderer::draw_glyph;

const MIN_PIXEL_GAP: i64 = 1;

// Round every contour point to the nearest pixel. Crude on purpose: no stem
// detection, no notion of which edges matter — just quantize everything.
fn snap_glyph(glyph: &Glyph) -> Glyph {
    let contours = glyph.contours.iter()
        .map(|c| c.iter().map(|(x, y)| (x.round(), y.round())).collect())
        .collect();
    Glyph::new(glyph.width.round(), glyph.height.round(), glyph.advance_width, glyph.descent.round(), glyph.lsb.round(), contours)
}

#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
fn draw_snapped(text: &str, font_path: &str, x: u32, y: u32, size: f32, weight: f32, color: &Color, canvas: &mut Canvas) {
    let font_data = load_font(font_path).unwrap();
    let mut cursor_x = x as f32;
    let mut prev_right_edge: Option<i64> = None;

    for ch in text.chars() {
        let raw_glyph = exctract_glyph(&font_data, ch, size, weight).unwrap();
        let glyph = snap_glyph(&raw_glyph);
        let advance = get_advance(&glyph);

        let mut glyph_x = (cursor_x + glyph.lsb).round() as i64;
        if let Some(prev_edge) = prev_right_edge {
            let min_x = prev_edge + MIN_PIXEL_GAP;
            if glyph_x < min_x {
                cursor_x += (min_x - glyph_x) as f32;
                glyph_x = min_x;
            }
        }

        let glyph_y = (y as f32 + size - glyph.height + glyph.descent).round() as u32;
        draw_glyph(&glyph, glyph_x as u32, glyph_y, color, canvas);

        prev_right_edge = Some(glyph_x + glyph.width.ceil() as i64);
        cursor_x += advance;
    }
}

#[test]
fn compare_snap_vs_normal() {
    let font = "src/Roboto-VariableFont_wdth,wght.ttf";
    let sample = "AaBbGgQqYyJj oe 12";
    let mut canvas = Canvas::new(600, 220);
    canvas.buffer.chunks_mut(4).for_each(|p| { p[0] = 50; p[1] = 30; p[2] = 30; p[3] = 255; });
    let green = Color::new(0, 255, 0, 255);

    // Unsnapped, current renderer — top half, a few size/weight combos.
    draw_text(sample, font, 10, 10, 12.0, 100.0, &green, &mut canvas).unwrap();
    draw_text(sample, font, 10, 40, 12.0, 400.0, &green, &mut canvas).unwrap();
    draw_text(sample, font, 10, 70, 24.0, 400.0, &green, &mut canvas).unwrap();

    // Snapped experiment — bottom half, same combos.
    draw_snapped(sample, font, 10, 120, 12.0, 100.0, &green, &mut canvas);
    draw_snapped(sample, font, 10, 150, 12.0, 400.0, &green, &mut canvas);
    draw_snapped(sample, font, 10, 180, 24.0, 400.0, &green, &mut canvas);

    let mut out = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) {
        out.push(px[2]);
        out.push(px[1]);
        out.push(px[0]);
    }
    std::fs::create_dir_all(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure_text_check")).unwrap();
    std::fs::write(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure_text_check/snap_compare.ppm"), out).unwrap();
    println!("Wrote /tmp/azure_text_check/snap_compare.ppm (top=normal, bottom=snapped)");
}
