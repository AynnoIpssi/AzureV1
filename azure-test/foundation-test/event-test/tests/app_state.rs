#[cfg(test)]
mod tests {
    
    
    use azure_foundation::event::models::keys::{KEY_LEFTCTRL, KEY_LEFTSHIFT};
    
    
    use azure_foundation::event::models::app_state::*;

    #[test]
    fn new_starts_with_nothing_held_and_the_caret_visible() {
        let state = EventState::new(Vec::new());
        assert!(!state.shift_held());
        assert!(!state.ctrl_held());
        assert!(state.caret_visible);
        assert!(!state.dragging);
        assert_eq!(state.held_key, None);
    }

    #[test]
    fn shift_held_and_ctrl_held_reflect_held_modifiers() {
        let mut state = EventState::new(Vec::new());
        state.held_modifiers.insert(KEY_LEFTSHIFT);
        assert!(state.shift_held());
        assert!(!state.ctrl_held());

        state.held_modifiers.insert(KEY_LEFTCTRL);
        assert!(state.ctrl_held());
    }
}
