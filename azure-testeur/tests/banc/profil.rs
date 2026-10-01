// Boucle sans fin sur une etape, pour l'echantillonner de l'exterieur
// (gdb, perf) : AZURE_BANC_PROFIL=construire|dessiner|souris.
use crate::commun::*;
use azure_testeur::clics::cliquer;

#[test]
#[ignore]
fn profil() {
    let Some(env) = environnement() else { return };
    let t = testeur(Some(env));
    let rien = Valeurs(Default::default());
    if std::env::var("AZURE_BANC_ECRAN").as_deref() == Ok("detail") {
        cliquer(&t, "voir-0", &rien);
    }
    let quoi = std::env::var("AZURE_BANC_PROFIL").unwrap_or_else(|_| "construire".into());
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let nodes = construire(&t);
    while std::time::Instant::now() < fin {
        match quoi.as_str() {
            "dessiner" => dessiner(&nodes),
            _ => drop(construire(&t)),
        }
    }
}
