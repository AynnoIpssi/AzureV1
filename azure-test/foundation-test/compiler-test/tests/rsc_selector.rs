#[cfg(test)]
mod tests {
    use azure_foundation::compiler::rsc::models::selector::*;

    #[test]
    fn id_beats_any_number_of_classes() {
        let by_id = Specificity(1, 0, 0);
        let many_classes = Specificity(0, 10, 10);
        assert!(by_id > many_classes);
    }

    #[test]
    fn simple_selector_matches_tag_class_and_id_together() {
        let sel = SimpleSelector {
            tag: Some("container".to_string()),
            id: Some("main".to_string()),
            classes: vec!["card".to_string()],
            ..Default::default()
        };
        assert!(sel.matches("container", &["card".to_string(), "highlighted".to_string()], "main", PseudoState::default()));
        assert!(!sel.matches("container", &["card".to_string()], "other", PseudoState::default()));
        assert!(!sel.matches("button", &["card".to_string()], "main", PseudoState::default()));
    }

    #[test]
    fn hover_and_focus_only_match_when_that_state_is_active() {
        let hover_sel = SimpleSelector { pseudo_classes: vec!["hover".to_string()], ..Default::default() };
        let focus_sel = SimpleSelector { pseudo_classes: vec!["focus".to_string()], ..Default::default() };

        assert!(!hover_sel.matches("button", &[], "", PseudoState::default()));
        assert!(hover_sel.matches("button", &[], "", PseudoState { hover: true, focus: false, active: false }));
        // `:hover` ne doit pas matcher juste parce que `:focus` est actif,
        // et reciproquement.
        assert!(!hover_sel.matches("button", &[], "", PseudoState { hover: false, focus: true, active: false }));

        assert!(!focus_sel.matches("button", &[], "", PseudoState::default()));
        assert!(focus_sel.matches("button", &[], "", PseudoState { hover: false, focus: true, active: false }));
    }

    #[test]
    fn unrecognized_pseudo_class_never_matches_regardless_of_state() {
        let sel = SimpleSelector { pseudo_classes: vec!["first-child".to_string()], ..Default::default() };
        assert!(!sel.matches("button", &[], "", PseudoState { hover: true, focus: true, active: false }));
    }

    #[test]
    fn specificity_sums_across_the_whole_chain() {
        let selector = ComplexSelector {
            parts: vec![
                (
                    SimpleSelector { classes: vec!["card".to_string()], ..Default::default() },
                    None,
                ),
                (
                    SimpleSelector { tag: Some("button".to_string()), ..Default::default() },
                    Some(Combinator::Descendant),
                ),
            ],
        };
        assert_eq!(selector.specificity(), Specificity(0, 1, 1));
    }
}
