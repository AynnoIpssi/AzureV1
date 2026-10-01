#[cfg(test)]
mod tests {
    use azure_foundation::compiler::rsc::models::value::Value;
    use azure_foundation::compiler::rsc::models::property::*;
    use azure_foundation::compiler::rsc::models::value::Unit;

    #[test]
    fn from_name_recognizes_kebab_case_properties() {
        assert_eq!(Property::from_name("background-color"), Some(Property::BackgroundColor));
        assert_eq!(Property::from_name("z-index"), Some(Property::ZIndex));
        assert_eq!(Property::from_name("flex-direction"), Some(Property::FlexDirection));
    }

    #[test]
    fn unknown_property_name_is_none() {
        assert_eq!(Property::from_name("transform"), None);
    }

    #[test]
    fn merge_keeps_base_fields_not_overridden() {
        let mut base = ComputedStyle::new();
        base.set(Property::Width, Value::Length(50.0, Unit::Px));
        base.set(Property::Height, Value::Length(30.0, Unit::Px));

        let mut over = ComputedStyle::new();
        over.set(Property::Height, Value::Length(60.0, Unit::Px));

        let merged = base.merge(&over);
        assert_eq!(merged.get(Property::Width), Some(&Value::Length(50.0, Unit::Px)));
        assert_eq!(merged.get(Property::Height), Some(&Value::Length(60.0, Unit::Px)));
    }
}
