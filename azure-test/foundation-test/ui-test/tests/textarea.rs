#[cfg(test)]
mod tests {
    use azure_engine::rendering::models::color::Color;
    use azure_foundation::ui::models::textarea::*;
    use azure_foundation::layout::models::layout_props::LayoutProps;

    fn area(text: &str) -> TextArea {
        TextArea::new(LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), text.to_string())
    }

    #[test]
    fn new_places_cursor_at_the_end_of_the_initial_text() {
        assert_eq!(area("hi").cursor, 2);
        assert_eq!(area("").cursor, 0);
    }

    #[test]
    fn insert_char_inserts_at_the_cursor_and_advances_it() {
        let mut a = area("hllo");
        a.cursor = 1;
        a.insert_char('e');
        assert_eq!(a.text, "hello");
        assert_eq!(a.cursor, 2);
    }

    #[test]
    fn backspace_removes_the_character_before_the_cursor() {
        let mut a = area("hello");
        a.cursor = 3;
        assert!(a.backspace());
        assert_eq!(a.text, "helo");
        assert_eq!(a.cursor, 2);
    }

    #[test]
    fn backspace_at_the_start_does_nothing() {
        let mut a = area("hello");
        a.cursor = 0;
        assert!(!a.backspace());
        assert_eq!(a.text, "hello");
    }

    #[test]
    fn delete_forward_removes_the_character_after_the_cursor_without_moving_it() {
        let mut a = area("hello");
        a.cursor = 2;
        assert!(a.delete_forward());
        assert_eq!(a.text, "helo");
        assert_eq!(a.cursor, 2);
    }

    #[test]
    fn delete_forward_at_the_end_does_nothing() {
        let mut a = area("hello");
        a.cursor = 5;
        assert!(!a.delete_forward());
    }

    #[test]
    fn move_left_and_right_without_shift_just_move_the_cursor() {
        let mut a = area("hello");
        a.cursor = 3;
        a.move_left(false);
        assert_eq!(a.cursor, 2);
        assert_eq!(a.selection_anchor, None);
        a.move_right(false);
        assert_eq!(a.cursor, 3);
    }

    #[test]
    fn move_left_cannot_go_below_zero() {
        let mut a = area("hi");
        a.cursor = 0;
        a.move_left(false);
        assert_eq!(a.cursor, 0);
    }

    #[test]
    fn move_right_cannot_exceed_the_text_length() {
        let mut a = area("hi");
        a.cursor = 2;
        a.move_right(false);
        assert_eq!(a.cursor, 2);
    }

    #[test]
    fn shift_left_extends_a_selection_from_the_cursor() {
        let mut a = area("hello");
        a.cursor = 5;
        a.move_left(true);
        a.move_left(true);
        assert_eq!(a.selection_range(), Some((3, 5)));
        assert_eq!(a.selected_text(), Some("lo".to_string()));
    }

    #[test]
    fn moving_without_shift_after_a_selection_collapses_to_the_touched_edge() {
        let mut a = area("hello");
        a.cursor = 5;
        a.move_left(true);
        a.move_left(true);
        // Selection [3,5) ; un mouvement sans shift vers la gauche colle au bord gauche.
        a.move_left(false);
        assert_eq!(a.cursor, 3);
        assert_eq!(a.selection_anchor, None);
    }

    #[test]
    fn select_all_selects_the_entire_text() {
        let mut a = area("hello");
        a.cursor = 2;
        a.select_all();
        assert_eq!(a.selection_range(), Some((0, 5)));
        assert_eq!(a.selected_text(), Some("hello".to_string()));
    }

    #[test]
    fn insert_char_replaces_the_current_selection() {
        let mut a = area("hello");
        a.select_all();
        a.insert_char('x');
        assert_eq!(a.text, "x");
        assert_eq!(a.cursor, 1);
        assert_eq!(a.selection_anchor, None);
    }

    #[test]
    fn insert_str_pastes_at_the_cursor_and_advances_past_it() {
        let mut a = area("ac");
        a.cursor = 1;
        a.insert_str("b");
        assert_eq!(a.text, "abc");
        assert_eq!(a.cursor, 2);
    }

    #[test]
    fn insert_str_replaces_the_selection_like_insert_char() {
        let mut a = area("hello");
        a.select_all();
        a.insert_str("bye");
        assert_eq!(a.text, "bye");
        assert_eq!(a.cursor, 3);
    }

    #[test]
    fn home_and_end_move_to_the_bounds_and_clear_selection() {
        let mut a = area("hello");
        a.cursor = 2;
        a.select_all();
        a.move_home(false);
        assert_eq!(a.cursor, 0);
        assert_eq!(a.selection_anchor, None);
        a.move_end(false);
        assert_eq!(a.cursor, 5);
    }

    #[test]
    fn works_correctly_with_multi_byte_utf8_characters() {
        let mut a = area("caf");
        a.cursor = 3;
        a.insert_char('é');
        assert_eq!(a.text, "café");
        assert_eq!(a.cursor, 4);
        a.backspace();
        assert_eq!(a.text, "caf");
        assert_eq!(a.cursor, 3);
    }

    #[test]
    fn undo_restores_the_text_and_cursor_from_before_the_last_edit() {
        let mut a = area("hi");
        a.insert_char('!');
        assert_eq!(a.text, "hi!");
        assert!(a.undo());
        assert_eq!(a.text, "hi");
        assert_eq!(a.cursor, 2);
    }

    #[test]
    fn undo_can_step_back_through_several_edits_one_at_a_time() {
        let mut a = area("");
        a.insert_char('a');
        a.insert_char('b');
        a.insert_char('c');
        assert_eq!(a.text, "abc");
        assert!(a.undo());
        assert_eq!(a.text, "ab");
        assert!(a.undo());
        assert_eq!(a.text, "a");
        assert!(a.undo());
        assert_eq!(a.text, "");
        assert!(!a.undo()); // plus rien a annuler
    }

    #[test]
    fn undo_does_nothing_on_a_fresh_textarea() {
        let mut a = area("hello");
        assert!(!a.undo());
        assert_eq!(a.text, "hello");
    }

    #[test]
    fn no_op_backspace_and_delete_do_not_create_an_undo_step() {
        let mut a = area("hi");
        a.cursor = 0;
        a.backspace(); // rien a effacer avant le curseur
        assert!(!a.undo());

        a.cursor = 2;
        a.delete_forward(); // rien a effacer apres le curseur
        assert!(!a.undo());
    }

    #[test]
    fn undo_after_replacing_a_selection_is_a_single_step() {
        let mut a = area("hello");
        a.select_all();
        a.insert_char('x');
        assert_eq!(a.text, "x");
        // Une seule etape : on revient directement a "hello" avec la
        // selection d'origine, pas a un etat intermediaire "vide".
        assert!(a.undo());
        assert_eq!(a.text, "hello");
        assert_eq!(a.cursor, 5);
    }

    #[test]
    fn cut_selection_returns_the_text_removes_it_and_is_undoable() {
        let mut a = area("hello");
        a.select_all();
        assert_eq!(a.cut_selection(), Some("hello".to_string()));
        assert_eq!(a.text, "");
        assert!(a.undo());
        assert_eq!(a.text, "hello");
    }

    #[test]
    fn cut_selection_is_none_without_a_selection() {
        let mut a = area("hello");
        assert_eq!(a.cut_selection(), None);
        assert_eq!(a.text, "hello");
    }
}
