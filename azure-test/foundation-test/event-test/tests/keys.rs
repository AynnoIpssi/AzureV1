#[cfg(test)]
mod tests {
    use azure_foundation::event::models::keys::*;

    #[test]
    fn recognizes_both_shift_and_both_ctrl_keys() {
        assert!(is_shift_key(KEY_LEFTSHIFT));
        assert!(is_shift_key(KEY_RIGHTSHIFT));
        assert!(is_ctrl_key(KEY_LEFTCTRL));
        assert!(is_ctrl_key(KEY_RIGHTCTRL));
    }

    #[test]
    fn a_regular_letter_key_is_neither() {
        assert!(!is_shift_key(30)); // 'a'
        assert!(!is_ctrl_key(30));
    }
}
