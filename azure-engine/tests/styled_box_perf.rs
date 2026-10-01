// Cout de `draw_box` selon la decoration, sur une carte de 1200 x 350 dans
// une fenetre 1280 x 824 (`-- --nocapture` pour voir les temps).
use azure_engine::rendering::managers::renderer::draw_box;
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::{BoxStyle, ColorStop, Fill, Shadow};
use std::time::Instant;

fn time(name: &str, style: &BoxStyle) {
    let mut canvas = Canvas::new(1280, 824);
    let n = 20;
    let start = Instant::now();
    for _ in 0..n {
        draw_box(40, 200, 1200, 350, style, &mut canvas);
    }
    println!("{name:<28} {:?}", start.elapsed() / n);
}

#[test]
fn draw_box_cost_by_decoration() {
    let stops = vec![
        ColorStop { color: Color::new(255, 255, 255, 18), position: 0.0 },
        ColorStop { color: Color::new(255, 255, 255, 6), position: 1.0 },
    ];
    let card = BoxStyle {
        fill: Fill::Linear { angle: 180.0, stops: stops.clone() },
        radius: 18.0,
        border: azure_engine::rendering::models::paint::BorderWidths::uniform(1.0),
        border_color: Color::new(255, 255, 255, 23),
        border_style: azure_engine::rendering::models::paint::BorderStyle::Solid,
        shadow: None,
    };
    time("uni opaque", &BoxStyle::solid(Color::new(30, 30, 46, 255)));
    time("uni translucide", &BoxStyle::solid(Color::new(255, 255, 255, 30)));
    time("degrade + coins + bordure", &card);
    time("... + ombre 28px", &BoxStyle { shadow: Some(Shadow { offset_x: 0.0, offset_y: 10.0, blur: 28.0, spread: 0.0, color: Color::new(0, 0, 0, 115), inset: false }), ..card.clone() });
    time("radial", &BoxStyle { fill: Fill::Radial { stops }, ..BoxStyle::solid(Color::new(0, 0, 0, 0)) });
}
