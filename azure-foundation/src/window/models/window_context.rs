// Poignee passee a `AzureWindow::on_click` (voir celle-ci) : le SEUL endroit
// ou une app distingue explicitement navigation INTRA-app (`goto`, transportee
// par `azure_rooter::managers::intra_router::IntraRouter` - voir
// `navigation::managers::intra_navigation_manager`, aucune IPC) de navigation
// INTER-app (`navigate_to`, envoyee via `azure-rooter` a une AUTRE app - voir
// `navigation::managers::navigation_manager::navigate`, une vraie socket
// Unix). Les deux partagent le meme principe : un message qui, une fois
// recu, resout un chemin en arbre `UiNode` final (voir
// `navigation::models::route_table::RouteTable`) - seul le transport differe.
use crate::navigation::managers::navigation_manager;
use crate::navigation::models::intra_client::IntraClient;
use crate::navigation::models::navigation_client::NavigationClient;
use crate::navigation::models::route_table::RouteTable;
use crate::navigation::models::router::named_path;
use crate::navigation::managers::intra_navigation_manager;
use crate::window::models::window_table::WindowTable;
use crate::storage::models::stockage::Stockage;
use crate::window::models::shared_window::SharedWindow;
use azure_core::models::window_model::WindowKind;

// `Default` : tous les champs a `None`, pour construire un contexte partiel
// (`WindowContext { intra: Some(&intra), ..Default::default() }`).
#[derive(Default)]
pub struct WindowContext<'a> {
    pub intra: Option<&'a IntraClient>,
    pub nav: Option<&'a mut NavigationClient>,
    pub windows: Option<&'a WindowTable>,
    /// Les routes de CETTE fenetre (voir `AzureWindow::routes`), pour
    /// retrouver une route par son nom (voir `goto_route`).
    pub routes: Option<&'a RouteTable>,
    /// Le stockage de l'app (voir `AzureWindow::stockage`).
    pub stockage: Option<&'a Stockage>,
    /// L'app qui a cree la fenetre de CE clic (voir `AzureWindow::spec`),
    /// `None` si cette fenetre n'a pas de spec - elle ne peut alors ouvrir
    /// aucune fenetre (voir `open_window`).
    pub app_id: Option<u32>,
    /// `#id` rsH du bouton qui a declenche CE clic (`<button#tab-rsh>` ->
    /// `Some("tab-rsh")`), `None` si le clic n'a touche aucun bouton
    /// identifie (une textarea, un bouton sans id...).
    pub clicked: Option<&'a str>,
    /// Les valeurs des champs de la page au moment du clic (voir `value`).
    pub values: Option<&'a std::collections::BTreeMap<String, crate::ui::services::form::FieldValue>>,
    /// Element vers lequel defiler apres ce rappel (voir `scroll_to`).
    pub scroll_request: Option<String>,
    /// Ce que le rappel demande a la fenetre en plus (voir `copy`, `flash`).
    pub effects: Effects,
    /// Demande un jeton d'activation au compositeur (voir
    /// `activation_token`) ; seulement pendant un clic.
    pub activation: Option<&'a mut dyn FnMut() -> Option<String>>,
}

/// Demandes d'un rappel, appliquees par la fenetre juste apres lui.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Effects {
    /// Texte a mettre dans le presse-papiers du systeme (voir `copy`).
    pub copy: Option<String>,
    /// `(#id du bouton, texte, duree)` (voir `flash`).
    pub flashes: Vec<(String, String, std::time::Duration)>,
    /// Le prochain ecran garde le defilement de l'actuel (voir `refresh`).
    pub keep_scroll: bool,
}

impl<'a> WindowContext<'a> {
    /// Un jeton pour mettre au premier plan la fenetre d'une AUTRE app,
    /// lie a ce clic (protocole Wayland `xdg-activation`). A passer a
    /// `AzureApp::open` : l'app ouverte passe devant au lieu de rester
    /// derriere. `None` hors d'un clic, ou si le compositeur ne le permet pas.
    pub fn activation_token(&mut self) -> Option<String> {
        (self.activation.as_mut()?)()
    }

    /// Valeur d'un champ de la page, en texte : `ctx.value("email")`
    /// (`<input#email>`), `ctx.value("taille")` (le radio coche du groupe
    /// `name="taille"`), `ctx.value("pays")` (`<select#pays>`), `"true"`
    /// pour une case cochee, `"42"` pour un curseur.
    pub fn value(&self, id: &str) -> Option<String> {
        self.values?.get(id).map(|v| v.as_text())
    }

    /// Met `text` dans le presse-papiers du systeme, comme un Ctrl+C : il
    /// se colle ensuite dans n'importe quelle app. Le compositeur ne
    /// l'accepte qu'apres une action de l'utilisateur, ce qu'est un clic.
    pub fn copy(&mut self, text: &str) {
        self.effects.copy = Some(text.to_string());
    }

    /// Affiche `text` sur le bouton `#id` pendant `duration`, puis lui rend
    /// son texte : `ctx.flash("copier", "Copié", Duration::from_millis(1500))`.
    /// Sans effet si la page change entre-temps.
    pub fn flash(&mut self, id: &str, text: &str, duration: std::time::Duration) {
        self.effects.flashes.push((id.to_string(), text.to_string(), duration));
    }

    /// Fait defiler la page jusqu'a l'element `#id` (comme un lien d'ancre
    /// `#ancre-<id>`), apres ce rappel.
    pub fn scroll_to(&mut self, id: &str) {
        self.scroll_request = Some(id.to_string());
    }

    /// Case, interrupteur ou radio coche.
    pub fn checked(&self, id: &str) -> bool {
        matches!(self.values.and_then(|v| v.get(id)), Some(crate::ui::services::form::FieldValue::Bool(true)))
    }

    /// Valeur d'un curseur, d'une note, ou d'un champ `<number>`.
    pub fn number(&self, id: &str) -> Option<f64> {
        match self.values?.get(id)? {
            crate::ui::services::form::FieldValue::Number(n) => Some(*n),
            other => other.as_text().trim().replace(',', ".").parse().ok(),
        }
    }

    /// Le stockage de l'app, `None` si la fenetre n'en a pas
    /// (`AzureWindow::stockage`) :
    /// `if let Some(store) = ctx.stockage() { store.update("clics", 0, |n: i64| n + 1).ok(); }`
    pub fn stockage(&self) -> Option<&'a Stockage> {
        self.stockage
    }

    /// Navigation INTRA-app : envoie a SOI-MEME (voir
    /// `intra_navigation_manager::navigate`, `target_view_id == intra.view_id`)
    /// une demande de navigation vers `path` - deposee dans la boite de
    /// cette vue, sans le moindre appel a `azure-rooter` par socket. Prend
    /// effet au prochain sondage (voir `AzureWindow::run`, le bras
    /// `on_tick`), pas immediatement : `false` seulement si cette fenetre
    /// n'a pas ete branchee a un `IntraRouter` (voir `AzureWindow::intra`).
    pub fn goto(&mut self, path: &str, payload: &str) -> bool {
        let Some(intra) = self.intra else { return false };
        intra_navigation_manager::navigate(intra, intra.view_id, path, payload);
        true
    }

    /// Comme `goto`, pour redessiner la page ouverte avec de nouvelles
    /// donnees : les zones qui defilent restent ou elles etaient (au lieu de
    /// remonter en haut). `ctx.refresh("/tests", "")`.
    pub fn refresh(&mut self, path: &str, payload: &str) -> bool {
        self.effects.keep_scroll = true;
        self.goto(path, payload)
    }

    /// Comme `goto`, mais vers une route NOMMEE de cette fenetre (voir
    /// `Router::name`), parametres remplis :
    /// `ctx.goto_route("user.show", &[("id", "42")], "")` va vers
    /// `/user/42`. Erreur si la fenetre n'a pas de routes, si le nom est
    /// inconnu, s'il manque un parametre obligatoire ou si aucun
    /// `IntraRouter` n'est branche (voir `AzureWindow::intra`).
    pub fn goto_route(&mut self, name: &str, params: &[(&str, &str)], payload: &str) -> Result<(), String> {
        let routes = self.routes.ok_or_else(|| "Aucune RouteTable branchee (voir AzureWindow::routes)".to_string())?;
        let path = routes.url(name, params).ok_or_else(|| format!("Route '{name}' inconnue, ou parametre obligatoire manquant"))?;
        if self.goto(&path, payload) {
            Ok(())
        } else {
            Err("Aucun IntraRouter branche (voir AzureWindow::intra)".to_string())
        }
    }

    /// Navigation INTRA-app vers une AUTRE vue du meme process (branchee au
    /// meme `IntraRouter`) : `path` est un ecran de CETTE vue. Deposee
    /// dans sa boite, prise au prochain tic de la vue visee. `false` si
    /// aucun `IntraRouter` n'est branche.
    pub fn goto_view(&mut self, view_id: u32, path: &str, payload: &str) -> bool {
        let Some(intra) = self.intra else { return false };
        intra_navigation_manager::navigate(intra, view_id, path, payload);
        true
    }

    /// Comme `goto_route`, mais vers la route nommee `name` d'une AUTRE vue
    /// du meme process. Le nom est resolu PAR la vue visee (elle seule
    /// connait ses routes) : un nom inconnu y tombe dans son `fallback`,
    /// sinon est ignore. Erreur seulement si aucun `IntraRouter` n'est
    /// branche ou si un nom/parametre contient un caractere reserve.
    pub fn goto_view_route(&mut self, view_id: u32, name: &str, params: &[(&str, &str)], payload: &str) -> Result<(), String> {
        let path = named_path(name, params)?;
        if self.goto_view(view_id, &path, payload) {
            Ok(())
        } else {
            Err("Aucun IntraRouter branche (voir AzureWindow::intra)".to_string())
        }
    }

    /// Navigation INTER-app vers la route nommee `name` de l'app
    /// `target_app_id`, via azure-rooter. Comme `goto_view_route`, le nom
    /// est resolu par l'app qui recoit.
    pub fn navigate_to_route(&mut self, target_app_id: u32, name: &str, params: &[(&str, &str)], payload: &str) -> Result<(), String> {
        let path = named_path(name, params)?;
        self.navigate_to(target_app_id, &path, payload)
    }

    /// Navigation INTER-app : envoie une demande de navigation vers l'ecran
    /// `path` de l'app `target_app_id`, via le routeur `azure-rooter` (voir
    /// `navigation::managers::navigation_manager::navigate`). Erreur si
    /// cette fenetre n'a pas ete branchee au routeur (voir
    /// `AzureWindow::navigation`).
    pub fn navigate_to(&mut self, target_app_id: u32, path: &str, payload: &str) -> Result<(), String> {
        let nav = self.nav.as_mut().ok_or_else(|| "Aucun client de navigation connecte (voir AzureWindow::navigation)".to_string())?;
        navigation_manager::navigate(nav, target_app_id, path, payload)
    }

    /// Troisieme "expression routeur" (avec `goto` et `navigate_to`) : ouvre
    /// une NOUVELLE fenetre a partir de la `WindowTable` de cette fenetre
    /// (voir `AzureWindow::windows`). Resout `path`/`payload` en une
    /// `AzureWindow` fraiche (jamais encore lancee, voir
    /// `window_table::WindowTable::resolve`) puis la lance dans SON PROPRE
    /// thread (voir `AzureWindow::run`) - chaque fenetre ouvre sa propre
    /// connexion Wayland independante (voir
    /// `azure_engine::platform::wayland::managers::window_manager::window_create`),
    /// donc ni la fenetre appelante ni la nouvelle ne bloquent l'autre.
    ///
    /// La fenetre resolue doit avoir une spec (voir `AzureWindow::spec`)
    /// de type `WindowKind::Internal`, creee par la meme app que cette
    /// fenetre (`app_id`). Une fenetre externe ou inter-app est refusee :
    /// elle doit partir vers une autre app, et aucun transport ne le fait
    /// encore. En cas d'erreur, aucune fenetre n'est ouverte.
    pub fn open_window(&mut self, path: &str, payload: &str) -> Result<(), String> {
        let windows = self.windows.ok_or_else(|| "Aucune WindowTable branchee (voir AzureWindow::windows)".to_string())?;
        let app_id = self.app_id.ok_or_else(|| "Cette fenetre n'a pas de spec : impossible de savoir quelle app ouvre la fenetre (voir AzureWindow::spec)".to_string())?;
        let window = windows.resolve(path, payload).ok_or_else(|| format!("Aucune fenetre enregistree pour '{path}'"))?;
        let spec = window.window_spec().ok_or_else(|| format!("La fenetre '{path}' n'a pas de spec (voir AzureWindow::spec)"))?;
        if spec.kind() != WindowKind::Internal {
            return Err(format!("La fenetre '{path}' est {:?} : seule une fenetre interne peut etre ouverte par son app", spec.kind()));
        }
        if spec.owner_app_id() != app_id {
            return Err(format!("La fenetre '{path}' appartient a l'app {}, pas a l'app {app_id}", spec.owner_app_id()));
        }
        std::thread::spawn(move || window.run());
        Ok(())
    }

    /// Envoie une fenetre aux autres apps selon son scope (abonnes de cette
    /// app ou toutes les apps, voir `navigation_manager::send_window`). Une
    /// fenetre `InterApp` s'ouvre AUSSI ici, dans l'app qui l'envoie ; une
    /// fenetre `External` part seulement chez les autres.
    pub fn send_window(&mut self, window: SharedWindow) -> Result<(), String> {
        let nav = self.nav.as_mut().ok_or_else(|| "Aucun client de navigation connecte (voir AzureWindow::navigation)".to_string())?;
        navigation_manager::send_window(nav, &window)?;
        if window.spec.kind() == WindowKind::InterApp {
            let local = window.to_window()?;
            std::thread::spawn(move || local.run());
        }
        Ok(())
    }
}
