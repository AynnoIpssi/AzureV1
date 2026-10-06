use crate::compiler::rsc::mangers::parser::parse as parse_rsc;
use crate::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use crate::compiler::rsh::mangers::parser::parse as parse_rsh;
use crate::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use crate::compiler::services::codegen::StyleSource;
use crate::compiler::services::condition::Context;
use crate::compiler::services::interpreter::build_ui_with_context;
use crate::compiler::components::{with_default_styles, Library};
use crate::navigation::models::route::Route;
use crate::navigation::models::router::{Request, Router};
use crate::ui::models::ui_node::UiNode;

/// Table locale des ecrans qu'UNE app accepte de servir (a elle-meme via
/// `goto`, ou aux autres apps via `navigate_to`) : chaque route reconstruit
/// l'arbre de `UiNode` a afficher. Syntaxe facon Laravel, voir
/// `router::Router`. Purement locale - ne parle jamais elle-meme au
/// routeur, voir `AzureWindow::routes` pour le branchement dans la boucle
/// d'evenements.
// `+ Send` sur les handlers (voir `Router`) : `RouteTable` est un champ de
// `AzureWindow`, et une fenetre ouverte depuis une autre est deplacee vers
// un thread a part.
pub type RouteTable = Router<Vec<UiNode>>;

impl Router<Vec<UiNode>> {
    /// Route vers une page rsH + rsC, en une ligne :
    /// `.view("/aide", "ui/aide.rsh", "ui/app.rsc")`. Les fichiers sont
    /// relus a chaque navigation (une page modifiee se voit sans relancer
    /// l'app). Les parametres `{nom}` du chemin et `payload` sont passes aux
    /// conditions rsH : `<if.id == "42">`. Un fichier illisible ou invalide
    /// est signale sur stderr et la navigation est ignoree.
    pub fn view(self, path: &str, rsh_file: &str, rsc_file: &str) -> RouteTable {
        let (rsh_file, rsc_file) = (rsh_file.to_string(), rsc_file.to_string());
        self.route(path, move |request| match load_view(&rsh_file, &rsc_file, request, Context::new()) {
            Ok(nodes) => nodes,
            Err(err) => {
                eprintln!("RouteTable: vue '{}' : {err}", request.path);
                Vec::new()
            }
        })
    }

    /// Comme `view`, avec des donnees : `data` fournit les variables de la
    /// page (listes, objets...) a chaque affichage, lues dans le rsH par
    /// `{{nom}}`, `<for.app in apps>`, `<if.app.actif>`. Les parametres du
    /// chemin et `payload` s'y ajoutent.
    pub fn view_with(self, path: &str, rsh_file: &str, rsc_file: &str, data: impl Fn(&Request) -> Context + Send + 'static) -> RouteTable {
        let (rsh_file, rsc_file) = (rsh_file.to_string(), rsc_file.to_string());
        self.route(path, move |request| {
            // Ce que coute la page (voir `perf`) : les donnees de l'app, puis
            // la construction par la fondation.
            let donnees = crate::perf::mesurer("Données de page", &request.path, || data(request));
            match crate::perf::mesurer("Page", &request.path, || load_view(&rsh_file, &rsc_file, request, donnees)) {
                Ok(nodes) => nodes,
                Err(err) => {
                    eprintln!("RouteTable: vue '{}' : {err}", request.path);
                    Vec::new()
                }
            }
        })
    }

    /// Le nouvel arbre `ui_nodes` pour `route`, ou `None` si aucune route
    /// ne correspond (et pas de `fallback`).
    pub fn resolve(&self, route: &Route) -> Option<Vec<UiNode>> {
        self.dispatch(&route.path, &route.payload)
    }
}

/// Ce que fait `view_with` pour une requete : lit, compile et construit la
/// page. Pour une route qui retouche l'arbre avant de l'afficher
/// (`.route(chemin, |r| { let mut n = load_view(...)?; ... })`).
pub fn load_view(rsh_file: &str, rsc_file: &str, request: &Request, data: Context) -> Result<Vec<UiNode>, String> {
    let vue = compiled(rsh_file, rsc_file)?;
    let mut ctx = data.with_library(vue.library.clone()).with_text("path", &request.path).with_text("payload", &request.payload);
    for (name, value) in request.params() {
        ctx = ctx.with_text(name, value);
    }
    Ok(build_ui_with_context(&vue.ast, &StyleSource::Rsc(&vue.sheet), &ctx))
}

/// Une page compilee : son rsH, sa feuille (styles d'Azure, des composants
/// et de la page) et ses composants.
struct Compiled {
    /// Dates de modification du rsH et du rsC, et styles des composants :
    /// si l'un change, on recompile (rechargement a chaud).
    stamp: (Option<std::time::SystemTime>, Option<std::time::SystemTime>, String),
    ast: std::sync::Arc<Vec<crate::compiler::rsh::mangers::parser::AstNode>>,
    sheet: std::sync::Arc<crate::compiler::rsc::models::rule::RscStylesheet>,
    library: std::sync::Arc<Library>,
}

/// La page compilee, gardee entre deux affichages : redessiner une page
/// (a chaque clic, a chaque resultat) ne relit ni ne reanalyse ses fichiers
/// tant qu'ils n'ont pas change.
fn compiled(rsh_file: &str, rsc_file: &str) -> Result<std::sync::Arc<Compiled>, String> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<(String, String), Arc<Compiled>>>> = OnceLock::new();
    let quand = |f: &str| std::fs::metadata(f).and_then(|m| m.modified()).ok();
    let cle = (rsh_file.to_string(), rsc_file.to_string());
    let cache = CACHE.get_or_init(Default::default);
    let deja = cache.lock().ok().and_then(|c| c.get(&cle).cloned());
    let library = deja.as_ref().map(|v| v.library.clone()).unwrap_or_else(|| Arc::new(Library::for_page(std::path::Path::new(rsh_file))));
    // Le theme compte aussi : en changer recompile la feuille.
    let stamp = (quand(rsh_file), quand(rsc_file), format!("{}\n{}", crate::theme::version(), library.styles()));
    if let Some(v) = deja
        && v.stamp == stamp
    {
        return Ok(v);
    }
    let rsh = std::fs::read_to_string(rsh_file).map_err(|e| format!("{rsh_file} : {e}"))?;
    let rsc = std::fs::read_to_string(rsc_file).map_err(|e| format!("{rsc_file} : {e}"))?;
    // Composants : ceux d'Azure, et `components/` a cote de la page ; leurs
    // styles passent avant ceux de la page.
    let rsc = with_default_styles(&rsc, Some(&library));
    let sheet = parse_rsc(tokenize_rsc(&rsc)).map_err(|e| format!("rsC invalide ({rsc_file}) : {e:?}"))?;
    warn_once(rsc_file, &sheet);
    let ast = parse_rsh(tokenize_rsh(&rsh)).map_err(|e| format!("rsH invalide ({rsh_file}) : {e}"))?;
    let vue = Arc::new(Compiled { stamp, ast: Arc::new(ast), sheet: Arc::new(sheet), library });
    if let Ok(mut c) = cache.lock() {
        c.insert(cle, vue.clone());
    }
    Ok(vue)
}

/// Propriétés rsC inconnues ou sans effet (voir `rsc::warnings`), signalées
/// une seule fois par fichier.
fn warn_once(file: &str, sheet: &crate::compiler::rsc::models::rule::RscStylesheet) {
    use std::sync::Mutex;
    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let Ok(mut seen) = SEEN.lock() else { return };
    if seen.iter().any(|f| f == file) {
        return;
    }
    seen.push(file.to_string());
    for warning in crate::compiler::rsc::warnings(sheet) {
        eprintln!("rsC ({file}) : {warning}");
    }
}
