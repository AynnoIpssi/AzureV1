// `accent-color` s'herite comme en CSS : pose sur un conteneur, il colore
// tous ses champs ; une regle sur le champ lui-meme passe devant.
use azure_foundation::compiler::components::with_default_styles;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::ui::models::control::DEFAULT_ACCENT;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_engine::rendering::models::color::Color;

fn accents(rsh: &str, rsc: &str) -> Vec<Color> {
    let sheet = parse_rsc(tokenize_rsc(&with_default_styles(rsc, None))).unwrap();
    let nodes = build_ui(&parse_rsh(tokenize_rsh(rsh)).unwrap(), &StyleSource::Rsc(&sheet));
    fn collect(nodes: &[UiNode], out: &mut Vec<Color>) {
        for n in nodes {
            match n {
                UiNode::Control(c) => out.push(c.accent),
                UiNode::Container(c) => collect(&c.children, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    collect(&nodes, &mut out);
    out
}

#[test]
fn le_conteneur_colore_ses_champs() {
    let sable = Color::new(0xc9, 0xa8, 0x78, 255);
    let vert = Color::new(0x9b, 0xb0, 0x8f, 255);
    let rsh = "<container.formulaire><container><checkbox>A<!checkbox><!container><slider/><radio.autre name=\"g\" value=\"x\">X<!radio><!container><checkbox>hors<!checkbox>";
    let rsc = ".formulaire { accent-color: #c9a878; } .autre { accent-color: #9bb08f; }";
    assert_eq!(accents(rsh, rsc), vec![sable, sable, vert, DEFAULT_ACCENT]);
}
