// `projet` : l'environnement, la liste des projets, relier un dossier.
use crate::commun::*;
use azure_testeur::projet::{self, EnMemoire, Projets};

const Z: &str = "projet";

pub fn mesurer() {
    eprintln!("\n=== projet ===");
    mesure(Z, "environnement() (lit ~/.local/share/azure/source)", true, || {
        let _ = projet::environnement();
    });
    mesure(Z, "Projets::new (memoire vide)", true, || {
        let _ = Projets::new(projet::environnement(), Box::new(EnMemoire::default()));
    });
    let d = crate_demo("relier");
    let chemin = d.to_string_lossy().into_owned();
    mesure(Z, "Projets::lier (dossier deja relie ou nouveau)", true, || {
        let mut p = Projets::new(None, Box::new(EnMemoire::default()));
        p.lier(&chemin).unwrap();
    });
    // Ce que fait vraiment le clic « Relier » : relier + ouvrir (analyse).
    let t = testeur(None);
    noter(Z, "clic Relier (lier + ouvrir + analyser un petit crate)", true, chrono(|| {
        let i = t.projets().lier(&chemin).unwrap();
        t.ouvrir(i);
    }).1);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
#[ignore]
fn projet() {
    mesurer();
    fin_de_zone(Z);
}
