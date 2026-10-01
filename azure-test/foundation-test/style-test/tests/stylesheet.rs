#[cfg(test)]
mod tests {
    
    use azure_foundation::style::models::style::Style;
    use azure_foundation::style::models::stylesheet::*;

    #[test]
    fn resolve_applies_named_style_over_base() {
        let mut sheet = Stylesheet::new();
        sheet.define("card", Style { width: Some(40.0), ..Style::default() });

        let base = Style { width: Some(100.0), height: Some(50.0), ..Style::default() };
        let resolved = sheet.resolve("card", &base);

        assert_eq!(resolved.width, Some(40.0));
        assert_eq!(resolved.height, Some(50.0));
    }

    #[test]
    fn resolve_falls_back_to_base_when_class_unknown() {
        let sheet = Stylesheet::new();
        let base = Style { width: Some(100.0), ..Style::default() };

        assert_eq!(sheet.resolve("nope", &base), base);
    }
}
