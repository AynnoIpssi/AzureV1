// Degrades, transparence, coins arrondis, bordures, ombres et opacite
// ecrits en rsC : du parsing jusqu'aux pixels. Captures dans
// target/tmp/rsc_decoration.ppm.
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::paint::Fill;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::{generate_with_rsc, StyleSource};
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;

const W: u32 = 200;
const H: u32 = 100;

fn build(rsh: &str, rsc: &str) -> Vec<UiNode> {
    let ast = parse_rsh(tokenize_rsh(rsh)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(rsc)).unwrap();
    build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new())
}

fn render(nodes: &[UiNode]) -> Canvas {
    let mut canvas = Canvas::new(W, H);
    draw_ui(nodes, (0, 0, W, H), &mut canvas, -1, -1, false);
    canvas
}

// (r, g, b) du pixel (x, y) - le buffer est en B, G, R, A.
fn px(c: &Canvas, x: u32, y: u32) -> (u8, u8, u8) {
    let i = ((y * c.width + x) * 4) as usize;
    (c.buffer[i + 2], c.buffer[i + 1], c.buffer[i])
}

fn near(a: (u8, u8, u8), b: (u8, u8, u8)) -> bool {
    a.0.abs_diff(b.0) <= 3 && a.1.abs_diff(b.1) <= 3 && a.2.abs_diff(b.2) <= 3
}

fn root(nodes: &[UiNode]) -> &azure_foundation::ui::models::container::Container {
    match &nodes[0] {
        UiNode::Container(c) => c,
        _ => panic!(),
    }
}

#[test]
fn rsc_values_become_a_decoration() {
    let nodes = build(
        "<container.a><!container>",
        ".a { background: linear-gradient(to right, #ff0000, #0000ff 100%); border-radius: 12px;
              border: 2px solid rgba(255, 255, 255, 0.5); box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5); opacity: 80%; }",
    );
    let d = &root(&nodes).decoration;
    match &d.fill {
        Some(Fill::Linear { angle, stops }) => {
            assert!((angle - 90.0).abs() < 0.01, "to right = 90deg, recu {angle}");
            assert_eq!(stops.len(), 2);
            assert_eq!((stops[0].position, stops[1].position), (0.0, 1.0));
        }
        other => panic!("degrade attendu, recu {other:?}"),
    }
    assert_eq!(d.radius, 12.0);
    assert_eq!(d.border, azure_engine::rendering::models::paint::BorderWidths::uniform(2.0));
    assert_eq!(d.border_color.a, 128);
    let shadow = d.shadow.expect("ombre");
    assert_eq!((shadow.offset_x, shadow.offset_y, shadow.blur), (0.0, 4.0, 12.0));
    assert!((d.opacity - 0.8).abs() < 0.001);

    let radial = build("<container.b><!container>", ".b { background-image: radial-gradient(circle, #ffffff, #000000); }");
    assert!(matches!(root(&radial).decoration.fill, Some(Fill::Radial { .. })));
}

#[test]
fn gradient_runs_from_first_to_last_color() {
    let nodes = build("<container.a><!container>", ".a { height: 100%; background: linear-gradient(90deg, #ff0000 0%, #0000ff 100%); }");
    let c = render(&nodes);
    assert!(near(px(&c, 0, 50), (255, 0, 0)), "{:?}", px(&c, 0, 50));
    assert!(near(px(&c, W - 1, 50), (0, 0, 255)), "{:?}", px(&c, W - 1, 50));
    let mid = px(&c, W / 2, 50);
    assert!(mid.0.abs_diff(128) < 6 && mid.2.abs_diff(128) < 6, "milieu {mid:?}");
}

#[test]
fn translucent_background_blends_with_its_parent() {
    let nodes = build(
        "<container.parent><container.child><!container><!container>",
        ".parent { height: 100%; background-color: #000000; } .child { height: 100%; background-color: rgba(255, 255, 255, 0.5); }",
    );
    let c = render(&nodes);
    let p = px(&c, 100, 50);
    assert!(near(p, (128, 128, 128)), "blanc a 50% sur noir : {p:?}");
}

#[test]
fn rounded_corners_leave_the_parent_visible() {
    let nodes = build(
        "<container.parent><container.child><!container><!container>",
        ".parent { height: 100%; background-color: #000000; } .child { height: 100%; background-color: #ffffff; border-radius: 20px; }",
    );
    let c = render(&nodes);
    assert_eq!(px(&c, 0, 0), (0, 0, 0), "le coin reste au parent");
    assert_eq!(px(&c, 100, 50), (255, 255, 255));
    let edge = px(&c, 5, 5);
    assert!(edge.0 > 0 && edge.0 < 255 || edge.0 == 0, "bord anti-alias ou dehors : {edge:?}");
}

#[test]
fn opacity_fades_a_whole_subtree() {
    let nodes = build(
        "<container.parent><container.group><container.inner><!container><!container><!container>",
        ".parent { height: 100%; background-color: #000000; }
         .group { height: 100%; box-sizing: border-box; opacity: 0.5; background-color: #ff0000; padding: 10px; }
         .inner { height: 100%; background-color: #ffffff; }",
    );
    let c = render(&nodes);
    assert!(near(px(&c, 2, 2), (128, 0, 0)), "fond du groupe a 50% : {:?}", px(&c, 2, 2));
    assert!(near(px(&c, 100, 50), (128, 128, 128)), "enfant a 50% aussi : {:?}", px(&c, 100, 50));
}

#[test]
fn shadow_darkens_around_the_box() {
    let nodes = build(
        "<container.parent><container.card><!container><!container>",
        ".parent { background-color: #808080; padding: 20%; }
         .card { background-color: #ffffff; box-shadow: 0 6px 8px rgba(0, 0, 0, 0.8); }",
    );
    let c = render(&nodes);
    save(&c, "rsc_decoration");
    let below = px(&c, 100, 20 + 60 + 3);
    assert!(below.0 < 110, "sous la carte, l'ombre assombrit le gris : {below:?}");
    assert_eq!(px(&c, 100, 5), (128, 128, 128), "loin de la carte, rien ne change");
}

#[test]
fn codegen_emits_the_same_decoration() {
    let ast = parse_rsh(tokenize_rsh("<container.a><button.b>Ok<!button><!container>")).unwrap();
    let sheet = parse_rsc(tokenize_rsc(
        ".a { background: linear-gradient(135deg, #111111, #222222); border-radius: 8px; }
         .b { font-size: 20px; } .b:hover { background: linear-gradient(90deg, #333333, #444444); }",
    ))
    .unwrap();
    let code = generate_with_rsc(&ast, &sheet);
    assert!(code.contains("Fill::Linear { angle: 135.00"), "{code}");
    assert!(code.contains("radius: 8.0"), "{code}");
    assert!(code.contains("node.font_size = 20.0"), "{code}");
    assert!(code.contains("hover_fill: Some("), "le degrade :hover doit etre emis : {code}");
}

fn save(c: &Canvas, name: &str) {
    let mut ppm = format!("P6\n{} {}\n255\n", c.width, c.height).into_bytes();
    for p in c.buffer.chunks(4) {
        ppm.extend_from_slice(&[p[2], p[1], p[0]]);
    }
    std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.ppm")), ppm).unwrap();
}
