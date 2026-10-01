#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};
    use azure_foundation::event::models::app_state::EventState;
    use azure_foundation::event::models::keys::BTN_LEFT;
    use azure_foundation::ui::services::interact::KeyboardLayout;
    use azure_core::rules::window_event::WindowEvent;
    use azure_foundation::event::services::dispatch::*;
    use azure_foundation::layout::models::layout_props::LayoutProps;
    use azure_foundation::ui::models::button::Button;
    use azure_foundation::ui::models::textarea::TextArea;
    use azure_foundation::ui::models::ui_node::UiNode;
    use azure_engine::rendering::models::color::Color;

    fn layout(x: f32, y: f32, w: f32, h: f32) -> LayoutProps {
        LayoutProps::new(x, y, w, h, 0.0, 0.0)
    }

    fn button_state(text: &str) -> EventState {
        EventState::new(vec![UiNode::Button(Button::new(layout(0.0, 0.0, 100.0, 100.0), Color::new(0, 0, 0, 255), false, text.to_string()))])
    }

    fn textarea_state(text: &str) -> EventState {
        EventState::new(vec![UiNode::TextArea(TextArea::new(
            layout(0.0, 0.0, 100.0, 100.0),
            Color::new(0, 0, 0, 255),
            Color::new(255, 255, 255, 255),
            text.to_string(),
        ))])
    }

    #[test]
    fn clicking_a_button_toggles_its_state_and_reports_a_change() {
        let mut state = button_state("ok");
        state.mouse_x = 10;
        state.mouse_y = 10;

        let changed = handle_event(state_mut(&mut state), WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, (0, 0, 100, 100));
        assert!(changed);
        match &state.ui_nodes[0] {
            UiNode::Button(b) => assert!(b.state),
            _ => unreachable!(),
        }
    }

    // Petit alias local : evite de repeter `&mut EventState` explicitement
    // partout dans les tests ci-dessous, purement cosmetique.
    fn state_mut(state: &mut EventState) -> &mut EventState {
        state
    }

    #[test]
    fn clicking_then_typing_into_a_textarea_works_end_to_end() {
        let mut state = textarea_state("");
        state.mouse_x = 10;
        state.mouse_y = 10;

        assert!(handle_event(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, (0, 0, 100, 100)));
        match &state.ui_nodes[0] {
            UiNode::TextArea(a) => assert!(a.focused),
            _ => unreachable!(),
        }

        // evdev 30 = 'a' en QWERTY.
        assert!(handle_event(&mut state, WindowEvent::WindowKeyPress(30, true), KeyboardLayout::Qwerty, (0, 0, 100, 100)));
        match &state.ui_nodes[0] {
            UiNode::TextArea(a) => assert_eq!(a.text, "a"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn releasing_the_left_button_stops_dragging() {
        let mut state = textarea_state("hello");
        handle_event(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, (0, 0, 100, 100));
        assert!(state.dragging);

        handle_event(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, (0, 0, 100, 100));
        assert!(!state.dragging);
    }

    #[test]
    fn shift_and_ctrl_key_presses_update_modifiers_without_typing_a_character() {
        let mut state = textarea_state("");
        // evdev 30 = position "A" americaine -> focus d'abord pour pouvoir observer un non-changement de texte.
        state.mouse_x = 10;
        state.mouse_y = 10;
        handle_event(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, (0, 0, 100, 100));

        assert!(!handle_event(&mut state, WindowEvent::WindowKeyPress(42, true), KeyboardLayout::Qwerty, (0, 0, 100, 100))); // KEY_LEFTSHIFT
        assert!(state.shift_held());
        match &state.ui_nodes[0] {
            UiNode::TextArea(a) => assert_eq!(a.text, ""),
            _ => unreachable!(),
        }

        handle_event(&mut state, WindowEvent::WindowKeyPress(42, false), KeyboardLayout::Qwerty, (0, 0, 100, 100));
        assert!(!state.shift_held());
    }

    #[test]
    fn handle_tick_blinks_the_caret_only_when_a_textarea_is_focused() {
        let mut state = textarea_state("hi");
        // Rien de focalise : pas de clignotement, meme apres le delai.
        state.last_blink = Instant::now() - CARET_BLINK_INTERVAL - Duration::from_millis(1);
        assert!(!handle_tick(&mut state, KeyboardLayout::Qwerty, (0, 0, 100, 100)));

        match &mut state.ui_nodes[0] {
            UiNode::TextArea(a) => a.focused = true,
            _ => unreachable!(),
        }
        state.last_blink = Instant::now() - CARET_BLINK_INTERVAL - Duration::from_millis(1);
        let was_visible = state.caret_visible;
        assert!(handle_tick(&mut state, KeyboardLayout::Qwerty, (0, 0, 100, 100)));
        assert_ne!(state.caret_visible, was_visible);
    }

    #[test]
    fn handle_tick_repeats_a_held_key_once_the_initial_delay_has_passed() {
        let mut state = textarea_state("");
        state.mouse_x = 10;
        state.mouse_y = 10;
        handle_event(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, (0, 0, 100, 100));
        handle_event(&mut state, WindowEvent::WindowKeyPress(30, true), KeyboardLayout::Qwerty, (0, 0, 100, 100)); // 'a'
        assert_eq!(text_of(&state), "a");

        // Simule le delai initial deja ecoule.
        state.next_repeat_at = Some(Instant::now() - Duration::from_millis(1));
        assert!(handle_tick(&mut state, KeyboardLayout::Qwerty, (0, 0, 100, 100)));
        assert_eq!(text_of(&state), "aa");
    }

    fn text_of(state: &EventState) -> &str {
        match &state.ui_nodes[0] {
            UiNode::TextArea(a) => &a.text,
            _ => unreachable!(),
        }
    }
}
