// Mise en page "web" d'un arbre rsH + rsC : unites px, flux normal,
// hauteur selon le contenu, retour a la ligne, flex, grid, marges auto,
// heritage, plusieurs classes, display: none. Capture de la page de
// demonstration dans target/tmp/web_layout.ppm.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::layout::managers::layout_manager::{layout_container, Rect};
use azure_foundation::layout::managers::web_layout::layout_roots;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;

const W: u32 = 800;
const H: u32 = 600;

fn build(rsh: &str, rsc: &str) -> Vec<UiNode> {
    let ast = parse_rsh(tokenize_rsh(rsh)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(rsc)).unwrap();
    build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new())
}

// Boites des enfants du nœud racine (lui-meme dans la fenetre W x H).
fn children_of(nodes: &[UiNode]) -> (Rect, Vec<Rect>) {
    let root = layout_roots(nodes, (0, 0, W, H))[0];
    let UiNode::Container(c) = &nodes[0] else { panic!() };
    (root, layout_container(&c.layout, &c.children, 0, root).children)
}

#[test]
fn block_flow_stacks_children_with_px_sizes_and_margins() {
    let nodes = build(
        "<container.page><container.a><!container><container.b><!container><!container>",
        ".page { padding: 20px; } .a { height: 50px; margin-bottom: 10px; } .b { height: 30px; width: 200px; margin: 0 auto; }",
    );
    let (root, kids) = children_of(&nodes);
    assert_eq!(root, (0, 0, W, 40 + 50 + 10 + 30), "hauteur = contenu + padding");
    assert_eq!(kids[0], (20, 20, W - 40, 50));
    assert_eq!(kids[1], (((W - 200) / 2) as i32, 80, 200, 30), "margin: 0 auto centre le bloc");
}

#[test]
fn text_wraps_and_grows_its_box() {
    let long = "Ceci est un long paragraphe qui doit revenir a la ligne plusieurs fois dans une colonne etroite.";
    let nodes = build(
        &format!("<container.col><text.p>{long}<!text><!container>"),
        ".col { width: 200px; } .p { font-size: 16px; line-height: 1.5; }",
    );
    let (_, kids) = children_of(&nodes);
    let (_, _, w, h) = kids[0];
    assert_eq!(w, 200);
    assert!(h >= 72 && h % 24 == 0, "plusieurs lignes de 24px : {h}");
}

#[test]
fn flex_row_with_grow_gap_and_auto_margin() {
    let nodes = build(
        "<container.bar><button.logo>Logo<!button><button.push>Docs<!button><button>API<!button><!container>",
        ".bar { display: flex; gap: 10px; padding: 0 16px; height: 60px; align-items: center; }
         .logo { width: 100px; } .push { margin-left: auto; }",
    );
    let (_, kids) = children_of(&nodes);
    assert_eq!(kids[0].0, 16);
    assert_eq!(kids[0].2, 100);
    let right_edge = kids[2].0 + kids[2].2 as i32;
    assert_eq!(right_edge, W as i32 - 16, "margin-left: auto pousse la suite a droite");
    assert!(kids[1].0 > 200, "le bouton pousse est a droite : {:?}", kids[1]);
    assert!(kids[0].3 < 60, "align-items: center : hauteur du contenu, pas de la barre");
}

#[test]
fn sidebar_layout_with_flex_grow_and_grid() {
    let nodes = build(
        "<container.shell><container.side><!container><container.main><container.grid>
            <container.cell><!container><container.cell><!container><container.cell><!container>
         <!container><!container><!container>",
        ".shell { display: flex; height: 100%; } .side { width: 240px; flex-shrink: 0; } .main { flex-grow: 1; padding: 24px; }
         .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; } .cell { height: 80px; }",
    );
    let (root, kids) = children_of(&nodes);
    assert_eq!(root.3, H, "height: 100% de la fenetre");
    assert_eq!(kids[0], (0, 0, 240, H), "barre laterale etiree sur toute la hauteur");
    assert_eq!(kids[1], (240, 0, W - 240, H));
    let UiNode::Container(shell) = &nodes[0] else { panic!() };
    let UiNode::Container(main) = &shell.children[1] else { panic!() };
    let grid = layout_container(&main.layout, &main.children, 0, kids[1]).children[0];
    let UiNode::Container(g) = &main.children[0] else { panic!() };
    let cells = layout_container(&g.layout, &g.children, 0, grid).children;
    let expected_w = ((W - 240 - 48) - 24) / 3;
    assert!(cells.iter().all(|c| c.2.abs_diff(expected_w) <= 1 && c.3 == 80), "{cells:?}");
    assert!((cells[1].0 - cells[0].0 - expected_w as i32 - 12).abs() <= 1, "gap de 12px entre les colonnes");
}

#[test]
fn inheritance_classes_and_display_none() {
    let nodes = build(
        "<container.card><text.note.warn>Attention<!text><text.hidden>Cache<!text><text>Herite<!text><!container>",
        ".card { color: #ff0000; font-size: 20px; } .note { font-size: 12px; } .warn { color: #00ff00; } .hidden { display: none; }",
    );
    let UiNode::Container(card) = &nodes[0] else { panic!() };
    let [UiNode::Label(note), _, UiNode::Label(inherited)] = &card.children[..] else { panic!() };
    assert_eq!((note.color.g, note.font_size), (255, 12.0), "deux classes appliquees");
    assert_eq!((inherited.color.r, inherited.font_size), (255, 20.0), "couleur et taille heritees du parent");
    let (_, kids) = children_of(&nodes);
    assert_eq!(kids[1].2 * kids[1].3, 0, "display: none ne prend aucune place");
}

#[test]
fn demo_page_renders() {
    let nodes = build(
        "<container.app>
            <container.nav><title3.brand>Azure<!title3><button.link>Guide<!button><button.link>API<!button><!container>
            <container.body>
                <title1>Bienvenue<!title1>
                <text.lead>Une page mise en page comme sur le web : flux normal, flexbox, texte qui revient a la ligne tout seul quand la colonne est trop etroite.<!text>
                <text.code>fn main() {
    println!(\"bonjour\");
}<!text>
            <!container>
         <!container>",
        ".app { height: 100%; background: linear-gradient(180deg, #10121f, #171a2e); color: #e6e8f2; }
         .nav { display: flex; align-items: center; gap: 8px; padding: 0 24px; height: 56px; border: 1px solid rgba(255,255,255,0.08); }
         .brand { margin-right: auto; color: #8ea2ff; }
         .link { padding: 6px 12px; border-radius: 8px; background-color: transparent; color: #aab0c8; }
         .link:hover { background-color: rgba(255,255,255,0.08); color: #ffffff; }
         .body { max-width: 560px; margin: 0 auto; padding: 32px 24px; }
         .lead { font-size: 16px; line-height: 1.6; color: #aab0c8; margin: 12px 0 20px 0; }
         .code { font-family: monospace; white-space: pre; font-size: 13px; line-height: 1.5; padding: 16px; border-radius: 10px; background-color: #0b0d17; }",
    );
    let mut canvas = Canvas::new(W, H);
    draw_ui(&nodes, (0, 0, W, H), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{W} {H}\n255\n").into_bytes();
    for p in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[p[2], p[1], p[0]]);
    }
    std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("web_layout.ppm"), ppm).unwrap();
}

// Barre du haut + corps en ligne (menu | page) qui prend le reste de la
// fenetre : la page, plus haute que la fenetre, doit s'etirer a la hauteur
// du corps (ligne flex unique) et defiler, pas prendre sa propre hauteur.
#[test]
fn row_flex_body_stretches_children_to_its_height_so_they_scroll() {
    let nodes = build(
        "<container.app><container.top><!container><container.body>
            <container.side><!container>
            <container.main><container.long><!container><!container>
         <!container><!container>",
        ".app { display: flex; flex-direction: column; height: 100%; } .top { height: 60px; flex-shrink: 0; }
         .body { display: flex; flex-grow: 1; min-height: 0; } .side { width: 200px; overflow-y: auto; }
         .main { flex-grow: 1; overflow-y: auto; } .long { height: 2000px; }",
    );
    let (_, kids) = children_of(&nodes);
    assert_eq!(kids[1], (0, 60, W, H - 60), "le corps prend le reste de la fenetre");
    let UiNode::Container(app) = &nodes[0] else { panic!() };
    let UiNode::Container(body) = &app.children[1] else { panic!() };
    let body_layout = layout_container(&body.layout, &body.children, 0, kids[1]);
    assert_eq!(body_layout.children[1].3, H - 60, "la page s'etire a la hauteur du corps");
    let UiNode::Container(main) = &body.children[1] else { panic!() };
    let main_layout = layout_container(&main.layout, &main.children, 0, body_layout.children[1]);
    assert_eq!(main_layout.max_scroll, 2000 - (H - 60), "la page defile sur tout son contenu");
}
