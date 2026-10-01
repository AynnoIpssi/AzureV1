// Banc de mesure de Testeur : chaque partie du code est chronometree, et
// tout ce qui tourne sur le thread de la fenetre est classe par cout. Une
// image = 16 ms ; au-dela la fenetre saccade, au-dela de 100 ms elle fige.
//
// Tout (dans l'ordre, avec le bilan general et « Tout lancer » sur Azure) :
//   cargo test --release -p azure-testeur --test banc -- --ignored tout --nocapture
// Une partie : ... --test banc -- --ignored langage --nocapture
// Rapports : target/tmp/azure-testeur/banc-*.txt
//
// En debug les durees sont gonflees : les seuils ne sont verifies qu'en
// release.
mod clics;
mod commun;
mod ecran;
mod execution;
mod fenetre;
mod langage;
mod profil;
mod projet;
mod tout_lancer;

/// Toutes les mesures, puis le bilan : ce qui fige la fenetre, du pire au
/// moins grave. Echoue s'il reste un gel.
#[test]
#[ignore]
fn tout() {
    projet::mesurer();
    langage::mesurer();
    execution::mesurer();
    ecran::mesurer();
    clics::mesurer();
    fenetre::mesurer();
    tout_lancer::mesurer();
    let bilan = commun::rapport(None);
    eprintln!("\n======== BILAN GENERAL ========\n{bilan}");
    commun::ecrire_rapport("tout", &bilan);
    commun::verifier_gels(None);
}
