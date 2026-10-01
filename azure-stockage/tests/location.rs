// Choisir ou vivent les donnees : le dossier du daemon (argument, variable,
// config) et, par app, un dossier a part (ex. disque externe).
use azure_stockage::managers::stockage::AzureStockage;
use azure_stockage::{arg_value, resolve_root_from};
use std::path::{Path, PathBuf};

fn tmp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("stockage-location-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn global_root_priority_argument_then_env_then_config() {
    let config = Some("# config Azure\nroot = /srv/azure-config\n");
    let env = Some("/srv/azure-env".to_string());
    assert_eq!(resolve_root_from(&args(&["--root", "/srv/arg"]), env.clone(), config).unwrap(), Path::new("/srv/arg"));
    assert_eq!(resolve_root_from(&args(&["--root=/srv/arg2"]), env.clone(), config).unwrap(), Path::new("/srv/arg2"));
    assert_eq!(resolve_root_from(&[], env, config).unwrap(), Path::new("/srv/azure-env"));
    assert_eq!(resolve_root_from(&[], None, config).unwrap(), Path::new("/srv/azure-config"));
    assert!(resolve_root_from(&[], None, None).unwrap().ends_with("azure/stockage"));
    assert_eq!(arg_value(&args(&["--socket", "/tmp/x.sock"]), "--socket").as_deref(), Some("/tmp/x.sock"));
}

#[test]
fn an_app_moves_its_data_elsewhere_and_back() {
    let root = tmp("root");
    let disk = tmp("disque");
    let mut store = AzureStockage::open(&root).unwrap();
    store.put(1, "photo", b"pixels").unwrap();
    store.put(2, "autre", b"reste").unwrap();

    store.set_location(1, Some(&disk)).unwrap();
    assert_eq!(store.location(1), Some(disk.as_path()));
    assert_eq!(store.get(1, "photo").unwrap().as_deref(), Some(&b"pixels"[..]), "les donnees ont suivi");
    assert_eq!(std::fs::read_dir(&disk).unwrap().count(), 1, "l'espace de l'app 1 est sur le disque");
    store.put(1, "nouvelle", b"x").unwrap();

    // Le choix survit a un redemarrage du daemon.
    let mut store = AzureStockage::open(&root).unwrap();
    assert_eq!(store.keys(1).unwrap(), ["nouvelle", "photo"]);
    assert_eq!(store.location(2), None);
    assert_eq!(store.get(2, "autre").unwrap().as_deref(), Some(&b"reste"[..]), "les autres apps ne bougent pas");

    store.set_location(1, None).unwrap();
    assert_eq!(store.location(1), None);
    assert_eq!(store.keys(1).unwrap(), ["nouvelle", "photo"]);
    assert_eq!(std::fs::read_dir(&disk).unwrap().count(), 0);
}

#[test]
fn relative_or_occupied_locations_are_refused() {
    let root = tmp("refus");
    let mut store = AzureStockage::open(&root).unwrap();
    assert!(store.set_location(1, Some(Path::new("relatif/dossier"))).is_err());

    let disk = tmp("refus-disque");
    store.put(1, "a", b"1").unwrap();
    store.set_location(1, Some(&disk)).unwrap();
    store.set_location(1, None).unwrap();
    // Un espace laisse la par quelqu'un d'autre n'est jamais ecrase.
    let app_dir = std::fs::read_dir(root.join("private")).unwrap().next().unwrap().unwrap().file_name();
    std::fs::create_dir_all(disk.join(&app_dir)).unwrap();
    assert!(store.set_location(1, Some(&disk)).is_err());
    assert_eq!(store.get(1, "a").unwrap().as_deref(), Some(&b"1"[..]));
}
