#[cfg(test)]
mod tests {
    
    
    use azure_foundation::window::models::header_bar::*;

    #[test]
    fn button_at_finds_each_button_in_right_to_left_order() {
        let width = 300;
        let layout = ButtonLayout::default_right();
        assert_eq!(button_at(width, &layout, (width - 1) as i32, 10), Some(HeaderButton::Close));
        assert_eq!(button_at(width, &layout, (width - BUTTON_WIDTH - 1) as i32, 10), Some(HeaderButton::Fullscreen));
        assert_eq!(button_at(width, &layout, (width - BUTTON_WIDTH * 2 - 1) as i32, 10), Some(HeaderButton::Minimize));
    }

    #[test]
    fn button_at_is_none_outside_any_button_or_below_the_header() {
        let width = 300;
        let layout = ButtonLayout::default_right();
        assert_eq!(button_at(width, &layout, 10, 10), None); // loin a gauche, dans l'en-tete mais hors bouton
        assert_eq!(button_at(width, &layout, (width - 1) as i32, (HEADER_HEIGHT + 5) as i32), None); // sous l'en-tete
        assert_eq!(button_at(width, &layout, -5, 10), None);
    }

    #[test]
    fn button_at_returns_none_when_the_window_is_too_narrow_for_all_three_buttons() {
        let layout = ButtonLayout::default_right();
        assert_eq!(button_at(BUTTON_WIDTH * 2, &layout, 10, 10), None);
    }

    #[test]
    fn button_at_finds_buttons_left_to_right_when_on_the_left() {
        let width = 300;
        let layout = ButtonLayout::mac();
        // Feux tricolores colles : une case de TRAFFIC_STEP px chacun, apres
        // une marge de TRAFFIC_START px.
        assert_eq!(button_at(width, &layout, 2, 10), None, "dans la marge");
        assert_eq!(button_at(width, &layout, (TRAFFIC_START + 5) as i32, 10), Some(HeaderButton::Close));
        assert_eq!(button_at(width, &layout, (TRAFFIC_START + TRAFFIC_STEP + 5) as i32, 10), Some(HeaderButton::Minimize));
        assert_eq!(button_at(width, &layout, (TRAFFIC_START + TRAFFIC_STEP * 2 + 5) as i32, 10), Some(HeaderButton::Fullscreen));
        assert_eq!(button_at(width, &layout, (TRAFFIC_START + TRAFFIC_STEP * 3 + 5) as i32, 10), None, "apres le dernier bouton");
    }

    #[test]
    fn button_at_handles_a_layout_with_fewer_than_three_buttons() {
        let width = 300;
        let layout = ButtonLayout { order: vec![HeaderButton::Close], on_left: false };
        assert_eq!(button_at(width, &layout, (width - 1) as i32, 10), Some(HeaderButton::Close));
        assert_eq!(button_at(width, &layout, (width - BUTTON_WIDTH - 1) as i32, 10), None);
    }

    #[test]
    fn content_box_starts_right_below_the_header_and_shrinks_height() {
        assert_eq!(content_box(800, 600), (0, HEADER_HEIGHT, 800, 600 - HEADER_HEIGHT));
    }

    #[test]
    fn content_box_never_underflows_when_the_window_is_shorter_than_the_header() {
        assert_eq!(content_box(800, 10), (0, HEADER_HEIGHT, 800, 0));
    }
}
