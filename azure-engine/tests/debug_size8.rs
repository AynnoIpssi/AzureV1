use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::managers::renderer::draw_text;

#[test]
fn dump_size8() {
    std::fs::create_dir_all(concat!(env!("CARGO_TARGET_TMPDIR"), "/imgcheck")).unwrap();
    let mut canvas = Canvas::new(300, 30);
    canvas.buffer.chunks_mut(4).for_each(|p| { p[0]=50; p[1]=30; p[2]=30; p[3]=255; });
    let green = Color::new(0,255,0,255);
    draw_text("abcdefghijklmnopqrstuvwxyz", "src/Sora-VariableFont_wght.ttf", 10, 10, 8.0, 100.0, &green, &mut canvas).unwrap();
    let mut out = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) { out.push(px[2]); out.push(px[1]); out.push(px[0]); }
    std::fs::write(concat!(env!("CARGO_TARGET_TMPDIR"), "/imgcheck/size8.ppm"), out).unwrap();
}
