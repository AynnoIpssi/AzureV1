// Le manifeste d'une app (`app.azure`) : tout ce qu'elle est et tout ce
// qu'elle echange, declare en un seul endroit.
//
// ```text
// [app]
// name = boutique                 # nom unique (a-z, 0-9, -), sert a la designer
// title = Ma boutique
// version = 1.2
// exec = boutique                 # binaire de l'app (pour `azure install`)
// icon = icon.png                 # icone du lanceur
// files = ui, images              # dossiers / fichiers a installer avec l'app
//
// [window]                        # fenetre principale
// width = 900
// height = 600
// start = /accueil                # premiere page
// style = ui/app.rsc              # rsC par defaut des pages
//
// [route /accueil]                # une page rsH
// view = ui/accueil.rsh
// name = accueil
//
// [share panier]                  # flux partage
// to = caisse, stats              # ou : public = true
// persist = true                  # l'etat survit aux redemarrages
//
// [listen commandes@caisse]       # flux ecoute : <flux>@<app>
// paths = total, items.*.prix
// events = paye
//
// [provide prix]                  # methode que les autres apps appellent
// to = caisse                     # ou : public = true
// description = Prix d'un produit
//
// [use stock@entrepot]            # methode d'une autre app que celle-ci appelle
//
// [open docs]                     # app que celle-ci peut lancer (et y naviguer)
//
// [service sync]                  # tache de fond, surveillee par azure-provider
// command = boutique_sync
// health = /tmp/boutique-sync.sock
//
// [storage]
// location = /mnt/disque/boutique # emplacement des donnees (optionnel)
//
// [permissions]                   # ce que l'app peut toucher (voir `Permissions`)
// lecture = ~/Documents           # dossiers lus en plus du sien
// ecriture = ~/Documents/Boutique # dossiers ou elle ecrit
// reseau = true                   # connexions TCP (coupees sinon)
// stockage = false                # pas de stockage Azure
// ```
//
// Les chemins de fichiers sont relatifs au dossier du manifeste.
use azure_core::models::wire::{Reader, Writer};
use azure_provider::models::config::{parse_bool, split_args};
use azure_provider::{Restart, ServiceSpec};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WindowDecl {
    pub title: Option<String>,
    pub width: u32,
    pub height: u32,
    pub start: Option<String>,
    pub style: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteDecl {
    pub path: String,
    pub view: PathBuf,
    pub style: Option<PathBuf>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ShareDecl {
    pub flux: String,
    /// Noms des apps autorisees.
    pub to: Vec<String>,
    pub public: bool,
    /// Etat garde sur disque par azure-service (`persist = true`).
    pub persist: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ListenDecl {
    pub flux: String,
    /// App qui partage le flux.
    pub owner: String,
    pub paths: Vec<String>,
    pub events: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProvideDecl {
    pub method: String,
    /// Noms des apps autorisees a appeler.
    pub to: Vec<String>,
    pub public: bool,
    pub description: String,
    /// La tache de fond (`[service <nom>]` du meme manifeste) qui sert la
    /// methode quand l'app n'est pas ouverte : lancee par azure-manager au
    /// premier appel. Vide : servie seulement par l'app ouverte.
    pub service: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UseDecl {
    pub method: String,
    /// App qui sert la methode.
    pub owner: String,
}

/// Ce qu'une app enfermee peut toucher (voir
/// `azure_core::security::sandbox`). Par defaut : son dossier, les dossiers
/// systeme, le stockage Azure ; pas le reste du dossier personnel, pas le
/// reseau.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Permissions {
    pub read: Vec<PathBuf>,
    pub write: Vec<PathBuf>,
    pub network: bool,
    pub storage: bool,
    /// `false` : pas enfermee (seulement en developpement, jamais pour une
    /// app installee).
    pub sandbox: bool,
}

impl Default for Permissions {
    fn default() -> Permissions {
        Permissions { read: Vec::new(), write: Vec::new(), network: false, storage: true, sandbox: true }
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match std::env::var_os(var) {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => home().join(fallback),
    }
}

/// Dossiers qu'aucune permission ne peut ouvrir : donnees, cles, journaux,
/// configuration d'Azure, services systemd de l'utilisateur.
pub fn protected_dirs() -> Vec<PathBuf> {
    vec![
        azure_provider::install_root(),
        xdg("XDG_CONFIG_HOME", ".config").join("azure"),
        xdg("XDG_CONFIG_HOME", ".config").join("systemd"),
        xdg("XDG_STATE_HOME", ".local/state").join("azure"),
    ]
}

/// `~/Documents` -> chemin absolu.
fn expand(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home().join(rest),
        None if path == "~" => home(),
        None => PathBuf::from(path),
    }
}

impl Permissions {
    /// Refuse une permission qui ouvrirait un dossier protege (ou un de ses
    /// parents : `~` donnerait acces aux cles d'Azure).
    pub fn check(&self) -> Result<(), String> {
        for path in self.read.iter().chain(&self.write) {
            if !path.is_absolute() {
                return Err(format!("permission '{}' : chemin absolu ou ~/... attendu", path.display()));
            }
            for protected in protected_dirs() {
                if protected.starts_with(path) || path.starts_with(&protected) {
                    return Err(format!("permission '{}' refusee : elle donnerait acces a {} (donnees d'Azure)", path.display(), protected.display()));
                }
            }
        }
        Ok(())
    }

    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        parts.push(if self.network { "reseau" } else { "sans reseau" }.to_string());
        parts.push(if self.storage { "stockage" } else { "sans stockage" }.to_string());
        if !self.read.is_empty() {
            parts.push(format!("lit {}", self.read.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")));
        }
        if !self.write.is_empty() {
            parts.push(format!("ecrit {}", self.write.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")));
        }
        parts.join(" · ")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub title: String,
    pub version: String,
    pub window: Option<WindowDecl>,
    pub routes: Vec<RouteDecl>,
    pub shares: Vec<ShareDecl>,
    pub listens: Vec<ListenDecl>,
    pub provides: Vec<ProvideDecl>,
    pub uses: Vec<UseDecl>,
    /// Les apps que celle-ci peut lancer (`[open <app>]`, voir `OPEN`).
    pub opens: Vec<String>,
    /// Noms courts (`sync`) ; le provider les connait sous `<app>-<nom>`
    /// (voir `provider_services`).
    pub services: Vec<ServiceSpec>,
    pub storage_location: Option<PathBuf>,
    /// Nom du binaire de l'app (`azure install`).
    pub exec: Option<String>,
    pub icon: Option<PathBuf>,
    /// En plus des pages, styles et icone : ce qu'il faut installer avec
    /// l'app (par defaut `ui` s'il existe).
    pub files: Vec<PathBuf>,
    pub permissions: Permissions,
    /// Dossier du manifeste.
    pub dir: PathBuf,
}

/// Nom d'app : a-z, 0-9 et `-`, 32 caracteres max.
pub fn check_app_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty() && name.len() <= 32 && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !name.starts_with('-');
    if ok { Ok(()) } else { Err(format!("Nom d'app invalide : '{name}' (a-z, 0-9 et -, 32 caracteres max)")) }
}

enum Section {
    App,
    Window,
    Route(usize),
    Share(usize),
    Listen(usize),
    Service(usize),
    Provide(usize),
    Use,
    Open,
    Storage,
    Permissions,
}

fn list(value: &str) -> Vec<String> {
    value.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{} : {e}", path.display()))?;
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        Manifest::parse(&text, &dir).map_err(|e| format!("{} : {e}", path.display()))
    }

    pub fn parse(text: &str, dir: &Path) -> Result<Manifest, String> {
        let mut m = Manifest { name: String::new(), title: String::new(), version: "0".to_string(), window: None, routes: Vec::new(), shares: Vec::new(), listens: Vec::new(), provides: Vec::new(), uses: Vec::new(), opens: Vec::new(), services: Vec::new(), storage_location: None, exec: None, icon: None, files: Vec::new(), permissions: Permissions::default(), dir: dir.to_path_buf() };
        let file = |value: &str| dir.join(value);
        let mut section: Option<Section> = None;

        for (number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            let at = |message: String| format!("ligne {} : {message}", number + 1);
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(header) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
                let (kind, arg) = header.trim().split_once(char::is_whitespace).map(|(k, a)| (k, a.trim())).unwrap_or((header.trim(), ""));
                let need_arg = |what: &str| if arg.is_empty() { Err(at(format!("[{kind} ...] : {what} attendu"))) } else { Ok(()) };
                section = Some(match kind {
                    "app" => Section::App,
                    "window" => {
                        m.window.get_or_insert_with(|| WindowDecl { width: 800, height: 600, ..Default::default() });
                        Section::Window
                    }
                    "route" => {
                        need_arg("chemin")?;
                        if !arg.starts_with('/') {
                            return Err(at(format!("route '{arg}' : un chemin commence par /")));
                        }
                        m.routes.push(RouteDecl { path: arg.to_string(), view: PathBuf::new(), style: None, name: None });
                        Section::Route(m.routes.len() - 1)
                    }
                    "share" => {
                        need_arg("nom du flux")?;
                        azure_service::flux::protocol::check_name(arg).map_err(at)?;
                        m.shares.push(ShareDecl { flux: arg.to_string(), ..Default::default() });
                        Section::Share(m.shares.len() - 1)
                    }
                    "listen" => {
                        need_arg("<flux>@<app>")?;
                        let (flux, owner) = arg.split_once('@').ok_or_else(|| at(format!("listen '{arg}' : <flux>@<app> attendu")))?;
                        azure_service::flux::protocol::check_name(flux).map_err(at)?;
                        check_app_name(owner).map_err(at)?;
                        m.listens.push(ListenDecl { flux: flux.to_string(), owner: owner.to_string(), ..Default::default() });
                        Section::Listen(m.listens.len() - 1)
                    }
                    "service" => {
                        need_arg("nom")?;
                        m.services.push(ServiceSpec::new(arg, ""));
                        Section::Service(m.services.len() - 1)
                    }
                    "provide" => {
                        need_arg("nom de la methode")?;
                        azure_service::flux::protocol::check_name(arg).map_err(at)?;
                        m.provides.push(ProvideDecl { method: arg.to_string(), ..Default::default() });
                        Section::Provide(m.provides.len() - 1)
                    }
                    "use" => {
                        need_arg("<methode>@<app>")?;
                        let (method, owner) = arg.split_once('@').ok_or_else(|| at(format!("use '{arg}' : <methode>@<app> attendu")))?;
                        azure_service::flux::protocol::check_name(method).map_err(at)?;
                        check_app_name(owner).map_err(at)?;
                        m.uses.push(UseDecl { method: method.to_string(), owner: owner.to_string() });
                        Section::Use
                    }
                    "open" => {
                        need_arg("nom d'app")?;
                        check_app_name(arg).map_err(at)?;
                        if !m.opens.iter().any(|o| o == arg) {
                            m.opens.push(arg.to_string());
                        }
                        Section::Open
                    }
                    "storage" => Section::Storage,
                    "permissions" => Section::Permissions,
                    other => return Err(at(format!("section inconnue [{other}] (app, window, route, share, listen, provide, use, open, service, storage, permissions)"))),
                });
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(|| at(format!("'cle = valeur' attendu, trouve '{line}'")))?;
            let (key, value) = (key.trim(), value.trim());
            let unknown = || Err(at(format!("cle inconnue '{key}'")));
            let number = |value: &str| value.parse::<u32>().map_err(|_| at(format!("nombre attendu, trouve '{value}'")));
            match section.as_ref().ok_or_else(|| at("cle hors d'une section".to_string()))? {
                Section::App => match key {
                    "name" => m.name = value.to_string(),
                    "title" => m.title = value.to_string(),
                    "version" => m.version = value.to_string(),
                    "exec" => {
                        if value.is_empty() || value.contains('/') {
                            return Err(at(format!("exec '{value}' : un nom de binaire, sans /")));
                        }
                        m.exec = Some(value.to_string());
                    }
                    "icon" => m.icon = Some(file(value)),
                    "files" => m.files = list(value).iter().map(|f| file(f)).collect(),
                    _ => return unknown(),
                },
                Section::Window => {
                    let window = m.window.as_mut().expect("cree a l'ouverture de la section");
                    match key {
                        "title" => window.title = Some(value.to_string()),
                        "width" => window.width = number(value)?,
                        "height" => window.height = number(value)?,
                        "start" => window.start = Some(value.to_string()),
                        "style" => window.style = Some(file(value)),
                        _ => return unknown(),
                    }
                }
                Section::Route(i) => match key {
                    "view" => m.routes[*i].view = file(value),
                    "style" => m.routes[*i].style = Some(file(value)),
                    "name" => m.routes[*i].name = Some(value.to_string()),
                    _ => return unknown(),
                },
                Section::Share(i) => match key {
                    "to" => {
                        let to = list(value);
                        for app in &to {
                            check_app_name(app).map_err(at)?;
                        }
                        m.shares[*i].to = to;
                    }
                    "public" => m.shares[*i].public = parse_bool(value).map_err(at)?,
                    "persist" => m.shares[*i].persist = parse_bool(value).map_err(at)?,
                    _ => return unknown(),
                },
                Section::Listen(i) => match key {
                    "paths" => m.listens[*i].paths = list(value),
                    "events" => m.listens[*i].events = list(value),
                    _ => return unknown(),
                },
                Section::Service(i) => {
                    let spec = &mut m.services[*i];
                    match key {
                        "command" => spec.command = value.to_string(),
                        "args" => spec.args = split_args(value).map_err(at)?,
                        "health" => spec.health = (!value.is_empty()).then(|| value.to_string()),
                        "restart" => spec.restart = Restart::from_name(value).ok_or_else(|| at(format!("restart inconnu '{value}' (always, on-failure, never)")))?,
                        "autostart" => spec.autostart = parse_bool(value).map_err(at)?,
                        _ => return unknown(),
                    }
                }
                Section::Provide(i) => match key {
                    "to" => {
                        let to = list(value);
                        for app in &to {
                            check_app_name(app).map_err(at)?;
                        }
                        m.provides[*i].to = to;
                    }
                    "public" => m.provides[*i].public = parse_bool(value).map_err(at)?,
                    "description" => m.provides[*i].description = value.to_string(),
                    "service" => m.provides[*i].service = value.to_string(),
                    _ => return unknown(),
                },
                Section::Use | Section::Open => return unknown(),
                Section::Permissions => match key {
                    "lecture" | "read" => m.permissions.read = list(value).iter().map(|p| expand(p)).collect(),
                    "ecriture" | "write" => m.permissions.write = list(value).iter().map(|p| expand(p)).collect(),
                    "reseau" | "network" => m.permissions.network = parse_bool(value).map_err(at)?,
                    "stockage" | "storage" => m.permissions.storage = parse_bool(value).map_err(at)?,
                    "bac_a_sable" | "sandbox" => m.permissions.sandbox = parse_bool(value).map_err(at)?,
                    _ => return unknown(),
                },
                Section::Storage => match key {
                    "location" => {
                        let path = PathBuf::from(value);
                        if !path.is_absolute() {
                            return Err(at(format!("location '{value}' : chemin absolu attendu")));
                        }
                        m.storage_location = Some(path);
                    }
                    _ => return unknown(),
                },
            }
        }

        if m.name.is_empty() {
            return Err("[app] name manquant".to_string());
        }
        m.permissions.check().map_err(|e| format!("[permissions] {e}"))?;
        check_app_name(&m.name)?;
        if m.title.is_empty() {
            m.title = m.name.clone();
        }
        for route in &m.routes {
            if route.view.as_os_str().is_empty() {
                return Err(format!("[route {}] : view manquant", route.path));
            }
            if route.style.is_none() && m.window.as_ref().and_then(|w| w.style.as_ref()).is_none() {
                return Err(format!("[route {}] : style manquant (ou [window] style)", route.path));
            }
        }
        for (i, share) in m.shares.iter().enumerate() {
            if m.shares[..i].iter().any(|s| s.flux == share.flux) {
                return Err(format!("[share {}] declare deux fois", share.flux));
            }
        }
        for (i, provide) in m.provides.iter().enumerate() {
            if m.provides[..i].iter().any(|p| p.method == provide.method) {
                return Err(format!("[provide {}] declare deux fois", provide.method));
            }
            if !provide.service.is_empty() && !m.services.iter().any(|s| s.name == provide.service) {
                return Err(format!("[provide {}] : service = {} , mais aucun [service {}]", provide.method, provide.service, provide.service));
            }
        }
        for spec in &m.services {
            if spec.command.is_empty() {
                return Err(format!("[service {}] : command manquant", spec.name));
            }
            ServiceSpec { name: format!("{}-{}", m.name, spec.name), ..spec.clone() }.validate()?;
        }
        Ok(m)
    }

    /// Les taches de fond telles que le provider les connait : `<app>-<nom>`.
    /// Une commande qui est un fichier du dossier de l'app (son binaire)
    /// devient son chemin complet.
    pub fn provider_services(&self) -> Vec<ServiceSpec> {
        self.services
            .iter()
            .map(|spec| {
                let local = self.dir.join(&spec.command);
                let command = if !spec.command.contains('/') && local.is_file() { local.to_string_lossy().into_owned() } else { spec.command.clone() };
                ServiceSpec { name: format!("{}-{}", self.name, spec.name), command, ..spec.clone() }
            })
            .collect()
    }

    /// Les taches de fond (noms du provider) qui servent une methode quand
    /// l'app est fermee (`[provide] service = ...`) : lancees au premier
    /// appel, pas avec l'app.
    pub fn on_demand_services(&self) -> Vec<String> {
        self.provides.iter().filter(|p| !p.service.is_empty()).map(|p| format!("{}-{}", self.name, p.service)).collect()
    }

    /// Tous les fichiers a installer avec l'app : pages, styles, icone et
    /// `files` (`ui` par defaut s'il existe). Sans doublon.
    pub fn bundle_files(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        let files = if self.files.is_empty() { vec![self.dir.join("ui")].into_iter().filter(|p| p.exists()).collect() } else { self.files.clone() };
        let declared = self.routes.iter().flat_map(|r| [Some(r.view.clone()), r.style.clone()]).flatten();
        let window_style = self.window.as_ref().and_then(|w| w.style.clone());
        for path in files.into_iter().chain(declared).chain(window_style).chain(self.icon.clone()) {
            if !out.iter().any(|known| path.starts_with(known)) {
                out.retain(|known| !known.starts_with(&path));
                out.push(path);
            }
        }
        out
    }

    pub fn share(&self, flux: &str) -> Option<&ShareDecl> {
        self.shares.iter().find(|s| s.flux == flux)
    }

    pub fn provide(&self, method: &str) -> Option<&ProvideDecl> {
        self.provides.iter().find(|p| p.method == method)
    }

    pub fn uses(&self, method: &str, owner: &str) -> bool {
        self.uses.iter().any(|u| u.method == method && u.owner == owner)
    }

    pub fn listen(&self, flux: &str, owner: &str) -> Option<&ListenDecl> {
        self.listens.iter().find(|l| l.flux == flux && l.owner == owner)
    }

    pub fn summary(&self) -> Summary {
        Summary {
            name: self.name.clone(),
            title: self.title.clone(),
            version: self.version.clone(),
            shares: self.shares.clone(),
            listens: self.listens.clone(),
            provides: self.provides.clone(),
            uses: self.uses.clone(),
            opens: self.opens.clone(),
            services: self.services.iter().map(|s| s.name.clone()).collect(),
            permissions: self.permissions.clone(),
        }
    }
}

/// Ce que le manager garde d'un manifeste (ce que le tableau de bord montre).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Summary {
    pub name: String,
    pub title: String,
    pub version: String,
    pub shares: Vec<ShareDecl>,
    pub listens: Vec<ListenDecl>,
    pub provides: Vec<ProvideDecl>,
    pub uses: Vec<UseDecl>,
    pub opens: Vec<String>,
    pub services: Vec<String>,
    pub permissions: Permissions,
}

fn write_list(w: Writer, items: &[String]) -> Writer {
    items.iter().fold(w.u32(items.len() as u32), |w, s| w.str(s))
}

fn read_list(r: &mut Reader) -> Result<Vec<String>, String> {
    let count = r.u32()?.min(4096);
    (0..count).map(|_| r.str()).collect()
}

impl Summary {
    pub fn write(&self, w: Writer) -> Writer {
        let mut w = w.str(&self.name).str(&self.title).str(&self.version).u32(self.shares.len() as u32);
        for s in &self.shares {
            w = write_list(w.str(&s.flux), &s.to).u8(s.public as u8);
        }
        w = w.u32(self.listens.len() as u32);
        for l in &self.listens {
            w = write_list(write_list(w.str(&l.flux).str(&l.owner), &l.paths), &l.events);
        }
        w = write_list(w, &self.services).u32(self.provides.len() as u32);
        for p in &self.provides {
            w = write_list(w.str(&p.method), &p.to).u8(p.public as u8).str(&p.description);
        }
        w = w.u32(self.uses.len() as u32);
        for u in &self.uses {
            w = w.str(&u.method).str(&u.owner);
        }
        let p = &self.permissions;
        let paths = |w: Writer, list: &[PathBuf]| list.iter().fold(w.u32(list.len() as u32), |w, x| w.str(&x.to_string_lossy()));
        w = paths(paths(w, &p.read), &p.write);
        w = w.u8(p.network as u8).u8(p.storage as u8).u8(p.sandbox as u8);
        // Version 4 : la tache de fond de chaque methode, dans l'ordre.
        w = self.provides.iter().fold(w, |w, p| w.str(&p.service));
        // Version 5 : les apps qu'elle peut lancer.
        write_list(w, &self.opens)
    }

    pub fn read(r: &mut Reader) -> Result<Summary, String> {
        Summary::read_version(r, 5)
    }

    /// `version` 1 : sans appels (fichier manager.bin d'avant les appels).
    pub fn read_version(r: &mut Reader, version: u32) -> Result<Summary, String> {
        let (name, title, app_version) = (r.str()?, r.str()?, r.str()?);
        let mut shares = Vec::new();
        for _ in 0..r.u32()?.min(4096) {
            shares.push(ShareDecl { flux: r.str()?, to: read_list(r)?, public: r.u8()? != 0, persist: false });
        }
        let mut listens = Vec::new();
        for _ in 0..r.u32()?.min(4096) {
            listens.push(ListenDecl { flux: r.str()?, owner: r.str()?, paths: read_list(r)?, events: read_list(r)? });
        }
        let services = read_list(r)?;
        let (mut provides, mut uses) = (Vec::new(), Vec::new());
        if version >= 2 {
            for _ in 0..r.u32()?.min(4096) {
                provides.push(ProvideDecl { method: r.str()?, to: read_list(r)?, public: r.u8()? != 0, description: r.str()?, service: String::new() });
            }
            for _ in 0..r.u32()?.min(4096) {
                uses.push(UseDecl { method: r.str()?, owner: r.str()? });
            }
        }
        let mut permissions = Permissions::default();
        if version >= 3 {
            let paths = |r: &mut Reader| -> Result<Vec<PathBuf>, String> { (0..r.u32()?.min(4096)).map(|_| r.str().map(PathBuf::from)).collect() };
            permissions.read = paths(r)?;
            permissions.write = paths(r)?;
            permissions.network = r.u8()? != 0;
            permissions.storage = r.u8()? != 0;
            permissions.sandbox = r.u8()? != 0;
        }
        if version >= 4 {
            for p in &mut provides {
                p.service = r.str()?;
            }
        }
        let opens = if version >= 5 { read_list(r)? } else { Vec::new() };
        Ok(Summary { name, title, version: app_version, shares, listens, provides, uses, opens, services, permissions })
    }
}
