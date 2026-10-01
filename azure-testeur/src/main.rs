// azure_testeur : les tests d'un projet (ou d'Azure lui-meme) - les voir,
// les lancer, en ecrire de nouveaux directement dans le code.
use azure_foundation::storage::models::stockage::Stockage;
use azure_foundation::window::models::window_context::WindowContext;
use azure_testeur::composants::{Garde, GardeMemoire};
use azure_testeur::clics::{cliquer, ranger, Lecture, Redessin};
use azure_testeur::ecran::{routes, Rafraichir, Testeur};
use azure_testeur::projet::{self, EnMemoire, Memoire, Projets};
use std::sync::Arc;

fn main() {
    if let Err(e) = run() {
        eprintln!("azure-testeur : {e}");
        std::process::exit(1);
    }
}

/// Les projets relies, dans le stockage prive de l'app.
struct DansLeStockage(Stockage);

impl Memoire for DansLeStockage {
    fn lire(&self) -> Vec<String> {
        self.0.get::<String>("projets").ok().flatten().unwrap_or_default().lines().filter(|l| !l.is_empty()).map(str::to_string).collect()
    }

    fn ecrire(&self, chemins: &[String]) -> Result<(), String> {
        self.0.set("projets", chemins.join("\n"))
    }
}

/// Les composants de l'utilisateur, dans le meme stockage prive.
struct ComposantsDansLeStockage(Stockage);

impl Garde for ComposantsDansLeStockage {
    fn charger(&self) -> String {
        self.0.get::<String>("composants").ok().flatten().unwrap_or_default()
    }

    fn garder(&self, texte: &str) -> Result<(), String> {
        self.0.set("composants", texte.to_string())
    }
}

/// Les champs de l'ecran, lus au moment du clic.
struct Champs<'a, 'b>(&'a WindowContext<'b>);

impl Lecture for Champs<'_, '_> {
    fn valeur(&self, id: &str) -> Option<String> {
        self.0.value(id)
    }

    fn coche(&self, id: &str) -> bool {
        self.0.checked(id)
    }
}

fn run() -> Result<(), String> {
    // Les sources d'Azure sont notees dans un dossier que l'app ne pourra
    // plus lire une fois enfermee : on les lit avant.
    let environnement = projet::environnement();
    let app = azure_foundation::azure_app!()?;
    let dir = app.manifest().dir.clone();
    let (memoire, garde): (Box<dyn Memoire>, Box<dyn Garde>) = match (app.stockage(), app.stockage()) {
        (Ok(projets), Ok(composants)) => (Box::new(DansLeStockage(projets)), Box::new(ComposantsDansLeStockage(composants))),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("azure-testeur : stockage indisponible, projets reliés et composants ne seront pas gardés : {e}");
            (Box::new(EnMemoire::default()), Box::new(GardeMemoire::default()))
        }
    };
    let testeur = Testeur::avec(Projets::new(environnement, memoire), garde);
    let mut fenetre = app.window_with(routes(&dir.join("ui"), &testeur))?;
    if let Some(icone) = app.manifest().icon.as_ref().and_then(|p| p.to_str()) {
        fenetre = fenetre.icon(icone);
    }
    let (clics, tics) = (Arc::clone(&testeur), testeur);
    let mut rafraichir = Rafraichir::new();
    fenetre
        .on_click(move |ctx| {
            let Some(id) = ctx.clicked.map(str::to_string) else { return };
            let r = cliquer(&clics, &id, &Champs(ctx));
            match r.redessin {
                Redessin::Rien => return,
                Redessin::Garder => ctx.refresh("/", ""),
                Redessin::Haut => ctx.goto("/", ""),
            };
            if let Some(cible) = &r.defiler {
                ctx.scroll_to(cible);
            }
        })
        // Les resultats arrivent (thread d'execution) : voir `Rafraichir`.
        .on_tick(move |ctx| {
            if rafraichir.maintenant(&tics) {
                ranger(&tics, &Champs(ctx));
                ctx.refresh("/", "");
            }
        })
        .run();
    Ok(())
}
