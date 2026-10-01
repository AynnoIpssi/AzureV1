// `langage` (Rust) sur les vraies sources d'Azure : trouver les crates et
// les fichiers, analyser, lire un fichier de tests, ecrire dedans, lire la
// sortie de cargo.
use crate::commun::*;
use azure_testeur::ecran::code_riche;
use azure_testeur::langage::rust::{cargo, lexique, Rust};
use azure_testeur::langage::{self, Etape, Langage, Operation, Test, TestSource};
use std::collections::BTreeMap;

const Z: &str = "langage";

pub fn mesurer() {
    eprintln!("\n=== langage ===");
    let Some(env) = environnement() else { return };
    let p = env.chemin.clone();
    mesure(Z, "detecter (Cargo.toml + crates)", true, || {
        let _ = langage::detecter(&p);
    });
    let crates = cargo::crates(&p);
    mesure(Z, &format!("cargo::crates ({} crates)", crates.len()), true, || {
        let _ = cargo::crates(&p);
    });
    let nb_fichiers: usize = crates.iter().map(|k| cargo::fichiers(&p.join(&k.dossier)).len()).sum();
    mesure(Z, &format!("cargo::fichiers de tous les crates ({nb_fichiers} .rs)"), true, || {
        for k in &crates {
            let _ = cargo::fichiers(&p.join(&k.dossier));
        }
    });
    let analyse = Rust.analyser(&p);
    // Au clic sur un projet, sur « Analyser », et apres CHAQUE ecriture de
    // l'atelier (`clics::modifier`).
    mesure(Z, &format!("analyser Azure ({} tests) - ouvrir/Analyser/chaque ecriture", analyse.tests.len()), true, || {
        let _ = Rust.analyser(&p);
    });

    // Chaque fichier de tests : ouvert dans l'atelier, et relu a CHAQUE
    // redessin quand un test est montre dans le detail.
    let mut fichiers: Vec<String> = analyse.tests.iter().map(|t| t.fichier.clone()).collect();
    fichiers.dedup();
    let mut par_fichier: Vec<(String, std::time::Duration)> = fichiers.iter().map(|f| (f.clone(), meilleur(3, || drop(Rust.lire_fichier(&p, f))))).collect();
    let total: std::time::Duration = par_fichier.iter().map(|x| x.1).sum();
    noter(Z, &format!("lire_fichier x {} fichiers (total)", par_fichier.len()), false, total);
    par_fichier.sort_by(|a, b| b.1.cmp(&a.1));
    for (f, d) in par_fichier.iter().take(3) {
        noter(Z, &format!("lire_fichier {f}"), true, *d);
    }
    let Some((gros, _)) = par_fichier.first().cloned() else { return };
    let src = std::fs::read_to_string(p.join(&gros)).unwrap();
    mesure(Z, &format!("lexique::jetons ({} Ko)", src.len() / 1024), true, || drop(lexique::jetons(&src)));
    mesure(Z, "lexique::carte (meme fichier)", true, || drop(lexique::carte(&src)));

    // Coloration facon Azure Note (le test ouvert, le detail).
    let lu = Rust.lire_fichier(&p, &gros).unwrap();
    let plus_long = lu.tests.iter().max_by_key(|t| t.code.len()).map(|t| t.code.clone()).unwrap_or_default();
    mesure(Z, &format!("code_riche du plus long test ({} lignes)", plus_long.lines().count()), true, || drop(code_riche(&plus_long)));
    mesure(Z, &format!("code_riche du code generique ({} lignes)", lu.generique.lines().count()), true, || drop(code_riche(&lu.generique)));

    let refs: Vec<&Test> = analyse.tests.iter().collect();
    mesure(Z, &format!("etape_tests ({} tests)", refs.len()), true, || drop(Rust.etape_tests(&refs)));
    mesure(Z, &format!("Test::cle x {}", analyse.tests.len()), true, || {
        for t in &analyse.tests {
            let _ = t.cle();
        }
    });

    // Lecture de la sortie de cargo (thread d'execution).
    let sortie: Vec<String> = std::iter::once("     Running unittests src/lib.rs (target/debug/deps/x-1)".to_string()).chain((0..20_000).map(|i| format!("test m::t_{i} ... {}", if i % 50 == 0 { "FAILED" } else { "ok" }))).collect();
    let krate = crates.iter().find(|k| k.lib().is_some()).cloned().unwrap_or_default();
    let etape = Etape { paquet: krate.dossier.clone(), ..Default::default() };
    mesure(Z, "LecteurCargo : 20 000 lignes de sortie", false, || {
        let mut l = Rust.lecteur(&p, &etape);
        for x in &sortie {
            let _ = l.ligne(x);
        }
        let _ = l.fin();
    });

    // Ecrire dans le code : sur une copie du plus gros fichier.
    let d = crate_demo("ecrire");
    std::fs::create_dir_all(d.join("tests")).unwrap();
    std::fs::write(d.join("tests/gros.rs"), &src).unwrap();
    let f = "tests/gros.rs".to_string();
    let test = |nom: &str| TestSource { nom: nom.to_string(), code: "assert_eq!(1 + 1, 2);".to_string(), options: BTreeMap::new(), ligne: 0 };
    noter(Z, "appliquer CreerTest (gros fichier)", true, chrono(|| Rust.appliquer(&d, &Operation::CreerTest { fichier: f.clone(), test: test("banc_nouveau") }).unwrap()).1);
    noter(Z, "appliquer ModifierTest", true, chrono(|| Rust.appliquer(&d, &Operation::ModifierTest { fichier: f.clone(), ancien: "banc_nouveau".into(), test: test("banc_renomme") }).unwrap()).1);
    noter(Z, "appliquer Generique", true, chrono(|| Rust.appliquer(&d, &Operation::Generique { fichier: f.clone(), code: lu.generique.clone() }).unwrap()).1);
    noter(Z, "appliquer SupprimerTest", true, chrono(|| Rust.appliquer(&d, &Operation::SupprimerTest { fichier: f.clone(), nom: "banc_renomme".into() }).unwrap()).1);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
#[ignore]
fn langage() {
    mesurer();
    fin_de_zone(Z);
}
