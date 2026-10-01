// Verification de bout en bout (voir scripts/verification-e2e.sh) : la
// caisse ecoute le panier de la boutique, appelle sa methode `prix`, lui
// envoie un message, et tente de sortir de son bac a sable.
//   e2e_caisse phase1   (avant et pendant la boutique)
//   e2e_caisse verif    (apres redemarrage d'azure-service : flux persistant)
//   e2e_caisse reveil   (la boutique n'a jamais tourne : `taxe` la reveille)
use azure_foundation::app::AzureApp;
use azure_foundation::flux::Value;
use std::time::{Duration, Instant};

fn main() {
    let phase = std::env::args().nth(1).unwrap_or_else(|| "phase1".to_string());
    let app = match AzureApp::find(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/e2e/caisse")) {
        Ok(app) => app,
        Err(e) => {
            println!("E2E caisse ERREUR demarrage : {e}");
            std::process::exit(1);
        }
    };
    println!("E2E caisse [{phase}] id={} enfermee={}", app.id(), azure_core::security::sandbox::is_sandboxed());

    if phase == "reveil" {
        // Personne n'a ouvert la boutique : Azure lance sa tache de fond.
        let debut = Instant::now();
        match app.call("e2e-boutique", "taxe", Value::Null) {
            Ok(v) => println!("E2E caisse [reveil] taxe={v} en {} ms", debut.elapsed().as_millis()),
            Err(e) => println!("E2E caisse ERREUR taxe : {e}"),
        }
        match app.call("e2e-boutique", "taxe", Value::Null) {
            Ok(v) => println!("E2E caisse [reveil] taxe-encore={v}"),
            Err(e) => println!("E2E caisse ERREUR taxe-encore : {e}"),
        }
        println!("E2E caisse [reveil] fin");
        return;
    }

    if phase == "phase1" {
        // La boutique ne tourne pas encore : le message l'attendra.
        match app.send("e2e-boutique", "bonjour de la caisse") {
            Ok(()) => println!("E2E caisse message envoye"),
            Err(e) => println!("E2E caisse ERREUR message : {e}"),
        }
        match app.call_timeout("e2e-boutique", "prix", Value::map([("produit", 2.into())]), Duration::from_millis(500)) {
            Ok(v) => println!("E2E caisse prix-avant={v}"),
            Err(e) => println!("E2E caisse prix-avant-erreur={e}"),
        }
        let _ = app.stockage().and_then(|s| s.set("caisse", "ouverte"));
        // Lire le dossier de la boutique : interdit.
        let voisine = std::fs::read(azure_provider::install_root().join("apps/e2e-boutique/app.azure"));
        println!("E2E caisse dossier-voisin-lisible={}", voisine.is_ok());
        app.error("erreur de test (volontaire)");
    }

    // Le panier de la boutique (etat actuel puis modifications).
    let mut ecoute = match app.listen("panier@e2e-boutique").and_then(|b| b.start()) {
        Ok(l) => l,
        Err(e) => {
            println!("E2E caisse ERREUR ecoute : {e}");
            std::process::exit(1);
        }
    };
    let deadline = Instant::now() + Duration::from_secs(25);
    while (ecoute.get("total") != Some(&Value::Int(42)) || ecoute.get("items").is_none()) && Instant::now() < deadline {
        ecoute.wait(Duration::from_millis(200));
    }
    println!("E2E caisse [{phase}] panier={}", ecoute.state());

    if phase == "phase1" {
        // La boutique vient de demarrer : `prix` peut ne pas etre encore
        // servie. `call_wait` attend qu'elle le soit.
        match app.call_wait("e2e-boutique", "prix", Value::map([("produit", 14.into())]), Duration::from_secs(5)) {
            Ok(v) => println!("E2E caisse prix={v}"),
            Err(e) => println!("E2E caisse ERREUR prix : {e}"),
        }
        match app.call("e2e-boutique", "prix", Value::Null) {
            Ok(v) => println!("E2E caisse prix-sans-produit={v}"),
            Err(e) => println!("E2E caisse prix-sans-produit-erreur={e}"),
        }
    }
    println!("E2E caisse [{phase}] fin");
}
