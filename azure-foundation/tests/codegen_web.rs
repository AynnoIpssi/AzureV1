// Le code Rust genere (codegen) pour une page rsH + rsC "web" doit compiler
// et donner exactement le meme rendu que l'interpreteur. Le code genere est
// garde dans tests/generated/web_page.rs (inclus ci-dessous) ; s'il ne
// correspond plus a ce que produit le codegen, le test le reecrit et echoue :
// relancer le test suffit alors.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::{generate_with_rsc, StyleSource};
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::ui::services::draw_ui::draw_ui;

mod generated {
    include!("generated/web_page.rs");
}

const RSH: &str = "<container.app>
    <container.nav><title3.brand>Azure<!title3><button.link#guide>Guide<!button><button.link>API<!button><button.badge>3<!button><!container>
    <container.body>
        <title1>Bienvenue<!title1>
        <text.lead.muted>Une page mise en page comme sur le web, avec du texte qui revient a la ligne.<!text>
        <container.grid><container.card><text>Un<!text><!container><container.card><text>Deux<!text><!container><!container>
        <text.code>let x = 1;
    let y = 2;<!text>
        <text.cache>invisible<!text>
    <!container>
    <container.toast><text>Enregistre<!text><!container>
<!container>";

const RSC: &str = ".app { height: 100%; background: linear-gradient(180deg, #10121f, #171a2e); color: #e6e8f2; }
.nav { display: flex; align-items: center; gap: 8px; padding: 0 24px; height: 56px; flex-wrap: wrap; align-content: center; }
.brand { margin-right: auto; font-style: italic; letter-spacing: 1px; text-decoration: underline; }
.link { padding: 6px 12px; border-radius: 8px; background-color: transparent; text-align: left; border: 1px dashed #445566; transition: background-color 150ms ease; cursor: pointer; }
.badge { position: absolute; top: 4px; right: 8px; padding: 2px 6px; border-radius: 9px; background: #e05555; }
.cache { visibility: hidden; }
.toast { position: fixed; bottom: 12px; right: 12px; z-index: 5; padding: 10px; background: #2a3050; box-shadow: inset 0 1px 3px #000000; }
.link:hover { color: #ffffff; }
.body { max-width: 520px; margin: 0 auto; padding: 32px 24px; }
.lead { line-height: 1.6; margin: 12px 0; } .muted { color: #aab0c8; }
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 16px; overflow-x: auto; }
.card { padding: 16px; border-radius: 10px; background-color: rgba(255, 255, 255, 0.06); position: relative; top: 2px; }
.code { font-family: monospace; white-space: pre; padding: 12px; background-color: #0b0d17; }";

fn render(nodes: &[azure_foundation::ui::models::ui_node::UiNode]) -> Vec<u8> {
    let mut canvas = Canvas::new(700, 500);
    draw_ui(nodes, (0, 0, 700, 500), &mut canvas, -1, -1, false);
    canvas.buffer
}

#[test]
fn generated_code_matches_the_interpreter() {
    let ast = parse_rsh(tokenize_rsh(RSH)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(RSC)).unwrap();

    let code = generate_with_rsc(&ast, &sheet);
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/generated/web_page.rs");
    if std::fs::read_to_string(path).ok().as_deref() != Some(code.as_str()) {
        std::fs::write(path, &code).unwrap();
        panic!("tests/generated/web_page.rs vient d'etre regenere : relancer le test");
    }

    let interpreted = build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new());
    let (a, b) = (render(&generated::build_ui()), render(&interpreted));
    let diff: Vec<(usize, usize)> = a.chunks(4).zip(b.chunks(4)).enumerate().filter(|(_, (p, q))| p != q).map(|(i, _)| (i % 700, i / 700)).collect();
    assert!(diff.is_empty(), "le code genere doit dessiner exactement la meme page : {} pixels differents, premiers {:?}", diff.len(), &diff[..diff.len().min(5)]);
}

fn describe(nodes: &[azure_foundation::ui::models::ui_node::UiNode], out: &mut Vec<String>) {
    use azure_foundation::ui::models::ui_node::UiNode;
    for n in nodes {
        let css = format!(
            "{:?} {:?}",
            n.layout().css.as_ref().map(|c| (c.display, c.width, c.height, c.margin, c.padding, c.border, c.flex_grow, c.align_items, c.justify_content)),
            n.layout().css.as_ref().map(|c| (c.fixed, c.position, c.inset, c.z_index, c.align_content, n.layout().overflow_x))
        );
        // Ce que l'inspecteur (F12) sait d'un element n'existe que cote
        // interpreteur : hors de la comparaison.
        let mut deco = n.decoration().clone();
        deco.inspect = None;
        let deco = format!("{deco:?}");
        match n {
            UiNode::Label(l) => out.push(format!("L {:?} {:?} {} {} {:?} {css} {deco}", l.text, l.color, l.font_size, l.weight, l.text_style.as_ref().map(|t| (t.font, t.line_height, t.align, t.white_space, t.options, t.decoration)))),
            UiNode::Button(b) => out.push(format!("B {:?} {:?} {:?} {} {:?} {:?} {css} {deco}", b.text, b.color, b.text_color, b.font_size, b.hover_text_color, b.text_style.as_ref().map(|t| (t.line_height, t.align, t.white_space)))),
            UiNode::Container(c) => {
                out.push(format!("C {:?} {css} {deco}", c.background));
                describe(&c.children, out);
            }
            _ => out.push("?".into()),
        }
    }
}

#[test]
fn generated_and_interpreted_trees_are_identical() {
    let ast = parse_rsh(tokenize_rsh(RSH)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(RSC)).unwrap();
    let interpreted = build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new());
    let (mut a, mut b) = (Vec::new(), Vec::new());
    describe(&generated::build_ui(), &mut a);
    describe(&interpreted, &mut b);
    for (x, y) in a.iter().zip(&b) {
        if x != y {
            println!("GEN {x}\nINT {y}\n");
        }
    }
    assert_eq!(a, b, "codegen et interpreteur construisent les memes widgets");
}
