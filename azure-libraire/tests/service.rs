// Les services de la librairie, appeles directement (sans azure-service).
use azure_libraire::service::{appeler, manifeste, methode, service, services, Valeur};

fn args<const N: usize>(champs: [(&str, Valeur); N]) -> Valeur {
    Valeur::table(champs)
}

#[test]
fn le_catalogue_est_coherent() {
    let mut noms = Vec::new();
    for s in services() {
        assert!(!s.description.is_empty() && !s.methodes.is_empty(), "{}", s.nom);
        for m in s.methodes {
            let nom = s.nom_complet(m);
            // Les regles d'un nom de methode dans un manifeste.
            assert!(nom.len() <= 128 && nom.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')), "{nom}");
            assert!(!m.description.is_empty() && !m.arguments.is_empty() && !m.reponse.is_empty(), "{nom}");
            assert!(methode(&nom).is_some(), "{nom}");
            noms.push(nom);
        }
    }
    let total = noms.len();
    noms.sort();
    noms.dedup();
    assert_eq!(noms.len(), total, "deux methodes portent le meme nom");
    assert!(service("diff").is_some() && service("rien").is_none());
    assert!(appeler("diff-rien", &Valeur::Rien).unwrap_err().contains("inconnue"));
}

#[test]
fn le_manifeste_declare_chaque_methode() {
    let texte = manifeste(services(), "methodes", "public = true");
    for s in services() {
        for m in s.methodes {
            assert!(texte.contains(&format!("[provide {}]\npublic = true\nservice = methodes\ndescription = ", s.nom_complet(m))));
        }
    }
    assert!(!manifeste(&[service("temps").unwrap()], "", "to = note").contains("service ="));
}

#[test]
fn diff() {
    let a = args([("avant", "un\ndeux\ntrois\n".into()), ("apres", "un\nDEUX\ntrois\n".into())]);
    let lignes = appeler("diff-lignes", &a).unwrap();
    let l = lignes.en_liste().unwrap();
    assert_eq!(l.len(), 4);
    assert_eq!(l[1], args([("genre", "retire".into()), ("texte", "deux".into()), ("ancien", 2usize.into()), ("nouveau", Valeur::Rien)]));
    let blocs = appeler("diff-blocs", &a).unwrap();
    assert_eq!(blocs.champ("ajoutees"), Some(&Valeur::Entier(1)));
    assert_eq!(blocs.champ("blocs").unwrap().en_liste().unwrap()[0].texte("entete").unwrap(), "@@ -1,3 +1,3 @@");
    assert_eq!(appeler("diff-unifie", &a).unwrap().en_texte().unwrap(), "--- avant\n+++ apres\n@@ -1,3 +1,3 @@\n un\n-deux\n+DEUX\n trois\n");
    let mots = appeler("diff-mots", &args([("avant", "a = 1".into()), ("apres", "a = 2".into())])).unwrap();
    assert_eq!(mots.en_liste().unwrap().len(), 3);
    assert!(appeler("diff-lignes", &args([("avant", "x".into())])).unwrap_err().contains("« apres »"));
}

#[test]
fn code() {
    let source = "fn aide() {}\nimpl Boite {\n    fn ouvrir(&self) {\n        aide();\n    }\n}\n";
    let f = appeler("code-fonctions", &args([("chemin", "src/boite.rs".into()), ("source", source.into())])).unwrap();
    let fonctions = f.champ("fonctions").unwrap().en_liste().unwrap();
    assert_eq!(fonctions.len(), 2);
    assert_eq!(fonctions[1].texte("proprietaire").unwrap(), "Boite");
    assert_eq!(fonctions[1].champ("appels").unwrap().en_liste().unwrap()[0].texte("nom").unwrap(), "aide");
    assert!(appeler("code-fonctions", &args([("chemin", "notes.txt".into()), ("source", "".into())])).unwrap_err().contains("langage non lu"));
    let couleurs = appeler("code-colorer", &args([("langage", "rust".into()), ("code", "let x = 1;\n// fin".into())])).unwrap();
    let lignes = couleurs.en_liste().unwrap();
    assert_eq!(lignes.len(), 2);
    assert_eq!(lignes[1].en_liste().unwrap()[0].texte("genre").unwrap(), "commentaire");
}

#[test]
fn archive_tableur_et_empreinte() {
    let fichiers = Valeur::liste([args([("nom", "a.csv".into()), ("texte", "nom;âge\nLéa;31\n".into())]), args([("nom", "b.bin".into()), ("base64", "AAEC".into())])]);
    let zip = appeler("archive-creer", &args([("fichiers", fichiers)])).unwrap();
    let code = zip.texte("base64").unwrap().to_string();
    assert_eq!(appeler("archive-lister", &args([("base64", code.clone().into())])).unwrap(), Valeur::from(vec!["a.csv", "b.bin"]));
    let extrait = appeler("archive-extraire", &args([("base64", code.clone().into()), ("nom", "a.csv".into())])).unwrap();
    let texte = appeler("encodage-texte", &args([("base64", extrait.texte("base64").unwrap().into())])).unwrap();
    assert_eq!(appeler("tableur-csv", &args([("texte", texte)])).unwrap(), Valeur::from(vec![vec!["nom", "âge"], vec!["Léa", "31"]]));
    assert_eq!(appeler("archive-extraire", &args([("base64", code.into()), ("nom", "absent".into())])).unwrap(), Valeur::Rien);
    assert!(appeler("archive-lister", &args([("texte", "pas un zip".into())])).is_err());
    assert!(appeler("tableur-xlsx", &args([("base64", "AAEC".into())])).unwrap_err().contains(".xlsx"));

    assert_eq!(appeler("empreinte-calculer", &args([("texte", "abc".into())])).unwrap().en_texte().unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(appeler("empreinte-calculer", &args([("base64", "YWJj".into()), ("algo", "sha1".into())])).unwrap().en_texte().unwrap(), "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert_eq!(appeler("empreinte-calculer", &args([("texte", "123456789".into()), ("algo", "crc32".into())])).unwrap().en_texte().unwrap(), "cbf43926");
    assert!(appeler("empreinte-calculer", &args([("texte", "x".into()), ("algo", "sha512".into())])).unwrap_err().contains("algo inconnu"));
}

#[test]
fn encodage_et_temps() {
    assert_eq!(appeler("encodage-base64", &args([("texte", "foobar".into())])).unwrap().en_texte().unwrap(), "Zm9vYmFy");
    assert_eq!(appeler("encodage-sans-accent", &args([("texte", "été à Nîmes".into())])).unwrap().en_texte().unwrap(), "ete a Nimes");
    let lu = appeler("temps-lire", &args([("texte", "2026-10-06 12:00:00".into())])).unwrap();
    assert_eq!(lu.texte("date").unwrap(), "2026-10-06");
    assert_eq!(appeler("temps-ecrire", &args([("secondes", lu.champ("secondes").unwrap().clone())])).unwrap(), lu);
    assert!(appeler("temps-lire", &args([("texte", "demain".into())])).is_err());
    assert!(appeler("temps-maintenant", &Valeur::Rien).unwrap().champ("secondes").unwrap().en_entier().unwrap() > 1_700_000_000);
}
