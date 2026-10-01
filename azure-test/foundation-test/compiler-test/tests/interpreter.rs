#[cfg(test)]
mod tests {
    
    use azure_foundation::compiler::services::codegen::StyleSource;
    use azure_foundation::compiler::services::condition::{ConditionValue, Context};
    
    
    
    
    
    
    
    
    
    use azure_foundation::ui::models::ui_node::UiNode;
    
    use azure_engine::rendering::models::color::Color;
    use azure_foundation::compiler::services::interpreter::*;
    use azure_foundation::compiler::rsh::mangers::parser::parse;
    use azure_foundation::compiler::rsh::services::lexer::tokenize;
    use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
    use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;

    #[test]
    fn build_ui_without_a_context_never_shows_an_if_branch_without_else() {
        // Comportement par defaut de `build_ui` (contexte vide, voir sa
        // doc) : avant ce correctif, TOUTES les branches d'un if/elseif/else
        // s'affichaient empilees, quelle que soit la condition. Desormais,
        // sans variable fournie, aucune condition ne peut etre vraie.
        let ast = parse(tokenize("<if.userisadmin>Admin<!if>")).expect("rsh valide");
        let nodes = build_ui(&ast, &StyleSource::Rsc(&Default::default()));
        assert!(nodes.is_empty());
    }

    #[test]
    fn build_ui_with_context_shows_only_the_matching_branch_of_an_if_chain() {
        let ast = parse(tokenize(
            "<if.userisadmin>Admin<!if><elseif.userisguest>Invite<!elseif><else>Connecte-toi<!else>",
        ))
        .expect("rsh valide");
        let source = StyleSource::Rsc(&Default::default());

        let admin = build_ui_with_context(&ast, &source, &Context::new().with_bool("userisadmin", true).with_bool("userisguest", false));
        assert_eq!(admin.len(), 1);
        match &admin[0] {
            UiNode::Label(label) => assert_eq!(label.text, "Admin"),
            _ => panic!("expected a Label"),
        }

        let guest = build_ui_with_context(&ast, &source, &Context::new().with_bool("userisadmin", false).with_bool("userisguest", true));
        match &guest[0] {
            UiNode::Label(label) => assert_eq!(label.text, "Invite"),
            _ => panic!("expected a Label"),
        }

        let neither = build_ui_with_context(&ast, &source, &Context::new().with_bool("userisadmin", false).with_bool("userisguest", false));
        match &neither[0] {
            UiNode::Label(label) => assert_eq!(label.text, "Connecte-toi"),
            _ => panic!("expected a Label"),
        }
    }

    #[test]
    fn build_ui_with_context_never_shows_two_branches_of_the_same_chain_at_once() {
        let ast = parse(tokenize("<if.a>A<!if><elseif.b>B<!elseif>")).expect("rsh valide");
        let source = StyleSource::Rsc(&Default::default());

        // Les deux conditions sont vraies : seule la PREMIERE qui matche
        // (le if) doit s'afficher, pas les deux empilees.
        let nodes = build_ui_with_context(&ast, &source, &Context::new().with_bool("a", true).with_bool("b", true));
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            UiNode::Label(label) => assert_eq!(label.text, "A"),
            _ => panic!("expected a Label"),
        }
    }

    #[test]
    fn build_ui_with_context_treats_an_unevaluable_condition_as_false() {
        // "has_next_notification()" est un appel de methode, hors du
        // sous-ensemble supporte par `condition::evaluate` - la branche ne
        // doit pas s'afficher "par defaut", elle doit rester masquee.
        let ast = parse(tokenize("<if.has_next_notification()>Notif<!if>")).expect("rsh valide");
        let nodes = build_ui_with_context(&ast, &StyleSource::Rsc(&Default::default()), &Context::new());
        assert!(nodes.is_empty());
    }

    #[test]
    fn build_ui_with_context_while_shows_children_once_when_true_and_never_when_false() {
        let ast = parse(tokenize("<while.has_next>Notif<!while>")).expect("rsh valide");
        let source = StyleSource::Rsc(&Default::default());

        let shown = build_ui_with_context(&ast, &source, &Context::new().with_bool("has_next", true));
        assert_eq!(shown.len(), 1);

        let hidden = build_ui_with_context(&ast, &source, &Context::new().with_bool("has_next", false));
        assert!(hidden.is_empty());
    }

    #[test]
    fn build_ui_with_context_for_repeats_children_once_per_item_in_the_bound_list() {
        let ast = parse(tokenize("<for.item in liste_items>Element<!for>")).expect("rsh valide");
        let items = vec![ConditionValue::Bool(true), ConditionValue::Bool(true), ConditionValue::Bool(true)];
        let nodes = build_ui_with_context(
            &ast,
            &StyleSource::Rsc(&Default::default()),
            &Context::new().with_list("liste_items", items),
        );
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn build_ui_with_context_for_shows_nothing_when_the_list_is_not_in_context() {
        let ast = parse(tokenize("<for.item in liste_items>Element<!for>")).expect("rsh valide");
        let nodes = build_ui_with_context(&ast, &StyleSource::Rsc(&Default::default()), &Context::new());
        assert!(nodes.is_empty());
    }

    #[test]
    fn match_shows_only_the_children_of_the_first_matching_arm() {
        let ast = parse(tokenize(
            "<match.role><arm.\"admin\">Admin<!arm><arm.\"guest\">Invite<!arm><arm._>Autre<!arm><!match>",
        ))
        .expect("rsh valide");
        let source = StyleSource::Rsc(&Default::default());

        let admin = build_ui_with_context(&ast, &source, &Context::new().with_text("role", "admin"));
        assert_eq!(admin.len(), 1);
        match &admin[0] {
            UiNode::Label(label) => assert_eq!(label.text, "Admin"),
            _ => panic!("expected a Label"),
        }

        let stranger = build_ui_with_context(&ast, &source, &Context::new().with_text("role", "root"));
        match &stranger[0] {
            UiNode::Label(label) => assert_eq!(label.text, "Autre"),
            _ => panic!("expected a Label"),
        }
    }

    #[test]
    fn match_shows_nothing_when_no_arm_matches_and_there_is_no_wildcard() {
        let ast = parse(tokenize("<match.role><arm.\"admin\">Admin<!arm><!match>")).expect("rsh valide");
        let nodes = build_ui_with_context(
            &ast,
            &StyleSource::Rsc(&Default::default()),
            &Context::new().with_text("role", "guest"),
        );
        assert!(nodes.is_empty());
    }

    #[test]
    fn match_with_an_unevaluable_subject_only_the_wildcard_arm_can_match() {
        let ast = parse(tokenize("<match.role><arm.\"admin\">Admin<!arm><arm._>Autre<!arm><!match>")).expect("rsh valide");
        // "role" absent du contexte : le sujet n'est pas evaluable.
        let nodes = build_ui_with_context(&ast, &StyleSource::Rsc(&Default::default()), &Context::new());
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            UiNode::Label(label) => assert_eq!(label.text, "Autre"),
            _ => panic!("expected a Label"),
        }
    }

    #[test]
    fn builds_real_ui_nodes_styled_by_rsc() {
        let ast = parse(tokenize("<container.card><button>Ok<!button><!container>")).expect("rsh valide");
        let sheet = parse_rsc(tokenize_rsc(".card { width: 40%; } .card button { background-color: #ff0000; }"))
            .expect("rsc valide");

        let nodes = build_ui(&ast, &StyleSource::Rsc(&sheet));
        match &nodes[0] {
            UiNode::Container(container) => {
                assert_eq!(container.layout.width, 40.0);
                match &container.children[0] {
                    UiNode::Button(button) => assert_eq!(button.color, Color::new(255, 0, 0, 255)),
                    _ => panic!("expected a Button"),
                }
            }
            _ => panic!("expected a Container"),
        }
    }

    #[test]
    fn button_hover_rule_sets_hover_color_without_touching_the_base_color() {
        let ast = parse(tokenize("<button>Ok<!button>")).expect("rsh valide");
        let sheet = parse_rsc(tokenize_rsc("button { background-color: #202020; } button:hover { background-color: #ff0000; }"))
            .expect("rsc valide");

        let nodes = build_ui(&ast, &StyleSource::Rsc(&sheet));
        match &nodes[0] {
            UiNode::Button(button) => {
                assert_eq!(button.color, Color::new(0x20, 0x20, 0x20, 255));
                assert_eq!(button.hover_color, Some(Color::new(255, 0, 0, 255)));
            }
            _ => panic!("expected a Button"),
        }
    }

    #[test]
    fn button_without_a_hover_rule_leaves_hover_color_none() {
        let ast = parse(tokenize("<button>Ok<!button>")).expect("rsh valide");
        let sheet = parse_rsc(tokenize_rsc("button { background-color: #202020; }")).expect("rsc valide");

        let nodes = build_ui(&ast, &StyleSource::Rsc(&sheet));
        match &nodes[0] {
            UiNode::Button(button) => assert_eq!(button.hover_color, None),
            _ => panic!("expected a Button"),
        }
    }

    #[test]
    fn textarea_hover_and_focus_rules_set_independent_overrides() {
        let ast = parse(tokenize("<textarea><!textarea>")).expect("rsh valide");
        let sheet = parse_rsc(tokenize_rsc(
            "textarea:hover { background-color: #ffcc00; } textarea:focus { background-color: #00ccff; }",
        ))
        .expect("rsc valide");

        let nodes = build_ui(&ast, &StyleSource::Rsc(&sheet));
        match &nodes[0] {
            UiNode::TextArea(area) => {
                assert_eq!(area.hover_background, Some(Color::new(0xff, 0xcc, 0x00, 255)));
                assert_eq!(area.focus_background, Some(Color::new(0x00, 0xcc, 0xff, 255)));
            }
            _ => panic!("expected a TextArea"),
        }
    }

    #[test]
    fn image_src_attribute_flows_through_to_the_ui_node() {
        let ast = parse(tokenize("<image src=\"logo.png\"><!image>")).expect("rsh valide");
        let nodes = build_ui(&ast, &StyleSource::Rsc(&Default::default()));
        match &nodes[0] {
            UiNode::Image(image) => assert_eq!(image.src, "logo.png"),
            _ => panic!("expected an Image"),
        }
    }

    #[test]
    fn raw_text_between_tags_is_skipped_when_blank() {
        let ast = parse(tokenize("<container>\n<!container>")).expect("rsh valide");
        let nodes = build_ui(&ast, &StyleSource::Rsc(&Default::default()));
        match &nodes[0] {
            UiNode::Container(container) => assert!(container.children.is_empty()),
            _ => panic!("expected a Container"),
        }
    }
}
