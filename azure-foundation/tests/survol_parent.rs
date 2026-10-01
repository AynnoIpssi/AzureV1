// `.bloc:hover .outil { opacity: 1 }` : l'outil n'apparait qu'au survol de
// son bloc (voir `Decoration::hover_group` / `group_hover_opacity`).
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;

const RSH: &str = "<container.page>
<container.bloc><container.outil><!container><!container>
<container.bloc><container.outil><!container><!container>
<!container>";
const RSC: &str = ".page { display: flex; flex-direction: column; width: 200px; height: 200px; background-color: #000000; }
.bloc { height: 100px; width: 200px; }
.outil { width: 20px; height: 20px; background-color: #ffffff; opacity: 0; }
.bloc:hover .outil { opacity: 1; }";

fn pixel(canvas: &Canvas, x: u32, y: u32) -> u8 {
    canvas.buffer[((y * canvas.width + x) * 4) as usize]
}

#[test]
fn outil_visible_au_survol_du_bloc() {
    let ast = parse_rsh(tokenize_rsh(RSH)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(RSC)).unwrap();
    let ui = build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new());
    let UiNode::Container(page) = &ui[0] else { panic!() };
    let UiNode::Container(bloc) = &page.children[0] else { panic!() };
    assert!(bloc.decoration.hover_group);
    assert!(!page.decoration.hover_group);
    assert_eq!(bloc.children[0].decoration().group_hover_opacity, Some(1.0));

    let draw = |x, y| {
        let mut canvas = Canvas::new(200, 200);
        draw_ui(&ui, (0, 0, 200, 200), &mut canvas, x, y, false);
        canvas
    };
    // Souris sur le 2e bloc : seul son outil se voit.
    let c = draw(150, 150);
    assert_eq!(pixel(&c, 5, 5), 0);
    assert_eq!(pixel(&c, 5, 105), 255);
    // Souris ailleurs : aucun.
    let c = draw(-1, -1);
    assert_eq!(pixel(&c, 5, 105), 0);
}
