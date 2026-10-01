use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::managers::renderer::draw_text;

#[test]
fn dump_case_compare() {
    std::fs::create_dir_all(concat!(env!("CARGO_TARGET_TMPDIR"), "/imgcheck")).unwrap();
    let mut canvas = Canvas::new(500, 300);
    canvas.buffer.chunks_mut(4).for_each(|p| { p[0]=50; p[1]=30; p[2]=30; p[3]=255; });
    let green = Color::new(0,255,0,255);
    let font = "src/Sora-VariableFont_wght.ttf";
    draw_text("abcdefghijklmnopqrstuvwxyz", font, 10, 10, 12.0, 800.0, &green, &mut canvas).unwrap();
    draw_text("ABCDEFGHIJKLMNOPQRSTUVWXYZ", font, 10, 40, 12.0, 800.0, &green, &mut canvas).unwrap();
    draw_text("abcdefghijklmnopqrstuvwxyz", font, 10, 80, 28.0, 800.0, &green, &mut canvas).unwrap();
    draw_text("ABCDEFGHIJKLMNOPQRSTUVWXYZ", font, 10, 130, 28.0, 800.0, &green, &mut canvas).unwrap();

    let mut out = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) { out.push(px[2]); out.push(px[1]); out.push(px[0]); }
    std::fs::write(concat!(env!("CARGO_TARGET_TMPDIR"), "/imgcheck/case_compare.ppm"), out).unwrap();
}
