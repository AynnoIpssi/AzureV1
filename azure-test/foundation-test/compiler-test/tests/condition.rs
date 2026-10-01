#[cfg(test)]
mod tests {
    
    use azure_foundation::compiler::services::condition::*;

    #[test]
    fn bare_bool_flag_evaluates_directly() {
        let ctx = Context::new().with_bool("userisadmin", true);
        assert_eq!(evaluate("userisadmin", &ctx), Some(true));
    }

    #[test]
    fn missing_identifier_is_unevaluable() {
        let ctx = Context::new();
        assert_eq!(evaluate("userisadmin", &ctx), None);
    }

    #[test]
    fn numeric_comparison_operators_all_work() {
        let ctx = Context::new().with_number("count", 3.0);
        assert_eq!(evaluate("count == 3", &ctx), Some(true));
        assert_eq!(evaluate("count != 3", &ctx), Some(false));
        assert_eq!(evaluate("count > 0", &ctx), Some(true));
        assert_eq!(evaluate("count < 0", &ctx), Some(false));
        assert_eq!(evaluate("count >= 3", &ctx), Some(true));
        assert_eq!(evaluate("count <= 2", &ctx), Some(false));
    }

    #[test]
    fn logical_operators_combine_comparisons() {
        let ctx = Context::new().with_number("count", 3.0).with_bool("is_admin", true);
        assert_eq!(evaluate("count != 0 && is_admin", &ctx), Some(true));
        assert_eq!(evaluate("count == 0 || is_admin", &ctx), Some(true));
        assert_eq!(evaluate("!is_admin", &ctx), Some(false));
    }

    #[test]
    fn parentheses_group_subexpressions() {
        let ctx = Context::new().with_bool("a", true).with_bool("b", false).with_bool("c", false);
        assert_eq!(evaluate("a && (b || c)", &ctx), Some(false));
        assert_eq!(evaluate("(a && b) || !c", &ctx), Some(true));
    }

    #[test]
    fn string_literal_comparison_works() {
        let ctx = Context::new().with_text("role", "admin");
        assert_eq!(evaluate("role == \"admin\"", &ctx), Some(true));
        assert_eq!(evaluate("role == \"guest\"", &ctx), Some(false));
    }

    #[test]
    fn a_method_call_is_not_evaluable() {
        let ctx = Context::new().with_bool("has_next_notification", true);
        // L'identifiant existe bien, mais sous forme d'appel de methode
        // (`()`) dans la condition - hors du sous-ensemble supporte : les
        // jetons `(`/`)` restent non consommes apres l'atome, `evaluate`
        // echoue plutot que d'ignorer silencieusement l'appel.
        assert_eq!(evaluate("has_next_notification()", &ctx), None);
    }

    #[test]
    fn a_bare_number_without_comparison_is_not_evaluable() {
        let ctx = Context::new().with_number("count", 0.0);
        assert_eq!(evaluate("count", &ctx), None);
    }

    #[test]
    fn evaluate_value_resolves_an_identifier_to_its_context_value() {
        let ctx = Context::new().with_text("role", "admin");
        assert_eq!(evaluate_value("role", &ctx), Some(ConditionValue::Text("admin".to_string())));
    }

    #[test]
    fn evaluate_value_resolves_literals_without_any_context() {
        assert_eq!(evaluate_value("3", &Context::new()), Some(ConditionValue::Number(3.0)));
        assert_eq!(evaluate_value("\"admin\"", &Context::new()), Some(ConditionValue::Text("admin".to_string())));
        assert_eq!(evaluate_value("true", &Context::new()), Some(ConditionValue::Bool(true)));
    }

    #[test]
    fn evaluate_value_rejects_anything_beyond_a_bare_atom() {
        // Une comparaison n'est pas un "atome" simple - hors du sous-ensemble
        // de `evaluate_value` (reserve au sujet/motifs de `match`).
        assert_eq!(evaluate_value("count == 3", &Context::new().with_number("count", 3.0)), None);
    }

    #[test]
    fn matches_pattern_wildcard_matches_even_an_unevaluable_subject() {
        assert!(matches_pattern("_", None));
        assert!(matches_pattern("_", Some(&ConditionValue::Number(3.0))));
    }

    #[test]
    fn matches_pattern_compares_the_subject_by_strict_equality() {
        let subject = ConditionValue::Text("admin".to_string());
        assert!(matches_pattern("\"admin\"", Some(&subject)));
        assert!(!matches_pattern("\"guest\"", Some(&subject)));
    }

    #[test]
    fn matches_pattern_accepts_several_values_separated_by_a_pipe() {
        let subject = ConditionValue::Number(2.0);
        assert!(matches_pattern("1 | 2 | 3", Some(&subject)));
        assert!(!matches_pattern("1 | 3", Some(&subject)));
    }

    #[test]
    fn matches_pattern_is_false_when_the_subject_itself_is_unevaluable_and_the_pattern_is_not_wildcard() {
        assert!(!matches_pattern("\"admin\"", None));
    }
}
