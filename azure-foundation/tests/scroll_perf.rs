// Cout d'une image pendant un defilement, sur une longue page ecrite en
// rsH + rsC (`overflow-y: auto`). Affiche les temps :
// `cargo test --release -p azure-foundation --test scroll_perf -- --nocapture`
// (ou sans `--release` pour le profil de dev, celui de `cargo run`).
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::{ConditionValue, Context};
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::interact::{animate_scroll, scroll_at};
use std::time::{Duration, Instant};

const W: u32 = 1280;
const H: u32 = 824;

const RSH: &str = "<container.page><for.l in lignes><text.ligne>Le rapide renard brun saute par-dessus le chien paresseux, encore et encore<!text><!for><!container>";
const RSC: &str = ".page { display: flex; flex-direction: column; overflow-y: auto; padding: 1%; background-color: #1e1e2e; }
.ligne { flex-basis: 3%; flex-shrink: 0; color: #f2f2f7; }";

fn page(lines: usize) -> Vec<UiNode> {
    let ast = parse_rsh(tokenize_rsh(RSH)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(RSC)).unwrap();
    let ctx = Context::new().with_list("lignes", vec![ConditionValue::Bool(true); lines]);
    build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &ctx)
}

// Molette vers le bas (un cran par image) + animation, comme dans la
// fenetre : renvoie le temps moyen d'une image.
fn scroll_frames(nodes: &mut [UiNode], frames: u32) -> Duration {
    let mut canvas = Canvas::new(W, H);
    let start = Instant::now();
    for _ in 0..frames {
        scroll_at(nodes, 600, 400, 15.0, (0, 0, W, H));
        animate_scroll(nodes);
        draw_ui(nodes, (0, 0, W, H), &mut canvas, -1, -1, false);
    }
    start.elapsed() / frames
}

#[test]
fn frame_cost_is_bounded_by_what_is_visible() {
    let mut small = page(400);
    let mut huge = page(20_000);
    let small_frame = scroll_frames(&mut small, 60);
    let huge_frame = scroll_frames(&mut huge, 60);
    println!("400 lignes    : {small_frame:?} par image");
    println!("20 000 lignes : {huge_frame:?} par image");
    let UiNode::Container(c) = &huge[0] else { panic!() };
    assert!(c.scroll_offset > 0, "la page doit avoir defile");
}
