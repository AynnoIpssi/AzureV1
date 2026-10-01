// Demonstration bout-en-bout du systeme de lien rsC/rsH : demo.rsc (vraie
// syntaxe CSS) est parse puis applique directement sur l'AST de demo.rsh
// via `generate_with_rsc`, sans passer par l'ancien systeme `.style`.
use std::fs;

fn main() {
    let src = fs::read_to_string("examples/demo.rsh").expect("lecture demo.rsh");
    let rsc_src = fs::read_to_string("examples/demo.rsc").expect("lecture demo.rsc");

    let rsc_tokens = azure_foundation::compiler::rsc::services::lexer::tokenize(&rsc_src);
    let rsc_sheet = azure_foundation::compiler::rsc::mangers::parser::parse(rsc_tokens).unwrap_or_else(|err| {
        eprintln!("Erreur rsC: {err}");
        std::process::exit(1);
    });

    let tokens = azure_foundation::compiler::rsh::services::lexer::tokenize(&src);
    let ast = match azure_foundation::compiler::rsh::mangers::parser::parse(tokens) {
        Ok(ast) => ast,
        Err(err) => {
            eprintln!("Erreur de parsing: {err}");
            std::process::exit(1);
        }
    };

    let code = azure_foundation::compiler::services::codegen::generate_with_rsc(&ast, &rsc_sheet);
    println!("{code}");
}
