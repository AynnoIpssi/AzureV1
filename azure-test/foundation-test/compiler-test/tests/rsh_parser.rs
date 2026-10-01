#[cfg(test)]
mod tests {
    
    use azure_foundation::compiler::rsh::models::token::{Position, TokenRsH};
    use azure_foundation::compiler::rsh::mangers::parser::*;
    use azure_foundation::compiler::rsh::services::lexer::tokenize;

    #[test]
    fn test_nested_elements() {
        let tokens = tokenize("<container><title>Hello<!title><!container>");
        let ast = parse(tokens).expect("devrait parser sans erreur");

        assert_eq!(ast.len(), 1);
        match &ast[0] {
            AstNode::Container { children, .. } => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    AstNode::Title { children, .. } => {
                        assert_eq!(children.len(), 1);
                        assert!(matches!(children[0], AstNode::RawText(_)));
                    }
                    other => panic!("expected Title node, got {other:?}"),
                }
            }
            other => panic!("expected Container node, got {other:?}"),
        }
    }

    #[test]
    fn test_if_condition_carried_raw() {
        let tokens = tokenize("<if.userisadmin>Hello<!if>");
        let ast = parse(tokens).expect("devrait parser sans erreur");

        assert_eq!(ast.len(), 1);
        match &ast[0] {
            AstNode::If { condition, children } => {
                assert_eq!(condition, "userisadmin");
                assert_eq!(children.len(), 1);
            }
            other => panic!("expected If node, got {other:?}"),
        }
    }

    #[test]
    fn test_if_condition_accepts_underscore_and_digits() {
        let tokens = tokenize("<if.user_is_admin2>Hello<!if>");
        let ast = parse(tokens).expect("devrait parser sans erreur");

        match &ast[0] {
            AstNode::If { condition, .. } => assert_eq!(condition, "user_is_admin2"),
            other => panic!("expected If node, got {other:?}"),
        }
    }

    #[test]
    fn test_match_with_arms_parses_the_pattern_of_each_branch() {
        let tokens = tokenize("<match.role><arm.\"admin\">Admin<!arm><arm._>Autre<!arm><!match>");
        let ast = parse(tokens).expect("devrait parser sans erreur");

        match &ast[0] {
            AstNode::Match { condition, children } => {
                assert_eq!(condition, "role");
                assert_eq!(children.len(), 2);
                match &children[0] {
                    AstNode::Arm { pattern, .. } => assert_eq!(pattern, "\"admin\""),
                    other => panic!("expected Arm node, got {other:?}"),
                }
                match &children[1] {
                    AstNode::Arm { pattern, .. } => assert_eq!(pattern, "_"),
                    other => panic!("expected Arm node, got {other:?}"),
                }
            }
            other => panic!("expected Match node, got {other:?}"),
        }
    }

    #[test]
    fn test_mismatched_closing_tag_is_an_error() {
        let tokens = tokenize("<container>Hello<!title>");
        let err = parse(tokens).expect_err("devrait echouer");

        assert!(matches!(err.kind, ParseErrorKind::MismatchedClosingTag { .. }));
    }

    #[test]
    fn test_unclosed_element_is_an_error() {
        let tokens = tokenize("<container>Hello");
        let err = parse(tokens).expect_err("devrait echouer");

        assert_eq!(
            err.kind,
            ParseErrorKind::UnclosedElement(TokenRsH::Container(String::new(), String::new()))
        );
        assert_eq!(err.pos, Position { line: 1, column: 1 });
    }

    #[test]
    fn test_other_tags_are_components_that_keep_their_name() {
        // Une balise qui n'est pas un element de base (ex: <foo>) est un
        // composant, resolu a la construction (voir compiler::components) ;
        // `/>` la ferme d'elle-meme.
        let nodes = parse(tokenize("<foo.a#b titre=\"x\"/>")).unwrap();
        assert!(matches!(&nodes[0], AstNode::Element { tag, class, id, attrs, children } if tag == "foo" && class == "a" && id == "b" && attrs == &vec![("titre".to_string(), "x".to_string())] && children.is_empty()));
        // Non fermee : erreur avec son nom.
        let err = parse(tokenize("<foo>")).expect_err("devrait echouer");
        assert!(matches!(err.kind, ParseErrorKind::UnclosedElement(TokenRsH::Element(ref name, ..)) if name == "foo"));
    }

    #[test]
    fn test_empty_tag_is_an_error() {
        let err = parse(tokenize("<>")).expect_err("devrait echouer");
        assert_eq!(err.kind, ParseErrorKind::ExpectedElement(TokenRsH::Unknown(String::new())));
    }

    #[test]
    fn test_stray_closing_tag_is_an_error() {
        let tokens = tokenize("<!container>");
        let err = parse(tokens).expect_err("devrait echouer");

        assert!(matches!(err.kind, ParseErrorKind::UnexpectedClosingTag(_)));
    }

    #[test]
    fn test_error_position_points_to_the_right_line() {
        let tokens = tokenize("<container>\nHello<!title>");
        let err = parse(tokens).expect_err("devrait echouer");

        assert!(matches!(err.kind, ParseErrorKind::MismatchedClosingTag { .. }));
        assert_eq!(err.pos.line, 2);
    }
}
