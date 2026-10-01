#[cfg(test)]
mod tests {
    
    
    
    
    
    
    use azure_foundation::style::models::style::Style;
    use azure_foundation::style::models::stylesheet::Stylesheet;
    use azure_engine::rendering::models::color::Color;
    use azure_foundation::compiler::services::codegen::*;
    use azure_foundation::compiler::rsh::mangers::parser::parse;
    use azure_foundation::compiler::rsh::services::lexer::tokenize;

    fn generate_default(source: &str) -> String {
        let tokens = tokenize(source);
        let ast = parse(tokens).expect("parse ok");
        generate(&ast, &Stylesheet::new())
    }

    #[test]
    fn test_generate_container_with_title_and_text() {
        let code = generate_default(
            "<container><title>Salut<!title><text>Bienvenue<!text><!container>",
        );

        assert!(code.contains("pub fn build_ui() -> Vec<UiNode> {"));
        assert!(code.contains("UiNode::Container(Container::new("));
        assert!(code.contains("\"Salut\".to_string()"));
        assert!(code.contains("\"Bienvenue\".to_string()"));
    }

    #[test]
    fn test_generate_if_else_chain() {
        let code = generate_default("<if.flag>Oui<!if><else>Non<!else>");

        assert!(code.contains("if flag {"));
        assert!(code.contains("} else {"));
        // Une seule chaine if/else, pas deux blocs `if` independants.
        assert_eq!(code.matches("if flag").count(), 1);
    }

    #[test]
    fn test_generate_button_uses_children_text() {
        let code = generate_default("<button>Clique<!button>");

        assert!(code.contains("UiNode::Button("));
        assert!(code.contains("Button::new("));
        assert!(code.contains("\"Clique\".to_string()"));
    }

    #[test]
    fn test_generate_for_loop_emits_real_rust_for() {
        let code = generate_default("<for.item in liste>Salut<!for>");

        assert!(code.contains("for item in liste {"));
        assert!(!code.contains("TODO"));
    }

    #[test]
    fn test_generate_match_emits_a_real_rust_match_with_each_arm_pattern() {
        let code = generate_default("<match.role><arm.\"admin\">Admin<!arm><arm._>Autre<!arm><!match>");

        assert!(code.contains("match role {"));
        assert!(code.contains("\"admin\" => {"));
        assert!(code.contains("_ => {"));
        assert!(code.contains("\"Admin\".to_string()"));
        assert!(code.contains("\"Autre\".to_string()"));
        assert!(!code.contains("TODO"));
    }

    #[test]
    fn test_generate_match_without_any_arm_falls_back_to_unconditional_content() {
        let code = generate_default("<match.role>Salut<!match>");

        assert!(!code.contains("match role {"));
        assert!(code.contains("ATTENTION: match sans aucun arm"));
        assert!(code.contains("\"Salut\".to_string()"));
    }

    #[test]
    fn test_generate_applies_named_style_component() {
        let tokens = tokenize("<container.card>Salut<!container>");
        let ast = parse(tokens).expect("parse ok");

        let mut stylesheet = Stylesheet::new();
        stylesheet.define(
            "card",
            Style { width: Some(40.0), background: Some(Color::new(10, 20, 30, 255)), ..Style::default() },
        );

        let code = generate(&ast, &stylesheet);

        assert!(code.contains("LayoutProps::new(0.0, 0.0, 40.0, 100.0, 0.0, 0.0)"));
        assert!(code.contains("Color::new(10, 20, 30, 255)"));
    }

    #[test]
    fn test_generate_unknown_class_falls_back_to_defaults() {
        let code = generate_default("<container.nope>Salut<!container>");
        assert!(code.contains("LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0)"));
    }

    #[test]
    fn test_generated_code_compiles() {
        // Verification "boite noire" : le code genere doit au moins etre
        // syntaxiquement plausible (equilibre des accolades/parentheses).
        let code = generate_default(
            "<container><title.hero>Titre<!title><if.userisadmin><button>Admin<!button><!if><!container>",
        );

        let open_braces = code.matches('{').count();
        let close_braces = code.matches('}').count();
        assert_eq!(open_braces, close_braces);

        let open_parens = code.matches('(').count();
        let close_parens = code.matches(')').count();
        assert_eq!(open_parens, close_parens);
    }

    #[test]
    fn test_generate_with_rsc_resolves_by_real_selector() {
        let tokens = tokenize("<container.card><button>Ok<!button><!container>");
        let ast = parse(tokens).expect("parse ok");

        let rsc_tokens = azure_foundation::compiler::rsc::services::lexer::tokenize(".card button { background-color: #ff0000; }");
        let rsc_sheet = azure_foundation::compiler::rsc::mangers::parser::parse(rsc_tokens).expect("rsc valide");

        let code = generate_with_rsc(&ast, &rsc_sheet);
        assert!(code.contains("Color::new(255, 0, 0, 255)"));
    }

    #[test]
    fn test_generate_with_rsc_descendant_selector_does_not_leak_to_siblings() {
        let tokens = tokenize("<container.card><button>Ok<!button><!container><button>Autre<!button>");
        let ast = parse(tokens).expect("parse ok");

        let rsc_tokens = azure_foundation::compiler::rsc::services::lexer::tokenize(".card button { background-color: #ff0000; }");
        let rsc_sheet = azure_foundation::compiler::rsc::mangers::parser::parse(rsc_tokens).expect("rsc valide");

        let code = generate_with_rsc(&ast, &rsc_sheet);
        // Le second bouton (hors de .card) doit garder la couleur par defaut.
        assert_eq!(code.matches("Color::new(255, 0, 0, 255)").count(), 1);
    }

    #[test]
    fn test_generate_with_rsc_emits_flex_properties_as_a_block_expression() {
        let tokens = tokenize("<container.row>Salut<!container>");
        let ast = parse(tokens).expect("parse ok");

        let rsc_tokens = azure_foundation::compiler::rsc::services::lexer::tokenize(
            ".row { display: flex; flex-direction: row; justify-content: space-between; gap: 5%; }",
        );
        let rsc_sheet = azure_foundation::compiler::rsc::mangers::parser::parse(rsc_tokens).expect("rsc valide");

        let code = generate_with_rsc(&ast, &rsc_sheet);
        assert!(code.contains("layout.display = azure_foundation::layout::models::layout_props::DisplayMode::Flex;"));
        assert!(code.contains("layout.justify_content = azure_foundation::layout::models::layout_props::JustifyContent::SpaceBetween;"));
        assert!(code.contains("layout.gap = 5.0;"));
    }

    #[test]
    fn test_generate_with_rsc_emits_grid_template_columns_as_a_track_vec() {
        let tokens = tokenize("<container.grid>Salut<!container>");
        let ast = parse(tokens).expect("parse ok");

        let rsc_tokens = azure_foundation::compiler::rsc::services::lexer::tokenize(".grid { display: grid; grid-template-columns: 1fr 2fr 20%; }");
        let rsc_sheet = azure_foundation::compiler::rsc::mangers::parser::parse(rsc_tokens).expect("rsc valide");

        let code = generate_with_rsc(&ast, &rsc_sheet);
        assert!(code.contains(
            "layout.grid_template_columns = vec![azure_foundation::layout::models::layout_props::Track::Fr(1.0), \
             azure_foundation::layout::models::layout_props::Track::Fr(2.0), \
             azure_foundation::layout::models::layout_props::Track::Percent(20.0)];"
        ));
    }

    #[test]
    fn test_generate_without_flex_or_grid_properties_still_emits_a_plain_call() {
        // Zero churn sur le code genere existant : sans aucune propriete
        // flex/grid, on retombe sur le simple appel `LayoutProps::new(...)`,
        // jamais l'expression bloc.
        let code = generate_default("<container>Salut<!container>");
        assert!(code.contains("LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0)"));
        assert!(!code.contains("let mut layout ="));
    }
}
