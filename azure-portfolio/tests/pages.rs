// Chaque page dessinee hors fenetre dans target/tmp/portfolio-<page>.ppm :
// le rsH et le rsC se lisent, et le menu mene a toutes les pages.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_portfolio::{routes, target, PAGES};
use std::path::Path;

fn buttons(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Button(b) => out.push(b.id.clone()),
            UiNode::Container(c) => buttons(&c.children, out),
            _ => {}
        }
    }
}

#[test]
fn every_page_renders() {
    let table = routes(RouteTable::new(), &Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"));
    for (path, _, name) in PAGES {
        let nodes = table.resolve(&Route::new(path, "")).unwrap();
        assert!(!nodes.is_empty(), "page {path} vide (rsH ou rsC invalide ?)");
        let (w, h) = (1200, 4600);
        let mut canvas = Canvas::new(w, h);
        draw_ui(&nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        for px in canvas.buffer.chunks(4) {
            ppm.extend_from_slice(&[px[2], px[1], px[0]]);
        }
        std::fs::write(Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("portfolio-{name}.ppm")), ppm).unwrap();

        let mut found = Vec::new();
        buttons(&nodes, &mut found);
        for (_, _, other) in PAGES {
            let id = format!("nav-{other}");
            assert!(found.contains(&id), "{path} : pas de #{id}");
            assert!(target(&id).is_some());
        }
    }
}
