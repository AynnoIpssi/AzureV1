use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::managers::renderer::draw_text;

#[test]
fn dump_roboto_matrix() {
    std::fs::create_dir_all(concat!(env!("CARGO_TARGET_TMPDIR"), "/imgcheck")).unwrap();
    let font = "src/Roboto-VariableFont_wdth,wght.ttf";
    let sizes: [f32; 8] = [10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0];
    let weights: [f32; 4] = [100.0, 400.0, 700.0, 900.0];
    let sample = "AaBbGgQqYyJj";
    let row_h = 34u32;
    let rows = (sizes.len() * weights.len()) as u32;
    let win_w = 700u32;
    let win_h = rows * row_h + 20;

    let mut canvas = Canvas::new(win_w, win_h);
    canvas.buffer.chunks_mut(4).for_each(|p| { p[0]=50; p[1]=30; p[2]=30; p[3]=255; });
    let green = Color::new(0,255,0,255);

    let mut y = 10u32;
    for &size in sizes.iter() {
        for &weight in weights.iter() {
            let label = format!("{:.0}px/{:.0} {}", size, weight, sample);
            draw_text(&label, font, 10, y, size, weight, &green, &mut canvas).unwrap();
            y += row_h;
        }
    }

    let mut out = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) { out.push(px[2]); out.push(px[1]); out.push(px[0]); }
    std::fs::write(concat!(env!("CARGO_TARGET_TMPDIR"), "/imgcheck/roboto_matrix.ppm"), out).unwrap();
}
