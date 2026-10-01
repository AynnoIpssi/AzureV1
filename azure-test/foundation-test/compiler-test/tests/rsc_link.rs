#[cfg(test)]
mod tests {
    
    use azure_foundation::compiler::rsc::models::property::{ComputedStyle, Property};
    
    use azure_foundation::compiler::rsc::models::selector::PseudoState;
    use azure_foundation::compiler::rsc::models::value::{Unit, Value};
    
    
    
    use azure_foundation::compiler::rsc::services::link::*;
    use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
    use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
    use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
    use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;

    fn styled_tree(rsh_src: &str, rsc_src: &str) -> Vec<StyledNode> {
        let ast = parse_rsh(tokenize_rsh(rsh_src)).expect("rsh valide");
        let sheet = parse_rsc(tokenize_rsc(rsc_src)).expect("rsc valide");
        resolve_tree(&ast, &sheet)
    }

    #[test]
    fn type_selector_matches_by_tag_name() {
        let tree = styled_tree("<container><!container>", "container { width: 10px; }");
        assert_eq!(tree[0].style.width, Some(Value::Length(10.0, Unit::Px)));
    }

    #[test]
    fn class_selector_only_matches_elements_carrying_it() {
        let tree = styled_tree(
            "<container.card><!container><container><!container>",
            ".card { width: 10px; }",
        );
        assert_eq!(tree[0].style.width, Some(Value::Length(10.0, Unit::Px)));
        assert_eq!(tree[1].style.width, None);
    }

    #[test]
    fn descendant_selector_matches_nested_but_not_sibling_elements() {
        let tree = styled_tree(
            "<container.card><button>Ok<!button><!container><button>Autre<!button>",
            ".card button { color: red; }",
        );
        let card = &tree[0];
        let nested_button = &card.children[0];
        assert!(nested_button.style.color.is_some());

        let sibling_button = &tree[1];
        assert_eq!(sibling_button.style.color, None);
    }

    #[test]
    fn child_combinator_does_not_match_a_deeper_descendant() {
        // Le bouton est un enfant direct de <text>, pas de <container> - meme
        // si <container> est son grand-parent, le combinateur enfant
        // (contrairement au descendant) ne doit pas remonter au-dela du
        // parent immediat.
        let tree = styled_tree(
            "<container><text><button>Ok<!button><!text><!container>",
            "container > button { color: red; }",
        );
        let container = &tree[0];
        let text = &container.children[0];
        let button = &text.children[0];
        assert_eq!(button.style.color, None);
    }

    #[test]
    fn child_combinator_matches_the_immediate_parent_regardless_of_depth() {
        // A l'inverse : si le parent immediat porte bien le type recherche,
        // peu importe la profondeur totale de l'arbre - ici le parent direct
        // du bouton est lui-meme un <container> (imbrique dans un autre).
        let tree = styled_tree(
            "<container><container><button>Ok<!button><!container><!container>",
            "container > button { color: red; }",
        );
        let inner_container = &tree[0].children[0];
        let button = &inner_container.children[0];
        assert!(button.style.color.is_some());
    }

    #[test]
    fn id_selector_wins_over_class_selector_regardless_of_source_order() {
        let tree = styled_tree(
            "<container.card#main><!container>",
            "#main { color: blue; } .card { color: red; }",
        );
        assert_eq!(tree[0].style.color, Some(Value::Color(azure_engine::rendering::models::color::Color::new(0, 0, 255, 255))));
    }

    #[test]
    fn later_rule_wins_when_specificity_is_equal() {
        let tree = styled_tree(
            "<container.card><!container>",
            ".card { color: red; } .card { color: blue; }",
        );
        assert_eq!(tree[0].style.color, Some(Value::Color(azure_engine::rendering::models::color::Color::new(0, 0, 255, 255))));
    }

    #[test]
    fn important_wins_even_with_lower_specificity() {
        let tree = styled_tree(
            "<container.card#main><!container>",
            "#main { color: blue; } .card { color: red !important; }",
        );
        assert_eq!(tree[0].style.color, Some(Value::Color(azure_engine::rendering::models::color::Color::new(255, 0, 0, 255))));
    }

    #[test]
    fn structural_nodes_are_transparent_for_descendant_matching() {
        // Le <if> ne compte pas comme un niveau d'imbrication CSS : le
        // bouton qu'il contient reste un descendant direct de .card.
        let tree = styled_tree(
            "<container.card><if.flag><button>Ok<!button><!if><!container>",
            ".card button { color: red; }",
        );
        let button = &tree[0].children[0];
        assert!(button.style.color.is_some());
    }

    #[test]
    fn hover_rule_only_applies_when_hover_is_reported_active() {
        let sheet = parse_rsc(tokenize_rsc("button:hover { color: red; }")).expect("rsc valide");
        let button = ElementInfo::new("button", "", "");

        let base = resolve_element(&sheet, &[], &[], &button, PseudoState::default());
        assert_eq!(base.color, None);

        let hovered = resolve_element(&sheet, &[], &[], &button, PseudoState { hover: true, focus: false, active: false });
        assert_eq!(hovered.color, Some(Value::Color(azure_engine::rendering::models::color::Color::new(255, 0, 0, 255))));
    }

    #[test]
    fn focus_pseudo_class_is_independent_of_hover() {
        let sheet = parse_rsc(tokenize_rsc("textarea:focus { color: blue; }")).expect("rsc valide");
        let area = ElementInfo::new("textarea", "", "");

        // Survole mais pas focalise : la regle `:focus` ne doit pas
        // s'appliquer juste parce qu'un AUTRE pseudo-etat est actif.
        let hovered_only = resolve_element(&sheet, &[], &[], &area, PseudoState { hover: true, focus: false, active: false });
        assert_eq!(hovered_only.color, None);

        let focused = resolve_element(&sheet, &[], &[], &area, PseudoState { hover: false, focus: true, active: false });
        assert_eq!(focused.color, Some(Value::Color(azure_engine::rendering::models::color::Color::new(0, 0, 255, 255))));
    }

    #[test]
    fn hover_state_of_an_ancestor_does_not_affect_a_descendant_selector() {
        // `pseudo` ne s'applique qu'au sujet (le maillon le plus a droite)
        // du selecteur - un `.card button` ne doit pas se mettre a matcher
        // juste parce qu'on a passe `hover: true`, meme si aucune regle ne
        // porte `:hover` : ca verifie que le pseudo-etat ne "fuit" pas vers
        // les ancetres dans `complex_matches`.
        let sheet = parse_rsc(tokenize_rsc(".card button { color: red; }")).expect("rsc valide");
        let card = ElementInfo::new("container", "card", "");
        let button = ElementInfo::new("button", "", "");

        let resolved = resolve_element(&sheet, &[card], &[], &button, PseudoState { hover: true, focus: true, active: false });
        assert_eq!(resolved.color, Some(Value::Color(azure_engine::rendering::models::color::Color::new(255, 0, 0, 255))));
    }

    #[test]
    fn adjacent_sibling_combinator_matches_only_the_immediately_preceding_element() {
        let tree = styled_tree(
            "<container><title>Titre<!title><text>Direct<!text><text>Pas direct<!text><!container>",
            "title + text { color: red; }",
        );
        let container = &tree[0];
        assert!(container.children[1].style.color.is_some(), "le <text> juste apres <title> doit matcher");
        assert_eq!(container.children[2].style.color, None, "le <text> suivant, pas adjacent, ne doit pas matcher");
    }

    #[test]
    fn adjacent_sibling_combinator_does_not_match_without_a_preceding_element() {
        let tree = styled_tree("<container><text>Seul<!text><!container>", "title + text { color: red; }");
        assert_eq!(tree[0].children[0].style.color, None);
    }

    #[test]
    fn general_sibling_combinator_matches_any_earlier_sibling_not_just_the_adjacent_one() {
        let tree = styled_tree(
            "<container><title>Titre<!title><text>Entre<!text><text>Plus loin<!text><!container>",
            "title ~ text { color: red; }",
        );
        let container = &tree[0];
        assert!(container.children[1].style.color.is_some());
        assert!(container.children[2].style.color.is_some(), "un frere general doit matcher meme non adjacent");
    }

    #[test]
    fn sibling_combinator_ignores_structural_wrapper_nodes() {
        // <if>/<while>/... sont transparents pour les freres aussi, pas
        // seulement pour les ancetres (voir `structural_nodes_are_transparent_for_descendant_matching`) :
        // le <text> reste le frere direct du <title>, meme enveloppe.
        let tree = styled_tree(
            "<container><title>Titre<!title><if.flag><text>Dans le if<!text><!if><!container>",
            "title + text { color: red; }",
        );
        assert!(tree[0].children[1].style.color.is_some());
    }

    #[test]
    fn sibling_combinator_does_not_leak_across_a_container_boundary() {
        // Un <text> a l'interieur d'un <container> different n'est pas un
        // frere du <title> qui precede ce container.
        let tree = styled_tree(
            "<title>Titre<!title><container><text>Dedans<!text><!container>",
            "title + text { color: red; }",
        );
        let inner_text = &tree[1].children[0];
        assert_eq!(inner_text.style.color, None);
    }

    #[test]
    fn to_legacy_style_extracts_pixel_lengths_and_colors() {
        let mut computed = ComputedStyle::new();
        computed.set(Property::Width, Value::Length(42.0, Unit::Px));
        computed.set(Property::Color, Value::Color(azure_engine::rendering::models::color::Color::new(1, 2, 3, 255)));
        computed.set(Property::FontWeight, Value::Keyword("bold".to_string()));

        let legacy = to_legacy_style(&computed);
        assert_eq!(legacy.width, Some(42.0));
        assert_eq!(legacy.color, Some(azure_engine::rendering::models::color::Color::new(1, 2, 3, 255)));
        assert_eq!(legacy.font_weight, Some(700.0));
    }
}
