use std::fs;

fn main() {
    let src = fs::read_to_string("examples/demo.rsh").expect("lecture demo.rsh");
    let style_src = fs::read_to_string("examples/demo.style").expect("lecture demo.style");

    // Demonstration de l'ancienne DSL `.style` (remplacee par rsC, voir
    // `run_demo_rsc`), gardee pour les pages qui l'utilisent encore.
    #[allow(deprecated)]
    let stylesheet = azure_foundation::style::services::stylesheet_parser::parse(&style_src)
        .unwrap_or_else(|err| {
            eprintln!("Erreur de style: {err}");
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
    let code = azure_foundation::compiler::services::codegen::generate(&ast, &stylesheet);
    println!("{code}");
}
