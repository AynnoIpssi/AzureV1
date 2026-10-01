#[cfg(test)]
mod tests {
    use azure_foundation::compiler::rsh::models::token::{Position, TokenRsH};
    use azure_foundation::compiler::rsh::services::lexer::*;

    #[test]
    fn test_simple_title() {
        let tokens = tokenize("<if.condition#header>");
        println!("{:?}", tokens);
    }

    #[test]
    fn test_tag_name_with_digit_is_read_in_full() {
        // "title1" contient un chiffre : le nom de balise ne doit pas etre
        // tronque en "title" (ce qui produirait Title au lieu de Title1 et
        // laisserait le '1' polluer le flux de tokens suivant).
        let tokens = tokenize("<title1>Salut<!title1>");
        assert_eq!(
            tokens[1].value,
            TokenRsH::Title1(String::new(), String::new())
        );
        assert_eq!(tokens[2].value, TokenRsH::CloseTag);
    }

    #[test]
    fn test_unknown_tag_name_is_preserved() {
        let tokens = tokenize("<foo>");
        assert_eq!(tokens[0].value, TokenRsH::OpenTag);
        assert_eq!(tokens[1].value, TokenRsH::Element("foo".to_string(), String::new(), String::new(), Vec::new(), false));
    }

    #[test]
    fn test_condition_accepts_alphanumeric_and_underscore() {
        let tokens = tokenize("<if.user_is_admin2>");
        assert_eq!(
            tokens[1].value,
            TokenRsH::If("user_is_admin2".to_string(), String::new())
        );
    }

    #[test]
    fn test_positions_track_lines() {
        let tokens = tokenize("<container>\n<title>");

        assert_eq!(tokens[0].pos, Position { line: 1, column: 1 });

        let title_pos = tokens
            .iter()
            .find(|t| matches!(t.value, TokenRsH::Title(_, _)))
            .unwrap()
            .pos;
        assert_eq!(title_pos.line, 2);
    }

    #[test]
    fn test_for_loop_accepts_full_rust_expression() {
        // <for.item in liste> : condition/boucle "reelle", pas juste un
        // identifiant seul - le codegen peut l'emettre telle quelle dans un
        // `for item in liste { ... }`.
        let tokens = tokenize("<for.item in liste>Salut<!for>");
        assert_eq!(
            tokens[1].value,
            TokenRsH::For("item in liste".to_string(), String::new())
        );
    }

    #[test]
    fn test_if_condition_accepts_operators_that_dont_clash_with_close_tag() {
        // != , == , && , || : aucun conflit avec le '>' qui ferme la balise.
        let tokens = tokenize("<if.count != 0 && is_admin>Salut<!if>");
        assert_eq!(
            tokens[1].value,
            TokenRsH::If("count != 0 && is_admin".to_string(), String::new())
        );
    }

    #[test]
    fn test_if_condition_with_greater_than_is_a_known_ambiguity() {
        // '>' est a la fois l'operateur de comparaison et le caractere qui
        // ferme la balise : la capture brute s'arrete donc au premier '>'
        // rencontre, coupant la condition avant "0". C'est une limitation
        // documentee (voir is_control_flow_tag), pas un comportement voulu -
        // ce test fixe ce que le lexer fait reellement pour ne pas le
        // decouvrir en aval, silencieusement, dans du code genere errone.
        let tokens = tokenize("<if.count > 0>Salut<!if>");
        assert_eq!(
            tokens[1].value,
            TokenRsH::If("count ".to_string(), String::new())
        );
    }

    #[test]
    fn test_if_condition_escapes_greater_than_with_backslash() {
        let tokens = tokenize("<if.count \\> 0>Salut<!if>");
        assert_eq!(tokens[1].value, TokenRsH::If("count > 0".to_string(), String::new()));
    }

    #[test]
    fn test_if_condition_escapes_less_than_with_backslash() {
        let tokens = tokenize("<if.count \\< 10>Salut<!if>");
        assert_eq!(tokens[1].value, TokenRsH::If("count < 10".to_string(), String::new()));
    }

    #[test]
    fn test_if_condition_escaped_greater_than_still_lets_the_real_close_tag_end_the_capture() {
        // Le '>' NON echappe apres l'expression doit toujours fermer la
        // balise normalement, meme quand un '>' echappe est deja passe.
        let tokens = tokenize("<if.a \\> b && c \\> d>Salut<!if>");
        assert_eq!(tokens[1].value, TokenRsH::If("a > b && c > d".to_string(), String::new()));
        assert_eq!(tokens[2].value, TokenRsH::CloseTag);
    }

    #[test]
    fn test_backslash_not_followed_by_an_angle_bracket_stays_literal() {
        let tokens = tokenize("<if.path \\ contains x>Salut<!if>");
        assert_eq!(tokens[1].value, TokenRsH::If("path \\ contains x".to_string(), String::new()));
    }

    #[test]
    fn test_arm_pattern_accepts_a_quoted_string_literal() {
        let tokens = tokenize("<arm.\"admin\">Salut<!arm>");
        assert_eq!(tokens[1].value, TokenRsH::Arm("\"admin\"".to_string(), String::new()));
    }

    #[test]
    fn test_arm_pattern_accepts_the_wildcard_underscore() {
        let tokens = tokenize("<arm._>Salut<!arm>");
        assert_eq!(tokens[1].value, TokenRsH::Arm("_".to_string(), String::new()));
    }

    #[test]
    fn test_arm_pattern_accepts_several_values_separated_by_a_pipe() {
        let tokens = tokenize("<arm.1 | 2>Salut<!arm>");
        assert_eq!(tokens[1].value, TokenRsH::Arm("1 | 2".to_string(), String::new()));
    }

    #[test]
    fn test_element_class_name_still_restricted_to_identifier_chars() {
        // Les balises d'element (pas de controle) gardent l'ancienne regle
        // stricte : un nom de classe, pas une expression.
        let tokens = tokenize("<container.card#main>");
        assert_eq!(
            tokens[1].value,
            TokenRsH::Container("card".to_string(), "main".to_string())
        );
    }

    #[test]
    fn test_element_class_name_accepts_hyphens() {
        let tokens = tokenize("<button.primary-button>Clique<!button>");
        assert_eq!(
            tokens[1].value,
            TokenRsH::Button("primary-button".to_string(), String::new())
        );
    }

    #[test]
    fn test_image_reads_the_src_attribute() {
        let tokens = tokenize("<image src=\"logo.png\">");
        assert_eq!(
            tokens[1].value,
            TokenRsH::Image(String::new(), String::new(), "logo.png".to_string())
        );
    }

    #[test]
    fn test_image_src_attribute_combines_with_class_and_id() {
        let tokens = tokenize("<image.card#hero src=\"logo.png\">");
        assert_eq!(
            tokens[1].value,
            TokenRsH::Image("card".to_string(), "hero".to_string(), "logo.png".to_string())
        );
    }

    #[test]
    fn test_video_reads_the_src_attribute() {
        let tokens = tokenize("<video src=\"clip.mp4\">");
        assert_eq!(
            tokens[1].value,
            TokenRsH::Video(String::new(), String::new(), "clip.mp4".to_string())
        );
    }

    #[test]
    fn test_image_without_src_attribute_has_an_empty_src() {
        let tokens = tokenize("<image>");
        assert_eq!(tokens[1].value, TokenRsH::Image(String::new(), String::new(), String::new()));
    }

    #[test]
    fn test_entities_decode_literal_angle_brackets_and_ampersand_in_text() {
        let tokens = tokenize("<text>1 &lt; 2 &amp;&amp; 3 &gt; 0<!text>");
        assert_eq!(tokens[3].value, TokenRsH::RawText("1 < 2 && 3 > 0".to_string()));
    }

    #[test]
    fn test_ampersand_entity_is_decoded_last_so_it_does_not_double_decode() {
        // "&amp;lt;" doit rester le texte "&lt;" (un '&' litteral suivi de
        // "lt;"), pas etre redecode une seconde fois en '<'.
        let tokens = tokenize("<text>&amp;lt;<!text>");
        assert_eq!(tokens[3].value, TokenRsH::RawText("&lt;".to_string()));
    }

    #[test]
    fn test_text_without_entities_is_unaffected() {
        let tokens = tokenize("<text>Salut, rien a decoder ici.<!text>");
        assert_eq!(tokens[3].value, TokenRsH::RawText("Salut, rien a decoder ici.".to_string()));
    }

    #[test]
    fn test_unrelated_attributes_are_parsed_but_ignored() {
        // Seul `src` est retenu pour l'instant (voir TokenRsH::Image) - un
        // attribut inconnu ne doit pas faire echouer le lexer ni polluer
        // le flux de tokens qui suit.
        let tokens = tokenize("<image alt=\"logo\" src=\"logo.png\">Salut<!image>");
        assert_eq!(
            tokens[1].value,
            TokenRsH::Image(String::new(), String::new(), "logo.png".to_string())
        );
        assert_eq!(tokens[2].value, TokenRsH::CloseTag);
        assert_eq!(tokens[3].value, TokenRsH::RawText("Salut".to_string()));
    }
}
