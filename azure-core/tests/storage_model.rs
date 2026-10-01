use azure_core::models::storage_model::*;

#[test]
fn storage_keys_are_validated() {
    assert_eq!(StorageKey::new("theme").unwrap().as_str(), "theme");
    assert!(StorageKey::new("").is_err());
    assert!(StorageKey::new("a\u{1F}b").is_err());
    assert!(StorageKey::new("ligne\nsuivante").is_err());
    assert!(StorageKey::new(&"x".repeat(MAX_KEY_LEN)).is_ok());
    assert!(StorageKey::new(&"x".repeat(MAX_KEY_LEN + 1)).is_err());
}

#[test]
fn values_have_a_size_limit() {
    assert!(check_value(&[0; 10]).is_ok());
    assert!(check_value(&vec![0; MAX_VALUE_LEN + 1]).is_err());
}

#[test]
fn roles_and_access_round_trip_their_codes() {
    for role in [Role::Reader, Role::Writer] {
        assert_eq!(Role::from_code(role.code()), Some(role));
    }
    for access in [ShareAccess::Public, ShareAccess::Protected] {
        assert_eq!(ShareAccess::from_code(access.code()), Some(access));
    }
    assert_eq!(Role::from_code(9), None);
    assert!(Role::Reader.can_read() && !Role::Reader.can_write());
    assert!(Role::Writer.can_read() && Role::Writer.can_write());
}
