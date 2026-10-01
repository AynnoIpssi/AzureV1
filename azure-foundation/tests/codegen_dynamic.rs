// Code genere pour une page avec composants, champs et `{{...}}` : les
// parties dynamiques sont construites a l'execution par l'interpreteur, le
// resultat doit etre identique pixel pour pixel, avec les memes donnees.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::{generate_with_rsc, StyleSource};
use azure_foundation::compiler::services::condition::{ConditionValue, Context};
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::ui::services::draw_ui::draw_ui;

mod generated {
    include!("generated/dynamic_page.rs");
}

const RSH: &str = "<container.page>
    <title2>Bonjour {{nom}}<!title2>
    <text.muted>Statique, genere en Rust.<!text>
    <badge.ok>{{compte}} nouveaux<!badge>
    <container.liste>
        <for.app in apps><text.item#app-{{app_index}}>{{app}}<!text><!for>
    <!container>
    <if.admin == true><btn.primary#gerer>Gerer<!btn><!if>
    <checkbox#notif checked=\"true\">Notifications<!checkbox>
    <select#pays options=\"France, Belgique, Suisse\" value=\"Belgique\"/>
    <text>Fin<!text>
<!container>";

const RSC: &str = ".page { padding: 20px; background: #14172a; color: #e6e8f2; }
.muted { color: #9aa0b8; } .liste { display: flex; gap: 8px; margin: 8px 0; } .item { padding: 4px 8px; background: #22263d; }
.az-badge { padding: 2px 8px; border-radius: 9px; background: #2d6a4f; } .az-btn { padding: 8px 14px; background: #4353ff; }";

fn context() -> Context {
    Context::new()
        .with_text("nom", "Ana")
        .with_text("compte", "3")
        .with_bool("admin", true)
        .with_value("apps", ConditionValue::List(["notes", "photos", "musique"].iter().map(|s| ConditionValue::from(*s)).collect()))
}

fn render(nodes: &[azure_foundation::ui::models::ui_node::UiNode]) -> Vec<u8> {
    let mut canvas = Canvas::new(600, 400);
    draw_ui(nodes, (0, 0, 600, 400), &mut canvas, -1, -1, false);
    canvas.buffer
}

#[test]
fn dynamic_parts_match_the_interpreter() {
    let ast = parse_rsh(tokenize_rsh(RSH)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(RSC)).unwrap();
    let code = generate_with_rsc(&ast, &sheet);
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/generated/dynamic_page.rs");
    if std::fs::read_to_string(path).ok().as_deref() != Some(code.as_str()) {
        std::fs::write(path, &code).unwrap();
        panic!("tests/generated/dynamic_page.rs vient d'etre regenere : relancer le test");
    }
    assert!(code.contains("build_nodes"), "des parties construites a l'execution");
    assert!(code.contains("Label::new") || code.contains("UiNode::Label"), "et des parties generees en Rust");

    let ctx = context();
    let interpreted = build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &ctx);
    let (a, b) = (render(&generated::build_ui_with(&ctx)), render(&interpreted));
    let diff = a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count();
    assert_eq!(diff, 0, "le code genere doit dessiner exactement la meme page");
    // Les donnees comptent vraiment : sans elles, autre chose.
    assert_ne!(render(&generated::build_ui()), a);
}
