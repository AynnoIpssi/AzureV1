#[cfg(test)]
mod tests {
    
    use azure_foundation::compiler::rsc::models::rule::RscStylesheet;
    use azure_foundation::compiler::rsc::models::selector::Combinator;
    
    use azure_foundation::compiler::rsc::models::value::{Unit, Value};
    use azure_foundation::compiler::rsc::mangers::parser::*;
    use azure_foundation::compiler::rsc::services::lexer::tokenize;
    use azure_engine::rendering::models::color::Color;

    fn parse_src(src: &str) -> RscStylesheet {
        parse(tokenize(src)).expect("devrait parser")
    }

    #[test]
    fn parses_type_class_id_compound_selector() {
        let sheet = parse_src("container.card#main { width: 10px; }");
        let selector = &sheet.rules[0].selectors[0].parts[0].0;
        assert_eq!(selector.tag, Some("container".to_string()));
        assert_eq!(selector.classes, vec!["card".to_string()]);
        assert_eq!(selector.id, Some("main".to_string()));
    }

    #[test]
    fn parses_descendant_and_child_combinators() {
        let sheet = parse_src(".card button { color: white; } .card > title { color: red; }");

        let descendant = &sheet.rules[0].selectors[0].parts;
        assert_eq!(descendant.len(), 2);
        assert_eq!(descendant[1].1, Some(Combinator::Descendant));

        let child = &sheet.rules[1].selectors[0].parts;
        assert_eq!(child.len(), 2);
        assert_eq!(child[1].1, Some(Combinator::Child));
    }

    #[test]
    fn parses_comma_separated_selector_list() {
        let sheet = parse_src("title1, title2, title3 { font-weight: 700; }");
        assert_eq!(sheet.rules[0].selectors.len(), 3);
    }

    #[test]
    fn unsupported_pseudo_class_is_parsed_but_never_matches() {
        // Ne doit pas faire echouer le parsing - voir SimpleSelector::pseudo_classes.
        let sheet = parse_src("button:hover { color: blue; }");
        assert_eq!(sheet.rules[0].selectors[0].parts[0].0.pseudo_classes, vec!["hover".to_string()]);
    }

    #[test]
    fn parses_dimension_percentage_and_number_values() {
        let sheet = parse_src(".hero { width: 90%; padding: 8px; flex-grow: 1; }");
        let decls = &sheet.rules[0].declarations;
        assert_eq!(decls[0].value, Value::Length(90.0, Unit::Percent));
        assert_eq!(decls[1].value, Value::Length(8.0, Unit::Px));
        assert_eq!(decls[2].value, Value::Number(1.0));
    }

    #[test]
    fn parses_hex_named_and_rgba_colors() {
        let sheet = parse_src(".a { color: #ff0000; } .b { color: red; } .c { color: rgba(0, 128, 255, 0.5); }");
        assert_eq!(sheet.rules[0].declarations[0].value, Value::Color(Color::new(255, 0, 0, 255)));
        assert_eq!(sheet.rules[1].declarations[0].value, Value::Color(Color::new(255, 0, 0, 255)));
        assert_eq!(sheet.rules[2].declarations[0].value, Value::Color(Color::new(0, 128, 255, 128)));
    }

    #[test]
    fn parses_space_separated_shorthand_as_a_list() {
        let sheet = parse_src(".a { margin: 10px 20px; }");
        assert_eq!(
            sheet.rules[0].declarations[0].value,
            Value::List(vec![Value::Length(10.0, Unit::Px), Value::Length(20.0, Unit::Px)])
        );
    }

    #[test]
    fn parses_important_flag() {
        let sheet = parse_src(".a { color: red !important; }");
        assert!(sheet.rules[0].declarations[0].important);
    }

    #[test]
    fn unknown_property_name_is_kept_not_rejected() {
        // Contrairement a l'ancien parser `.style`, une propriete inconnue
        // ne doit PAS faire echouer le parsing - comme en CSS reel.
        let sheet = parse_src(".a { transform: rotate(3deg); }");
        assert_eq!(sheet.rules[0].declarations[0].name, "transform");
    }

    #[test]
    fn ignores_block_comments_between_declarations() {
        let sheet = parse_src(".a {\n /* commentaire */ \n width: 1px;\n}");
        assert_eq!(sheet.rules[0].declarations.len(), 1);
    }

    #[test]
    fn missing_open_brace_is_an_error() {
        let err = parse(tokenize(".card")).expect_err("devrait echouer");
        assert!(matches!(err.kind, ParseErrorKind::ExpectedOpenBrace(_)));
    }

    #[test]
    fn unclosed_block_is_an_error() {
        let err = parse(tokenize(".card { width: 1px;")).expect_err("devrait echouer");
        assert_eq!(err.kind, ParseErrorKind::UnclosedBlock);
    }

    #[test]
    fn invalid_hex_color_skips_only_that_declaration() {
        // Comme un navigateur : la declaration est sautee (et signalee), le
        // reste de la feuille est garde.
        let sheet = parse(tokenize(".card { color: #zz; width: 10px; }")).expect("la feuille reste lisible");
        let names: Vec<&str> = sheet.rules[0].declarations.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["width"]);
        assert!(sheet.skipped[0].contains("InvalidColor"), "{:?}", sheet.skipped);
    }
}
