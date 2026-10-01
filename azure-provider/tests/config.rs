use azure_provider::models::config::{load, parse};
use azure_provider::models::service::{builtin, Restart};
use std::path::Path;

#[test]
fn a_real_config_file_changes_disables_and_adds_services() {
    let services = load(Some(Path::new("tests/config/provider.conf"))).unwrap();
    let names: Vec<&str> = services.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["stockage", "service", "manager", "notes-sync"]);

    // Un daemon d'Azure : seul `args` change.
    let stockage = &services[0];
    assert_eq!(stockage.command, "stockage_daemon");
    assert_eq!(stockage.args, ["--root", "/mnt/disque azure"]);
    assert_eq!(stockage.health.as_deref(), Some(azure_core::paths::socket("stockage").as_str()));
    assert!(stockage.autostart);

    let notes = &services[3];
    assert_eq!(notes.command, "/usr/local/bin/notes-sync");
    assert_eq!(notes.args, ["--rapide", "--niveau", "2"]);
    assert_eq!(notes.health.as_deref(), Some("/tmp/notes-sync.sock"));
    assert_eq!(notes.restart, Restart::OnFailure);
    assert!(notes.autostart);
}

#[test]
fn no_file_means_the_azure_daemons() {
    assert_eq!(load(Some(Path::new("tests/config/absent.conf"))).unwrap(), builtin());
    assert_eq!(load(None).unwrap(), builtin());
}

#[test]
fn mistakes_are_reported_with_their_line() {
    let cases = [
        ("[x]\ncommand = a\nrestart = parfois", "ligne 3", "restart inconnu"),
        ("command = a", "ligne 1", "hors d'une section"),
        ("[x]\ncolor = bleu", "ligne 2", "cle inconnue"),
        ("[x]\ncommand = a\nargs = \"pas ferme", "ligne 3", "guillemet"),
        ("[x]\nautostart = peut-etre", "ligne 2", "true ou false"),
        ("[x]\nsans egal", "ligne 2", "cle = valeur"),
    ];
    for (text, line, message) in cases {
        let error = parse(text).unwrap_err();
        assert!(error.contains(line) && error.contains(message), "{text:?} -> {error}");
    }
    // Un nouveau service sans commande, un nom invalide.
    assert!(parse("[vide]\nautostart = true").unwrap_err().contains("commande vide"));
    assert!(parse("[Mon Service]\ncommand = a").unwrap_err().contains("Nom de service invalide"));
}

#[test]
fn an_app_file_only_adds_its_own_tasks() {
    let specs = azure_provider::models::config::parse_app("[docs-methodes]\ncommand = /apps/docs/azure_docs\nargs = --service\nrestart = on-failure\n").unwrap();
    assert_eq!(specs.len(), 1, "sans les daemons d'Azure");
    assert_eq!(specs[0].args, ["--service"]);
    assert!(azure_provider::models::config::parse_app("[rooter]\ncommand = /bin/sh\n").unwrap_err().contains("daemon d'Azure"));
}
