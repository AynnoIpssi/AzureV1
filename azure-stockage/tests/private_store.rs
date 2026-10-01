// Espace prive de chaque app : chiffre, lisible seulement par elle.
use azure_stockage::managers::stockage::AzureStockage;
use std::path::PathBuf;

fn fresh_root(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("stockage-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn all_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() { all_files(&path, out) } else { out.push(path) }
    }
}

#[test]
fn put_get_delete_and_list() {
    let store = AzureStockage::open(&fresh_root("crud")).unwrap();
    store.put(1, "theme", b"sombre").unwrap();
    store.put(1, "langue", b"fr").unwrap();
    assert_eq!(store.get(1, "theme").unwrap().as_deref(), Some(&b"sombre"[..]));
    assert_eq!(store.keys(1).unwrap(), ["langue", "theme"]);

    store.put(1, "theme", b"clair").unwrap();
    assert_eq!(store.get(1, "theme").unwrap().as_deref(), Some(&b"clair"[..]));

    assert!(store.delete(1, "theme").unwrap());
    assert!(!store.delete(1, "theme").unwrap());
    assert_eq!(store.get(1, "theme").unwrap(), None);
    assert!(store.put(1, "", b"x").is_err(), "cle vide refusee");
}

#[test]
fn an_app_never_sees_another_apps_private_data() {
    let store = AzureStockage::open(&fresh_root("isolation")).unwrap();
    store.put(1, "secret", b"a moi").unwrap();
    assert_eq!(store.get(2, "secret").unwrap(), None);
    assert!(store.keys(2).unwrap().is_empty());
    store.put(2, "secret", b"a lui").unwrap();
    assert_eq!(store.get(1, "secret").unwrap().as_deref(), Some(&b"a moi"[..]));
}

#[test]
fn data_survives_a_restart_and_nothing_is_readable_on_disk() {
    let root = fresh_root("disk");
    AzureStockage::open(&root).unwrap().put(7, "mot-de-passe-wifi", b"hunter2-tres-secret").unwrap();
    let store = AzureStockage::open(&root).unwrap();
    assert_eq!(store.get(7, "mot-de-passe-wifi").unwrap().as_deref(), Some(&b"hunter2-tres-secret"[..]));

    let mut files = Vec::new();
    all_files(&root, &mut files);
    for file in files {
        let data = std::fs::read(&file).unwrap();
        let text = String::from_utf8_lossy(&data);
        assert!(!text.contains("hunter2") && !text.contains("wifi"), "{} contient du clair", file.display());
        assert!(!file.to_string_lossy().contains("wifi"), "le nom de cle ne doit pas apparaitre dans les chemins");
    }
}

#[test]
fn a_file_copied_over_another_key_is_refused() {
    let root = fresh_root("moved");
    let store = AzureStockage::open(&root).unwrap();
    store.put(1, "a", b"valeur a").unwrap();
    store.put(1, "b", b"valeur b").unwrap();
    let mut files = Vec::new();
    all_files(&root.join("private"), &mut files);
    std::fs::copy(&files[0], &files[1]).unwrap();

    // Les deux fichiers contiennent maintenant la meme cle : celle qui est
    // a sa place se lit, l'autre est refusee (au lieu de rendre la valeur
    // d'une autre cle).
    let results = [store.get(1, "a"), store.get(1, "b")];
    assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1, "{results:?}");
    assert!(store.keys(1).is_err());
}

#[test]
fn a_modified_byte_is_refused() {
    let root = fresh_root("tamper");
    let store = AzureStockage::open(&root).unwrap();
    store.put(1, "a", b"valeur a").unwrap();
    let mut files = Vec::new();
    all_files(&root.join("private"), &mut files);
    let mut data = std::fs::read(&files[0]).unwrap();
    data[20] ^= 1;
    std::fs::write(&files[0], data).unwrap();
    let err = store.get(1, "a").unwrap_err();
    assert!(err.contains("alteree"), "{err}");
}

#[test]
fn directory_and_key_file_are_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt;
    let root = fresh_root("perms");
    AzureStockage::open(&root).unwrap();
    assert_eq!(std::fs::metadata(&root).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(root.join("master.key")).unwrap().permissions().mode() & 0o777, 0o600);
}
