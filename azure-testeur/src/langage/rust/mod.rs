// Rust (cargo) : les tests `#[test]` des crates du projet, lances par
// `cargo test`, et ecrits par l'atelier directement dans les fichiers .rs.
pub mod cargo;
pub mod lexique;

use crate::langage::{Analyse, Etape, FichierTests, Langage, Lecteur, Operation, OptionTest, Paquet, Test, TestSource};
use lexique::{carte, Carte, Trouve};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub struct Rust;

const OPTIONS: &[OptionTest] = &[
    OptionTest { code: "ignore", libelle: "Ignoré par défaut (#[ignore])" },
    OptionTest { code: "panique", libelle: "Doit paniquer (#[should_panic])" },
];

/// Attribut Rust de chaque option.
fn attribut(option: &str) -> Option<(&'static str, &'static str)> {
    match option {
        "ignore" => Some(("ignore", "#[ignore]")),
        "panique" => Some(("should_panic", "#[should_panic]")),
        _ => None,
    }
}

fn options_de(t: &Trouve) -> BTreeMap<String, String> {
    OPTIONS.iter().filter(|o| attribut(o.code).is_some_and(|(chemin, _)| t.a(chemin))).map(|o| (o.code.to_string(), "oui".to_string())).collect()
}

const MOTS_RESERVES: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

impl Langage for Rust {
    fn code(&self) -> &'static str {
        "rust"
    }

    fn nom(&self) -> &'static str {
        "Rust · cargo"
    }

    fn reconnait(&self, projet: &Path) -> bool {
        projet.join("Cargo.toml").is_file() || !cargo::crates(projet).is_empty()
    }

    fn analyser(&self, projet: &Path) -> Analyse {
        let mut analyse = Analyse::default();
        for krate in cargo::crates(projet) {
            let dossier = projet.join(&krate.dossier);
            for fichier in cargo::fichiers(&dossier) {
                let Some((cible, base)) = krate.racine_de(&fichier) else { continue };
                let src = match std::fs::read_to_string(dossier.join(&fichier)) {
                    Ok(s) => s,
                    Err(e) => {
                        analyse.avertissements.push(format!("{fichier} : {e}"));
                        continue;
                    }
                };
                // Pas de test sans le mot : on evite de tout decouper.
                if !src.contains("test") {
                    continue;
                }
                let relatif = if krate.dossier.is_empty() { fichier.clone() } else { format!("{}/{fichier}", krate.dossier) };
                for t in carte(&src).tests {
                    let chemin: Vec<&str> = std::iter::once(base.as_str()).filter(|b| !b.is_empty()).chain(t.modules.iter().map(String::as_str)).chain(std::iter::once(t.nom.as_str())).collect();
                    analyse.tests.push(Test { paquet: krate.dossier.clone(), fichier: relatif.clone(), cible: cible.clone(), chemin: chemin.join("::"), nom: t.nom.clone(), ligne: t.ligne, options: options_de(&t) });
                }
            }
            analyse.paquets.push(Paquet { dossier: krate.dossier.clone(), nom: krate.nom.clone(), bibliotheque: krate.lib().is_some() });
        }
        analyse
    }

    fn commande(&self, projet: &Path, etape: &Etape) -> Result<Command, String> {
        cargo::commande(projet, etape)
    }

    fn lecteur(&self, projet: &Path, etape: &Etape) -> Box<dyn Lecteur> {
        Box::new(cargo::LecteurCargo::new(projet, etape))
    }

    fn options(&self) -> &'static [OptionTest] {
        OPTIONS
    }

    fn modele_test(&self) -> &'static str {
        "let resultat = 2 + 2;\nassert_eq!(resultat, 4);"
    }

    fn modele_generique(&self, paquet: &Paquet) -> String {
        let mut s = String::from("// Code générique : partagé par tous les tests de ce fichier.\n");
        if paquet.bibliotheque {
            s.push_str(&format!("use {}::*;\n", paquet.nom.replace('-', "_")));
        }
        s
    }

    fn valider_nom(&self, nom: &str) -> Result<(), String> {
        let mut c = nom.chars();
        let premier_ok = c.next().is_some_and(|p| p.is_ascii_alphabetic() || p == '_');
        if !premier_ok || !c.all(|x| x.is_ascii_alphanumeric() || x == '_') {
            return Err(format!("« {nom} » : lettres, chiffres et _ seulement, sans commencer par un chiffre (ex. calcule_la_somme)"));
        }
        if MOTS_RESERVES.contains(&nom) {
            return Err(format!("« {nom} » est un mot réservé de Rust"));
        }
        if nom.len() > 100 {
            return Err("nom trop long (100 caractères au plus)".to_string());
        }
        Ok(())
    }

    fn chemin_fichier(&self, paquet: &str, nom: &str) -> String {
        if paquet.is_empty() { format!("tests/{nom}.rs") } else { format!("{paquet}/tests/{nom}.rs") }
    }

    fn lire_fichier(&self, projet: &Path, fichier: &str) -> Result<FichierTests, String> {
        let src = std::fs::read_to_string(projet.join(fichier)).map_err(|e| format!("{fichier} : {e}"))?;
        let c = carte(&src);
        let generique = zone_generique(&src, &c, integration(fichier)).map(|(a, b)| desindenter(&src[a..b])).unwrap_or_default();
        let tests = c.tests.iter().map(|t| TestSource { nom: t.nom.clone(), code: desindenter(&src[t.corps.0..t.corps.1]), options: options_de(t), ligne: t.ligne }).collect();
        Ok(FichierTests { chemin: fichier.to_string(), generique, tests })
    }

    fn appliquer(&self, projet: &Path, op: &Operation) -> Result<String, String> {
        match op {
            Operation::CreerFichier { paquet, nom, generique } => {
                self.valider_nom(nom)?;
                let fichier = self.chemin_fichier(paquet, nom);
                let chemin = projet.join(&fichier);
                if chemin.exists() {
                    return Err(format!("{fichier} existe déjà"));
                }
                if let Some(d) = chemin.parent() {
                    std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
                }
                ecrire(projet, &fichier, &format!("{}\n", generique.trim_end()))?;
                Ok(fichier)
            }
            Operation::Generique { fichier, code } => {
                let src = lire(projet, fichier)?;
                ecrire(projet, fichier, &avec_generique(&src, integration(fichier), code))?;
                Ok(fichier.clone())
            }
            Operation::CreerTest { fichier, test } => {
                self.valider_nom(&test.nom)?;
                let src = std::fs::read_to_string(projet.join(fichier)).unwrap_or_default();
                let c = carte(&src);
                if c.tests.iter().any(|t| t.nom == test.nom) {
                    return Err(format!("un test « {} » existe déjà dans ce fichier", test.nom));
                }
                ecrire(projet, fichier, &avec_test(&src, &c, integration(fichier), test))?;
                Ok(fichier.clone())
            }
            Operation::ModifierTest { fichier, ancien, test } => {
                self.valider_nom(&test.nom)?;
                let src = lire(projet, fichier)?;
                let c = carte(&src);
                let t = c.tests.iter().find(|t| &t.nom == ancien).ok_or_else(|| format!("« {ancien} » n'est plus dans {fichier}"))?;
                if &test.nom != ancien && c.tests.iter().any(|x| x.nom == test.nom) {
                    return Err(format!("un test « {} » existe déjà dans ce fichier", test.nom));
                }
                let retrait = indentation(&src, t.debut);
                let texte = item(Some((t, &src)), test, &retrait);
                ecrire(projet, fichier, &format!("{}{}{}", &src[..t.debut], texte, &src[t.fin..]))?;
                Ok(fichier.clone())
            }
            Operation::SupprimerTest { fichier, nom } => {
                let src = lire(projet, fichier)?;
                let c = carte(&src);
                let t = c.tests.iter().find(|t| &t.nom == nom).ok_or_else(|| format!("« {nom} » n'est plus dans {fichier}"))?;
                let mut debut = debut_de_ligne(&src, t.debut);
                let fin = src[t.fin..].find('\n').map(|p| t.fin + p + 1).unwrap_or(src.len());
                // Une ligne vide de moins entre les voisins.
                if src[..debut].ends_with("\n\n") {
                    debut -= 1;
                }
                ecrire(projet, fichier, &format!("{}{}", &src[..debut], &src[fin..]))?;
                Ok(fichier.clone())
            }
        }
    }
}

fn lire(projet: &Path, fichier: &str) -> Result<String, String> {
    std::fs::read_to_string(projet.join(fichier)).map_err(|e| format!("{fichier} : {e}"))
}

fn ecrire(projet: &Path, fichier: &str, texte: &str) -> Result<(), String> {
    std::fs::write(projet.join(fichier), texte).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => format!("{fichier} : écriture refusée (ajoutez le dossier à [permissions] ecriture dans app.azure)"),
        _ => format!("{fichier} : {e}"),
    })
}

/// Un fichier de tests d'integration (tests/...) : ses tests sont a la
/// racine du fichier, pas dans un `mod tests`.
fn integration(fichier: &str) -> bool {
    fichier.starts_with("tests/") || fichier.contains("/tests/")
}

fn debut_de_ligne(src: &str, o: usize) -> usize {
    src[..o].rfind('\n').map(|p| p + 1).unwrap_or(0)
}

/// Les espaces en debut de la ligne de `o`.
fn indentation(src: &str, o: usize) -> String {
    src[debut_de_ligne(src, o)..].chars().take_while(|c| *c == ' ' || *c == '\t').collect()
}

/// Retire l'indentation commune et les lignes vides autour.
pub fn desindenter(texte: &str) -> String {
    let lignes: Vec<&str> = texte.lines().collect();
    let commune = lignes.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    let lignes: Vec<String> = lignes.iter().map(|l| if l.len() >= commune { l[commune..].trim_end().to_string() } else { l.trim_end().to_string() }).collect();
    let debut = lignes.iter().position(|l| !l.is_empty()).unwrap_or(lignes.len());
    let fin = lignes.iter().rposition(|l| !l.is_empty()).map(|p| p + 1).unwrap_or(debut);
    lignes[debut..fin].join("\n")
}

/// `code` avec `retrait` devant chaque ligne non vide.
fn indenter(code: &str, retrait: &str) -> String {
    code.trim_end().lines().map(|l| if l.trim().is_empty() { String::new() } else { format!("{retrait}{l}") }).collect::<Vec<_>>().join("\n")
}

/// Ou est le code generique : de l'ouverture de la zone (fichier ou
/// `mod tests`) jusqu'au premier test.
fn zone_generique(src: &str, c: &Carte, integration: bool) -> Option<(usize, usize)> {
    if let Some(t) = c.tests.first() {
        return Some((t.zone, debut_de_ligne(src, t.debut)));
    }
    if integration {
        return Some((0, src.len()));
    }
    c.module_de_tests().map(|m| m.corps)
}

/// `src` avec `code` pour code generique.
fn avec_generique(src: &str, integration: bool, code: &str) -> String {
    let c = carte(src);
    let code = code.trim_end();
    match (zone_generique(src, &c, integration), c.tests.first()) {
        (Some((a, b)), Some(t)) => {
            let retrait = indentation(src, t.debut);
            let texte = indenter(code, &retrait);
            let avant = if a > 0 { "\n" } else { "" };
            let apres = if texte.is_empty() { "" } else { "\n\n" };
            format!("{}{avant}{texte}{apres}{}", &src[..a], &src[b..])
        }
        (Some((a, b)), None) if integration => format!("{}{}\n{}", &src[..a], code, &src[b..]),
        (Some((a, b)), None) => {
            let m = c.module_de_tests().map(|m| indentation(src, m.ligne_debut)).unwrap_or_default();
            format!("{}\n{}\n{m}{}", &src[..a], indenter(code, &format!("{m}    ")), &src[b..])
        }
        (None, _) => format!("{}\n\n#[cfg(test)]\nmod tests {{\n{}\n}}\n", src.trim_end(), indenter(code, "    ")),
    }
}

/// Le texte d'un test, a partir de son premier attribut (la ligne garde son
/// indentation), chaque ligne suivante precedee de `retrait`. `ancien` :
/// ses attributs (hors options) et sa signature sont gardes.
fn item(ancien: Option<(&Trouve, &str)>, test: &TestSource, retrait: &str) -> String {
    let geres: Vec<&str> = OPTIONS.iter().filter_map(|o| attribut(o.code).map(|a| a.0)).collect();
    let mut attributs: Vec<String> = ancien.map(|(t, _)| t.attributs.iter().filter(|a| !geres.contains(&a.chemin.as_str())).map(|a| a.texte.clone()).collect()).unwrap_or_default();
    if attributs.is_empty() {
        attributs.push("#[test]".to_string());
    }
    attributs.extend(test.options.keys().filter_map(|o| attribut(o).map(|a| a.1.to_string())));
    let signature = match ancien {
        Some((t, src)) => renommer(src[t.signature.0..t.signature.1].trim(), &t.nom, &test.nom),
        None => format!("fn {}()", test.nom),
    };
    let mut s = attributs.join(&format!("\n{retrait}"));
    s.push_str(&format!("\n{retrait}{signature} {{\n{}\n{retrait}}}", indenter(&test.code, &format!("{retrait}    "))));
    s
}

/// `fn ancien(` -> `fn nouveau(` dans la signature.
fn renommer(signature: &str, ancien: &str, nouveau: &str) -> String {
    match signature.find(&format!("fn {ancien}")) {
        Some(p) => format!("{}fn {nouveau}{}", &signature[..p], &signature[p + 3 + ancien.len()..]),
        None => signature.to_string(),
    }
}

/// `src` avec `test` ajoute : a la fin du fichier (test d'integration) ou
/// du `mod tests` (cree s'il n'y en a pas).
fn avec_test(src: &str, c: &Carte, integration: bool, test: &TestSource) -> String {
    let construit = |retrait: &str| item(None, test, retrait);
    if integration {
        let tete = src.trim_end();
        let sep = if tete.is_empty() { "" } else { "\n\n" };
        return format!("{tete}{sep}{}\n", construit(""));
    }
    match c.module_de_tests() {
        Some(m) => {
            let retrait_mod = indentation(src, m.ligne_debut);
            let retrait = format!("{retrait_mod}    ");
            let tete = src[..m.corps.1].trim_end();
            format!("{tete}\n\n{retrait}{}\n{retrait_mod}{}", construit(&retrait), &src[m.corps.1..])
        }
        None => format!("{}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n    {}\n}}\n", src.trim_end(), construit("    ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projet(nom: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("azure-testeur-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("Cargo.toml"), "[package]\nname = \"demo-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
        std::fs::write(d.join("src/lib.rs"), "pub fn double(x: i32) -> i32 { x * 2 }\n").unwrap();
        d
    }

    #[test]
    fn atelier_ecrit_dans_un_fichier_d_integration() {
        let d = projet("integration");
        let r = Rust;
        let a = r.analyser(&d);
        assert_eq!(a.paquets[0].nom, "demo-app");
        let generique = r.modele_generique(&a.paquets[0]);
        assert!(generique.contains("use demo_app::*;"));
        let f = r.appliquer(&d, &Operation::CreerFichier { paquet: String::new(), nom: "calculs".into(), generique }).unwrap();
        assert_eq!(f, "tests/calculs.rs");
        let t = TestSource { nom: "double_de_deux".into(), code: "assert_eq!(double(2), 4);".into(), ..TestSource::default() };
        r.appliquer(&d, &Operation::CreerTest { fichier: f.clone(), test: t }).unwrap();
        let mut ko = TestSource { nom: "negatif".into(), code: "let x = double(-1);\nassert!(x < 0);".into(), ..TestSource::default() };
        ko.options.insert("ignore".into(), "oui".into());
        r.appliquer(&d, &Operation::CreerTest { fichier: f.clone(), test: ko.clone() }).unwrap();
        let lu = r.lire_fichier(&d, &f).unwrap();
        assert_eq!(lu.tests.iter().map(|t| t.nom.as_str()).collect::<Vec<_>>(), ["double_de_deux", "negatif"]);
        assert_eq!(lu.tests[1].code, "let x = double(-1);\nassert!(x < 0);");
        assert!(lu.tests[1].options.contains_key("ignore"));
        assert!(lu.generique.contains("use demo_app::*;"));

        // Renommer, changer le corps et les options.
        let modifie = TestSource { nom: "moins_un".into(), code: "assert_eq!(double(-1), -2);".into(), ..TestSource::default() };
        r.appliquer(&d, &Operation::ModifierTest { fichier: f.clone(), ancien: "negatif".into(), test: modifie }).unwrap();
        r.appliquer(&d, &Operation::Generique { fichier: f.clone(), code: "use demo_app::double;\n\nfn aide() -> i32 { 1 }".into() }).unwrap();
        let lu = r.lire_fichier(&d, &f).unwrap();
        assert_eq!(lu.tests[1].nom, "moins_un");
        assert!(lu.tests[1].options.is_empty());
        assert_eq!(lu.generique, "use demo_app::double;\n\nfn aide() -> i32 { 1 }");
        r.appliquer(&d, &Operation::SupprimerTest { fichier: f.clone(), nom: "double_de_deux".into() }).unwrap();
        let texte = std::fs::read_to_string(d.join(&f)).unwrap();
        assert_eq!(texte, "use demo_app::double;\n\nfn aide() -> i32 { 1 }\n\n#[test]\nfn moins_un() {\n    assert_eq!(double(-1), -2);\n}\n");

        let a = r.analyser(&d);
        assert_eq!(a.tests.len(), 1);
        assert_eq!((a.tests[0].cible.as_str(), a.tests[0].chemin.as_str()), ("tests/calculs.rs", "moins_un"));
        assert!(r.valider_nom("fn").is_err() && r.valider_nom("2x").is_err() && r.valider_nom("a-b").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn atelier_ecrit_un_test_unitaire_dans_mod_tests() {
        let d = projet("unitaire");
        let r = Rust;
        let t = TestSource { nom: "double_zero".into(), code: "assert_eq!(double(0), 0);".into(), ..TestSource::default() };
        r.appliquer(&d, &Operation::CreerTest { fichier: "src/lib.rs".into(), test: t }).unwrap();
        let t2 = TestSource { nom: "double_un".into(), code: "assert_eq!(double(1), 2);".into(), ..TestSource::default() };
        r.appliquer(&d, &Operation::CreerTest { fichier: "src/lib.rs".into(), test: t2 }).unwrap();
        let texte = std::fs::read_to_string(d.join("src/lib.rs")).unwrap();
        assert_eq!(texte, "pub fn double(x: i32) -> i32 { x * 2 }\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn double_zero() {\n        assert_eq!(double(0), 0);\n    }\n\n    #[test]\n    fn double_un() {\n        assert_eq!(double(1), 2);\n    }\n}\n");
        let lu = r.lire_fichier(&d, "src/lib.rs").unwrap();
        assert_eq!(lu.generique, "use super::*;");
        let a = r.analyser(&d);
        assert_eq!(a.tests.iter().map(|t| t.chemin.as_str()).collect::<Vec<_>>(), ["tests::double_zero", "tests::double_un"]);
        let _ = std::fs::remove_dir_all(&d);
    }
}
