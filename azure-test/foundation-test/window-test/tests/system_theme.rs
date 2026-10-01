#[cfg(test)]
mod tests {
    use azure_foundation::window::models::header_bar::HeaderButton;
    use azure_foundation::window::services::system_theme::*;

    #[test]
    fn mac_style_layout_puts_all_three_buttons_on_the_left_in_close_minimize_maximize_order() {
        let layout = parse_button_layout("close,minimize,maximize:").unwrap();
        assert!(layout.on_left);
        assert_eq!(layout.order, vec![HeaderButton::Close, HeaderButton::Minimize, HeaderButton::Fullscreen]);
    }

    #[test]
    fn default_gnome_layout_puts_buttons_on_the_right() {
        let layout = parse_button_layout("appmenu:minimize,maximize,close").unwrap();
        assert!(!layout.on_left);
        assert_eq!(layout.order, vec![HeaderButton::Minimize, HeaderButton::Fullscreen, HeaderButton::Close]);
    }

    #[test]
    fn unrecognized_tokens_are_dropped_without_affecting_recognized_ones() {
        let layout = parse_button_layout("icon:spacer,minimize,close").unwrap();
        assert_eq!(layout.order, vec![HeaderButton::Minimize, HeaderButton::Close]);
    }

    #[test]
    fn empty_or_meaningless_value_falls_back_to_none() {
        assert_eq!(parse_button_layout(""), None);
        assert_eq!(parse_button_layout("appmenu:icon"), None);
    }
}
