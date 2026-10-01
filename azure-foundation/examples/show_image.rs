// Ouvre une fenetre affichant l'image PNG referencee par examples/show_image.rsh
// (un simple `<image src="...">`) - demonstration du decodage PNG "aucune
// dependance" de azure-engine::codec::png, bout-en-bout via le pipeline
// rsH habituel (pas de UiNode construit a la main).
//
// Lancer avec `cargo run --example show_image` depuis azure-foundation/.
use std::fs;

use azure_foundation::compiler::rsc::models::rule::RscStylesheet;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::window::models::window::AzureWindow;

fn main() {
    let rsh_src = fs::read_to_string("examples/show_image.rsh").expect("lecture show_image.rsh");

    let ast = parse_rsh(tokenize_rsh(&rsh_src)).unwrap_or_else(|err| {
        eprintln!("Erreur rsH: {err}");
        std::process::exit(1);
    });

    // Aucune regle de style necessaire : <image> remplit deja 100% de son
    // parent par defaut (voir `compiler::services::codegen::base_media_style`).
    let sheet = RscStylesheet::default();
    let nodes = build_ui(&ast, &StyleSource::Rsc(&sheet));

    AzureWindow::new("Image PNG - demonstration")
        .size(400, 400)
        .ui(nodes)
        .run();
}
