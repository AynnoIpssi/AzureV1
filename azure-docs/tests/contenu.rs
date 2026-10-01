// Regles du contenu, verifiees sur tout contenu/ : les exemples rsH, rsC et RsS
// sont lus par les vrais parseurs ; chaque bloc de code a un
// identifiant `<section>.<page>.<n>` unique (n = 1, 2, 3... dans l'ordre de
// la page), un langage connu ; chaque page a un titre et un resume. Et la
// recherche retrouve un exemple par son identifiant (ce que fera l'IDE).
use azure_docs::index::{chercher, Genre};
use azure_docs::Docs;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use std::collections::HashSet;
use std::path::Path;

const LANGAGES: &[&str] = &["rsh", "rsc", "rust", "toml", "sh", "texte", "rss"];

fn docs() -> Docs {
    Docs::charger(&Path::new(env!("CARGO_MANIFEST_DIR")).join("contenu")).unwrap()
}

#[test]
fn chaque_exemple_a_un_identifiant_unique_et_bien_forme() {
    let docs = docs();
    let mut vus = HashSet::new();
    let mut total = 0;
    for page in docs.pages() {
        for (i, e) in page.exemples().enumerate() {
            let attendu = format!("{}.{}.{}", page.section, page.id, i + 1);
            assert_eq!(e.id, attendu, "{} : identifiants numerotes dans l'ordre de la page", page.fichier.display());
            assert!(vus.insert(e.id.clone()), "identifiant en double : {}", e.id);
            assert!(LANGAGES.contains(&e.langage.as_str()), "{} : langage inconnu '{}' ({LANGAGES:?})", e.id, e.langage);
            assert!(!e.titre.is_empty(), "{} : titre manquant", e.id);
            assert!(!e.code.trim().is_empty(), "{} : code vide", e.id);
            total += 1;
        }
    }
    assert!(total > 0);
}

#[test]
fn chaque_page_a_un_titre_un_resume_et_un_identifiant_utilisable() {
    let docs = docs();
    assert!(!docs.sections.is_empty());
    for s in &docs.sections {
        assert!(!s.titre.is_empty() && !s.resume.is_empty(), "section {} : titre et resume", s.id);
        assert!(!s.pages.is_empty(), "section {} vide", s.id);
        for p in &s.pages {
            assert!(!p.resume.is_empty(), "{} : resume manquant", p.fichier.display());
            // `__` separe section et page dans les #id des boutons.
            for id in [&s.id, &p.id] {
                assert!(!id.contains("__") && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'), "identifiant '{id}' : minuscules, chiffres, tirets");
            }
        }
    }
}

#[test]
fn la_recherche_trouve_un_exemple_par_son_identifiant() {
    let docs = docs();
    for page in docs.pages() {
        for e in page.exemples() {
            let r = chercher(&docs, &e.id);
            assert_eq!(r.first().map(|r| (r.genre.clone(), r.id.as_str())), Some((Genre::Exemple, e.id.as_str())), "{}", e.id);
            assert_eq!(docs.exemple(&e.id).map(|(p, _)| p.chemin()), Some(page.chemin()));
        }
    }
}

#[test]
fn la_recherche_ignore_accents_et_majuscules() {
    let docs = docs();
    let premiere = docs.pages().next().unwrap();
    let mot = premiere.titre.split_whitespace().next().unwrap();
    let a = chercher(&docs, &mot.to_uppercase());
    let b = chercher(&docs, &azure_docs::index::normaliser(mot));
    assert!(!a.is_empty());
    assert_eq!(a.iter().map(|r| &r.id).collect::<Vec<_>>(), b.iter().map(|r| &r.id).collect::<Vec<_>>());
}

#[test]
fn une_page_mal_formee_est_signalee() {
    use azure_docs::contenu::lire_page;
    assert!(lire_page("s", "p", "resume: sans titre\n").is_err());
    let e = lire_page("s", "p", "titre: T\nresume: R\n\n```rsh\n<text>x<!text>\n```\n").unwrap_err();
    assert!(e.contains("ligne 4") && e.contains("identifiant"), "{e}");
    assert!(lire_page("s", "p", "titre: T\n\n```rsh s.p.1 \"x\"\njamais ferme\n").unwrap_err().contains("jamais ferme"));
}

#[test]
fn les_exemples_rsh_rsc_et_rss_sont_lus_par_les_vrais_parseurs() {
    // Un exemple faux dans la doc est un bug de la doc : chaque exemple rsH,
    // rsC et RsS passe par le meme parseur qu'une app, et une declaration rsC
    // ignoree (propriete inconnue, valeur illisible) est une erreur.
    let docs = docs();
    for page in docs.pages() {
        for e in page.exemples() {
            match e.langage.as_str() {
                "rsh" => {
                    if let Err(err) = parse_rsh(tokenize_rsh(&e.code)) {
                        panic!("{} : rsH illisible : {err:?}", e.id);
                    }
                }
                "rss" => {
                    if let Err(err) = azure_stockage::rss::parser::parse(&e.code) {
                        panic!("{} : RsS illisible : {err}", e.id);
                    }
                }
                "rsc" => {
                    let sheet = parse_rsc(tokenize_rsc(&e.code)).unwrap_or_else(|err| panic!("{} : rsC illisible : {err:?}", e.id));
                    let warnings = azure_foundation::compiler::rsc::warnings(&sheet);
                    assert!(warnings.is_empty(), "{} : {warnings:?}", e.id);
                }
                _ => {}
            }
        }
    }
}
