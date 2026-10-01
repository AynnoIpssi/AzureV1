// `ecran` : chaque etat de l'ecran sur Azure - donnees, construction
// (rsH + rsC), dessin, lecture des champs (faite a CHAQUE tic par la
// fenetre, voir `with_context`), report du defilement.
use crate::commun::*;
use azure_foundation::ui::services::form::form_values;
use azure_foundation::ui::services::interact;
use azure_testeur::clics::cliquer;
use azure_testeur::ecran::{contexte, fichiers_atelier, visibles, Testeur};
use std::sync::Arc;

const Z: &str = "ecran";

fn etat(nom: &str, t: &Arc<Testeur>) {
    let _ = mesure(Z, &format!("[{nom}] donnees (contexte)"), true, || drop(contexte(t)));
    mesure(Z, &format!("[{nom}] construire (donnees + rsH + rsC)"), true, || drop(construire(t)));
    let nodes = construire(t);
    mesure(Z, &format!("[{nom}] dessiner"), true, || dessiner(&nodes));
    mesure(Z, &format!("[{nom}] form_values (a chaque tic, 60/s)"), true, || drop(form_values(&nodes)));
    let mut neuf = construire(t);
    mesure(Z, &format!("[{nom}] carry_scroll (chaque redessin)"), true, || {
        interact::carry_scroll(&nodes, &mut neuf);
    });
}

pub fn mesurer() {
    eprintln!("\n=== ecran ===");
    let Some(env) = environnement() else { return };
    let (t, d) = chrono(|| testeur(Some(env)));
    noter(Z, "Testeur::new (ouvre et analyse Azure, avant la fenetre)", true, d);
    let rien = Valeurs(Default::default());
    etat("tests replies", &t);
    cliquer(&t, "ouvrir-tout", &rien);
    etat("tests tout deplies", &t);
    cliquer(&t, "fermer-tout", &rien);
    {
        let mut e = t.lanceur.lire();
        for i in 0..4000 {
            e.lignes.push(format!("test azure::module::test_{i} ... ok"));
        }
    }
    cliquer(&t, "console", &rien);
    etat("console 4000 lignes", &t);
    cliquer(&t, "console", &rien);
    cliquer(&t, "voir-0", &rien);
    etat("detail d'un test", &t);
    {
        let e = t.etat();
        let exec = t.lanceur.lire();
        mesure(Z, "visibles (filtre + vue)", true, || drop(visibles(&e, &|c| exec.statut(c))));
        mesure(Z, "fichiers_atelier", true, || drop(fichiers_atelier(&e)));
    }
    cliquer(&t, "onglet-atelier", &rien);
    etat("atelier", &t);
    cliquer(&t, "ouvrir-0", &rien);
    etat("atelier + test ouvert", &t);
    cliquer(&t, "ouvrir-generique", &rien);
    etat("atelier + generique ouvert", &t);
}

#[test]
#[ignore]
fn ecran() {
    mesurer();
    fin_de_zone(Z);
}
