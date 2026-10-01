// La petite fenetre « Doc » : une recherche dans la documentation, puis la
// page choisie. Le contenu vient d'Azure Docs, par ses methodes `chercher`
// et `page` (voir azure-docs/src/service.rs) ; Note ne fait que l'afficher
// (ui/doc.rsh + ui/doc.rsc, le meme rendu que dans Docs).
//
//   /       (donnees : la recherche)  les resultats
//   /page   (donnees : `/doc/<section>/<page>`, ou `exemple:<id>`)  une page
use azure_core::models::window_model::{WindowSize, WindowSpec};
use azure_foundation::compiler::services::condition::{ConditionValue, Context};
use azure_foundation::flux::{to_rsh, Value};
use azure_foundation::navigation::managers::intra_navigation_manager::connect;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::window::models::window::AzureWindow;
use azure_foundation::window::models::window_context::WindowContext;
use azure_foundation::window::models::window_table::WindowTable;
use azure_rooter::managers::intra_router::IntraRouter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// D'ou vient la doc : Azure Docs (`AzureApp::call`), ou autre chose dans
/// les tests.
pub trait SourceDoc: Send + Sync {
    /// Appelle la methode `methode` d'Azure Docs.
    fn appeler(&self, methode: &str, args: Value) -> Result<Value, String>;
    /// Ouvre `chemin` dans la fenetre d'Azure Docs.
    fn ouvrir_dans_docs(&self, chemin: &str) -> Result<(), String>;
}

pub type Source = Arc<dyn SourceDoc>;

/// Taille de la fenetre Doc.
pub const TAILLE: (u32, u32) = (760, 680);

/// Une erreur d'appel, dite simplement. Docs n'a pas besoin d'etre
/// ouverte (Azure lance sa tache de fond) : si elle ne repond pas, c'est
/// qu'elle n'est pas installee ou que son service n'a pas pu demarrer.
pub fn expliquer(erreur: &str) -> String {
    if erreur.contains("n'est pas disponible") || erreur.contains("inconnue") {
        "Azure Docs ne répond pas : est-elle installée ? (azure install azure-docs)".to_string()
    } else {
        format!("Azure Docs a répondu : {erreur}")
    }
}

fn texte(s: &str) -> ConditionValue {
    ConditionValue::Text(s.to_string())
}

fn erreur(e: &str) -> Context {
    Context::new().with_text("vue", "erreur").with_text("q", "").with_text("erreur", &expliquer(e))
}

/// Ce que la fenetre a affiche en dernier (pour les clics).
#[derive(Default)]
pub struct Memoire {
    pub q: String,
    /// Cible (donnees de `/page`) de chaque resultat affiche.
    pub cibles: Vec<String>,
    /// La page affichee.
    pub page: Option<Value>,
    /// La derniere route, pour « Réessayer ».
    pub derniere: (String, String),
}

/// La cible d'un resultat : sa page, ou l'exemple lui-meme.
fn cible(r: &Value) -> String {
    let champ = |c: &str| r.get(c).and_then(Value::as_str).unwrap_or("").to_string();
    if champ("genre") == "exemple" { format!("exemple:{}", champ("id")) } else { champ("chemin") }
}

/// L'ecran des resultats pour `q`.
pub fn resultats(source: &dyn SourceDoc, memoire: &Mutex<Memoire>, q: &str) -> Context {
    let mut m = memoire.lock().unwrap_or_else(|e| e.into_inner());
    m.q = q.to_string();
    m.derniere = ("/".to_string(), q.to_string());
    m.cibles.clear();
    if q.trim().is_empty() {
        return Context::new().with_text("vue", "resultats").with_text("q", "").with_number("nb", 0.0).with_value("resultats", ConditionValue::List(Vec::new()));
    }
    let trouves = match source.appeler("chercher", Value::map([("q", Value::from(q))])) {
        Ok(v) => v,
        Err(e) => return erreur(&e).with_text("q", q),
    };
    let trouves = trouves.as_list().unwrap_or(&[]).to_vec();
    m.cibles = trouves.iter().map(cible).collect();
    let items = trouves.iter().enumerate().map(|(i, r)| match to_rsh(r) {
        ConditionValue::Map(mut map) => {
            map.insert("i".to_string(), texte(&i.to_string()));
            ConditionValue::Map(map)
        }
        autre => autre,
    });
    Context::new().with_text("vue", "resultats").with_text("q", q).with_number("nb", trouves.len() as f64).with_value("resultats", ConditionValue::List(items.collect()))
}

/// L'ecran d'une page (`cible` : voir `/page`).
pub fn page(source: &dyn SourceDoc, memoire: &Mutex<Memoire>, cible: &str) -> Context {
    let mut m = memoire.lock().unwrap_or_else(|e| e.into_inner());
    m.derniere = ("/page".to_string(), cible.to_string());
    let args = match cible.strip_prefix("exemple:") {
        Some(id) => Value::map([("exemple", Value::from(id))]),
        None => Value::map([("chemin", Value::from(cible))]),
    };
    let p = match source.appeler("page", args) {
        Ok(p) => p,
        Err(e) => return erreur(&e).with_text("q", &m.q),
    };
    // L'exemple cherche est aussi montre en tete de page.
    let vise = p.get("exemple").and_then(Value::as_str).unwrap_or("").to_string();
    let exemple: Vec<ConditionValue> = p.get("blocs").and_then(Value::as_list).unwrap_or(&[]).iter().filter(|b| !vise.is_empty() && b.get("cle").and_then(Value::as_str) == Some(vise.as_str())).map(to_rsh).collect();
    let ctx = Context::new().with_text("vue", "page").with_text("q", &m.q).with_value("doc", to_rsh(&p)).with_value("vise", ConditionValue::List(exemple));
    m.page = Some(p);
    ctx
}

/// Le code de l'exemple `cle` de la page affichee.
pub fn code(memoire: &Mutex<Memoire>, cle: &str) -> Option<String> {
    let m = memoire.lock().unwrap_or_else(|e| e.into_inner());
    let bloc = m.page.as_ref()?.get("blocs")?.as_list()?.iter().find(|b| b.get("cle").and_then(Value::as_str) == Some(cle))?.clone();
    let lignes = bloc.get("lignes")?.as_list()?.iter().map(|l| l.as_list().unwrap_or(&[]).iter().filter_map(|m| m.get("v").and_then(Value::as_str)).collect::<String>());
    Some(lignes.collect::<Vec<_>>().join("\n"))
}

#[derive(Clone)]
struct Visionneuse {
    ui: PathBuf,
    app_id: u32,
    icone: Option<String>,
    source: Source,
}

impl Visionneuse {
    fn routes(&self, memoire: &Arc<Mutex<Memoire>>) -> RouteTable {
        let rsh = self.ui.join("doc.rsh").to_string_lossy().into_owned();
        let rsc = self.ui.join("doc.rsc").to_string_lossy().into_owned();
        let (s1, m1, s2, m2) = (self.source.clone(), memoire.clone(), self.source.clone(), memoire.clone());
        RouteTable::new().view_with("/", &rsh, &rsc, move |r| resultats(&*s1, &m1, &r.payload)).view_with("/page", &rsh, &rsc, move |r| page(&*s2, &m2, &r.payload))
    }

    fn fenetre(&self, q: &str) -> AzureWindow {
        let memoire = Arc::new(Mutex::new(Memoire::default()));
        let routes = self.routes(&memoire);
        let ecran = routes.resolve(&Route::new("/", q)).unwrap_or_default();
        let taille = WindowSize::new(TAILLE.0, TAILLE.1).expect("taille valide");
        let titre = if q.trim().is_empty() { "Doc".to_string() } else { format!("Doc — {}", q.trim()) };
        let mut w = AzureWindow::new(&titre).spec(WindowSpec::internal(self.app_id, taille)).app_id("azure-note");
        if let Some(icone) = &self.icone {
            w = w.icon(icone);
        }
        let source = self.source.clone();
        w.ui(ecran).intra(connect(&IntraRouter::new(), 1)).routes(routes).on_click(move |ctx| clic(ctx, &*source, &memoire))
    }
}

/// Un clic dans la fenetre Doc.
pub fn clic(ctx: &mut WindowContext, source: &dyn SourceDoc, memoire: &Mutex<Memoire>) {
    let Some(id) = ctx.clicked else { return };
    let lire = |f: fn(&Memoire) -> String| f(&memoire.lock().unwrap_or_else(|e| e.into_inner()));
    match id {
        "chercher" | "q" => {
            let q = ctx.value("q").unwrap_or_default();
            ctx.goto("/", q.trim());
        }
        "retour" => {
            ctx.goto("/", &lire(|m| m.q.clone()));
        }
        "reessayer" => {
            let (chemin, donnees) = memoire.lock().unwrap_or_else(|e| e.into_inner()).derniere.clone();
            ctx.goto(&chemin, &donnees);
        }
        "ouvrir-docs" => {
            let chemin = lire(|m| m.page.as_ref().and_then(|p| p.get("chemin")).and_then(Value::as_str).unwrap_or("").to_string());
            let dit = match source.ouvrir_dans_docs(&chemin) {
                Ok(()) => "Ouverte dans Azure Docs",
                Err(e) => {
                    eprintln!("azure-note : ouvrir {chemin} dans Azure Docs : {e}");
                    "Azure Docs indisponible"
                }
            };
            ctx.flash(id, dit, Duration::from_millis(1800));
        }
        _ => {
            if let Some(i) = id.strip_prefix("r-").and_then(|i| i.parse::<usize>().ok()) {
                let cible = memoire.lock().unwrap_or_else(|e| e.into_inner()).cibles.get(i).cloned();
                if let Some(cible) = cible {
                    ctx.goto("/page", &cible);
                }
            } else if let Some(cle) = id.strip_prefix("copier-").or_else(|| id.strip_prefix("vu-copier-"))
                && let Some(code) = code(memoire, cle)
            {
                ctx.copy(&code);
                ctx.flash(id, "Copié", Duration::from_millis(1500));
            }
        }
    }
}

/// Les fenetres de Note, pour `AzureWindow::windows` : `/doc` (donnees : la
/// recherche) ouvre une fenetre Doc.
pub fn fenetres(ui: &Path, app_id: u32, icone: Option<String>, source: Source) -> WindowTable {
    let v = Visionneuse { ui: ui.to_path_buf(), app_id, icone, source };
    WindowTable::new().on("/doc", move |q| v.fenetre(q))
}
