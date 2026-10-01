// Regression : un if/elseif/else ecrit sur plusieurs lignes (indentation =
// RawText blanc entre les balises) doit rester UNE chaine - seule la
// premiere branche vraie s'affiche, jamais le else en plus.
use azure_foundation::compiler::rsh::mangers::parser::parse;
use azure_foundation::compiler::rsh::services::lexer::tokenize;
use azure_foundation::compiler::rsc::models::rule::RscStylesheet;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::ui::models::ui_node::UiNode;

const SRC: &str = "<container>\n    <if.est_admin>Admin<!if>\n    <elseif.est_invite>Invite<!elseif>\n    <else>Anonyme<!else>\n<!container>";

fn texts(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(label) => out.push(label.text.clone()),
            UiNode::Container(container) => texts(&container.children, out),
            _ => {}
        }
    }
}

fn shown(admin: bool, invite: bool) -> Vec<String> {
    let ast = parse(tokenize(SRC)).unwrap();
    let sheet = RscStylesheet::default();
    let ctx = Context::new().with_bool("est_admin", admin).with_bool("est_invite", invite);
    let mut out = Vec::new();
    texts(&build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &ctx), &mut out);
    out
}

#[test]
fn only_first_true_branch_is_shown() {
    assert_eq!(shown(true, false), ["Admin"]);
    assert_eq!(shown(false, true), ["Invite"]);
    assert_eq!(shown(false, false), ["Anonyme"]);
}
