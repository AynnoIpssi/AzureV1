#[cfg(test)]
mod tests {
    
    
    use azure_foundation::style::models::style::*;

    #[test]
    fn merge_keeps_base_values_not_overridden() {
        let base = Style { width: Some(50.0), height: Some(30.0), ..Style::default() };
        let over = Style { height: Some(60.0), ..Style::default() };

        let merged = base.merge(&over);

        assert_eq!(merged.width, Some(50.0));
        assert_eq!(merged.height, Some(60.0));
    }
}
