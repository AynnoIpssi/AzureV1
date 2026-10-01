// Captures hors fenetre de la demo `examples/shared_window_demo.rs` (meme
// `draw_ui` que la vraie fenetre, sans la barre d'en-tete) :
// target/tmp/shared-window-<nom>.ppm. La fenetre "note" passe par le vrai
// aller-retour encode -> decode de `SharedWindow`, comme chez l'app A.
use azure_core::models::window_model::{WindowKind, WindowScope, WindowSize, WindowSpec, WindowState};
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::window::models::shared_window::SharedWindow;

const RSC: &str = include_str!("../examples/shared_window/demo.rsc");

fn ui(rsh: &str, rsc: &str) -> Vec<UiNode> {
    let sheet = parse_rsc(tokenize_rsc(rsc)).expect("rsC");
    let ast = parse_rsh(tokenize_rsh(rsh)).expect("rsH");
    build_ui(&ast, &StyleSource::Rsc(&sheet))
}

fn save(nodes: &[UiNode], w: u32, h: u32, name: &str) {
    let mut canvas = Canvas::new(w, h);
    draw_ui(nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("shared-window-{name}.ppm"));
    std::fs::write(&path, ppm).unwrap();
    println!("{name}: {}", path.display());
}

#[test]
fn demo_windows_render_and_are_saved() {
    save(&ui(include_str!("../examples/shared_window/app_a.rsh"), RSC), 520, 224, "app-a");
    save(&ui(include_str!("../examples/shared_window/app_b.rsh"), RSC), 520, 224, "app-b");

    let spec = WindowSpec::new(2, WindowSize::new(460, 240).unwrap(), WindowState::Active, WindowScope::Followers, WindowKind::External).unwrap();
    let sent = SharedWindow::new(spec, "Fenêtre envoyée par B", include_str!("../examples/shared_window/note.rsh"), RSC);
    let received = SharedWindow::decode(&sent.encode().unwrap()).unwrap();
    assert_eq!(received, sent);
    save(&ui(&received.rsh, &received.rsc), 460, 204, "note");
}
