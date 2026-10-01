#[cfg(test)]
mod tests {
    use azure_foundation::compiler::rsc::models::token::TokenRsC;
    use azure_foundation::compiler::rsc::services::lexer::*;

    #[test]
    fn tokenizes_type_class_id_selector() {
        let tokens = tokenize("container.card#main {}");
        assert_eq!(
            tokens.iter().map(|s| s.value.clone()).collect::<Vec<_>>(),
            vec![
                TokenRsC::Ident("container".to_string()),
                TokenRsC::ClassSel("card".to_string()),
                TokenRsC::Hash("main".to_string()),
                TokenRsC::Combinator,
                TokenRsC::OpenBrace,
                TokenRsC::CloseBrace,
            ]
        );
    }

    #[test]
    fn dot_before_digit_is_a_decimal_point_not_a_class_selector() {
        let tokens = tokenize("opacity: .5;");
        assert!(tokens.iter().any(|s| s.value == TokenRsC::Number(0.5)));
    }

    #[test]
    fn dimension_and_percentage_are_distinguished_from_bare_numbers() {
        let tokens = tokenize("10px 50% 3");
        assert_eq!(
            tokens
                .iter()
                .map(|s| s.value.clone())
                .filter(|t| *t != TokenRsC::Combinator)
                .collect::<Vec<_>>(),
            vec![
                TokenRsC::Dimension(10.0, "px".to_string()),
                TokenRsC::Percentage(50.0),
                TokenRsC::Number(3.0),
            ]
        );
    }

    #[test]
    fn negative_numbers_are_read_as_a_single_token() {
        let tokens = tokenize("margin-top: -10px;");
        assert!(tokens.iter().any(|s| s.value == TokenRsC::Dimension(-10.0, "px".to_string())));
    }

    #[test]
    fn block_and_line_comments_are_skipped() {
        let tokens = tokenize("/* commentaire */ width: 1px; // fin de ligne\nheight: 2px;");
        let idents: Vec<_> = tokens
            .iter()
            .filter_map(|s| match &s.value {
                TokenRsC::Ident(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(idents, vec!["width".to_string(), "height".to_string()]);
    }

    #[test]
    fn string_literal_reads_until_matching_quote() {
        let tokens = tokenize("font-family: \"Fira Sans\";");
        assert!(tokens
            .iter()
            .any(|s| s.value == TokenRsC::StringLiteral("Fira Sans".to_string())));
    }

    #[test]
    fn positions_track_lines() {
        let tokens = tokenize("container {\n    width: 1px;\n}");
        let width_pos = tokens
            .iter()
            .find(|t| t.value == TokenRsC::Ident("width".to_string()))
            .unwrap()
            .pos;
        assert_eq!(width_pos.line, 2);
    }
}
