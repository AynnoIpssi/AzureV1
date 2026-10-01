#[cfg(test)]
mod tests {
    
    use azure_foundation::ui::models::textarea::TextArea;
    use azure_foundation::ui::models::ui_node::UiNode;
    
    
    
    use azure_foundation::ui::services::interact::*;
    use azure_foundation::layout::models::layout_props::LayoutProps;
    use azure_foundation::ui::models::button::Button;
    use azure_foundation::ui::models::container::Container;
    use azure_engine::rendering::models::color::Color;

    fn layout(x: f32, y: f32, w: f32, h: f32) -> LayoutProps {
        LayoutProps::new(x, y, w, h, 0.0, 0.0)
    }

    #[test]
    fn toggle_button_at_flips_state_only_for_the_hit_button() {
        // Fenetre 100x100 : un bouton occupe [0,50)x[0,100), un autre [50,100)x[0,100).
        let mut nodes = vec![
            UiNode::Button(Button::new(layout(0.0, 0.0, 50.0, 100.0), Color::new(0, 0, 0, 255), false, "a".to_string())),
            UiNode::Button(Button::new(layout(50.0, 0.0, 50.0, 100.0), Color::new(0, 0, 0, 255), false, "b".to_string())),
        ];

        assert!(toggle_button_at(&mut nodes, 10, 10, (0, 0, 100, 100)));
        match &nodes[0] {
            UiNode::Button(b) => assert!(b.state),
            _ => unreachable!(),
        }
        match &nodes[1] {
            UiNode::Button(b) => assert!(!b.state),
            _ => unreachable!(),
        }
    }

    #[test]
    fn toggle_button_at_finds_buttons_nested_inside_containers() {
        let mut nodes = vec![UiNode::Container(Container::new(
            layout(0.0, 0.0, 100.0, 100.0),
            Color::new(0, 0, 0, 255),
            vec![UiNode::Button(Button::new(layout(0.0, 0.0, 100.0, 100.0), Color::new(0, 0, 0, 255), false, "a".to_string()))],
        ))];

        assert!(toggle_button_at(&mut nodes, 5, 5, (0, 0, 100, 100)));
    }

    #[test]
    fn toggle_button_at_misses_outside_any_button() {
        let mut nodes = vec![UiNode::Button(Button::new(layout(0.0, 0.0, 50.0, 50.0), Color::new(0, 0, 0, 255), false, "a".to_string()))];
        assert!(!toggle_button_at(&mut nodes, 90, 90, (0, 0, 100, 100)));
    }

    #[test]
    fn hover_kind_at_distinguishes_button_textarea_and_nothing() {
        let nodes = vec![
            UiNode::Button(Button::new(layout(0.0, 0.0, 50.0, 100.0), Color::new(0, 0, 0, 255), false, "a".to_string())),
            UiNode::TextArea(TextArea::new(layout(50.0, 0.0, 50.0, 100.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), String::new())),
        ];

        assert_eq!(hover_kind_at(&nodes, 10, 10, (0, 0, 100, 100)), HoverKind::Button);
        assert_eq!(hover_kind_at(&nodes, 60, 10, (0, 0, 100, 100)), HoverKind::TextArea);
        assert_eq!(hover_kind_at(&nodes, 10, 200, (0, 0, 100, 100)), HoverKind::None);
    }

    #[test]
    fn hover_kind_at_looks_inside_nested_containers() {
        let nodes = vec![UiNode::Container(Container::new(
            layout(0.0, 0.0, 100.0, 100.0),
            Color::new(0, 0, 0, 255),
            vec![UiNode::Button(Button::new(layout(0.0, 0.0, 100.0, 100.0), Color::new(0, 0, 0, 255), false, "a".to_string()))],
        ))];

        assert_eq!(hover_kind_at(&nodes, 5, 5, (0, 0, 100, 100)), HoverKind::Button);
    }

    #[test]
    fn focus_textarea_at_focuses_the_hit_one_and_unfocuses_the_rest() {
        let mut nodes = vec![
            UiNode::TextArea(TextArea::new(layout(0.0, 0.0, 50.0, 100.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), String::new())),
            UiNode::TextArea(TextArea::new(layout(50.0, 0.0, 50.0, 100.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), String::new())),
        ];
        match &mut nodes[1] {
            UiNode::TextArea(area) => area.focused = true,
            _ => unreachable!(),
        }

        assert!(focus_textarea_at(&mut nodes, 10, 10, (0, 0, 100, 100)));
        match &nodes[0] {
            UiNode::TextArea(area) => assert!(area.focused),
            _ => unreachable!(),
        }
        match &nodes[1] {
            UiNode::TextArea(area) => assert!(!area.focused),
            _ => unreachable!(),
        }
    }

    #[test]
    fn clicking_a_textarea_places_the_cursor_and_clears_any_selection() {
        let mut nodes = focused_area("hello");
        match &mut nodes[0] {
            UiNode::TextArea(area) => area.select_all(),
            _ => unreachable!(),
        }
        // Boite [0,100)x[0,100), texte pousse de TEXTAREA_TEXT_PADDING : un
        // clic tres a droite doit placer le curseur en fin de texte.
        assert!(focus_textarea_at(&mut nodes, 90, 10, (0, 0, 100, 100)));
        let area = area_of(&nodes);
        assert_eq!(area.cursor, area.char_count());
        assert_eq!(area.selection_anchor, None);
    }

    #[test]
    fn clicking_where_two_textareas_overlap_focuses_only_the_first_one() {
        // Layout degenere (le moteur de `layout` ne l'empeche pas) : les
        // deux boites couvrent entierement la fenetre et se chevauchent
        // donc partout - seule la premiere (ordre de parcours) doit finir
        // focalisee, jamais les deux a la fois.
        let mut nodes = vec![
            UiNode::TextArea(TextArea::new(layout(0.0, 0.0, 100.0, 100.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), String::new())),
            UiNode::TextArea(TextArea::new(layout(0.0, 0.0, 100.0, 100.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), String::new())),
        ];

        assert!(focus_textarea_at(&mut nodes, 10, 10, (0, 0, 100, 100)));
        match (&nodes[0], &nodes[1]) {
            (UiNode::TextArea(first), UiNode::TextArea(second)) => {
                assert!(first.focused, "la premiere textarea touchee doit gagner le focus");
                assert!(!second.focused, "la seconde, bien que chevauchante, ne doit pas etre focalisee aussi");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn clicking_outside_any_textarea_unfocuses_it() {
        let mut nodes = focused_area("hi");
        assert!(focus_textarea_at(&mut nodes, 500, 500, (0, 0, 100, 100)));
        assert!(!area_of(&nodes).focused);
    }

    #[test]
    fn dragging_from_the_start_extends_a_selection_to_the_click_point() {
        let mut nodes = focused_area("hello world");
        // Clic au tout debut : place le curseur en 0, sans selection.
        focus_textarea_at(&mut nodes, 0, 10, (0, 0, 200, 100));
        assert_eq!(area_of(&nodes).cursor, 0);
        assert_eq!(area_of(&nodes).selection_anchor, None);

        // Glisser vers la droite etend la selection depuis ce point de
        // depart (l'ancre), sans jamais la reinitialiser tant que le
        // clic initial n'est pas repete.
        assert!(extend_selection_to(&mut nodes, 60, 10, (0, 0, 200, 100)));
        let area = area_of(&nodes);
        assert_eq!(area.selection_anchor, Some(0));
        assert!(area.cursor > 0);
    }

    #[test]
    fn clicking_the_second_visual_line_places_the_cursor_there() {
        // Deux lignes explicites ("hello" \n "world") - fenetre large pour
        // ecarter tout retour a la ligne automatique, seul le '\n' compte.
        let mut nodes = focused_area("hello\nworld");
        // ligne 0 en y~6 (marge interne), ligne 1 environ line_height plus
        // bas (16px*1.1 ~ 17px) - un clic a y=25 doit tomber sur la 2e ligne.
        assert!(focus_textarea_at(&mut nodes, 0, 25, (0, 0, 300, 100)));
        let area = area_of(&nodes);
        // "hello\n" fait 6 caracteres (le '\n' inclus dans la 1ere ligne) :
        // le curseur doit etre sur la 2e ligne, donc a l'index 6 ou plus.
        assert!(area.cursor >= 6, "cursor {} devrait etre sur la 2e ligne (>= 6)", area.cursor);
    }

    #[test]
    fn extend_selection_to_does_nothing_when_no_textarea_is_focused() {
        let mut nodes = vec![UiNode::TextArea(TextArea::new(
            layout(0.0, 0.0, 100.0, 100.0),
            Color::new(0, 0, 0, 255),
            Color::new(255, 255, 255, 255),
            "hi".to_string(),
        ))];
        assert!(!extend_selection_to(&mut nodes, 50, 50, (0, 0, 100, 100)));
    }

    #[test]
    fn scroll_at_moves_the_offset_of_the_hovered_textarea_one_line_at_a_time() {
        // Fenetre large (pas de retour a la ligne automatique) et basse :
        // ligne ~17-18px, boite de 60px moins 2*padding(6) ~ 2 lignes
        // visibles a la fois pour 5 lignes de texte.
        let mut nodes = focused_area("l1\nl2\nl3\nl4\nl5");
        assert!(scroll_at(&mut nodes, 50, 10, 1.0, (0, 0, 200, 60)));
        assert_eq!(area_of(&nodes).scroll_offset, 1);
        assert!(scroll_at(&mut nodes, 50, 10, 1.0, (0, 0, 200, 60)));
        assert_eq!(area_of(&nodes).scroll_offset, 2);
    }

    #[test]
    fn scroll_at_clamps_to_the_last_possible_offset() {
        let mut nodes = focused_area("l1\nl2\nl3\nl4\nl5");
        for _ in 0..10 {
            scroll_at(&mut nodes, 50, 10, 1.0, (0, 0, 200, 60));
        }
        // 5 lignes, 2 visibles a la fois -> le dernier decalage possible est 3.
        assert_eq!(area_of(&nodes).scroll_offset, 3);
    }

    #[test]
    fn scroll_at_ignores_a_point_outside_the_textarea() {
        let mut nodes = focused_area("l1\nl2\nl3\nl4\nl5");
        assert!(!scroll_at(&mut nodes, 500, 500, 1.0, (0, 0, 200, 60)));
        assert_eq!(area_of(&nodes).scroll_offset, 0);
    }

    #[test]
    fn ensure_cursor_visible_scrolls_down_when_the_cursor_is_below_the_visible_window() {
        let mut nodes = focused_area("l1\nl2\nl3\nl4\nl5");
        // Curseur en toute fin de texte (derniere ligne, index 4 sur 5).
        ensure_cursor_visible(&mut nodes, (0, 0, 200, 60));
        // 2 lignes visibles, curseur sur la ligne 4 -> il faut defiler
        // jusqu'a scroll_offset = 4 + 1 - 2 = 3 pour la rendre visible.
        assert_eq!(area_of(&nodes).scroll_offset, 3);
    }

    #[test]
    fn ensure_cursor_visible_scrolls_up_when_the_cursor_is_above_the_visible_window() {
        let mut nodes = focused_area("l1\nl2\nl3\nl4\nl5");
        match &mut nodes[0] {
            UiNode::TextArea(area) => {
                area.scroll_offset = 3;
                area.cursor = 0; // remonte tout en haut (ex: Ctrl+Home un jour, ou juste programmé)
            }
            _ => unreachable!(),
        }
        ensure_cursor_visible(&mut nodes, (0, 0, 200, 60));
        assert_eq!(area_of(&nodes).scroll_offset, 0);
    }

    #[test]
    fn type_into_focused_only_affects_the_focused_textarea() {
        let mut nodes = vec![
            UiNode::TextArea(TextArea::new(layout(0.0, 0.0, 50.0, 100.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), String::new())),
            UiNode::TextArea(TextArea::new(layout(50.0, 0.0, 50.0, 100.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), String::new())),
        ];
        match &mut nodes[1] {
            UiNode::TextArea(area) => area.focused = true,
            _ => unreachable!(),
        }

        let mut clipboard = String::new();
        assert!(type_into_focused(&mut nodes, KeyInput::Char('h'), &mut clipboard));
        assert!(type_into_focused(&mut nodes, KeyInput::Char('i'), &mut clipboard));

        match &nodes[0] {
            UiNode::TextArea(area) => assert_eq!(area.text, ""),
            _ => unreachable!(),
        }
        match &nodes[1] {
            UiNode::TextArea(area) => assert_eq!(area.text, "hi"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let mut nodes = vec![UiNode::TextArea(TextArea::new(
            layout(0.0, 0.0, 100.0, 100.0),
            Color::new(0, 0, 0, 255),
            Color::new(255, 255, 255, 255),
            "hi".to_string(),
        ))];
        match &mut nodes[0] {
            UiNode::TextArea(area) => area.focused = true,
            _ => unreachable!(),
        }

        let mut clipboard = String::new();
        assert!(type_into_focused(&mut nodes, KeyInput::Backspace, &mut clipboard));
        match &nodes[0] {
            UiNode::TextArea(area) => assert_eq!(area.text, "h"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn backspace_on_empty_text_reports_no_change() {
        let mut nodes = vec![UiNode::TextArea(TextArea::new(
            layout(0.0, 0.0, 100.0, 100.0),
            Color::new(0, 0, 0, 255),
            Color::new(255, 255, 255, 255),
            String::new(),
        ))];
        match &mut nodes[0] {
            UiNode::TextArea(area) => area.focused = true,
            _ => unreachable!(),
        }

        let mut clipboard = String::new();
        assert!(!type_into_focused(&mut nodes, KeyInput::Backspace, &mut clipboard));
    }

    fn focused_area(text: &str) -> Vec<UiNode> {
        let mut nodes = vec![UiNode::TextArea(TextArea::new(
            layout(0.0, 0.0, 100.0, 100.0),
            Color::new(0, 0, 0, 255),
            Color::new(255, 255, 255, 255),
            text.to_string(),
        ))];
        match &mut nodes[0] {
            UiNode::TextArea(area) => area.focused = true,
            _ => unreachable!(),
        }
        nodes
    }

    fn area_of(nodes: &[UiNode]) -> &TextArea {
        match &nodes[0] {
            UiNode::TextArea(area) => area,
            _ => unreachable!(),
        }
    }

    #[test]
    fn delete_removes_the_character_after_the_cursor() {
        let mut nodes = focused_area("hello");
        match &mut nodes[0] {
            UiNode::TextArea(area) => area.cursor = 2,
            _ => unreachable!(),
        }
        let mut clipboard = String::new();
        assert!(type_into_focused(&mut nodes, KeyInput::Delete, &mut clipboard));
        assert_eq!(area_of(&nodes).text, "helo");
    }

    #[test]
    fn arrow_keys_move_the_cursor() {
        let mut nodes = focused_area("hi"); // cursor demarre a 2 (fin)
        let mut clipboard = String::new();
        assert!(type_into_focused(&mut nodes, KeyInput::MoveLeft(false), &mut clipboard));
        assert_eq!(area_of(&nodes).cursor, 1);
        assert!(type_into_focused(&mut nodes, KeyInput::MoveRight(false), &mut clipboard));
        assert_eq!(area_of(&nodes).cursor, 2);
    }

    #[test]
    fn shift_arrow_selects_then_copy_cut_paste_work_on_it() {
        let mut nodes = focused_area("hello");
        let mut clipboard = String::new();

        type_into_focused(&mut nodes, KeyInput::MoveLeft(true), &mut clipboard);
        type_into_focused(&mut nodes, KeyInput::MoveLeft(true), &mut clipboard);
        assert_eq!(area_of(&nodes).selected_text(), Some("lo".to_string()));

        type_into_focused(&mut nodes, KeyInput::Copy, &mut clipboard);
        assert_eq!(clipboard, "lo");
        // Copier ne modifie pas le texte.
        assert_eq!(area_of(&nodes).text, "hello");

        assert!(type_into_focused(&mut nodes, KeyInput::Cut, &mut clipboard));
        assert_eq!(area_of(&nodes).text, "hel");
        assert_eq!(clipboard, "lo");

        assert!(type_into_focused(&mut nodes, KeyInput::Paste, &mut clipboard));
        assert_eq!(area_of(&nodes).text, "hello");
    }

    #[test]
    fn ctrl_a_selects_everything_regardless_of_layout() {
        let mut nodes = focused_area("hello");
        let mut clipboard = String::new();
        // evdev 30 = position physique du "A" americain -> 'q' sur AZERTY,
        // mais Ctrl+cette-touche doit rester "selectionner tout" (la
        // combinaison suit la LETTRE produite par la disposition, pas la
        // position physique) - donc sur AZERTY c'est evdev 16 ('a') qu'il
        // faut presser pour Ctrl+A, pas evdev 30.
        let input = key_to_input(16, KeyboardLayout::Azerty, false, true);
        assert_eq!(input, Some(KeyInput::SelectAll));
        assert!(type_into_focused(&mut nodes, input.unwrap(), &mut clipboard));
        assert_eq!(area_of(&nodes).selected_text(), Some("hello".to_string()));
    }

    #[test]
    fn key_to_input_maps_the_us_qwerty_letter_row_and_digits() {
        assert_eq!(key_to_input(30, KeyboardLayout::Qwerty, false, false), Some(KeyInput::Char('a')));
        assert_eq!(key_to_input(2, KeyboardLayout::Qwerty, false, false), Some(KeyInput::Char('1')));
        assert_eq!(key_to_input(57, KeyboardLayout::Qwerty, false, false), Some(KeyInput::Char(' ')));
        assert_eq!(key_to_input(14, KeyboardLayout::Qwerty, false, false), Some(KeyInput::Backspace));
        assert_eq!(key_to_input(28, KeyboardLayout::Qwerty, false, false), Some(KeyInput::Enter));
        assert_eq!(key_to_input(999, KeyboardLayout::Qwerty, false, false), None);
    }

    #[test]
    fn shift_uppercases_letters_but_not_digits_or_space() {
        assert_eq!(key_to_input(30, KeyboardLayout::Qwerty, true, false), Some(KeyInput::Char('A')));
        assert_eq!(key_to_input(2, KeyboardLayout::Qwerty, true, false), Some(KeyInput::Char('1')));
        assert_eq!(key_to_input(57, KeyboardLayout::Qwerty, true, false), Some(KeyInput::Char(' ')));
    }

    #[test]
    fn ctrl_maps_c_x_v_to_copy_cut_paste_and_other_letters_to_nothing() {
        assert_eq!(key_to_input(46, KeyboardLayout::Qwerty, false, true), Some(KeyInput::Copy)); // 'c'
        assert_eq!(key_to_input(45, KeyboardLayout::Qwerty, false, true), Some(KeyInput::Cut)); // 'x'
        assert_eq!(key_to_input(47, KeyboardLayout::Qwerty, false, true), Some(KeyInput::Paste)); // 'v'
        assert_eq!(key_to_input(18, KeyboardLayout::Qwerty, false, true), None); // 'e' : pas de raccourci
    }

    #[test]
    fn ctrl_z_maps_to_undo() {
        assert_eq!(key_to_input(44, KeyboardLayout::Qwerty, false, true), Some(KeyInput::Undo)); // 'z'
    }

    #[test]
    fn ctrl_z_undoes_the_last_typed_character() {
        let mut nodes = focused_area("hi");
        let mut clipboard = String::new();
        type_into_focused(&mut nodes, KeyInput::Char('!'), &mut clipboard);
        assert_eq!(area_of(&nodes).text, "hi!");

        assert!(type_into_focused(&mut nodes, KeyInput::Undo, &mut clipboard));
        assert_eq!(area_of(&nodes).text, "hi");
    }

    #[test]
    fn select_all_copy_cut_paste_are_not_repeatable_but_everything_else_is() {
        assert!(!KeyInput::SelectAll.is_repeatable());
        assert!(!KeyInput::Copy.is_repeatable());
        assert!(!KeyInput::Cut.is_repeatable());
        assert!(!KeyInput::Paste.is_repeatable());
        assert!(KeyInput::Undo.is_repeatable());
        assert!(KeyInput::Backspace.is_repeatable());
        assert!(KeyInput::Char('a').is_repeatable());
        assert!(KeyInput::MoveLeft(false).is_repeatable());
    }

    #[test]
    fn navigation_keys_carry_the_shift_state_for_selection() {
        assert_eq!(key_to_input(105, KeyboardLayout::Qwerty, false, false), Some(KeyInput::MoveLeft(false)));
        assert_eq!(key_to_input(105, KeyboardLayout::Qwerty, true, false), Some(KeyInput::MoveLeft(true)));
        assert_eq!(key_to_input(106, KeyboardLayout::Qwerty, true, false), Some(KeyInput::MoveRight(true)));
        assert_eq!(key_to_input(102, KeyboardLayout::Qwerty, true, false), Some(KeyInput::Home(true)));
        assert_eq!(key_to_input(107, KeyboardLayout::Qwerty, true, false), Some(KeyInput::End(true)));
        assert_eq!(key_to_input(111, KeyboardLayout::Qwerty, false, false), Some(KeyInput::Delete));
    }

    #[test]
    fn key_to_input_swaps_a_z_and_m_comma_on_azerty() {
        // Position physique du "Q" americain (evdev 16) -> 'a' sur AZERTY,
        // et inversement la position du "A" americain (evdev 30) -> 'q'.
        assert_eq!(key_to_input(16, KeyboardLayout::Azerty, false, false), Some(KeyInput::Char('a')));
        assert_eq!(key_to_input(30, KeyboardLayout::Azerty, false, false), Some(KeyInput::Char('q')));
        // Le 'm' est deplace a la position du point-virgule americain,
        // et la position du 'm' americain devient une virgule.
        assert_eq!(key_to_input(39, KeyboardLayout::Azerty, false, false), Some(KeyInput::Char('m')));
        assert_eq!(key_to_input(50, KeyboardLayout::Azerty, false, false), Some(KeyInput::Char(',')));
    }

    #[test]
    fn key_to_input_swaps_y_and_z_on_qwertz() {
        assert_eq!(key_to_input(21, KeyboardLayout::Qwertz, false, false), Some(KeyInput::Char('z')));
        assert_eq!(key_to_input(44, KeyboardLayout::Qwertz, false, false), Some(KeyInput::Char('y')));
    }

    #[test]
    fn detect_keyboard_layout_falls_back_to_qwerty_without_crashing() {
        // Ne verifie pas de valeur precise (depend de la machine qui fait
        // tourner le test) - juste que l'appel ne panique jamais, y
        // compris si `localectl` est absent.
        let _ = detect_keyboard_layout();
    }

    #[test]
    fn any_focused_finds_a_focused_textarea_nested_in_a_container() {
        let mut nodes = vec![UiNode::Container(Container::new(
            layout(0.0, 0.0, 100.0, 100.0),
            Color::new(0, 0, 0, 255),
            vec![UiNode::TextArea(TextArea::new(
                layout(0.0, 0.0, 100.0, 100.0),
                Color::new(0, 0, 0, 255),
                Color::new(255, 255, 255, 255),
                String::new(),
            ))],
        ))];
        assert!(!any_focused(&nodes));

        match &mut nodes[0] {
            UiNode::Container(container) => match &mut container.children[0] {
                UiNode::TextArea(area) => area.focused = true,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
        assert!(any_focused(&nodes));
    }

    // ---------------------------------------------------------------
    // Integration flex/grid : le point le plus important de l'audit -
    // hit-testing et rendu (`ui::services::draw_ui`) doivent utiliser
    // EXACTEMENT le meme calcul de boite (`layout::managers::layout_manager::resolve_children`),
    // sinon un clic ne tomberait plus sur ce qui est visuellement dessine.
    // ---------------------------------------------------------------

    #[test]
    fn toggle_button_at_hits_the_right_button_inside_a_flex_row_container() {
        use azure_foundation::layout::models::layout_props::DisplayMode;

        // Conteneur flex sur toute la fenetre (100x100) avec 2 boutons qui
        // se partagent l'espace a parts egales par flex-grow (voir
        // `layout_manager::flex_layout`) -> bouton "a" occupe [0,50), "b"
        // occupe [50,100). Un simple positionnement Block (x/width en %)
        // donnerait exactement la meme geometrie ici ; ce test verifie que
        // le CHEMIN flex (nouveau) est bien celui emprunte par l'interaction.
        let mut container_layout = layout(0.0, 0.0, 100.0, 100.0);
        container_layout.display = DisplayMode::Flex;

        let mut a = layout(0.0, 0.0, 0.0, 100.0);
        a.flex_grow = 1.0;
        let mut b = layout(0.0, 0.0, 0.0, 100.0);
        b.flex_grow = 1.0;

        let mut nodes = vec![UiNode::Container(Container::new(
            container_layout,
            Color::new(0, 0, 0, 255),
            vec![
                UiNode::Button(Button::new(a, Color::new(0, 0, 0, 255), false, "a".to_string())),
                UiNode::Button(Button::new(b, Color::new(0, 0, 0, 255), false, "b".to_string())),
            ],
        ))];

        assert!(toggle_button_at(&mut nodes, 10, 10, (0, 0, 100, 100)));
        match &nodes[0] {
            UiNode::Container(c) => match (&c.children[0], &c.children[1]) {
                (UiNode::Button(a), UiNode::Button(b)) => {
                    assert!(a.state, "le clic a x=10 doit tomber sur le bouton 'a' (moitie gauche)");
                    assert!(!b.state);
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }

        assert!(toggle_button_at(&mut nodes, 60, 10, (0, 0, 100, 100)));
        match &nodes[0] {
            UiNode::Container(c) => match &c.children[1] {
                UiNode::Button(b) => assert!(b.state, "le clic a x=60 doit tomber sur le bouton 'b' (moitie droite)"),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn focus_textarea_at_hits_the_right_cell_inside_a_grid_container() {
        use azure_foundation::layout::models::layout_props::{DisplayMode, Track};

        // Grille 2 colonnes egales sur toute la fenetre (200x100), 2
        // textarea placees en flux automatique -> premiere cellule [0,100),
        // deuxieme [100,200).
        let mut container_layout = layout(0.0, 0.0, 100.0, 100.0);
        container_layout.display = DisplayMode::Grid;
        container_layout.grid_template_columns = vec![Track::Fr(1.0), Track::Fr(1.0)];

        let area = |text: &str| TextArea::new(layout(0.0, 0.0, 0.0, 0.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), text.to_string());

        let mut nodes = vec![UiNode::Container(Container::new(
            container_layout,
            Color::new(0, 0, 0, 255),
            vec![UiNode::TextArea(area("left")), UiNode::TextArea(area("right"))],
        ))];

        assert!(focus_textarea_at(&mut nodes, 150, 10, (0, 0, 200, 100)));
        match &nodes[0] {
            UiNode::Container(c) => match (&c.children[0], &c.children[1]) {
                (UiNode::TextArea(left), UiNode::TextArea(right)) => {
                    assert!(!left.focused, "x=150 est dans la 2e colonne, pas la 1ere");
                    assert!(right.focused);
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
