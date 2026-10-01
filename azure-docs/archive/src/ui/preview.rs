// Le coeur "vraiment live" de l'app : reutilise EXACTEMENT le pipeline
// public d'azure-foundation (voir azure-foundation/examples/run_window_demo.rs,
// la reference dont ce module s'inspire) pour interpreter un snippet rsH/rsC
// au moment de construire la fenetre - le `Vec<UiNode>` obtenu est le vrai
// resultat du compilateur, pas une maquette dessinee a la main.
use azure_engine::rendering::models::color::Color;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::layout::models::layout_props::LayoutProps;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::ui_node::UiNode;

// Feuille rsC minimale appliquee a TOUT apercu qui ne fournit pas la
// sienne (voir `Recipe::rsc`) - donne un rendu lisible aux tags rsH de base
// (container/title/text/button) sans que chaque recette rsH n'ait a
// redefinir un style complet juste pour etre presentable dans le panneau
// d'apercu.
const DEFAULT_RSC: &str = "\
container { display: flex; flex-direction: column; gap: 6px; padding: 4px; }\n\
title, title1, title2, title3 { color: #f2f2f7; }\n\
text { color: #f2f2f7; }\n\
button { background-color: #7c9cff; color: #14141f; height: 22%; }\n\
";

pub fn render_rsh(rsh_src: &str) -> Vec<UiNode> {
    render_with(rsh_src, DEFAULT_RSC, &Context::new())
}

pub fn render_rsh_ctx(rsh_src: &str, ctx: &Context) -> Vec<UiNode> {
    render_with(rsh_src, DEFAULT_RSC, ctx)
}

pub fn render_rsh_with_rsc(rsh_src: &str, rsc_src: &str) -> Vec<UiNode> {
    render_with(rsh_src, rsc_src, &Context::new())
}

fn render_with(rsh_src: &str, rsc_src: &str, ctx: &Context) -> Vec<UiNode> {
    let rsc_sheet = match parse_rsc(tokenize_rsc(rsc_src)) {
        Ok(sheet) => sheet,
        Err(err) => return error_node(&format!("Erreur rsC: {err}")),
    };
    let ast = match parse_rsh(tokenize_rsh(rsh_src)) {
        Ok(ast) => ast,
        Err(err) => return error_node(&format!("Erreur rsH: {err}")),
    };
    build_ui_with_context(&ast, &StyleSource::Rsc(&rsc_sheet), ctx)
}

fn error_node(message: &str) -> Vec<UiNode> {
    vec![UiNode::Label(Label::new(
        LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0),
        message.to_string(),
        Color::new(245, 166, 35, 255),
        13.0,
        400.0,
    ))]
}
