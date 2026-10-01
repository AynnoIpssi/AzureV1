// azure_note : des pages dans des pages (texte mis en forme, code, tableaux,
// bases avec vues table / kanban / liste / galerie, proprietes calculees).
// La recherche en haut ouvre une petite fenetre Doc, remplie par Azure Docs
// (ses methodes `chercher` et `page`).
use azure_foundation::app::AzureApp;
use azure_foundation::flux::Value;
use azure_foundation::window::models::window_context::WindowContext;
use azure_note::carnet::maintenant;
use azure_note::classeur::Classeur;
use azure_note::clics::{agir, cliquer, deposer, Lecture, Suite};
use azure_note::doc::{self, SourceDoc};
use azure_note::ecrans::routes;
use azure_note::page::Etat;
use std::sync::{Arc, Mutex};

fn main() {
    if let Err(e) = run() {
        eprintln!("azure-note : {e}");
        std::process::exit(1);
    }
}

/// Azure Docs, par les appels entre apps.
struct AzureDocs(Arc<AzureApp>);

impl SourceDoc for AzureDocs {
    fn appeler(&self, methode: &str, args: Value) -> Result<Value, String> {
        self.0.call("docs", methode, args)
    }

    fn ouvrir_dans_docs(&self, chemin: &str) -> Result<(), String> {
        self.0.navigate("docs", chemin, "")
    }
}

/// Les champs de l'ecran, lus au moment du clic.
struct Champs<'a, 'b>(&'a WindowContext<'b>);

impl Lecture for Champs<'_, '_> {
    fn valeur(&self, id: &str) -> Option<String> {
        self.0.value(id)
    }
}

fn run() -> Result<(), String> {
    // Installe (`azure install azure-note`) ou lance par cargo run.
    let app = Arc::new(azure_foundation::azure_app!()?);
    let dir = app.manifest().dir.clone();
    let classeur = match app.stockage().and_then(Classeur::ouvrir) {
        Ok(classeur) => classeur,
        Err(e) => {
            eprintln!("azure-note : stockage indisponible, les pages ne seront pas gardées : {e}");
            Classeur::en_memoire()
        }
    };
    let etat = Arc::new(Mutex::new(Etat { page: classeur.lire(|e| e.enfants(None).first().map(|p| p.id)), ..Etat::default() }));
    let icone = app.manifest().icon.as_ref().and_then(|p| p.to_str()).map(str::to_string);
    let source: doc::Source = Arc::new(AzureDocs(Arc::clone(&app)));
    let mut fenetre = app.window_with(routes(&dir.join("ui"), &classeur, Arc::clone(&etat)))?.windows(doc::fenetres(&dir.join("ui"), app.id(), icone.clone(), source));
    if let Some(icone) = &icone {
        fenetre = fenetre.icon(icone);
    }
    let (c1, c2, c3) = (classeur.clone(), classeur.clone(), classeur);
    let (e1, e2, e3) = (Arc::clone(&etat), Arc::clone(&etat), etat);
    fenetre
        .on_click(move |ctx| {
            let Some(clique) = ctx.clicked.map(str::to_string) else { return };
            match agir(&c1, &e1, &Champs(ctx), |e, etat| cliquer(e, etat, &clique, &Champs(ctx), maintenant())) {
                Suite::Redessiner => {
                    ctx.goto("/", "");
                }
                Suite::ChercherDoc => {
                    let q = ctx.value("doc-q").unwrap_or_default();
                    if let Err(e) = ctx.open_window("/doc", q.trim()) {
                        eprintln!("azure-note : fenêtre Doc : {e}");
                    }
                }
                Suite::Rien => {}
            }
        })
        .on_drop(move |ctx, d| {
            let (source, cible, position) = (d.source.clone(), d.target.clone(), d.position);
            agir(&c2, &e2, &Champs(ctx), |e, etat| deposer(e, etat, &source, &cible, position).map(|_| Suite::Redessiner));
            ctx.goto("/", "");
        })
        .on_close(move |ctx| {
            agir(&c3, &e3, &Champs(ctx), |_, _| Ok(Suite::Rien));
        })
        .run();
    Ok(())
}
