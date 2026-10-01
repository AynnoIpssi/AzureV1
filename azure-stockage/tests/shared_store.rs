// Stockage partage : public, ou protege par des comptes (utilisateur + mot
// de passe + role) crees par l'app proprietaire.
use azure_core::models::storage_model::{Role, ShareAccess};
use azure_stockage::managers::stockage::{AzureStockage, Credentials, SharedInfo};
use std::path::PathBuf;

const OWNER: u32 = 1;
const OTHER: u32 = 2;

fn store(name: &str) -> AzureStockage {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("stockage-shared-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    AzureStockage::open(&root).unwrap()
}

fn creds<'a>(user: &'a str, password: &'a str) -> Option<Credentials<'a>> {
    Some(Credentials { user, password })
}

#[test]
fn public_data_is_readable_by_all_but_writable_only_by_its_owner() {
    let s = store("public");
    s.share(OWNER, "annonce", b"bonjour", ShareAccess::Public).unwrap();
    assert_eq!(s.read_shared(OTHER, OWNER, "annonce", None).unwrap(), b"bonjour");
    assert!(s.write_shared(OTHER, OWNER, "annonce", b"pirate", None).is_err());
    s.write_shared(OWNER, OWNER, "annonce", b"salut", None).unwrap();
    assert_eq!(s.read_shared(OTHER, OWNER, "annonce", None).unwrap(), b"salut");
}

#[test]
fn private_data_stays_private_even_with_the_same_name() {
    let s = store("separate");
    s.put(OWNER, "annonce", b"prive").unwrap();
    assert!(s.read_shared(OTHER, OWNER, "annonce", None).is_err(), "le prive n'est jamais partage tout seul");
}

#[test]
fn protected_data_needs_an_account_with_the_right_role() {
    let s = store("protected");
    s.share(OWNER, "photos", b"album", ShareAccess::Protected).unwrap();
    s.add_account(OWNER, "alice", "mdp-alice", Role::Reader).unwrap();
    s.add_account(OWNER, "bob", "mdp-bob", Role::Writer).unwrap();

    assert!(s.read_shared(OTHER, OWNER, "photos", None).is_err(), "sans compte");
    assert!(s.read_shared(OTHER, OWNER, "photos", creds("alice", "faux")).is_err(), "mauvais mot de passe");
    assert!(s.read_shared(OTHER, OWNER, "photos", creds("carol", "x")).is_err(), "compte inconnu");
    assert_eq!(s.read_shared(OTHER, OWNER, "photos", creds("alice", "mdp-alice")).unwrap(), b"album");

    assert!(s.write_shared(OTHER, OWNER, "photos", b"efface", creds("alice", "mdp-alice")).is_err(), "Reader ne peut pas ecrire");
    s.write_shared(OTHER, OWNER, "photos", b"album v2", creds("bob", "mdp-bob")).unwrap();
    assert_eq!(s.read_shared(OWNER, OWNER, "photos", None).unwrap(), b"album v2", "le proprietaire n'a pas besoin de compte");
}

#[test]
fn accounts_belong_to_one_app_and_can_be_removed() {
    let s = store("accounts");
    s.share(OWNER, "doc", b"x", ShareAccess::Protected).unwrap();
    s.share(OTHER, "doc", b"y", ShareAccess::Protected).unwrap();
    s.add_account(OWNER, "alice", "pw", Role::Reader).unwrap();

    assert!(s.read_shared(3, OTHER, "doc", creds("alice", "pw")).is_err(), "le compte d'une app n'ouvre pas les donnees d'une autre");
    assert_eq!(s.accounts(OWNER).unwrap(), [("alice".to_string(), Role::Reader)]);

    s.add_account(OWNER, "alice", "nouveau", Role::Writer).unwrap();
    assert_eq!(s.accounts(OWNER).unwrap(), [("alice".to_string(), Role::Writer)], "meme nom : remplace");
    assert!(s.read_shared(3, OWNER, "doc", creds("alice", "pw")).is_err(), "l'ancien mot de passe ne marche plus");

    assert!(s.remove_account(OWNER, "alice").unwrap());
    assert!(s.read_shared(3, OWNER, "doc", creds("alice", "nouveau")).is_err());
    assert!(s.add_account(OWNER, "vide", "", Role::Reader).is_err(), "mot de passe vide refuse");
}

#[test]
fn list_and_unshare() {
    let s = store("list");
    s.share(OWNER, "b", b"", ShareAccess::Protected).unwrap();
    s.share(OWNER, "a", b"", ShareAccess::Public).unwrap();
    s.share(OTHER, "c", b"", ShareAccess::Public).unwrap();
    assert_eq!(
        s.shared_list(OWNER).unwrap(),
        [SharedInfo { name: "a".into(), access: ShareAccess::Public }, SharedInfo { name: "b".into(), access: ShareAccess::Protected }]
    );
    assert!(s.unshare(OWNER, "a").unwrap());
    assert!(!s.unshare(OWNER, "a").unwrap());
    assert!(s.read_shared(OTHER, OWNER, "a", None).is_err());
}

#[test]
fn passwords_are_not_stored_in_clear() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stockage-shared-pw");
    let _ = std::fs::remove_dir_all(&root);
    let s = AzureStockage::open(&root).unwrap();
    s.add_account(OWNER, "alice", "motdepasse-unique-123", Role::Reader).unwrap();
    for entry in std::fs::read_dir(root.join("accounts")).unwrap() {
        let data = std::fs::read(entry.unwrap().path()).unwrap();
        assert!(!String::from_utf8_lossy(&data).contains("motdepasse-unique"));
        assert!(!String::from_utf8_lossy(&data).contains("alice"));
    }
}
