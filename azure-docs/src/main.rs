// azure_docs : la documentation d'Azure, en app Azure. Le contenu est dans
// contenu/ (un fichier par page), l'interface dans ui/docs.rsh + docs.rsc.
use azure_docs::ecrans::{action, cle, routes, Action};
use azure_docs::{demos, service, Docs};
use azure_foundation::flux::Value;
use std::time::Duration;

/// `azure_docs --service` : la tache de fond `methodes` du manifeste, sans
/// fenetre. Azure la lance au premier appel d'une autre app (Azure Note) :
/// Docs n'a pas besoin d'etre ouverte pour qu'on y cherche.
fn servir(app: &azure_foundation::app::AzureApp, contenu: &std::path::Path) -> Result<(), String> {
    type Methode = fn(&Docs, &Value) -> Result<Value, String>;
    let mut servies = Vec::new();
    for (nom, f) in [("chercher", service::chercher as Methode), ("page", service::page)] {
        let contenu = contenu.to_path_buf();
        servies.push(app.serve(nom, move |req| f(&Docs::charger(&contenu)?, &req.args))?);
    }
    app.info("méthodes chercher et page servies");
    loop {
        std::thread::park();
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("azure-docs : {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // Installe (`azure install azure-docs`) ou lance par cargo run.
    let app = azure_foundation::azure_app!()?;
    let dir = app.manifest().dir.clone();
    let contenu = dir.join("contenu");
    if std::env::args().any(|a| a == "--service") {
        return servir(&app, &contenu);
    }
    let icone = app.manifest().icon.as_ref().and_then(|p| p.to_str()).map(str::to_string);
    let mut fenetre = app.window_with(routes(&dir.join("ui"), &contenu))?.windows(demos::fenetres(&dir.join("ui"), app.id(), icone.clone()));
    if let Some(icone) = &icone {
        fenetre = fenetre.icon(icone);
    }
    fenetre
        .on_click(move |ctx| {
            let Some(clique) = ctx.clicked else { return };
            match action(clique) {
                Some(Action::Aller(chemin)) => {
                    ctx.goto(&chemin, "");
                }
                Some(Action::Chercher) => {
                    // Le bouton, ou Entree dans le champ.
                    let q = ctx.value("q").unwrap_or_default();
                    ctx.goto("/recherche", q.trim());
                }
                Some(Action::Copier(c)) => {
                    let exemple = Docs::charger(&contenu).ok().and_then(|d| d.pages().flat_map(|p| p.exemples()).find(|e| cle(&e.id) == c).cloned());
                    if let Some(exemple) = exemple {
                        ctx.copy(&exemple.code);
                        ctx.flash(clique, "Copié", Duration::from_millis(1500));
                    }
                }
                Some(Action::Demo(nom)) => {
                    if let Err(e) = demos::ouvrir(ctx, &nom) {
                        eprintln!("azure-docs : démonstration « {nom} » : {e}");
                    }
                }
                None => {}
            }
        })
        .run();
    Ok(())
}
