use azure_core::models::window_model::*;

fn size() -> WindowSize {
    WindowSize::new(800, 600).unwrap()
}

#[test]
fn a_window_size_cannot_be_zero() {
    assert!(WindowSize::new(0, 600).is_err());
    assert!(WindowSize::new(800, 0).is_err());
}

#[test]
fn an_internal_window_is_only_for_its_app() {
    assert!(WindowSpec::new(1, size(), WindowState::Active, WindowScope::Owner, WindowKind::Internal).is_ok());
    assert!(WindowSpec::new(1, size(), WindowState::Active, WindowScope::Followers, WindowKind::Internal).is_err());
    assert!(WindowSpec::new(1, size(), WindowState::Active, WindowScope::All, WindowKind::Internal).is_err());
}

#[test]
fn an_external_or_inter_app_window_must_leave_its_app() {
    for kind in [WindowKind::External, WindowKind::InterApp] {
        assert!(WindowSpec::new(1, size(), WindowState::Active, WindowScope::Owner, kind).is_err());
        assert!(WindowSpec::new(1, size(), WindowState::Active, WindowScope::Followers, kind).is_ok());
        assert!(WindowSpec::new(1, size(), WindowState::Active, WindowScope::All, kind).is_ok());
    }
}

#[test]
fn the_internal_shortcut_matches_the_full_constructor() {
    let full = WindowSpec::new(1, size(), WindowState::Active, WindowScope::Owner, WindowKind::Internal).unwrap();
    assert_eq!(WindowSpec::internal(1, size()), full);
}

#[test]
fn only_the_state_changes_after_creation() {
    let mut spec = WindowSpec::internal(1, size());
    spec.set_state(WindowState::Background);
    assert_eq!(spec.state(), WindowState::Background);
    assert_eq!(spec.owner_app_id(), 1);
    assert_eq!(spec.size(), size());
}

#[test]
fn visibility_follows_the_scope() {
    let owner = WindowSpec::internal(1, size());
    let followers = WindowSpec::new(1, size(), WindowState::Active, WindowScope::Followers, WindowKind::External).unwrap();
    let all = WindowSpec::new(1, size(), WindowState::Active, WindowScope::All, WindowKind::External).unwrap();

    // l'app creatrice voit toujours sa fenetre
    for spec in [owner, followers, all] {
        assert!(spec.visible_to(1, false));
    }
    assert!(!owner.visible_to(2, true));
    assert!(followers.visible_to(2, true));
    assert!(!followers.visible_to(2, false));
    assert!(all.visible_to(2, false));
}

#[test]
fn wire_codes_round_trip() {
    for scope in [WindowScope::Owner, WindowScope::Followers, WindowScope::All] {
        assert_eq!(WindowScope::from_code(scope.code()), Some(scope));
    }
    for state in [WindowState::Active, WindowState::Background] {
        assert_eq!(WindowState::from_code(state.code()), Some(state));
    }
    for kind in [WindowKind::Internal, WindowKind::External, WindowKind::InterApp] {
        assert_eq!(WindowKind::from_code(kind.code()), Some(kind));
    }
    assert_eq!(WindowScope::from_code(9), None);
}
