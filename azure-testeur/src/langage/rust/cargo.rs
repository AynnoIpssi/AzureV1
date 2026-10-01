// Cargo : les crates d'un projet (Cargo.toml), leurs cibles, la commande
// `cargo test` d'une etape et la lecture de sa sortie.
use crate::langage::{cle, Etape, Evenement, Lecteur, Statut};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Une cible compilee avec ses tests : bibliotheque, executable ou test
/// d'integration, reconnue a son fichier racine.
#[derive(Clone, Debug, PartialEq)]
pub enum Cible {
    Lib,
    Bin(String),
    Test(String),
}

/// Un crate : son dossier (relatif au projet), son nom, ses cibles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Crate {
    pub dossier: String,
    pub nom: String,
    /// (fichier racine relatif au crate, cible)
    pub cibles: Vec<(String, Cible)>,
}

impl Crate {
    pub fn lib(&self) -> Option<&str> {
        self.cibles.iter().find(|c| c.1 == Cible::Lib).map(|c| c.0.as_str())
    }

    pub fn cible(&self, racine: &str) -> Option<&Cible> {
        self.cibles.iter().find(|c| c.0 == racine).map(|c| &c.1)
    }

    /// La cible qui compile `fichier` (relatif au crate) et le module qu'il
    /// y forme (`a::b`), ou `None` (exemples, aides communes des tests...).
    pub fn racine_de(&self, fichier: &str) -> Option<(String, String)> {
        if self.cible(fichier).is_some() {
            return Some((fichier.to_string(), String::new()));
        }
        // Dossiers d'une cible : tests/x/main.rs, src/bin/x/main.rs.
        for dossier in ["tests/", "src/bin/"] {
            if let Some(reste) = fichier.strip_prefix(dossier) {
                let (d, sous) = reste.split_once('/')?;
                let racine = format!("{dossier}{d}/main.rs");
                self.cible(&racine)?;
                return Some((racine, module(sous)));
            }
        }
        let reste = fichier.strip_prefix("src/")?;
        let racine = self.lib().map(str::to_string).or_else(|| self.cibles.iter().find(|c| c.0 == "src/main.rs").map(|c| c.0.clone()))?;
        Some((racine, module(reste)))
    }
}

/// `a/b.rs` -> `a::b`, `a/mod.rs` -> `a`, `lib.rs` -> ``.
fn module(chemin: &str) -> String {
    let sans = chemin.strip_suffix(".rs").unwrap_or(chemin);
    let sans = sans.strip_suffix("/mod").unwrap_or(sans);
    if matches!(sans, "lib" | "main" | "mod") {
        return String::new();
    }
    sans.replace('/', "::")
}

/// Dossiers jamais parcourus.
fn ignore(nom: &str) -> bool {
    nom.starts_with('.') || matches!(nom, "target" | "node_modules")
}

/// Lit un Cargo.toml (juste ce qu'il faut) ; `None` sans `[package]`.
pub fn lire_crate(dossier: &Path, relatif: &str) -> Option<Crate> {
    let texte = std::fs::read_to_string(dossier.join("Cargo.toml")).ok()?;
    let (mut section, mut nom, mut lib) = (String::new(), None, None);
    let (mut bins, mut tests): (Vec<(String, Option<String>)>, Vec<(String, Option<String>)>) = (Vec::new(), Vec::new());
    for ligne in texte.lines().map(str::trim) {
        if ligne.starts_with('[') {
            section = ligne.trim_matches(|c| c == '[' || c == ']').trim().to_string();
            match section.as_str() {
                "bin" => bins.push((String::new(), None)),
                "test" => tests.push((String::new(), None)),
                _ => {}
            }
            continue;
        }
        let Some((cle, valeur)) = ligne.split_once('=') else { continue };
        let (cle, valeur) = (cle.trim(), valeur.trim().trim_matches('"').to_string());
        match (section.as_str(), cle) {
            ("package", "name") => nom = Some(valeur),
            ("lib", "path") => lib = Some(valeur),
            ("bin", "name") => bins.last_mut()?.0 = valeur,
            ("bin", "path") => bins.last_mut()?.1 = Some(valeur),
            ("test", "name") => tests.last_mut()?.0 = valeur,
            ("test", "path") => tests.last_mut()?.1 = Some(valeur),
            _ => {}
        }
    }
    let nom = nom?;
    let mut c = Crate { dossier: relatif.to_string(), nom: nom.clone(), cibles: Vec::new() };
    let existe = |p: &str| dossier.join(p).is_file();
    let lib = lib.unwrap_or_else(|| "src/lib.rs".to_string());
    if existe(&lib) {
        c.cibles.push((lib, Cible::Lib));
    }
    for (n, p) in bins {
        let p = p.unwrap_or_else(|| if n == nom { "src/main.rs".to_string() } else { format!("src/bin/{n}.rs") });
        c.cibles.push((p, Cible::Bin(n)));
    }
    if existe("src/main.rs") && c.cible("src/main.rs").is_none() {
        c.cibles.push(("src/main.rs".to_string(), Cible::Bin(nom.clone())));
    }
    for (n, p) in tests {
        let p = p.unwrap_or_else(|| format!("tests/{n}.rs"));
        c.cibles.push((p, Cible::Test(n)));
    }
    // Cibles trouvees seules : src/bin/*, tests/*.
    for (dossier_cibles, test) in [("src/bin", false), ("tests", true)] {
        let Ok(entrees) = std::fs::read_dir(dossier.join(dossier_cibles)) else { continue };
        let mut trouvees: Vec<(String, String)> = entrees
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                if let Some(stem) = n.strip_suffix(".rs") {
                    Some((format!("{dossier_cibles}/{n}"), stem.to_string()))
                } else if e.path().join("main.rs").is_file() {
                    Some((format!("{dossier_cibles}/{n}/main.rs"), n))
                } else {
                    None
                }
            })
            .collect();
        trouvees.sort();
        for (racine, n) in trouvees {
            if c.cible(&racine).is_none() {
                c.cibles.push((racine, if test { Cible::Test(n) } else { Cible::Bin(n) }));
            }
        }
    }
    Some(c)
}

/// Les crates sous `projet` (le projet lui-meme compris).
pub fn crates(projet: &Path) -> Vec<Crate> {
    fn aller(projet: &Path, dossier: &Path, prof: usize, out: &mut Vec<Crate>) {
        let relatif = dossier.strip_prefix(projet).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some(c) = lire_crate(dossier, &relatif) {
            out.push(c);
        }
        if prof == 0 {
            return;
        }
        let Ok(entrees) = std::fs::read_dir(dossier) else { return };
        let mut sous: Vec<PathBuf> = entrees.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir()) && !ignore(&e.file_name().to_string_lossy())).map(|e| e.path()).collect();
        sous.sort();
        for d in sous {
            aller(projet, &d, prof - 1, out);
        }
    }
    let mut out = Vec::new();
    aller(projet, projet, 5, &mut out);
    out
}

/// Les fichiers .rs d'un crate (relatifs au crate), sans ceux des crates
/// qu'il contient, ni exemples, ni benchs.
pub fn fichiers(racine: &Path) -> Vec<String> {
    fn aller(racine: &Path, dossier: &Path, out: &mut Vec<String>) {
        let Ok(entrees) = std::fs::read_dir(dossier) else { return };
        for e in entrees.flatten() {
            let nom = e.file_name().to_string_lossy().into_owned();
            let chemin = e.path();
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                let haut = dossier == racine;
                if ignore(&nom) || (haut && matches!(nom.as_str(), "examples" | "benches")) || chemin.join("Cargo.toml").is_file() {
                    continue;
                }
                aller(racine, &chemin, out);
            } else if nom.ends_with(".rs")
                && let Ok(r) = chemin.strip_prefix(racine)
            {
                out.push(r.to_string_lossy().into_owned());
            }
        }
    }
    let mut out = Vec::new();
    aller(racine, racine, &mut out);
    out.sort();
    out
}

/// `cargo` : `$CARGO`, le `PATH`, puis ~/.cargo/bin (une app lancee depuis
/// le bureau n'a pas toujours le `PATH` du terminal).
pub fn cargo() -> Result<PathBuf, String> {
    if let Some(c) = std::env::var_os("CARGO").map(PathBuf::from).filter(|p| p.is_file()) {
        return Ok(c);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let home = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo/bin"));
    std::env::split_paths(&path).chain(home).map(|d| d.join("cargo")).find(|p| p.is_file()).ok_or_else(|| "cargo introuvable (installez Rust : rustup.rs)".to_string())
}

/// `cargo test` pour l'etape.
pub fn commande(projet: &Path, etape: &Etape) -> Result<Command, String> {
    let dossier = projet.join(&etape.paquet);
    let krate = lire_crate(&dossier, &etape.paquet).ok_or_else(|| format!("pas de crate dans {}", dossier.display()))?;
    let cargo = cargo()?;
    let mut cmd = Command::new(&cargo);
    cmd.current_dir(&dossier).arg("test").arg("--manifest-path").arg(dossier.join("Cargo.toml")).args(["--color", "never", "--no-fail-fast"]);
    match &etape.cible {
        None => {
            cmd.arg("--tests");
        }
        Some(racine) => match krate.cible(racine) {
            Some(Cible::Lib) => {
                cmd.arg("--lib");
            }
            Some(Cible::Bin(n)) => {
                cmd.args(["--bin", n]);
            }
            Some(Cible::Test(n)) => {
                cmd.args(["--test", n]);
            }
            None => return Err(format!("{racine} n'est plus une cible de {}", krate.nom)),
        },
    }
    cmd.arg("--");
    cmd.args(&etape.filtres);
    if etape.exact {
        cmd.arg("--exact");
    }
    if etape.avec_ignores {
        cmd.arg("--include-ignored");
    }
    // Le dossier de cargo dans le PATH : rustc et les outils a cote.
    if let Some(bin) = cargo.parent() {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let dossiers = std::iter::once(bin.to_path_buf()).chain(std::env::split_paths(&path));
        if let Ok(p) = std::env::join_paths(dossiers) {
            cmd.env("PATH", p);
        }
    }
    cmd.env("CARGO_TERM_COLOR", "never");
    // Deux fils du processeur restent a la fenetre (et au bureau) : sur un
    // portable, cargo sur TOUS les coeurs fait tomber la frequence et
    // renvoie la fenetre sur un coeur econome - chaque redessin coutait
    // alors 5 a 8 fois plus. Un reglage de l'utilisateur passe avant.
    let fils = std::thread::available_parallelism().map_or(1, |n| n.get()).saturating_sub(2).max(1).to_string();
    if std::env::var_os("CARGO_BUILD_JOBS").is_none() {
        cmd.env("CARGO_BUILD_JOBS", &fils);
    }
    if std::env::var_os("RUST_TEST_THREADS").is_none() {
        cmd.env("RUST_TEST_THREADS", &fils);
    }
    Ok(cmd)
}

/// Lit la sortie de `cargo test` : `Running tests/x.rs (...)` dit quelle
/// cible suit, `test a::b ... ok` donne un resultat, `---- a::b stdout ----`
/// ouvre la sortie d'un echec.
pub struct LecteurCargo {
    paquet: String,
    racines: Vec<String>,
    cible: Option<String>,
    echec: Option<(String, Vec<String>)>,
}

impl LecteurCargo {
    pub fn new(projet: &Path, etape: &Etape) -> LecteurCargo {
        let racines = lire_crate(&projet.join(&etape.paquet), &etape.paquet).map(|c| c.cibles.into_iter().map(|c| c.0).collect()).unwrap_or_default();
        LecteurCargo { paquet: etape.paquet.clone(), racines, cible: etape.cible.clone(), echec: None }
    }

    fn fermer(&mut self) -> Vec<Evenement> {
        let Some((cle, mut lignes)) = self.echec.take() else { return Vec::new() };
        while lignes.last().is_some_and(|l| l.trim().is_empty()) {
            lignes.pop();
        }
        vec![Evenement::Sortie { cle, lignes }]
    }
}

impl Lecteur for LecteurCargo {
    fn ligne(&mut self, ligne: &str) -> Vec<Evenement> {
        let net = ligne.trim();
        if let Some(reste) = net.strip_prefix("Running ") {
            let out = self.fermer();
            let chemin = reste.strip_prefix("unittests ").unwrap_or(reste);
            let chemin = chemin.split(" (").next().unwrap_or(chemin);
            self.cible = self.racines.iter().find(|r| chemin == r.as_str() || chemin.ends_with(&format!("/{r}"))).cloned();
            return out;
        }
        if net.starts_with("Doc-tests ") {
            let out = self.fermer();
            self.cible = None;
            return out;
        }
        if let Some(nom) = net.strip_prefix("---- ").and_then(|r| r.strip_suffix(" stdout ----")) {
            let out = self.fermer();
            if let Some(cible) = &self.cible {
                self.echec = Some((cle(&self.paquet, cible, nom), Vec::new()));
            }
            return out;
        }
        if net == "failures:" || net.starts_with("test result:") {
            return self.fermer();
        }
        if let Some((_, lignes)) = &mut self.echec {
            lignes.push(ligne.to_string());
            return Vec::new();
        }
        if let (Some(reste), Some(cible)) = (ligne.strip_prefix("test "), &self.cible)
            && let Some((nom, issue)) = reste.split_once(" ... ")
        {
            let statut = if issue.starts_with("ok") {
                Statut::Reussi
            } else if issue.starts_with("FAILED") {
                Statut::Echoue
            } else if issue.starts_with("ignored") {
                Statut::Ignore
            } else {
                return Vec::new();
            };
            return vec![Evenement::Statut { cle: cle(&self.paquet, cible, nom.trim()), statut }];
        }
        Vec::new()
    }

    fn fin(&mut self) -> Vec<Evenement> {
        self.fermer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_la_sortie_de_cargo() {
        let mut l = LecteurCargo { paquet: "app".into(), racines: vec!["src/lib.rs".into(), "tests/ecran.rs".into()], cible: None, echec: None };
        let sortie = "   Compiling app v0.1.0\n     Running unittests src/lib.rs (target/debug/deps/app-1)\n\nrunning 2 tests\ntest a::ok ... ok\ntest a::ko ... FAILED\n\nfailures:\n\n---- a::ko stdout ----\nthread 'a::ko' panicked at src/lib.rs:3:5:\n1 != 2\n\n\nfailures:\n    a::ko\n\ntest result: FAILED. 1 passed; 1 failed\n     Running tests/ecran.rs (target/debug/deps/ecran-2)\ntest lent ... ignored, trop long\n";
        let mut ev: Vec<Evenement> = sortie.lines().flat_map(|x| l.ligne(x)).collect();
        ev.extend(l.fin());
        assert_eq!(ev[0], Evenement::Statut { cle: "app|src/lib.rs|a::ok".into(), statut: Statut::Reussi });
        assert_eq!(ev[1], Evenement::Statut { cle: "app|src/lib.rs|a::ko".into(), statut: Statut::Echoue });
        assert_eq!(ev[2], Evenement::Sortie { cle: "app|src/lib.rs|a::ko".into(), lignes: vec!["thread 'a::ko' panicked at src/lib.rs:3:5:".into(), "1 != 2".into()] });
        assert_eq!(ev[3], Evenement::Statut { cle: "app|tests/ecran.rs|lent".into(), statut: Statut::Ignore });
    }

    #[test]
    fn modules_des_fichiers() {
        let c = Crate { dossier: String::new(), nom: "app".into(), cibles: vec![("src/lib.rs".into(), Cible::Lib), ("src/main.rs".into(), Cible::Bin("app".into())), ("tests/ecran.rs".into(), Cible::Test("ecran".into()))] };
        assert_eq!(c.racine_de("src/a/b.rs"), Some(("src/lib.rs".into(), "a::b".into())));
        assert_eq!(c.racine_de("src/a/mod.rs"), Some(("src/lib.rs".into(), "a".into())));
        assert_eq!(c.racine_de("src/main.rs"), Some(("src/main.rs".into(), String::new())));
        assert_eq!(c.racine_de("tests/ecran.rs"), Some(("tests/ecran.rs".into(), String::new())));
        assert_eq!(c.racine_de("tests/common/mod.rs"), None);
    }
}
