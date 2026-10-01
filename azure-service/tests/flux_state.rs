use azure_service::flux::path::Path;
use azure_service::flux::{Change, Filter, Value};

fn apply_all(changes: &[Change]) -> Value {
    let mut state = Value::empty_map();
    for c in changes {
        c.apply(&mut state).unwrap();
    }
    state
}

#[test]
fn set_creates_missing_objects_and_push_creates_lists() {
    let state = apply_all(&[
        Change::set("panier.total", 42),
        Change::push("panier.items", Value::map([("nom", "pomme".into())])),
        Change::push("panier.items", Value::map([("nom", "poire".into())])),
        Change::set("panier.items.1.prix", 3.5),
    ]);
    assert_eq!(state.to_string(), r#"{"panier": {"items": [{"nom": "pomme"}, {"nom": "poire", "prix": 3.5}], "total": 42}}"#);
    assert_eq!(state.get("panier.items.1.nom").and_then(Value::as_str), Some("poire"));
    assert_eq!(state.get("panier.total").and_then(Value::as_i64), Some(42));
    assert_eq!(state.get("panier.rien"), None);
}

#[test]
fn delete_shifts_lists_and_ignores_missing_values() {
    let mut state = apply_all(&[Change::set("l", vec![1, 2, 3]), Change::set("a.b", true)]);
    Change::delete("l.0").apply(&mut state).unwrap();
    Change::delete("a.b").apply(&mut state).unwrap();
    Change::delete("pas.la").apply(&mut state).unwrap();
    Change::delete("l.9").apply(&mut state).unwrap();
    assert_eq!(state.to_string(), r#"{"a": {}, "l": [2, 3]}"#);
    // Racine : tout vider, ou tout remplacer par un objet.
    Change::delete("").apply(&mut state).unwrap();
    assert_eq!(state, Value::empty_map());
    Change::set("", Value::map([("x", 1.into())])).apply(&mut state).unwrap();
    assert_eq!(state.to_string(), r#"{"x": 1}"#);
}

#[test]
fn list_index_rules() {
    let mut state = apply_all(&[Change::set("l", vec![1, 2])]);
    // Juste apres la fin : ajoute.
    Change::set("l.2", 3).apply(&mut state).unwrap();
    assert_eq!(state.get("l").unwrap().to_string(), "[1, 2, 3]");
    assert!(Change::set("l.7", 0).apply(&mut state).unwrap_err().contains("pas de case 7"));
    assert!(Change::set("l.x", 0).apply(&mut state).is_err());
}

#[test]
fn invalid_changes_are_refused() {
    let mut state = apply_all(&[Change::set("n", 5)]);
    assert!(Change::set("n.sous", 1).apply(&mut state).unwrap_err().contains("ni un objet ni une liste"));
    assert!(Change::push("n", 1).apply(&mut state).unwrap_err().contains("pas une liste"));
    assert!(Change::set("", 3).apply(&mut state).unwrap_err().contains("attend un objet"));
    assert!(Change::set("a..b", 1).apply(&mut state).unwrap_err().contains("segment vide"));
    assert!(Change::set("a.*", 1).apply(&mut state).unwrap_err().contains("motifs"));
    assert!(Change::event("", 1).validate().is_err());
    // Un evenement ne change pas l'etat.
    let before = state.clone();
    Change::event("bip", "x").apply(&mut state).unwrap();
    assert_eq!(state, before);
}

#[test]
fn values_and_changes_survive_the_wire() {
    use azure_core::models::wire::{Reader, Writer};
    let value = Value::map([
        ("t", "texte".into()),
        ("n", Value::Null),
        ("f", (-1.25).into()),
        ("i", i64::MIN.into()),
        ("l", Value::list([true.into(), Value::empty_map()])),
    ]);
    let changes = [Change::Set("a.b".into(), value.clone()), Change::delete("x"), Change::push("l", 1), Change::event("e", "v")];
    for change in changes {
        let bytes = change.write(Writer::new()).finish();
        assert_eq!(Change::read(&mut Reader::new(&bytes)).unwrap(), change);
    }
    // Une valeur trop imbriquee est refusee (pas de debordement de pile).
    let mut deep = Value::Null;
    for _ in 0..100 {
        deep = Value::list([deep]);
    }
    let bytes = deep.write(Writer::new()).finish();
    assert!(Value::read(&mut Reader::new(&bytes)).unwrap_err().contains("trop imbriquee"));
}

#[test]
fn patterns_match_parents_children_and_wildcards() {
    let pattern = Path::pattern("panier.items.*.prix").unwrap();
    for (path, expected) in [
        ("panier", true),                // parent : peut tout changer
        ("", true),                      // racine
        ("panier.items.3.prix", true),
        ("panier.items.3", true),
        ("panier.items.3.prix.centimes", true),
        ("panier.items.3.nom", false),
        ("panier.total", false),
        ("client", false),
    ] {
        assert_eq!(pattern.related(&Path::parse(path).unwrap()), expected, "{path}");
    }
}

#[test]
fn a_filter_keeps_only_what_the_listener_asked_for() {
    let state = apply_all(&[
        Change::set("panier.total", 42),
        Change::set("panier.client", "Ana"),
        Change::set("stats.a", Value::map([("vues", 3.into()), ("clics", 1.into())])),
        Change::set("stats.b", Value::map([("vues", 7.into()), ("clics", 2.into())])),
        Change::set("items", vec![1, 2]),
    ]);
    let filter = Filter { paths: vec![Path::pattern("panier.total").unwrap(), Path::pattern("stats.*.vues").unwrap(), Path::pattern("items.0").unwrap()], events: vec!["paye".into()] };
    assert_eq!(filter.extract(&state).to_string(), r#"{"items": [1, 2], "panier": {"total": 42}, "stats": {"a": {"vues": 3}, "b": {"vues": 7}}}"#);

    assert!(filter.wants(&Change::set("panier.total", 1)));
    assert!(filter.wants(&Change::delete("panier")));
    assert!(!filter.wants(&Change::set("panier.client", "Bo")));
    assert!(filter.wants(&Change::event("paye", 1)));
    assert!(!filter.wants(&Change::event("autre", 1)));
    // Aucun filtre : tout.
    assert!(Filter::default().wants(&Change::event("autre", 1)));
    assert_eq!(Filter::default().extract(&state), state);
}
