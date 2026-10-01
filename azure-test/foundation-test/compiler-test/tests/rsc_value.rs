#[cfg(test)]
mod tests {
    use azure_engine::rendering::models::color::Color;
    use azure_foundation::compiler::rsc::models::value::*;

    #[test]
    fn parses_hex_colors_of_every_supported_length() {
        assert_eq!(parse_hex_color("fff"), Some(Color::new(255, 255, 255, 255)));
        assert_eq!(parse_hex_color("f00f"), Some(Color::new(255, 0, 0, 255)));
        assert_eq!(parse_hex_color("1e1e32"), Some(Color::new(0x1e, 0x1e, 0x32, 255)));
        assert_eq!(parse_hex_color("ffffffcc"), Some(Color::new(255, 255, 255, 0xcc)));
    }

    #[test]
    fn rejects_invalid_hex_length() {
        assert_eq!(parse_hex_color("ff"), None);
    }

    #[test]
    fn named_colors_used_in_examples_are_known() {
        assert_eq!(named_color("beige"), Some(Color::new(245, 245, 220, 255)));
        assert_eq!(named_color("transparent"), Some(Color::new(0, 0, 0, 0)));
    }

    #[test]
    fn unknown_named_color_is_none() {
        assert_eq!(named_color("notacolor"), None);
    }
}
