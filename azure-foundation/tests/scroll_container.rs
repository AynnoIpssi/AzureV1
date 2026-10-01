// Defilement d'un conteneur pilote par rsC (`overflow` / `overflow-y`) :
// etendue mesuree a partir des enfants, rendu decale au pixel pres (y
// compris un element a cheval sur le haut de la fenetre), clics limites a la
// partie visible, animation vers la cible de la molette.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::layout::managers::layout_manager::layout_container;
use azure_foundation::layout::models::layout_props::Overflow;
use azure_foundation::ui::models::container::Container;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::interact::{animate_scroll, button_id_at, scroll_at};

const W: u32 = 400;
const H: u32 = 300;

const RSH: &str = "<container.page>
    <text.ligne>Premiere ligne<!text>
    <button.bouton#b1>Bouton 1<!button>
    <text.ligne>Troisieme<!text>
    <text.ligne>Quatrieme<!text>
    <text.ligne>Cinquieme<!text>
    <text.ligne>Sixieme<!text>
    <text.ligne>Septieme<!text>
    <text.ligne>Huitieme<!text>
    <text.ligne>Neuvieme<!text>
    <text.ligne>Dixieme<!text>
    <text.ligne>Onzieme<!text>
    <button.bouton#b12>Bouton 12<!button>
<!container>";

fn page(overflow: &str) -> Vec<UiNode> {
    let rsc = format!(
        ".page {{ display: flex; flex-direction: column; height: 100%; overflow-y: {overflow}; background-color: #1e1e2e; }}
         .ligne {{ height: 30px; flex-shrink: 0; color: #f2f2f7; font-size: 20px; }}
         .bouton {{ height: 30px; flex-shrink: 0; background-color: #7c9cff; }}"
    );
    let ast = parse_rsh(tokenize_rsh(RSH)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new())
}

fn container(nodes: &mut [UiNode]) -> &mut Container {
    match &mut nodes[0] {
        UiNode::Container(c) => c,
        _ => panic!("la racine doit etre un conteneur"),
    }
}

fn render(nodes: &[UiNode]) -> Canvas {
    let mut canvas = Canvas::new(W, H);
    draw_ui(nodes, (0, 0, W, H), &mut canvas, -1, -1, false);
    canvas
}

#[test]
fn rsc_overflow_drives_scrolling() {
    let mut auto = page("auto");
    assert_eq!(container(&mut auto).layout.overflow, Overflow::Auto);
    assert!(container(&mut auto).layout.scrollable());

    let mut hidden = page("hidden");
    assert_eq!(container(&mut hidden).layout.overflow, Overflow::Hidden);
    assert!(!container(&mut hidden).layout.scrollable());
    assert!(!scroll_at(&mut hidden, 200, 150, 15.0, (0, 0, W, H)), "overflow: hidden ne defile pas");
}

#[test]
fn scroll_extent_is_measured_from_children() {
    let mut nodes = page("auto");
    let c = container(&mut nodes);
    let layout = layout_container(&c.layout, &c.children, 0, (0, 0, W, H));
    // 12 enfants de 30px = 360px de contenu pour 300px visibles.
    assert_eq!(layout.content_height, 360);
    assert_eq!(layout.max_scroll, 60);
}

#[test]
fn scrolled_content_is_shifted_pixel_for_pixel() {
    let mut nodes = page("auto");
    let reference = render(&nodes);
    // 8px : la premiere ligne (30px de haut, a y = 0) passe a y = -8, a
    // cheval sur le bord haut - l'ancien calcul en u32 la recollait a y = 0.
    container(&mut nodes).scroll_offset = 8;
    let scrolled = render(&nodes);

    let row = |c: &Canvas, y: u32| c.buffer[(y * W * 4) as usize..((y * W + W - 20) * 4) as usize].to_vec();
    for y in 0..H - 8 {
        assert!(row(&scrolled, y) == row(&reference, y + 8), "ligne {y} : le contenu doit etre decale de 8px exactement");
    }
}

#[test]
fn clicks_only_reach_the_visible_part() {
    let mut nodes = page("auto");
    // Le conteneur est place sous une bande de 100px (comme une barre
    // d'onglets) : ses enfants defiles vers le haut passent SOUS cette bande,
    // encore dans la fenetre mais hors de sa partie visible.
    let root = (0, 100, W, H);
    assert_eq!(button_id_at(&nodes, 200, 145, root).as_deref(), Some("b1"));
    container(&mut nodes).scroll_offset = 60;
    // b1 est maintenant a y = 100 + 30 - 60 = 70..100 : dans la fenetre, mais
    // au-dessus du conteneur.
    assert_eq!(button_id_at(&nodes, 200, 85, root), None);
    // b12 (tout en bas du contenu) est devenu visible et cliquable.
    assert_eq!(button_id_at(&nodes, 200, 100 + 330 - 60 + 15, root).as_deref(), Some("b12"));
}

#[test]
fn wheel_moves_the_target_and_ticks_glide_to_it() {
    let mut nodes = page("auto");
    // (Valeur deja acceleree par le moteur pour une molette.)
    assert!(scroll_at(&mut nodes, 200, 150, 30.0, (0, 0, W, H)));
    assert_eq!(container(&mut nodes).scroll_target, 30);
    assert_eq!(container(&mut nodes).scroll_offset, 0, "rien ne bouge avant le tic");

    let mut ticks = 0;
    while animate_scroll(&mut nodes) {
        ticks += 1;
        assert!(ticks < 60, "l'animation doit converger");
    }
    assert_eq!(container(&mut nodes).scroll_offset, 30);

    // En butee : la cible est bornee a l'etendue mesuree, et un cran de plus
    // ne change plus rien.
    for _ in 0..10 {
        scroll_at(&mut nodes, 200, 150, 10.0, (0, 0, W, H));
    }
    assert_eq!(container(&mut nodes).scroll_target, 60);
    assert!(!scroll_at(&mut nodes, 200, 150, 10.0, (0, 0, W, H)));
}
