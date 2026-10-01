// Les demonstrations d'Azure Docs. Un bloc `> demo <nom>: texte` d'une page
// affiche un bouton « Ouvrir » (`#demo-<nom>`) qui ouvre de VRAIES fenetres
// par `ctx.open_window`, comme n'importe quelle app :
//
//   fenetre    une fenetre avec son propre on_click (un compteur)
//   tailles    trois fenetres de tailles differentes
//   partagee   une fenetre construite depuis sa seule source rsH + rsC,
//              comme l'ouvre une app qui la recoit
//   routeur    deux fenetres : l'Emetteur envoie des routes au Recepteur,
//              qui se redessine a chaque message
//
// Les interfaces sont dans ui/demos/.
use azure_core::models::window_model::{WindowSize, WindowSpec};
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::navigation::managers::intra_navigation_manager::connect;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::window::models::shared_window::SharedWindow;
use azure_foundation::window::models::window::AzureWindow;
use azure_foundation::window::models::window_context::WindowContext;
use azure_foundation::window::models::window_table::WindowTable;
use azure_rooter::managers::intra_router::IntraRouter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Les noms acceptes par `> demo <nom>:`.
pub const DEMOS: [&str; 4] = ["fenetre", "tailles", "partagee", "routeur"];

/// `(nom, largeur, hauteur)` des fenetres de la demonstration « tailles ».
pub const TAILLES: [(&str, u32, u32); 3] = [("Petite", 380, 280), ("Moyenne", 560, 380), ("Grande", 780, 520)];

/// Separe les champs d'un message de l'Emetteur (le routeur, lui, separe
/// chemin et donnees par U+001F).
const SEP: char = '\u{1e}';

/// Chaque ouverture a ses propres vues dans le routeur : deux paires
/// Emetteur/Recepteur ouvertes l'une apres l'autre ne se melangent pas.
static SESSIONS: AtomicU32 = AtomicU32::new(0);

fn session() -> String {
    SESSIONS.fetch_add(1, Ordering::Relaxed).to_string()
}

/// Les vues `(emetteur, recepteur)` d'une session.
fn vues(session: &str) -> (u32, u32) {
    let n: u32 = session.parse().unwrap_or(0);
    (1 + 2 * n, 2 + 2 * n)
}

/// Ouvre la demonstration `nom` depuis un clic.
pub fn ouvrir(ctx: &mut WindowContext, nom: &str) -> Result<(), String> {
    match nom {
        "fenetre" => ctx.open_window("/demo/fenetre", &session()),
        "tailles" => (0..TAILLES.len()).try_for_each(|i| ctx.open_window("/demo/taille", &i.to_string())),
        "partagee" => ctx.open_window("/demo/partagee", ""),
        "routeur" => {
            // Le Recepteur d'abord : sa boite existe avant le premier envoi.
            let s = session();
            ctx.open_window("/demo/routeur/recepteur", &s)?;
            ctx.open_window("/demo/routeur/emetteur", &s)
        }
        _ => Err(format!("démonstration inconnue : {nom}")),
    }
}

#[derive(Clone)]
struct Demos {
    dossier: PathBuf,
    app_id: u32,
    icone: Option<String>,
    routeur: IntraRouter,
}

impl Demos {
    fn chemin(&self, fichier: &str) -> String {
        self.dossier.join(fichier).to_string_lossy().into_owned()
    }

    /// Une page d'ui/demos/ avec ses donnees.
    fn page(&self, rsh: &str, rsc: &str, donnees: impl Fn(&str) -> Context + Send + 'static) -> RouteTable {
        RouteTable::new().view_with("/", &self.chemin(rsh), &self.chemin(rsc), move |r| donnees(&r.payload))
    }

    fn fenetre(&self, titre: &str, largeur: u32, hauteur: u32) -> AzureWindow {
        let taille = WindowSize::new(largeur, hauteur).expect("taille de démonstration valide");
        let mut w = AzureWindow::new(titre).spec(WindowSpec::internal(self.app_id, taille)).app_id("azure-docs");
        if let Some(icone) = &self.icone {
            w = w.icon(icone);
        }
        w
    }

    /// Une page et son premier ecran, branchee au routeur sous `vue`.
    fn avec_page(&self, w: AzureWindow, routes: RouteTable, depart: &str, vue: u32) -> AzureWindow {
        let ecran = routes.resolve(&Route::new("/", depart)).unwrap_or_default();
        w.ui(ecran).intra(connect(&self.routeur, vue)).routes(routes)
    }

    fn compteur(&self, session: &str) -> AzureWindow {
        let routes = self.page("fenetre.rsh", "demos.rsc", |n| Context::new().with_text("n", if n.is_empty() { "0" } else { n }));
        let mut n = 0u32;
        // Au-dela des vues du routeur de demonstration.
        let vue = 1_000_000 + vues(session).0;
        self.avec_page(self.fenetre("Une vraie fenêtre", 540, 420), routes, "0", vue).on_click(move |ctx| {
            if ctx.clicked == Some("cliquer") {
                n += 1;
                ctx.goto("/", &n.to_string());
            }
        })
    }

    fn taille(&self, rang: &str) -> AzureWindow {
        let (nom, largeur, hauteur) = TAILLES[rang.parse::<usize>().unwrap_or(0).min(TAILLES.len() - 1)];
        let routes = self.page("taille.rsh", "demos.rsc", move |_| {
            Context::new().with_text("nom", nom).with_text("largeur", &largeur.to_string()).with_text("hauteur", &hauteur.to_string())
        });
        let ecran = routes.resolve(&Route::new("/", "")).unwrap_or_default();
        self.fenetre(&format!("{nom} fenêtre"), largeur, hauteur).ui(ecran)
    }

    fn partagee(&self) -> AzureWindow {
        let lire = |f: &str| std::fs::read_to_string(self.dossier.join(f)).map_err(|e| format!("{f} : {e}"));
        let taille = WindowSize::new(460, 320).expect("taille de démonstration valide");
        let recue = lire("note.rsh").and_then(|rsh| {
            let rsc = lire("note.rsc")?;
            SharedWindow::new(WindowSpec::internal(self.app_id, taille), "Note partagée", &rsh, &rsc).to_window()
        });
        recue.unwrap_or_else(|e| {
            eprintln!("azure-docs : démonstration « partagee » : {e}");
            self.fenetre("Note partagée (illisible)", 460, 320)
        })
    }

    fn emetteur(&self, session: &str) -> AzureWindow {
        let (emetteur, recepteur) = vues(session);
        let ecran = self.page("emetteur.rsh", "demos.rsc", |_| Context::new()).resolve(&Route::new("/", "")).unwrap_or_default();
        let mut etat = Etat::default();
        self.fenetre("Émetteur", 500, 500).ui(ecran).intra(connect(&self.routeur, emetteur)).on_click(move |ctx| {
            let Some(id) = ctx.clicked else { return };
            let Some(dernier) = etat.appliquer(id, ctx.value("message").as_deref()) else { return };
            ctx.goto_view(recepteur, "/", &etat.message(&dernier));
            if id == "envoyer" {
                ctx.flash(id, "Envoyé", Duration::from_millis(1200));
            }
        })
    }

    fn recepteur(&self, session: &str) -> AzureWindow {
        let routes = self.page("recepteur.rsh", "demos.rsc", recepteur);
        self.avec_page(self.fenetre("Récepteur", 500, 460), routes, "", vues(session).1)
    }
}

/// Ce que l'Emetteur a choisi jusqu'ici.
#[derive(Debug, Clone, PartialEq)]
pub struct Etat {
    pub couleur: String,
    pub texte: String,
    pub compteur: i64,
    pub envois: u32,
}

impl Default for Etat {
    fn default() -> Etat {
        Etat { couleur: "sable".to_string(), texte: "Bonjour !".to_string(), compteur: 0, envois: 0 }
    }
}

impl Etat {
    /// Applique le clic sur `#id` (`message` : le champ texte) ; rend ce
    /// qui a change, `None` si rien n'est a envoyer.
    pub fn appliquer(&mut self, id: &str, message: Option<&str>) -> Option<String> {
        let dernier = match id {
            "envoyer" => {
                let m = message.unwrap_or("").trim();
                if m.is_empty() {
                    return None;
                }
                self.texte = m.to_string();
                format!("texte « {m} »")
            }
            "plus" => {
                self.compteur += 1;
                "compteur +1".to_string()
            }
            "moins" => {
                self.compteur -= 1;
                "compteur −1".to_string()
            }
            _ => {
                let c = id.strip_prefix("c-")?;
                self.couleur = c.to_string();
                format!("couleur {c}")
            }
        };
        self.envois += 1;
        Some(dernier)
    }

    /// Les donnees de la route envoyee au Recepteur.
    pub fn message(&self, dernier: &str) -> String {
        [self.couleur.as_str(), &self.texte, &self.compteur.to_string(), &self.envois.to_string(), dernier].join(&SEP.to_string())
    }
}

/// L'ecran du Recepteur pour les donnees `payload` (vide : rien recu).
pub fn recepteur(payload: &str) -> Context {
    let champs: Vec<&str> = payload.splitn(5, SEP).collect();
    let [couleur, texte, compteur, envois, dernier] = champs[..] else {
        return Context::new().with_text("couleur", "vide").with_text("message", "En attente de l'Émetteur…").with_text("compteur", "").with_text("recus", "0").with_text("dernier", "—");
    };
    Context::new().with_text("couleur", couleur).with_text("message", texte).with_text("compteur", compteur).with_text("recus", envois).with_text("dernier", dernier)
}

/// Les fenetres de demonstration, pour `AzureWindow::windows`. `ui` : le
/// dossier ui/ de l'app ; `icone` : celle de sa barre de titre.
pub fn fenetres(ui: &Path, app_id: u32, icone: Option<String>) -> WindowTable {
    let d = Demos { dossier: ui.join("demos"), app_id, icone, routeur: IntraRouter::new() };
    let (d1, d2, d3, d4, d5) = (d.clone(), d.clone(), d.clone(), d.clone(), d);
    WindowTable::new()
        .on("/demo/fenetre", move |s| d1.compteur(s))
        .on("/demo/taille", move |rang| d2.taille(rang))
        .on("/demo/partagee", move |_| d3.partagee())
        .on("/demo/routeur/emetteur", move |s| d4.emetteur(s))
        .on("/demo/routeur/recepteur", move |s| d5.recepteur(s))
}
