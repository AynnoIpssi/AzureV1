use azure_manager::models::manifest::Manifest;
use azure_provider::Restart;
use std::path::Path;

#[test]
fn a_real_manifest_declares_everything() {
    let m = Manifest::load(Path::new("tests/apps/boutique/app.azure")).unwrap();
    let dir = Path::new("tests/apps/boutique");
    assert_eq!((m.name.as_str(), m.title.as_str(), m.version.as_str()), ("boutique", "Ma boutique", "1.2"));

    let w = m.window.as_ref().unwrap();
    assert_eq!((w.width, w.height, w.start.as_deref()), (900, 600, Some("/accueil")));
    assert_eq!(w.style.as_deref(), Some(dir.join("ui/app.rsc").as_path()), "relatif au manifeste");

    assert_eq!(m.routes.len(), 2);
    assert_eq!(m.routes[0].view, dir.join("ui/accueil.rsh"));
    assert_eq!(m.routes[0].name.as_deref(), Some("accueil"));
    assert_eq!(m.routes[1].path, "/produit/{id}");
    assert_eq!(m.routes[1].style.as_deref(), Some(dir.join("ui/produit.rsc").as_path()));

    assert_eq!(m.share("panier").unwrap().to, ["caisse", "stats"]);
    assert!(m.share("annonces").unwrap().public);
    let listen = m.listen("commandes", "caisse").unwrap();
    assert_eq!(listen.paths, ["total", "items.*.prix"]);
    assert_eq!(listen.events, ["paye"]);

    let services = m.provider_services();
    assert_eq!(services[0].name, "boutique-sync", "prefixe par le nom de l'app");
    assert_eq!(services[0].args, ["--rapide", "deux mots"]);
    assert_eq!(services[0].restart, Restart::OnFailure);
    assert_eq!(m.storage_location.as_deref(), Some(Path::new("/mnt/disque/boutique")));
}

#[test]
fn a_minimal_manifest_is_just_a_name() {
    let m = Manifest::parse("[app]\nname = notes\n", Path::new("/x")).unwrap();
    assert_eq!((m.title.as_str(), m.version.as_str()), ("notes", "0"));
    assert!(m.window.is_none() && m.routes.is_empty() && m.shares.is_empty());
}

#[test]
fn mistakes_are_reported_clearly() {
    let cases = [
        ("[app]\ntitle = x", "name manquant"),
        ("[app]\nname = Ma App", "Nom d'app invalide"),
        ("[app]\nname = a\n[fenetre]", "ligne 3 : section inconnue [fenetre]"),
        ("[app]\nname = a\ncouleur = bleu", "ligne 3 : cle inconnue 'couleur'"),
        ("[app]\nname = a\n[route accueil]\nview = a.rsh", "ligne 3 : route 'accueil' : un chemin commence par /"),
        ("[app]\nname = a\n[route /a]\nview = a.rsh", "[route /a] : style manquant"),
        ("[app]\nname = a\n[listen panier]", "ligne 3 : listen 'panier' : <flux>@<app> attendu"),
        ("[app]\nname = a\n[share p]\nto = Caisse", "ligne 4 : Nom d'app invalide : 'Caisse'"),
        ("[app]\nname = a\n[share p]\n[share p]", "[share p] declare deux fois"),
        ("[app]\nname = a\n[service s]\nargs = x", "[service s] : command manquant"),
        ("[app]\nname = a\n[window]\nwidth = large", "ligne 4 : nombre attendu"),
        ("[app]\nname = a\n[storage]\nlocation = relatif", "chemin absolu attendu"),
        ("name = a", "ligne 1 : cle hors d'une section"),
        ("[app]\nname = a\n[use prix]", "ligne 3 : use 'prix' : <methode>@<app> attendu"),
        ("[app]\nname = a\n[provide p]\n[provide p]", "[provide p] declare deux fois"),
        ("[app]\nname = a\n[use p@b]\nto = c", "ligne 4 : cle inconnue 'to'"),
    ];
    for (text, expected) in cases {
        let error = Manifest::parse(text, Path::new("/x")).unwrap_err();
        assert!(error.contains(expected), "{text:?} -> {error}");
    }
}


#[test]
fn methods_provided_and_used() {
    let m = Manifest::parse("[app]\nname = boutique\n[provide prix]\nto = caisse, stats\ndescription = Prix d'un produit\n[provide stock]\npublic = oui\n[use adresse@clients]", Path::new("/x")).unwrap();
    let prix = m.provide("prix").unwrap();
    assert_eq!(prix.to, ["caisse", "stats"]);
    assert_eq!(prix.description, "Prix d'un produit");
    assert!(m.provide("stock").unwrap().public);
    assert!(m.uses("adresse", "clients") && !m.uses("adresse", "boutique"));
    // Le resume garde tout (c'est ce que le manager recoit).
    let summary = m.summary();
    let bytes = summary.write(azure_core::models::wire::Writer::new()).finish();
    assert_eq!(azure_manager::models::manifest::Summary::read(&mut azure_core::models::wire::Reader::new(&bytes)).unwrap(), summary);
}

#[test]
fn a_method_can_be_served_by_a_background_task() {
    let dir = std::env::temp_dir().join(format!("azure-manifest-service-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("azure_docs"), "").unwrap();
    let text = "[app]\nname = docs\nexec = azure_docs\n[service methodes]\ncommand = azure_docs\nargs = --service\nrestart = on-failure\n[service autre]\ncommand = sleep\n[provide chercher]\nto = note\nservice = methodes";
    let m = Manifest::parse(text, &dir).unwrap();
    assert_eq!(m.provide("chercher").unwrap().service, "methodes");
    assert_eq!(m.on_demand_services(), ["docs-methodes"]);
    let services = m.provider_services();
    assert_eq!(services[0].command, dir.join("azure_docs").to_string_lossy(), "le binaire de l'app, par son chemin complet");
    assert_eq!(services[1].command, "sleep");
    // Garde dans le resume (format 4).
    let summary = m.summary();
    let bytes = summary.write(azure_core::models::wire::Writer::new()).finish();
    assert_eq!(azure_manager::models::manifest::Summary::read(&mut azure_core::models::wire::Reader::new(&bytes)).unwrap(), summary);
    // Une tache qui n'existe pas.
    let error = Manifest::parse("[app]\nname = docs\n[provide chercher]\nservice = fantome", &dir).unwrap_err();
    assert!(error.contains("aucun [service fantome]"), "{error}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn apps_it_can_open() {
    let m = Manifest::parse("[app]\nname = portfolio\n[open docs]\n[open note]\n[open docs]", Path::new("/x")).unwrap();
    assert_eq!(m.opens, ["docs", "note"], "sans doublon");
    let summary = m.summary();
    let bytes = summary.write(azure_core::models::wire::Writer::new()).finish();
    assert_eq!(azure_manager::models::manifest::Summary::read(&mut azure_core::models::wire::Reader::new(&bytes)).unwrap(), summary);
    assert!(Manifest::parse("[app]\nname = a\n[open]", Path::new("/x")).unwrap_err().contains("nom d'app"));
    assert!(Manifest::parse("[app]\nname = a\n[open docs]\nx = 1", Path::new("/x")).unwrap_err().contains("cle inconnue"));
}
