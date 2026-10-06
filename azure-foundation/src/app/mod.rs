// Le point d'entree unique d'une app Azure : tout vient du manifeste
// (`app.azure`, voir `azure_manager::models::manifest`), deja branche.
//
// ```text
// let app = azure_app!()?;                           // app.azure trouve tout seul (voir `find`)
// let app = AzureApp::from_manifest("app.azure")?;   // ou un chemin precis
//
// app.send("caisse", "bonjour")?;                     // les apps se designent par leur nom
// app.navigate("caisse", "/commande/42", "")?;
// let mut panier = app.share("panier")?;              // acces : manifeste + tableau de bord
// panier.set("total", 42)?;
// let ecoute = app.listen("commandes@caisse")?.start()?;
// let _prix = app.serve("prix", |demande| Ok(Value::from(3)))?;   // [provide prix]
// let stock = app.call("entrepot", "stock", Value::from(42))?;    // [use stock@entrepot]
// app.stockage()?.set("theme", "sombre")?;
//
// app.window()?                                       // fenetre, pages et routes du manifeste
//     .flux(ecoute, |ctx, event, etat| { /* ... */ })
//     .run();
// ```
use crate::navigation::managers::{intra_navigation_manager, navigation_manager};
use crate::navigation::models::navigation_client::NavigationClient;
use crate::navigation::models::route::Route;
use crate::navigation::models::route_table::RouteTable;
use crate::storage::models::stockage::Stockage;
use crate::window::models::window::AzureWindow;
use azure_core::models::window_model::{WindowSize, WindowSpec};
use azure_manager::managers::manager::{Kind, Level};
use azure_manager::models::manifest::Manifest;
use azure_manager::services::client::ManagerClient;
use azure_rooter::managers::intra_router::IntraRouter;
use azure_service::flux::{Flux, ListenerBuilder, Shared, Value};

pub use azure_service::call::{CallRequest, Server, DEFAULT_TIMEOUT};

/// Le temps laisse a une app reveillee (sa tache de fond) pour servir la
/// methode appelee.
const WAKE_WAIT: Duration = Duration::from_secs(10);
use std::time::Duration;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Mutex;

/// Les sockets des daemons. `AppSockets::default()` : ceux d'Azure, lances
/// a la demande par azure-provider. Un autre jeu (tests) : rien n'est lance.
#[derive(Clone, Debug)]
pub struct AppSockets {
    pub manager: String,
    pub service: String,
    pub stockage: String,
    pub router: String,
    /// Passer par azure-provider (lancer les daemons, confier les taches de
    /// fond du manifeste).
    pub provider: bool,
    /// Enfermer l'app (voir `sandbox_for`). `false` seulement pour des tests
    /// qui font tourner plusieurs apps dans un meme processus.
    pub sandbox: bool,
}

impl Default for AppSockets {
    fn default() -> AppSockets {
        // Un essai (voir `crate::essai`) : les daemons de son Azure jetable
        // tournent deja (`AZURE_RUNTIME_DIR`), rien a lancer ni a enfermer.
        let essai = std::env::var_os("AZURE_ESSAI").is_some();
        AppSockets {
            manager: azure_manager::SOCKET_PATH.to_string(),
            service: azure_service::SOCKET_PATH.to_string(),
            stockage: azure_stockage::SOCKET_PATH.to_string(),
            router: azure_rooter::SOCKET_PATH.to_string(),
            provider: !essai,
            sandbox: !essai,
        }
    }
}

pub struct AzureApp {
    manifest: Manifest,
    id: u32,
    sockets: AppSockets,
    // Connexion gardee ouverte : tant qu'elle l'est, le tableau de bord
    // voit l'app active.
    manager: Mutex<ManagerClient>,
    stockage: Mutex<Option<Stockage>>,
    flux: Mutex<Option<Flux>>,
    nav: Mutex<Option<NavigationClient>>,
}

/// Une ligne au journal de l'app (sa sortie d'erreur, voir `azure run`) pour
/// un appel entre apps : `14:02:11 appel vers docs : chercher -> ok (12 ms)`.
fn log_call(what: &str, start: std::time::Instant, result: &Result<Value, String>) {
    let ms = start.elapsed().as_millis();
    let heure = azure_manager::managers::manager::local_time(azure_manager::managers::manager::now());
    match result {
        Ok(_) => eprintln!("{heure} {what} -> ok ({ms} ms)"),
        Err(e) => eprintln!("{heure} {what} -> erreur ({ms} ms) : {e}"),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    // Un thread qui a plante en le tenant ne bloque pas les autres.
    Ok(mutex.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Ou est le manifeste de l'app : `$AZURE_APP_MANIFEST` s'il est donne,
/// sinon `app.azure` a cote de l'executable (app installee par `azure
/// install`), sinon dans `dev_dir` (le dossier du crate, en developpement).
pub fn locate_manifest(env: Option<&str>, exe_dir: Option<&Path>, dev_dir: &Path) -> std::path::PathBuf {
    if let Some(path) = env.filter(|p| !p.is_empty()) {
        return path.into();
    }
    match exe_dir.map(|dir| dir.join("app.azure")) {
        Some(installed) if installed.exists() => installed,
        _ => dev_dir.join("app.azure"),
    }
}

/// `azure_app!()` : l'`AzureApp` de ce crate, que l'app soit installee ou
/// lancee par `cargo run` (voir `AzureApp::find`).
#[macro_export]
macro_rules! azure_app {
    () => {
        $crate::app::AzureApp::find(env!("CARGO_MANIFEST_DIR"))
    };
}

impl AzureApp {
    /// Trouve le manifeste (voir `locate_manifest`) puis `from_manifest`.
    pub fn find(dev_dir: &str) -> Result<AzureApp, String> {
        let env = std::env::var("AZURE_APP_MANIFEST").ok();
        let exe_dir = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf));
        AzureApp::from_manifest(locate_manifest(env.as_deref(), exe_dir.as_deref(), Path::new(dev_dir)))
    }

    /// Lit le manifeste, presente l'app a azure-manager (qui lui donne son
    /// id) et confie ses taches de fond a azure-provider.
    pub fn from_manifest(path: impl AsRef<Path>) -> Result<AzureApp, String> {
        AzureApp::from_manifest_at(path, AppSockets::default())
    }

    pub fn from_manifest_at(path: impl AsRef<Path>, sockets: AppSockets) -> Result<AzureApp, String> {
        let manifest = Manifest::load(path.as_ref())?;
        // Ses fonctionnalites sont publiees sous ce nom (voir `perf`).
        crate::perf::nommer(&manifest.name);
        if sockets.sandbox {
            reexec_isolated(&manifest, sockets.provider);
        }
        // `kill` ferme l'app proprement, isolee ou non (voir la fenetre).
        azure_core::security::termination::install();
        let mut manager = if sockets.provider { ManagerClient::connect()? } else { ManagerClient::connect_at(&sockets.manager)? };
        let id = manager.register(&manifest.summary())?;
        if sockets.provider {
            let on_demand = manifest.on_demand_services();
            for mut service in manifest.provider_services() {
                let name = service.name.clone();
                // En developpement, le binaire de l'app est celui qui tourne.
                if manifest.exec.as_deref() == Some(service.command.as_str())
                    && let Ok(exe) = std::env::current_exe()
                {
                    service.command = exe.to_string_lossy().into_owned();
                }
                if on_demand.contains(&name) {
                    // Lancee au premier appel d'une autre app, pas maintenant.
                    // Installee : `azure install` l'a deja declaree.
                    if is_installed(&manifest) {
                        continue;
                    }
                    if let Err(e) = service.declare() {
                        eprintln!("AzureApp '{}' : tache de fond '{name}' non declaree : {e}", manifest.name);
                    }
                } else {
                    service.register().map_err(|e| format!("tache de fond '{name}' : {e}"))?;
                }
            }
        }
        install_panic_report(&manifest.name, &sockets.manager);
        let app = AzureApp { manifest, id, sockets, manager: Mutex::new(manager), stockage: Mutex::new(None), flux: Mutex::new(None), nav: Mutex::new(None) };
        if app.sockets.provider {
            // Les daemons tournent AVANT que l'app soit enfermee : enfermee,
            // elle ne pourrait plus les lancer (voir `Provider`).
            for service in ["rooter", "stockage", "service"] {
                if let Err(e) = azure_provider::Provider::ensure(service) {
                    app.warn(&format!("{service} indisponible : {e}"));
                }
            }
        }
        // Les dossiers autorises : les « lieux » de la boite « Ouvrir ».
        let permissions = &app.manifest.permissions;
        crate::selecteur::definir_lieux(permissions.write.iter().chain(&permissions.read).cloned().collect());
        if app.sockets.sandbox {
            app.enclose()?;
        }
        Ok(app)
    }

    /// Enferme l'app (voir `sandbox_for`) et le dit au tableau de bord.
    fn enclose(&self) -> Result<(), String> {
        use azure_core::security::sandbox::Enforcement;
        match sandbox_for(&self.manifest) {
            None => self.warn("non enfermee : bac_a_sable = false (developpement seulement)"),
            Some(mut sandbox) => {
                // Daemons sur des sockets hors de /tmp (tests, installations
                // a part) : l'app doit pouvoir les joindre.
                for socket in [&self.sockets.manager, &self.sockets.service, &self.sockets.stockage, &self.sockets.router] {
                    if let Some(dir) = Path::new(socket).parent() {
                        sandbox = sandbox.socket(dir);
                    }
                }
                match sandbox.apply()? {
                Enforcement::Full => self.info(&format!("enfermee : {}", self.manifest.permissions.describe())),
                Enforcement::FilesOnly => self.warn("enfermee pour les fichiers seulement (noyau sans Landlock reseau)"),
                Enforcement::Unsupported => self.warn("non enfermee : Landlock indisponible sur ce systeme"),
                }
            }
        }
        Ok(())
    }

    /// Signale une information au tableau de bord (page Evenements).
    pub fn info(&self, message: &str) {
        self.report(Level::Info, message);
    }

    /// Signale un probleme sans gravite.
    pub fn warn(&self, message: &str) {
        self.report(Level::Warning, message);
    }

    /// Signale une erreur : elle apparait sur la page de l'app et dans les
    /// Evenements du tableau de bord.
    pub fn error(&self, message: &str) {
        self.report(Level::Error, message);
    }

    fn report(&self, level: Level, message: &str) {
        if let Ok(mut manager) = self.manager.lock()
            && let Err(e) = manager.report(&self.manifest.name, level, message) {
                eprintln!("AzureApp '{}' : evenement non transmis ({e}) : {message}", self.manifest.name);
            }
    }

    /// Id attribue par azure-manager (le meme a chaque lancement).
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Id de l'app `name`.
    pub fn resolve(&self, name: &str) -> Result<u32, String> {
        lock(&self.manager)?.resolve(name)
    }

    /// Le stockage prive de l'app (deplace a `[storage] location` si le
    /// manifeste en donne un).
    pub fn stockage(&self) -> Result<Stockage, String> {
        let mut slot = lock(&self.stockage)?;
        if let Some(store) = slot.as_ref() {
            return Ok(store.clone());
        }
        if !self.manifest.permissions.storage {
            return Err(format!("'{}' n'a pas la permission stockage ([permissions] stockage = false)", self.manifest.name));
        }
        let store = if self.sockets.provider { Stockage::connect(self.id)? } else { Stockage::connect_at(&self.sockets.stockage, self.id)? };
        if let Some(place) = &self.manifest.storage_location {
            let place = place.to_string_lossy().into_owned();
            if store.location()?.as_deref() != Some(place.as_str()) {
                store.move_to(&place)?;
            }
        }
        *slot = Some(store.clone());
        Ok(store)
    }

    /// La connexion aux flux (azure-service).
    pub fn flux(&self) -> Result<Flux, String> {
        let mut slot = lock(&self.flux)?;
        if let Some(flux) = slot.as_ref() {
            return Ok(flux.clone());
        }
        let flux = if self.sockets.provider { Flux::connect(self.id)? } else { Flux::connect_at(&self.sockets.service, self.id)? };
        *slot = Some(flux.clone());
        Ok(flux)
    }

    /// Partage le flux `name`, declare par `[share name]` dans le manifeste.
    /// Qui peut l'ecouter : le manifeste, modifie par le tableau de bord.
    pub fn share(&self, name: &str) -> Result<Shared, String> {
        if self.manifest.share(name).is_none() {
            return Err(format!("Declarez [share {name}] dans le manifeste de '{}'", self.manifest.name));
        }
        let access = lock(&self.manager)?.access(Kind::Flux, name)?;
        let persist = self.manifest.share(name).is_some_and(|s| s.persist);
        let builder = self.flux()?.share(name).persist(persist);
        match access {
            azure_service::flux::Access::Public => builder.public(),
            azure_service::flux::Access::Apps(apps) => builder.to(&apps),
        }
        .open()
    }

    /// Ecoute `"<flux>@<app>"`, declare par `[listen ...]` dans le
    /// manifeste (ses `paths` et `events` sont deja appliques ; d'autres
    /// `.path(...)`, `.on(...)` peuvent suivre avant `.start()`).
    pub fn listen(&self, spec: &str) -> Result<ListenerBuilder, String> {
        let (flux, owner) = spec.split_once('@').ok_or_else(|| format!("'{spec}' : <flux>@<app> attendu"))?;
        let declared = self.manifest.listen(flux, owner).ok_or_else(|| format!("Declarez [listen {spec}] dans le manifeste de '{}'", self.manifest.name))?.clone();
        let owner_id = self.resolve(owner)?;
        let mut builder = self.flux()?.listen(owner_id, flux);
        for path in &declared.paths {
            builder = builder.path(path);
        }
        for event in &declared.events {
            builder = builder.event(event);
        }
        Ok(builder)
    }

    /// Sert toutes les methodes d'un service de la librairie
    /// (`azure_libraire::service`), sous leur nom complet
    /// `<service>-<methode>` ; chacune doit etre declaree `[provide ...]`
    /// dans le manifeste (`azure_libraire::service::manifeste` ecrit ces
    /// sections). Servies tant que les `Server` retournes existent.
    pub fn servir(&self, service: &'static azure_libraire::service::Service) -> Result<Vec<Server>, String> {
        service
            .methodes
            .iter()
            .map(|m| {
                let appeler = m.appeler;
                self.serve(&service.nom_complet(m), move |req| appeler(&azure_libraire::service::Valeur::from(&req.args)).map(Value::from))
            })
            .collect()
    }

    /// Sert la methode `method`, declaree par `[provide method]` dans le
    /// manifeste. Qui peut l'appeler : le manifeste, modifie par le tableau
    /// de bord. `handler` recoit chaque demande (plusieurs a la fois, dans
    /// des threads a part) ; servie tant que le `Server` retourne existe.
    pub fn serve(&self, method: &str, handler: impl Fn(&CallRequest) -> Result<Value, String> + Send + Sync + 'static) -> Result<Server, String> {
        if self.manifest.provide(method).is_none() {
            return Err(format!("Declarez [provide {method}] dans le manifeste de '{}'", self.manifest.name));
        }
        let access = lock(&self.manager)?.access(Kind::Call, method)?;
        // Chaque appel recu, au journal de l'app : qui, quoi, combien de temps.
        let (socket, method_name) = (self.sockets.manager.clone(), method.to_string());
        let names: std::sync::Mutex<std::collections::HashMap<u32, String>> = std::sync::Mutex::new(std::collections::HashMap::new());
        self.flux()?.serve(method, access, move |request| {
            let start = std::time::Instant::now();
            let result = handler(request);
            let caller = {
                let mut names = names.lock().unwrap_or_else(|e| e.into_inner());
                names
                    .entry(request.caller)
                    .or_insert_with(|| azure_core::security::registry::identify(&socket, request.caller).ok().flatten().map(|i| i.name).unwrap_or_else(|| format!("app {}", request.caller)))
                    .clone()
            };
            log_call(&format!("appel recu de {caller} : {method_name}"), start, &result);
            result
        })
    }

    /// Appelle `method` de l'app `app` (declare par `[use method@app]`) et
    /// attend la reponse, au plus `DEFAULT_TIMEOUT` (5 s). L'app appelee
    /// n'a pas besoin d'etre ouverte si une de ses taches de fond sert la
    /// methode (`[provide method] service = ...`) : Azure la lance.
    pub fn call(&self, app: &str, method: &str, args: impl Into<Value>) -> Result<Value, String> {
        self.call_timeout(app, method, args, DEFAULT_TIMEOUT)
    }

    pub fn call_timeout(&self, app: &str, method: &str, args: impl Into<Value>, timeout: Duration) -> Result<Value, String> {
        let start = std::time::Instant::now();
        let result = self.call_once(app, method, args.into(), timeout);
        log_call(&format!("appel vers {app} : {method}"), start, &result);
        result
    }

    fn call_once(&self, app: &str, method: &str, args: Value, timeout: Duration) -> Result<Value, String> {
        if !self.manifest.uses(method, app) {
            return Err(format!("Declarez [use {method}@{app}] dans le manifeste de '{}'", self.manifest.name));
        }
        let owner = self.resolve(app)?;
        match self.flux()?.call(owner, method, args.clone(), timeout) {
            // L'app appelee est fermee : si une de ses taches de fond sert
            // la methode (`[provide] service = ...`), Azure la lance.
            Err(e) if e.contains("n'est pas disponible") => {
                if let Err(reveil) = lock(&self.manager)?.wake(app, method) {
                    return Err(if reveil.contains("aucun service") { e } else { format!("{e} ; reveil impossible : {reveil}") });
                }
                // Lancee : elle sert la methode des qu'elle a demarre.
                self.call_wait_served(owner, method, args, WAKE_WAIT.max(timeout))
            }
            other => other,
        }
    }

    fn call_wait_served(&self, owner: u32, method: &str, args: Value, timeout: Duration) -> Result<Value, String> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match self.flux()?.call(owner, method, args.clone(), left.max(Duration::from_millis(100))) {
                Err(e) if e.contains("n'est pas disponible") && std::time::Instant::now() + Duration::from_millis(50) < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                other => return other,
            }
        }
    }

    /// Comme `call_timeout`, mais si `method` n'est pas (encore) servie -
    /// l'app appelee demarre, ou redemarre - on reessaie jusqu'a `timeout`
    /// au lieu d'echouer tout de suite. Les autres erreurs (refus, erreur de
    /// l'app appelee) reviennent aussitot.
    pub fn call_wait(&self, app: &str, method: &str, args: impl Into<Value>, timeout: Duration) -> Result<Value, String> {
        let args = args.into();
        let start = std::time::Instant::now();
        let deadline = start + timeout;
        // Une seule ligne au journal, pas une par essai.
        let result = loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match self.call_once(app, method, args.clone(), left.max(Duration::from_millis(100))) {
                Err(e) if e.contains("n'est pas disponible") && std::time::Instant::now() + Duration::from_millis(100) < deadline => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                other => break other,
            }
        };
        log_call(&format!("appel vers {app} : {method}"), start, &result);
        result
    }

    // Connexion au routeur pour ENVOYER seulement (sans s'y enregistrer : la
    // fenetre garde la connexion qui recoit, voir `window`).
    fn with_nav<T>(&self, action: impl FnOnce(&mut NavigationClient) -> Result<T, String>) -> Result<T, String> {
        let mut slot = lock(&self.nav)?;
        if slot.is_none() {
            if self.sockets.provider {
                let _ = azure_provider::Provider::ensure("rooter");
            }
            let connection = UnixStream::connect(&self.sockets.router).map_err(|e| format!("azure-rooter injoignable ({}) : {e}", self.sockets.router))?;
            *slot = Some(NavigationClient { router_id: self.id, connection });
        }
        action(slot.as_mut().expect("connecte juste au-dessus"))
    }

    /// Envoie un message a l'app `to`.
    pub fn send(&self, to: &str, message: &str) -> Result<(), String> {
        let target = self.resolve(to)?;
        self.with_nav(|nav| navigation_manager::send(nav, target, message))
    }

    /// Ouvre la page `path` dans l'app `to`.
    pub fn navigate(&self, to: &str, path: &str, payload: &str) -> Result<(), String> {
        let target = self.resolve(to)?;
        self.with_nav(|nav| navigation_manager::navigate(nav, target, path, payload))
    }

    /// Ouvre la page `path` de l'app `to`, en la lancant si elle ne tourne
    /// pas (`[open to]` dans le manifeste). La page part d'abord par le
    /// routeur : une app fermee la trouve dans sa boite aux lettres des
    /// qu'elle s'enregistre. `Ok(true)` : l'app a ete lancee.
    ///
    /// `activation` : le jeton du clic (`ctx.activation_token()`) ; avec
    /// lui, la fenetre ouverte passe au premier plan, qu'elle tourne deja ou
    /// qu'elle vienne d'etre lancee.
    pub fn open(&self, to: &str, path: &str, payload: &str, activation: Option<&str>) -> Result<bool, String> {
        let target = self.resolve(to)?;
        let token = activation.unwrap_or("");
        self.with_nav(|nav| navigation_manager::navigate_activated(nav, target, path, payload, token))?;
        lock(&self.manager)?.open(to, token)
    }

    /// Les pages `[route ...]` du manifeste. Se complete comme toute
    /// `RouteTable` (`.view_with(...)` pour une page avec des donnees) avant
    /// `window_with`.
    pub fn routes(&self) -> RouteTable {
        let default_style = self.manifest.window.as_ref().and_then(|w| w.style.clone());
        let mut table = RouteTable::new();
        for route in &self.manifest.routes {
            let style = route.style.clone().or_else(|| default_style.clone()).unwrap_or_default();
            table = table.view(&route.path, &route.view.to_string_lossy(), &style.to_string_lossy());
            if let Some(name) = &route.name {
                table = table.name(name);
            }
        }
        table
    }

    /// La fenetre principale : `[window]` et les pages du manifeste.
    pub fn window(&self) -> Result<AzureWindow, String> {
        self.window_with(self.routes())
    }

    /// Comme `window`, avec ces routes. Deja branchee : navigation dans
    /// l'app (`ctx.goto`, `ctx.goto_route`), pages recues des autres apps,
    /// stockage (`ctx.stockage()`).
    pub fn window_with(&self, routes: RouteTable) -> Result<AzureWindow, String> {
        let decl = self.manifest.window.clone().unwrap_or_default();
        let (width, height) = if decl.width == 0 { (800, 600) } else { (decl.width, decl.height) };
        let size = WindowSize::new(width, height)?;
        let mut window = AzureWindow::new(decl.title.as_deref().unwrap_or(&self.manifest.title)).app_id(&format!("azure-{}", self.manifest.name)).spec(WindowSpec::internal(self.id, size));
        if let Some(start) = &decl.start {
            let nodes = routes.resolve(&Route::new(start, "")).ok_or_else(|| format!("[window] start = {start} : aucune page a ce chemin"))?;
            window = window.ui(nodes);
        }
        let router = IntraRouter::new();
        window = window.intra(intra_navigation_manager::connect(&router, self.id)).routes(routes);
        // Inter-app et stockage : la fenetre marche sans (daemon absent).
        let nav = if self.sockets.provider { navigation_manager::connect(self.id) } else { navigation_manager::connect_at(&self.sockets.router, self.id) };
        match nav {
            Ok(nav) => window = window.navigation(nav),
            Err(e) => eprintln!("AzureApp '{}' : pages des autres apps indisponibles : {e}", self.manifest.name),
        }
        match self.stockage() {
            Ok(store) => window = window.stockage(store),
            Err(e) => eprintln!("AzureApp '{}' : stockage indisponible : {e}", self.manifest.name),
        }
        Ok(window)
    }
}

/// Un plantage (panic) de l'app est signale au tableau de bord avant de
/// continuer comme d'habitude (message sur la sortie d'erreur).
fn install_panic_report(name: &str, socket: &str) {
    let (name, socket) = (name.to_string(), socket.to_string());
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info.location().map(|l| format!(" ({}:{})", l.file(), l.line())).unwrap_or_default();
        let message = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_else(|| "plantage".to_string());
        if let Ok(mut client) = ManagerClient::connect_at(&socket) {
            let _ = client.report(&name, Level::Error, &format!("plantage : {message}{place}"));
        }
        previous(info);
    }));
}

/// L'app est-elle installee (`azure install`) ?
pub fn is_installed(manifest: &Manifest) -> bool {
    manifest.dir.starts_with(azure_provider::install_root().join("apps"))
}

/// Le bac a sable d'une app : dossiers systeme, son propre dossier (lecture),
/// la police d'Azure, et ce que declare `[permissions]`. `None` si
/// `bac_a_sable = false` - accepte seulement pour une app non installee.
/// Noyau ou Landlock ne couvre pas tout (ABI < 9 : bus de session, X11 ;
/// < 6 : signaux) et app installee lancee sans `azure run` : elle se relance
/// elle-meme dans des espaces de noms (voir
/// `azure_core::security::isolation`), et ce processus-ci attend sa fin.
fn reexec_isolated(manifest: &Manifest, provider: bool) {
    use std::os::unix::process::CommandExt;
    // SAFETY : getpid n'echoue jamais. Processus 1 : deja isole.
    let isolated = unsafe { libc::getpid() } == 1;
    if isolated || !is_installed(manifest) || azure_core::security::sandbox::abi_version() >= 9 || std::env::var_os("AZURE_ISOLATION_TENTEE").is_some() {
        return;
    }
    // Les daemons d'abord, HORS des espaces de noms de l'app.
    if provider {
        for service in ["manager", "rooter", "stockage", "service"] {
            let _ = azure_provider::Provider::ensure(service);
        }
    }
    let Ok(exe) = std::env::current_exe() else { return };
    let isolation = azure_core::security::isolation::Isolation::for_app(manifest.permissions.network).voir_processus(manifest.permissions.processes);
    let mut command = std::process::Command::new(exe);
    command.args(std::env::args_os().skip(1)).env("AZURE_ISOLATION_TENTEE", "1");
    // SAFETY : `enter` ne fait que des appels systeme.
    unsafe { command.pre_exec(move || isolation.enter()) };
    let waited = command.spawn().and_then(|mut child| {
        // `kill` de ce processus arrete l'app isolee.
        azure_core::security::termination::forward_to(child.id() as libc::pid_t);
        child.wait()
    });
    match waited {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(e) => eprintln!("isolation par espaces de noms impossible ({e}) : Landlock seul"),
    }
}

pub fn sandbox_for(manifest: &Manifest) -> Option<azure_core::security::sandbox::Sandbox> {
    let permissions = &manifest.permissions;
    if !permissions.sandbox && !is_installed(manifest) {
        return None;
    }
    let mut sandbox = azure_core::security::sandbox::Sandbox::system().read(&manifest.dir).network(permissions.network);
    if let Some(fonts) = Path::new(crate::ui::services::draw_ui::FONT_PATH).parent() {
        sandbox = sandbox.read(fonts);
    }
    for path in &permissions.read {
        sandbox = sandbox.read(path);
    }
    for path in &permissions.write {
        sandbox = sandbox.write(path);
    }
    Some(sandbox)
}
