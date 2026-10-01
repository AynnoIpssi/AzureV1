// Bordures sur un seul cote (probleme #34) : `border-bottom`, `border-left`...
// en rsC, du parsing jusqu'aux pixels. Captures dans
// target/tmp/rsc_border_sides.ppm.
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::paint::BorderWidths;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::layout::managers::web_layout::layout_roots;
use azure_foundation::ui::models::container::Container;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;

const W: u32 = 200;
const H: u32 = 100;

fn build(rsh: &str, rsc: &str) -> Vec<UiNode> {
    let ast = parse_rsh(tokenize_rsh(rsh)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(rsc)).unwrap();
    build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new())
}

fn root(nodes: &[UiNode]) -> &Container {
    match &nodes[0] {
        UiNode::Container(c) => c,
        _ => panic!("conteneur attendu"),
    }
}

fn render(nodes: &[UiNode]) -> Canvas {
    let mut canvas = Canvas::new(W, H);
    draw_ui(nodes, (0, 0, W, H), &mut canvas, -1, -1, false);
    canvas
}

fn px(c: &Canvas, x: u32, y: u32) -> (u8, u8, u8) {
    let i = ((y * c.width + x) * 4) as usize;
    (c.buffer[i + 2], c.buffer[i + 1], c.buffer[i])
}

#[test]
fn each_side_is_read_from_rsc() {
    let nodes = build("<container.a><!container>", ".a { border-bottom: 3px solid #ff0000; border-left-width: 2px; }");
    let d = &root(&nodes).decoration;
    assert_eq!(d.border, BorderWidths { top: 0.0, right: 0.0, bottom: 3.0, left: 2.0 });
    assert_eq!((d.border_color.r, d.border_color.g, d.border_color.b), (255, 0, 0), "couleur prise sur border-bottom");
}

#[test]
fn a_side_wins_over_the_shorthand() {
    let nodes = build("<container.a><!container>", ".a { border-bottom: 4px solid #ffffff; border: 1px solid #00ff00; }");
    let d = &root(&nodes).decoration;
    assert_eq!(d.border, BorderWidths { top: 1.0, right: 1.0, bottom: 4.0, left: 1.0 });
    assert_eq!(d.border_color.g, 255, "border-color / border l'emporte pour la couleur");
}

#[test]
fn a_bottom_border_takes_room_only_at_the_bottom() {
    // Comme en CSS : la bordure s'ajoute a la hauteur du contenu.
    let nodes = build("<container.a><!container>", ".a { height: 20px; border-bottom: 5px solid #ffffff; }");
    let rects = layout_roots(&nodes, (0, 0, W, H));
    assert_eq!(rects[0], (0, 0, W, 25));
}

#[test]
fn only_the_bottom_line_is_painted() {
    let nodes = build(
        "<container.page><container.hr><!container><!container>",
        ".page { height: 100%; background-color: #000000; }
         .hr { height: 40px; background-color: #202020; border-bottom: 4px solid #ffffff; }",
    );
    let c = render(&nodes);
    save(&c, "rsc_border_sides");
    // Fond du separateur, sans bordure en haut ni sur les cotes.
    assert_eq!(px(&c, 0, 0), (32, 32, 32));
    assert_eq!(px(&c, 100, 20), (32, 32, 32));
    assert_eq!(px(&c, W - 1, 20), (32, 32, 32));
    // Les 4px de bordure : de y = 40 a 43.
    for y in 40..44 {
        assert_eq!(px(&c, 100, y), (255, 255, 255), "ligne {y}");
    }
    assert_eq!(px(&c, 100, 44), (0, 0, 0), "sous la boite : le fond de la page");
}

#[test]
fn a_left_border_is_a_vertical_bar() {
    let nodes = build(
        "<container.page><container.quote><!container><!container>",
        ".page { height: 100%; background-color: #000000; }
         .quote { height: 50px; background-color: #202020; border-left: 6px solid #3b5bdb; }",
    );
    let c = render(&nodes);
    for x in 0..6 {
        assert_eq!(px(&c, x, 25), (59, 91, 219), "colonne {x}");
    }
    assert_eq!(px(&c, 6, 25), (32, 32, 32));
    assert_eq!(px(&c, 100, 0), (32, 32, 32), "pas de bordure en haut");
}

#[test]
fn uniform_borders_still_draw_the_same() {
    // Meme rendu qu'avant pour une bordure identique des 4 cotes.
    let nodes = build(
        "<container.page><container.card><!container><!container>",
        ".page { height: 100%; background-color: #000000; }
         .card { height: 40px; background-color: #202020; border: 3px solid #ffffff; border-radius: 8px; }",
    );
    let c = render(&nodes);
    assert_eq!(px(&c, 100, 1), (255, 255, 255), "haut");
    assert_eq!(px(&c, 100, 44), (255, 255, 255), "bas");
    assert_eq!(px(&c, 1, 23), (255, 255, 255), "gauche");
    assert_eq!(px(&c, 100, 23), (32, 32, 32), "interieur");
    assert_eq!(px(&c, 0, 0), (0, 0, 0), "coin arrondi");
}

fn save(c: &Canvas, name: &str) {
    let mut ppm = format!("P6\n{} {}\n255\n", c.width, c.height).into_bytes();
    for p in c.buffer.chunks(4) {
        ppm.extend_from_slice(&[p[2], p[1], p[0]]);
    }
    std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.ppm")), ppm).unwrap();
}
