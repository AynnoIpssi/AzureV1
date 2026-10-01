#[cfg(test)]
#[allow(deprecated)]
mod tests {
    
    
    use azure_engine::rendering::models::color::Color;
    
    
    use azure_foundation::style::services::stylesheet_parser::*;

    #[test]
    fn parses_numeric_and_color_properties() {
        let sheet = parse(
            ".card {
                x: 10;
                y: 20;
                width: 50;
                height: 30;
                background: #1e1e32;
                color: #ffffffcc;
                font-size: 24;
                font-weight: 700;
            }",
        )
        .expect("devrait parser");

        let style = sheet.get("card").expect("classe 'card' attendue");
        assert_eq!(style.x, Some(10.0));
        assert_eq!(style.width, Some(50.0));
        assert_eq!(style.background, Some(Color::new(0x1e, 0x1e, 0x32, 255)));
        assert_eq!(style.color, Some(Color::new(0xff, 0xff, 0xff, 0xcc)));
        assert_eq!(style.font_weight, Some(700.0));
    }

    #[test]
    fn parses_multiple_blocks() {
        let sheet = parse(".a { width: 10; } .b { width: 20; }").expect("devrait parser");
        assert_eq!(sheet.get("a").unwrap().width, Some(10.0));
        assert_eq!(sheet.get("b").unwrap().width, Some(20.0));
    }

    #[test]
    fn ignores_line_comments() {
        let sheet = parse(
            "// commentaire\n.card {\n    // encore un commentaire\n    width: 10;\n}",
        )
        .expect("devrait parser");
        assert_eq!(sheet.get("card").unwrap().width, Some(10.0));
    }

    #[test]
    fn unknown_property_is_an_error() {
        let err = parse(".card { flex: 1; }").expect_err("devrait echouer");
        assert!(err.contains("flex"));
    }

    #[test]
    fn invalid_color_is_an_error() {
        let err = parse(".card { color: red; }").expect_err("devrait echouer");
        assert!(err.contains("couleur invalide"));
    }

    #[test]
    fn unclosed_block_is_an_error() {
        let err = parse(".card { width: 10;").expect_err("devrait echouer");
        assert!(err.contains("jamais referme"));
    }
}
