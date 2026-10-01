// Le coeur du manager, sans socket.
use azure_manager::managers::manager::{Kind, Manager, FIRST_ID};
use azure_manager::models::manifest::Manifest;
use azure_service::flux::{Access, Value};
use std::path::{Path, PathBuf};

const EXE: &str = "/apps/exe";

fn summary(text: &str) -> azure_manager::models::manifest::Summary {
    Manifest::parse(text, Path::new("/x")).unwrap().summary()
}

fn boutique() -> azure_manager::models::manifest::Summary {
    summary("[app]\nname = boutique\n[share panier]\nto = caisse\n[share annonces]\npublic = true")
}

fn caisse() -> azure_manager::models::manifest::Summary {
    summary("[app]\nname = caisse\n[listen panier@boutique]\npaths = total")
}

fn stats() -> azure_manager::models::manifest::Summary {
    summary("[app]\nname = stats\n[listen panier@boutique]")
}

fn text(v: &Value, path: &str) -> String {
    match v.get(path) {
        Some(Value::Text(t)) => t.clone(),
        Some(other) => other.to_string(),
        None => "?".into(),
    }
}

#[test]
fn apps_get_ids_by_name_and_keep_them() {
    let mut m = Manager::in_memory(vec![]);
    let b = m.register(boutique(), EXE, None, 10).unwrap();
    let c = m.register(caisse(), EXE, None, 11).unwrap();
    assert_eq!((b, c), (FIRST_ID, FIRST_ID + 1));
    assert_eq!(m.register(boutique(), EXE, None, 12).unwrap(), b, "meme nom = meme id");
    assert_eq!(m.resolve("caisse").unwrap(), c);
    assert!(m.resolve("inconnue").unwrap_err().contains("jamais lancee"));

    // Un autre executable ne peut pas prendre le nom.
    let error = m.register(boutique(), "/autre/exe", None, 13).unwrap_err();
    assert!(error.contains("appartient a l'executable /apps/exe") && error.contains("forget boutique"), "{error}");
}

#[test]
fn access_follows_the_manifest_then_the_dashboard() {
    let mut m = Manager::in_memory(vec![]);
    m.register(boutique(), EXE, None, 1).unwrap();
    // `caisse` pas encore lancee : pas d'id, pas encore dans l'acces.
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Apps(vec![]));
    let c = m.register(caisse(), EXE, None, 2).unwrap();
    let s = m.register(stats(), EXE, None, 3).unwrap();
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Apps(vec![c]));
    assert_eq!(m.access(Kind::Flux, "boutique", "annonces").unwrap(), Access::Public);
    assert!(m.access(Kind::Flux, "boutique", "secret").unwrap_err().contains("ne declare pas [share secret]"));

    m.grant(Kind::Flux, "boutique", "panier", "stats").unwrap();
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Apps(vec![c, s]));
    m.revoke(Kind::Flux, "boutique", "panier", "caisse").unwrap();
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Apps(vec![s]));
    m.set_public(Kind::Flux, "boutique", "panier", true).unwrap();
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Public);
    m.reset(Kind::Flux, "boutique", "panier").unwrap();
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Apps(vec![c]), "retour au manifeste");

    assert!(m.grant(Kind::Flux, "boutique", "panier", "fantome").is_err());
    let pushes = m.pushes();
    assert_eq!(pushes.len(), 2);
}

#[test]
fn the_state_shows_apps_links_and_who_is_running() {
    let mut m = Manager::in_memory(vec!["/apps/tableau".into()]);
    m.register(boutique(), EXE, None, 1).unwrap();
    m.register(caisse(), EXE, None, 2).unwrap();
    m.register(stats(), EXE, None, 3).unwrap();
    let t = m.register(summary("[app]\nname = tableau"), "/apps/tableau", None, 4).unwrap();
    assert_eq!(m.admin_ids(), [t]);
    m.disconnected("stats");

    let state = m.state(&[], false);
    assert_eq!(text(&state, "nb_apps"), "4");
    assert_eq!(text(&state, "nb_actives"), "3");
    assert_eq!(text(&state, "apps.0.nom"), "boutique");
    assert_eq!(text(&state, "apps.0.partages.0.flux"), "panier");
    assert_eq!(text(&state, "apps.0.partages.0.autorises"), r#"["caisse"]"#);
    assert_eq!(text(&state, "apps.0.partages.0.ecoutent"), r#"["caisse", "stats"]"#);
    assert_eq!(text(&state, "apps.0.partages.0.autres"), r#"["stats", "tableau"]"#);
    assert_eq!(text(&state, "apps.2.nom"), "stats");
    assert_eq!(text(&state, "apps.2.actif"), "false");
    let links: Vec<String> = state.get("liens").unwrap().as_list().unwrap().iter().map(|l| format!("{}->{} {} {}", text(l, "de"), text(l, "vers"), text(l, "nom"), text(l, "statut"))).collect();
    assert_eq!(links, ["boutique->caisse panier autorisé", "boutique->stats panier refusé"]);
}

#[test]
fn everything_is_kept_on_disk() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("manager-disk");
    let _ = std::fs::remove_dir_all(&dir);
    {
        let mut m = Manager::open(&dir, vec![]).unwrap();
        m.register(boutique(), EXE, None, 1).unwrap();
        m.register(caisse(), EXE, None, 2).unwrap();
        m.register(stats(), EXE, None, 3).unwrap();
        m.revoke(Kind::Flux, "boutique", "panier", "caisse").unwrap();
        m.grant(Kind::Flux, "boutique", "panier", "stats").unwrap();
    }
    let mut m = Manager::open(&dir, vec![]).unwrap();
    let s = m.resolve("stats").unwrap();
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Apps(vec![s]));
    assert_eq!(m.app("caisse").unwrap().sessions, 0, "au redemarrage, rien ne tourne");
    // Nouvelles apps : les ids continuent.
    assert_eq!(m.register(summary("[app]\nname = neuve"), EXE, None, 5).unwrap(), FIRST_ID + 3);

    // Oublier une app qui tourne : refuse ; sinon son nom est libre.
    assert!(m.forget("neuve").unwrap_err().contains("tourne encore"));
    m.forget("caisse").unwrap();
    assert!(m.register(caisse(), "/autre", None, 6).is_ok());
}


#[test]
fn methods_are_declared_and_granted_like_flux() {
    let mut m = Manager::in_memory(vec![]);
    let b = m.register(summary("[app]\nname = boutique\n[provide prix]\nto = caisse\ndescription = Prix d'un produit\n[provide stock]\npublic = true"), EXE, None, 1).unwrap();
    let c = m.register(summary("[app]\nname = caisse\n[use prix@boutique]"), EXE, None, 2).unwrap();
    let s = m.register(summary("[app]\nname = stats\n[use prix@boutique]\n[use inconnue@boutique]"), EXE, None, 3).unwrap();
    assert_eq!(m.access(Kind::Call, "boutique", "prix").unwrap(), Access::Apps(vec![c]));
    assert_eq!(m.access(Kind::Call, "boutique", "stock").unwrap(), Access::Public);
    assert!(m.access(Kind::Call, "boutique", "panier").unwrap_err().contains("[provide panier]"));
    // Meme nom cote flux : rien a voir.
    assert!(m.access(Kind::Flux, "boutique", "prix").is_err());

    m.grant(Kind::Call, "boutique", "prix", "stats").unwrap();
    assert_eq!(m.access(Kind::Call, "boutique", "prix").unwrap(), Access::Apps(vec![c, s]));
    m.revoke(Kind::Call, "boutique", "prix", "caisse").unwrap();
    assert_eq!(m.access(Kind::Call, "boutique", "prix").unwrap(), Access::Apps(vec![s]));
    let pushes = m.pushes();
    assert!(pushes.iter().any(|p| p.kind == Kind::Call && p.owner == b && p.name == "prix" && p.access == Access::Apps(vec![s])));

    let state = m.state(&[], true);
    assert_eq!(text(&state, "apps.0.fournit.0.methode"), "prix");
    assert_eq!(text(&state, "apps.0.fournit.0.description"), "Prix d'un produit");
    assert_eq!(text(&state, "apps.0.fournit.0.appelants"), r#"["caisse", "stats"]"#);
    assert_eq!(text(&state, "apps.0.fournit.0.modifie"), "true");
    assert_eq!(text(&state, "apps.1.utilise.0.methode"), "prix");
    let links: Vec<String> = state.get("liens").unwrap().as_list().unwrap().iter().map(|l| format!("{} {} {} de {} : {}", text(l, "vers"), text(l, "verbe"), text(l, "nom"), text(l, "de"), text(l, "statut"))).collect();
    assert_eq!(links, ["caisse appelle prix de boutique : refusé", "stats appelle prix de boutique : autorisé", "stats appelle inconnue de boutique : non déclaré"]);
}

#[test]
fn a_registry_written_before_calls_existed_still_opens() {
    use azure_core::models::wire::Writer;
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("manager-v1");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Format 1 : resume sans [provide]/[use], choix sans genre.
    let v1 = Writer::new()
        .u32(1).u32(1001).u32(1)
        .u32(1000).str(EXE).str("boutique").str("Boutique").str("1").u32(1).str("panier").u32(1).str("caisse").u8(0).u32(0).u32(0)
        .u32(1).str("boutique").str("panier").u8(0).u32(1).str("stats").u32(0)
        .finish();
    std::fs::write(dir.join("manager.bin"), v1).unwrap();
    let mut m = Manager::open(&dir, vec![]).unwrap();
    assert_eq!(m.resolve("boutique").unwrap(), 1000);
    let c = m.register(caisse(), EXE, None, 1).unwrap();
    let s = m.register(stats(), EXE, None, 2).unwrap();
    assert_eq!(c, 1001, "les ids continuent");
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Apps(vec![c, s]), "choix du tableau de bord repris");
    // Reecrit au format 2, relu sans perte.
    drop(m);
    let m = Manager::open(&dir, vec![]).unwrap();
    assert_eq!(m.access(Kind::Flux, "boutique", "panier").unwrap(), Access::Apps(vec![c, s]));
}

#[test]
fn waking_a_closed_app_needs_the_right_to_call_it() {
    let mut m = Manager::in_memory(vec![]);
    let docs = summary("[app]\nname = docs\nexec = azure_docs\n[service methodes]\ncommand = azure_docs\nargs = --service\n[provide chercher]\nto = note\nservice = methodes\n[provide seule]\nto = note");
    m.register(docs, EXE, None, 1).unwrap();
    m.disconnected("docs");
    m.register(summary("[app]\nname = note\n[use chercher@docs]\n[use seule@docs]"), EXE, None, 2).unwrap();
    m.register(summary("[app]\nname = autre\n[use chercher@docs]"), EXE, None, 3).unwrap();

    assert_eq!(m.wake_service("note", "docs", "chercher").unwrap(), "docs-methodes");
    assert!(m.wake_service("autre", "docs", "chercher").unwrap_err().contains("ne permet pas a 'autre'"));
    assert!(m.wake_service("note", "docs", "seule").unwrap_err().contains("aucun service"));
    assert!(m.wake_service("note", "docs", "rien").unwrap_err().contains("ne declare pas [provide rien]"));
    // Le tableau de bord ouvre l'acces : le reveil suit.
    m.grant(Kind::Call, "docs", "chercher", "autre").unwrap();
    assert_eq!(m.wake_service("autre", "docs", "chercher").unwrap(), "docs-methodes");
}

#[test]
fn opening_an_app_needs_open_in_the_manifest_and_an_installed_target() {
    let mut m = Manager::in_memory(vec![]);
    m.install(summary("[app]\nname = docs"), "/apps/docs/azure_docs", [7; 32]).unwrap();
    m.register(summary("[app]\nname = portfolio\n[open docs]\n[open note]"), EXE, None, 1).unwrap();
    m.register(summary("[app]\nname = autre"), EXE, None, 2).unwrap();

    assert_eq!(m.open_check("portfolio", "docs"), Ok(true), "installee et fermee : a lancer");
    assert!(m.open_check("autre", "docs").unwrap_err().contains("[open docs]"));
    assert!(m.open_check("portfolio", "note").unwrap_err().contains("pas installee"));
    // Deja ouverte : rien a lancer (la page arrive par le routeur).
    m.register(summary("[app]\nname = docs"), "/apps/docs/azure_docs", Some([7; 32]), 3).unwrap();
    assert_eq!(m.open_check("portfolio", "docs"), Ok(false));
}
