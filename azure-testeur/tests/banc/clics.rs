// `clics` : CHAQUE bouton de chaque ecran, comme dans la fenetre (ranger
// les champs, l'action, puis reconstruire l'ecran). Sur Azure, ce qui ecrit
// dans le code ou lance cargo est saute ; c'est fait sur un petit crate.
use crate::commun::*;
use azure_testeur::clics::cliquer;
use azure_testeur::ecran::Testeur;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

const Z: &str = "clics";

/// Ecrit dans le projet, lance cargo, ou oublie le projet.
fn touche_au_projet(id: &str) -> bool {
    const P: &[&str] = &["lancer", "tout-lancer", "relancer", "supprimer-oui", "enregistrer-", "nom-", "generique-enregistrer", "creer-test", "nouveau-nom", "nf-creer", "nf-nom", "delier-", "lier", "chemin"];
    let id = id.strip_prefix("d-").unwrap_or(id);
    P.iter().any(|p| id.starts_with(p))
}

/// `groupe-12` -> `groupe-#` : une famille de boutons.
fn famille(id: &str) -> String {
    id.split('-').map(|p| if p.parse::<usize>().is_ok() { "#" } else { p }).collect::<Vec<_>>().join("-")
}

/// Un clic comme la fenetre le fait ; rend la duree.
fn clic(t: &Arc<Testeur>, id: &str) -> Duration {
    let nodes = construire(t);
    chrono(|| {
        let v = Valeurs::de(&nodes);
        cliquer(t, id, &v);
        let neuf = construire(t);
        dessiner(&neuf);
    })
    .1
}

/// Clique chaque famille de boutons visible sur l'ecran prepare par `aller`.
fn balayer(nom: &str, t: &Arc<Testeur>, aller: &dyn Fn(&Arc<Testeur>), vus: &mut HashSet<String>) {
    aller(t);
    for id in ids(&construire(t)) {
        let f = famille(&id);
        if touche_au_projet(&id) || !vus.insert(format!("{nom}/{f}")) {
            continue;
        }
        aller(t);
        if !ids(&construire(t)).contains(&id) {
            continue;
        }
        noter(Z, &format!("[{nom}] {f}"), true, clic(t, &id));
    }
}

pub fn mesurer() {
    eprintln!("\n=== clics ===");
    if let Some(env) = environnement() {
        let t = testeur(Some(env));
        let rien = Valeurs(Default::default());
        let mut vus = HashSet::new();
        let aller_tests = |t: &Arc<Testeur>| {
            let mut e = t.etat();
            e.onglet = Default::default();
            e.choisi = None;
            e.console = false;
        };
        balayer("tests", &t, &aller_tests, &mut vus);
        balayer("tests deplies", &t, &|t| {
            aller_tests(t);
            cliquer(t, "ouvrir-tout", &rien);
        }, &mut vus);
        balayer("detail", &t, &|t| {
            aller_tests(t);
            cliquer(t, "voir-0", &rien);
        }, &mut vus);
        balayer("atelier", &t, &|t| {
            cliquer(t, "onglet-atelier", &rien);
            t.etat().ouvert = None;
        }, &mut vus);
        balayer("atelier ouvert", &t, &|t| {
            cliquer(t, "onglet-atelier", &rien);
            t.etat().ouvert = None;
            cliquer(t, "ouvrir-0", &rien);
        }, &mut vus);
    }

    // Ce qui ecrit ou lance : sur un petit crate.
    let d = crate_demo("clics");
    let t = testeur(None);
    let i = t.projets().lier(&d.to_string_lossy()).unwrap();
    t.ouvrir(i);
    noter(Z, "[demo] tout-lancer (le clic, cargo tourne a part)", true, clic(&t, "tout-lancer"));
    attendre(&t, Duration::from_secs(300));
    noter(Z, "[demo] relancer-echecs", true, clic(&t, "relancer-echecs"));
    attendre(&t, Duration::from_secs(120));
    noter(Z, "[demo] lancer-0", true, clic(&t, "lancer-0"));
    attendre(&t, Duration::from_secs(120));
    noter(Z, "[demo] onglet-atelier", true, clic(&t, "onglet-atelier"));
    noter(Z, "[demo] ouvrir-0", true, clic(&t, "ouvrir-0"));
    noter(Z, "[demo] enregistrer-0 (ecrit + reanalyse)", true, clic(&t, "enregistrer-0"));
    t.etat().fixer_brouillon("nouveau-nom", Some("banc_cree".into()));
    noter(Z, "[demo] creer-test (ecrit + reanalyse)", true, clic(&t, "creer-test"));
    let n = t.etat().lu.as_ref().map(|l| l.tests.len()).unwrap_or(1) - 1;
    noter(Z, "[demo] supprimer-oui (ecrit + reanalyse)", true, clic(&t, &format!("supprimer-oui-{n}")));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
#[ignore]
fn clics() {
    mesurer();
    fin_de_zone(Z);
}
