// Visual check for the minimum-glyph-gap fix (see MIN_GLYPH_GAP_PX in
// src/rendering/managers/renderer.rs). Run with:
//   cargo test --test text_min_spacing_check -- --nocapture
// Then open the .ppm files under /tmp/azure_text_check/ (any image viewer
// that reads PPM, or convert to PNG with `convert foo.ppm foo.png`).

use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::managers::renderer::draw_text;

fn dump(name: &str, text: &str, size: f32, weight: f32) {
    let mut canvas = Canvas::new(500, 40);
    canvas.buffer.chunks_mut(4).for_each(|p| { p[0]=50; p[1]=30; p[2]=30; p[3]=255; });
    let color = Color::new(0, 255, 0, 255);
    draw_text(text, "src/Sora-VariableFont_wght.ttf", 10, 5, size, weight, &color, &mut canvas)
        .expect("Failed to draw text");

    let mut out = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    // canvas buffer is B,G,R,A (matches Wayland's ARGB8888); PPM wants R,G,B
    for px in canvas.buffer.chunks(4) {
        out.push(px[2]);
        out.push(px[1]);
        out.push(px[0]);
    }
    std::fs::create_dir_all(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure_text_check")).unwrap();
    std::fs::write(format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure_text_check/{}.ppm"), name), out).unwrap();
}

#[test]
fn dump_min_spacing_variants() {
    dump("size12_weight100", "abcdefghijklmnopqrstuvwxyz", 12.0, 100.0);
    dump("size12_weight400", "abcdefghijklmnopqrstuvwxyz", 12.0, 400.0);
    dump("size12_weight800", "abcdefghijklmnopqrstuvwxyz", 12.0, 800.0);
    dump("size14_weight800", "abcdefghijklmnopqrstuvwxyz", 14.0, 800.0);
    dump("size28_weight800", "abcdefghijklmnopqrstuvwxyz", 28.0, 800.0);
    println!("Wrote PPMs to /tmp/azure_text_check/");
}
