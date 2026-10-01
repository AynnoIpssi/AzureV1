// border-style (tirets, points, double, aucun) et ombre interieure, verifies
// sur les pixels reels.
use azure_engine::rendering::managers::renderer::draw_box;
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::{BorderStyle, BorderWidths, BoxStyle, Shadow};

const FILL: Color = Color::new(0, 0, 200, 255);
const BORDER: Color = Color::new(255, 0, 0, 255);

fn px(c: &Canvas, x: u32, y: u32) -> (u8, u8, u8) {
    let i = ((y * c.width + x) * 4) as usize;
    (c.buffer[i + 2], c.buffer[i + 1], c.buffer[i])
}

fn boxed(style: BorderStyle, width: f32) -> Canvas {
    let mut c = Canvas::new(200, 100);
    let mut s = BoxStyle::solid(FILL);
    s.border = BorderWidths::uniform(width);
    s.border_color = BORDER;
    s.border_style = style;
    draw_box(0, 0, 200, 100, &s, &mut c);
    c
}

/// Pixels rouges / bleus le long du haut de la bordure.
fn top_edge(c: &Canvas, y: u32) -> (usize, usize) {
    let row: Vec<_> = (10..190).map(|x| px(c, x, y)).collect();
    (row.iter().filter(|p| **p == (255, 0, 0)).count(), row.iter().filter(|p| **p == (0, 0, 200)).count())
}

#[test]
fn solid_dashed_dotted_double_and_none() {
    let (red, _) = top_edge(&boxed(BorderStyle::Solid, 4.0), 1);
    assert_eq!(red, 180, "trait plein : tout le haut est rouge");

    let (red, blue) = top_edge(&boxed(BorderStyle::Dashed, 4.0), 1);
    assert!(red > 60 && blue > 40, "tirets : alternance trait / fond ({red} rouges, {blue} bleus)");

    let (red, blue) = top_edge(&boxed(BorderStyle::Dotted, 6.0), 3);
    assert!(red > 30 && blue > 30, "points : alternance ({red} rouges, {blue} bleus)");

    let c = boxed(BorderStyle::Double, 9.0);
    assert_eq!(px(&c, 100, 1), (255, 0, 0), "double : trait exterieur");
    assert_eq!(px(&c, 100, 4), (0, 0, 200), "double : espace au milieu");
    assert_eq!(px(&c, 100, 7), (255, 0, 0), "double : trait interieur");

    let c = boxed(BorderStyle::None, 4.0);
    assert_eq!(px(&c, 100, 1), (0, 0, 200), "aucune : le fond va jusqu'au bord");
}

#[test]
fn inset_shadow_darkens_the_edges_only() {
    let mut c = Canvas::new(200, 100);
    let mut s = BoxStyle::solid(Color::new(200, 200, 200, 255));
    s.shadow = Some(Shadow { offset_x: 0.0, offset_y: 0.0, blur: 10.0, spread: 0.0, color: Color::new(0, 0, 0, 255), inset: true });
    draw_box(0, 0, 200, 100, &s, &mut c);
    let edge = px(&c, 100, 1).0;
    let center = px(&c, 100, 50).0;
    assert!(edge < 150, "bord assombri ({edge})");
    assert_eq!(center, 200, "centre intact");
    // Rien n'est peint autour de la boite.
    let mut c2 = Canvas::new(220, 120);
    draw_box(10, 10, 200, 100, &s, &mut c2);
    assert_eq!(px(&c2, 5, 60), (0, 0, 0), "une ombre interieure ne deborde pas");
    assert_eq!(c2.buffer[((60 * 220 + 5) * 4 + 3) as usize], c2.buffer[3], "pixel exterieur non touche");
}
