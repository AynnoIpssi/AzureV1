// Demonstration visuelle bout-en-bout : window_demo.rsh + window_demo.rsc
// sont parses, lies via `services::interpreter::build_ui` (rsC -> vrais
// UiNode, pas du texte a compiler), puis ouverts dans une vraie fenetre
// Wayland - chaque conteneur/bouton/image/video/texte y est visible,
// positionne et colore d'apres la feuille de style rsC.
//
// Lancer avec `cargo run --example run_window_demo` depuis azure-foundation/.
// Fermer la fenetre (bouton de fermeture) met fin au programme normalement.
use std::fs;

use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::window::models::window::AzureWindow;

fn main() {
    let rsh_src = fs::read_to_string("examples/window_demo.rsh").expect("lecture window_demo.rsh");
    let rsc_src = fs::read_to_string("examples/window_demo.rsc").expect("lecture window_demo.rsc");

    let rsc_sheet = parse_rsc(tokenize_rsc(&rsc_src)).unwrap_or_else(|err| {
        eprintln!("Erreur rsC: {err}");
        std::process::exit(1);
    });

    let ast = parse_rsh(tokenize_rsh(&rsh_src)).unwrap_or_else(|err| {
        eprintln!("Erreur rsH: {err}");
        std::process::exit(1);
    });

    let nodes = build_ui(&ast, &StyleSource::Rsc(&rsc_sheet));

    AzureWindow::new("rsH + rsC - demonstration")
        .size(1000, 700)
        .ui(nodes)
        .run();
}
