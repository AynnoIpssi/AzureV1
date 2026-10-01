// `execution` : une commande qui crache beaucoup de lignes, lue dans son
// thread, pendant que la « fenetre » lit l'etat toutes les 16 ms. On mesure
// le debit du lecteur et surtout l'attente du verrou cote fenetre.
use crate::commun::*;
use azure_testeur::ecran::contexte;
use azure_testeur::langage::rust::Rust;
use azure_testeur::langage::{cle, Analyse, Etape, Evenement, FichierTests, Langage, Lecteur, Operation, OptionTest, Paquet, Statut};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

const Z: &str = "execution";
const LIGNES: usize = 20_000;

/// Un « langage » dont la commande ecrit `LIGNES` resultats.
struct Bavard;

struct LecteurBavard;

impl Lecteur for LecteurBavard {
    fn ligne(&mut self, ligne: &str) -> Vec<Evenement> {
        match ligne.strip_prefix("test ").and_then(|r| r.split_once(" ... ")) {
            Some((nom, _)) => vec![Evenement::Statut { cle: cle("", "src/lib.rs", nom), statut: Statut::Reussi }],
            None => Vec::new(),
        }
    }

    fn fin(&mut self) -> Vec<Evenement> {
        Vec::new()
    }
}

impl Langage for Bavard {
    fn code(&self) -> &'static str {
        "bavard"
    }
    fn nom(&self) -> &'static str {
        "bavard"
    }
    fn reconnait(&self, _: &Path) -> bool {
        false
    }
    fn analyser(&self, _: &Path) -> Analyse {
        Analyse::default()
    }
    fn commande(&self, _: &Path, _: &Etape) -> Result<Command, String> {
        let mut c = Command::new("sh");
        // Par paquets espaces : la fenetre lit l'etat pendant que ca ecrit.
        c.arg("-c").arg(format!("for k in 1 2 3 4 5 6 7 8 9 10; do seq $(((k - 1) * {LIGNES} / 10 + 1)) $((k * {LIGNES} / 10)) | sed 's/.*/test m::t_& ... ok/'; sleep 0.03; done"));
        Ok(c)
    }
    fn lecteur(&self, _: &Path, _: &Etape) -> Box<dyn Lecteur> {
        Box::new(LecteurBavard)
    }
    fn options(&self) -> &'static [OptionTest] {
        Rust.options()
    }
    fn modele_test(&self) -> &'static str {
        ""
    }
    fn modele_generique(&self, p: &Paquet) -> String {
        Rust.modele_generique(p)
    }
    fn valider_nom(&self, n: &str) -> Result<(), String> {
        Rust.valider_nom(n)
    }
    fn chemin_fichier(&self, p: &str, n: &str) -> String {
        Rust.chemin_fichier(p, n)
    }
    fn lire_fichier(&self, p: &Path, f: &str) -> Result<FichierTests, String> {
        Rust.lire_fichier(p, f)
    }
    fn appliquer(&self, p: &Path, op: &Operation) -> Result<String, String> {
        Rust.appliquer(p, op)
    }
}

pub fn mesurer() {
    eprintln!("\n=== execution ===");
    let d = crate_demo("bavard");
    let t = testeur(None);
    let i = t.projets().lier(&d.to_string_lossy()).unwrap();
    t.ouvrir(i);
    t.etat().console = true;
    let attendus: Vec<String> = (1..=LIGNES).map(|i| cle("", "src/lib.rs", &format!("m::t_{i}"))).collect();
    let debut = Instant::now();
    t.lanceur.lancer(d.clone(), Arc::new(Bavard), "bavard", vec![Etape::default()], attendus).unwrap();
    // La fenetre : toutes les 16 ms, lit l'etat et construit les donnees.
    let (mut attente_max, mut contexte_max, mut tics) = (Duration::ZERO, Duration::ZERO, 0);
    while t.lanceur.lire().en_cours && debut.elapsed() < Duration::from_secs(60) {
        let (_, attente) = chrono(|| drop(t.lanceur.lire()));
        let (_, c) = chrono(|| drop(contexte(&t)));
        attente_max = attente_max.max(attente);
        contexte_max = contexte_max.max(c);
        tics += 1;
        std::thread::sleep(IMAGE);
    }
    let total = debut.elapsed();
    let lignes = t.lanceur.lire().lignes.len();
    noter(Z, &format!("{LIGNES} lignes lues et rangees ({lignes} gardees)"), false, total);
    noter(Z, &format!("attente max du verrou cote fenetre ({tics} tics)"), true, attente_max);
    noter(Z, "contexte() max pendant l'execution (console ouverte)", true, contexte_max);
    mesure(Z, &format!("contexte() apres : {LIGNES} resultats, console ouverte"), true, || drop(contexte(&t)));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
#[ignore]
fn execution() {
    mesurer();
    fin_de_zone(Z);
}
