// Commentaires rsH `<!-- ... -->` (probleme #33) : ignores par le lexer,
// partout ou ils apparaissent, sans casser les fermetures `<!balise>`.
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::models::token::TokenRsH;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::ui::models::ui_node::UiNode;

fn texts(rsh: &str, ctx: &Context) -> Vec<String> {
    let ast = parse_rsh(tokenize_rsh(rsh)).expect("rsH valide");
    let sheet = parse_rsc(tokenize_rsc("")).unwrap();
    let mut out = Vec::new();
    collect(&build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), ctx), &mut out);
    out
}

fn collect(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => out.push(l.text.clone()),
            UiNode::Button(b) => out.push(format!("[{}]", b.text)),
            UiNode::Container(c) => collect(&c.children, out),
            _ => {}
        }
    }
}

#[test]
fn a_comment_produces_no_token() {
    let tokens = tokenize_rsh("<!-- barre du haut --><text>Salut<!text>");
    assert!(!tokens.iter().any(|t| matches!(&t.value, TokenRsH::RawText(s) if s.contains("barre"))), "{tokens:?}");
    assert!(matches!(tokens[0].value, TokenRsH::OpenTag));
}

#[test]
fn comments_between_and_inside_tags_are_ignored() {
    let rsh = "<container>
    <!-- Section 1 : l'en-tete -->
    <text>Bonjour <!-- pas affiche --> monde<!text>
    <!--
        Commentaire sur plusieurs lignes, avec <text>des balises<!text>
        et des tirets - -- dedans.
    -->
    <button#ok>Ok<!button>
<!container>";
    assert_eq!(texts(rsh, &Context::new()), vec!["Bonjour monde", "[Ok]"]);
}

#[test]
fn a_comment_between_if_and_else_keeps_the_chain() {
    let rsh = "<if.admin><text>admin<!text><!if>
<!-- sinon : -->
<else><text>visiteur<!text><!else>";
    assert_eq!(texts(rsh, &Context::new().with_bool("admin", false)), vec!["visiteur"]);
    assert_eq!(texts(rsh, &Context::new().with_bool("admin", true)), vec!["admin"]);
}

#[test]
fn positions_after_a_comment_stay_exact() {
    let tokens = tokenize_rsh("<!-- a\nb -->\n<text>x<!text>");
    let text = tokens.iter().find(|t| matches!(t.value, TokenRsH::OpenTag)).unwrap();
    assert_eq!((text.pos.line, text.pos.column), (3, 1));
}

#[test]
fn an_unclosed_comment_runs_to_the_end() {
    assert!(tokenize_rsh("<text>a<!text><!-- jamais ferme <text>b<!text>").iter().all(|t| !matches!(&t.value, TokenRsH::RawText(s) if s == "b")));
}
