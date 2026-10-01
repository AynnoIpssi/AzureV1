// Le coeur d'azure-manager, sans socket : le registre des apps (nom -> id,
// executable, manifeste), les choix du tableau de bord sur les partages, et
// l'etat montre au tableau de bord.
//
// Qui peut ecouter un flux, ou appeler une methode = ce que declare le
// manifeste de son app (`to`, `public` de `[share]` / `[provide]`), modifie
// par le tableau de bord (autoriser / retirer une app, rendre public ou
// non). Ces choix sont gardes sur disque.
use crate::models::manifest::{check_app_name, Summary};
use azure_core::models::wire::{Reader, Writer};
use azure_provider::ServiceStatus;
use azure_service::flux::protocol::{FluxStats, MethodStats};
use azure_service::flux::{Access, Value};
use std::collections::VecDeque;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Premier id attribue (en dessous : ids ecrits a la main par les
/// anciennes apps, jamais attribues ici).
pub const FIRST_ID: u32 = 1000;

const FILE_VERSION: u32 = 5;

/// Ce qu'une app offre aux autres : un flux (`[share]`) ou une methode
/// (`[provide]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Flux,
    Call,
}

impl Kind {
    pub fn code(self) -> u8 {
        match self {
            Kind::Flux => 0,
            Kind::Call => 1,
        }
    }

    pub fn from_code(code: u8) -> Result<Kind, String> {
        match code {
            0 => Ok(Kind::Flux),
            1 => Ok(Kind::Call),
            other => Err(format!("Genre inconnu : {other}")),
        }
    }

    fn section(self) -> &'static str {
        match self {
            Kind::Flux => "share",
            Kind::Call => "provide",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppRecord {
    pub id: u32,
    pub exe: String,
    /// Empreinte SHA-256 de l'executable (voir
    /// `azure_core::managers::identity::fingerprint_pid`).
    pub fingerprint: Option<[u8; 32]>,
    /// Installee par `azure install` : son identite est son empreinte (elle
    /// peut etre deplacee, pas remplacee), et son manifeste est celui
    /// enregistre a l'installation (pas celui qu'elle annonce).
    pub installed: bool,
    pub summary: Summary,
    /// Connexions ouvertes (0 = l'app ne tourne pas) et pid de la derniere.
    pub sessions: u32,
    pub pid: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Override {
    public: Option<bool>,
    allow: BTreeSet<String>,
    deny: BTreeSet<String>,
}

/// Un acces a donner a azure-service (flux ou methode `name` de l'app
/// `owner`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Push {
    pub kind: Kind,
    pub owner: u32,
    pub name: String,
    pub access: Access,
}

/// Ce que declare un manifeste pour un flux ou une methode.
struct Offer {
    to: Vec<String>,
    public: bool,
}

fn offer(summary: &Summary, kind: Kind, name: &str) -> Option<Offer> {
    match kind {
        Kind::Flux => summary.shares.iter().find(|s| s.flux == name).map(|s| Offer { to: s.to.clone(), public: s.public }),
        Kind::Call => summary.provides.iter().find(|p| p.method == name).map(|p| Offer { to: p.to.clone(), public: p.public }),
    }
}

fn offers(summary: &Summary, kind: Kind) -> Vec<String> {
    match kind {
        Kind::Flux => summary.shares.iter().map(|s| s.flux.clone()).collect(),
        Kind::Call => summary.provides.iter().map(|p| p.method.clone()).collect(),
    }
}

/// Les apps qui declarent utiliser `name` de `owner` (`[listen]` / `[use]`).
fn declared_users<'a>(apps: &'a BTreeMap<String, AppRecord>, kind: Kind, owner: &str, name: &str) -> Vec<&'a str> {
    apps.values()
        .filter(|a| match kind {
            Kind::Flux => a.summary.listens.iter().any(|l| l.owner == owner && l.flux == name),
            Kind::Call => a.summary.uses.iter().any(|u| u.owner == owner && u.method == name),
        })
        .map(|a| a.summary.name.as_str())
        .collect()
}

/// Compteurs d'azure-service (flux et appels), relus chaque seconde.
#[derive(Clone, Debug, Default)]
pub struct Activity {
    pub flux: Vec<FluxStats>,
    pub methods: Vec<MethodStats>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Level {
        match code {
            0 => Level::Info,
            1 => Level::Warning,
            _ => Level::Error,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warning => "attention",
            Level::Error => "erreur",
        }
    }
}

/// Ce qu'une app a signale (voir `AzureApp::error`).
#[derive(Clone, Debug)]
pub struct Event {
    /// Secondes depuis 1970.
    pub at: u64,
    pub app: String,
    pub level: Level,
    pub message: String,
}

/// Evenements gardes (les plus anciens partent).
pub const MAX_EVENTS: usize = 500;

/// Heure locale `HH:MM:SS` de `at`.
pub fn local_time(at: u64) -> String {
    let t = at as libc::time_t;
    // SAFETY : `tm` est initialise par `localtime_r` avant d'etre lu.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return String::new();
    }
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Compteurs cumules d'un flux ou d'une methode : azure-service repart de
/// zero a chaque redemarrage, le manager garde le total.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Totals {
    /// Deja compte avant le dernier redemarrage d'azure-service.
    base: [u64; 4],
    /// Derniere valeur lue chez azure-service.
    last: [u64; 4],
}

impl Totals {
    fn absorb(&mut self, raw: [u64; 4]) -> [u64; 4] {
        // Un compteur qui recule : azure-service a redemarre.
        if raw.iter().zip(&self.last).any(|(r, l)| r < l) {
            for i in 0..4 {
                self.base[i] += self.last[i];
            }
        }
        self.last = raw;
        std::array::from_fn(|i| self.base[i] + raw[i])
    }
}

const ACTIVITY_VERSION: u32 = 1;

pub struct Manager {
    events: VecDeque<Event>,
    totals: BTreeMap<(u8, u32, String), Totals>,
    /// Evenements ou compteurs a enregistrer (`save_activity`).
    activity_dirty: bool,
    file: Option<PathBuf>,
    vault: Option<azure_core::security::vault::Vault>,
    apps: BTreeMap<String, AppRecord>,
    overrides: BTreeMap<(Kind, String, String), Override>,
    next_id: u32,
    /// Executables autorises a piloter le manager (tableau de bord, ligne de
    /// commande).
    admins: Vec<String>,
}

impl Manager {
    /// Sans fichier (tests).
    pub fn in_memory(admins: Vec<String>) -> Manager {
        Manager { events: VecDeque::new(), totals: BTreeMap::new(), activity_dirty: false, file: None, vault: None, apps: BTreeMap::new(), overrides: BTreeMap::new(), next_id: FIRST_ID, admins }
    }

    /// Registre garde dans `dir/manager.bin`.
    pub fn open(dir: &Path, admins: Vec<String>) -> Result<Manager, String> {
        let vault = azure_core::security::vault::Vault::open(dir)?;
        let file = dir.join("manager.bin");
        let mut manager = Manager { file: Some(file.clone()), vault: Some(vault.clone()), ..Manager::in_memory(admins) };
        if let Some(data) = vault.read(&file, "manager")? {
            manager.read(&data).map_err(|e| format!("{} : {e}", file.display()))?;
        }
        // Evenements et compteurs : un fichier illisible n'empeche pas de
        // demarrer (ce n'est que de l'historique).
        let activity = dir.join("activite.bin");
        match vault.read(&activity, "manager-activite") {
            Ok(Some(data)) => {
                if let Err(e) = manager.read_activity(&data) {
                    eprintln!("azure-manager : {} ignore ({e})", activity.display());
                    manager.events.clear();
                    manager.totals.clear();
                }
            }
            Ok(None) => {}
            Err(e) => eprintln!("azure-manager : {} ignore ({e})", activity.display()),
        }
        Ok(manager)
    }

    fn read_activity(&mut self, data: &[u8]) -> Result<(), String> {
        let mut r = Reader::new(data);
        if r.u32()? != ACTIVITY_VERSION {
            return Err("version inconnue".to_string());
        }
        for _ in 0..r.u32()?.min(MAX_EVENTS as u32) {
            let (at, app, level, message) = (r.u64()?, r.str()?, Level::from_code(r.u8()?), r.str()?);
            self.events.push_back(Event { at, app, level, message });
        }
        for _ in 0..r.u32()? {
            let key = (r.u8()?, r.u32()?, r.str()?);
            let mut t = Totals::default();
            for i in 0..4 {
                t.base[i] = r.u64()?;
            }
            for i in 0..4 {
                t.last[i] = r.u64()?;
            }
            self.totals.insert(key, t);
        }
        r.finish()
    }

    /// Enregistre evenements et compteurs s'ils ont change (appele chaque
    /// seconde par le daemon, pas a chaque evenement).
    pub fn save_activity(&mut self) -> Result<(), String> {
        if !self.activity_dirty {
            return Ok(());
        }
        let (Some(file), Some(vault)) = (&self.file, &self.vault) else { return Ok(()) };
        let mut w = Writer::new().u32(ACTIVITY_VERSION).u32(self.events.len() as u32);
        for e in &self.events {
            w = w.u64(e.at).str(&e.app).u8(e.level.code()).str(&e.message);
        }
        w = w.u32(self.totals.len() as u32);
        for ((kind, owner, name), t) in &self.totals {
            w = w.u8(*kind).u32(*owner).str(name);
            w = t.base.iter().chain(&t.last).fold(w, |w, v| w.u64(*v));
        }
        vault.write(&file.with_file_name("activite.bin"), "manager-activite", &w.finish())?;
        self.activity_dirty = false;
        Ok(())
    }

    /// Compteurs d'azure-service rendus cumulatifs (voir `Totals`).
    pub fn absorb(&mut self, mut activity: Activity) -> Activity {
        for f in &mut activity.flux {
            let t = self.totals.entry((0, f.owner, f.name.clone())).or_default();
            let before = t.clone();
            f.changes = t.absorb([f.changes, 0, 0, 0])[0];
            self.activity_dirty |= *t != before;
        }
        for m in &mut activity.methods {
            let t = self.totals.entry((1, m.owner, m.method.clone())).or_default();
            let before = t.clone();
            let [calls, errors, timeouts, total_ms] = t.absorb([m.calls, m.errors, m.timeouts, m.total_ms]);
            (m.calls, m.errors, m.timeouts, m.total_ms) = (calls, errors, timeouts, total_ms);
            self.activity_dirty |= *t != before;
        }
        activity
    }

    fn read(&mut self, data: &[u8]) -> Result<(), String> {
        let mut r = Reader::new(data);
        let version = r.u32()?;
        if version == 0 || version > FILE_VERSION {
            return Err(format!("version de fichier inconnue ({version})"));
        }
        self.next_id = r.u32()?;
        for _ in 0..r.u32()? {
            let (id, exe) = (r.u32()?, r.str()?);
            let summary = Summary::read_version(&mut r, version)?;
            let (fingerprint, installed) = if version >= 3 {
                let fp = r.bytes()?;
                let fp = (fp.len() == 32).then(|| {
                    let mut a = [0u8; 32];
                    a.copy_from_slice(fp);
                    a
                });
                (fp, r.u8()? != 0)
            } else {
                (None, false)
            };
            self.apps.insert(summary.name.clone(), AppRecord { id, exe, fingerprint, installed, summary, sessions: 0, pid: None });
        }
        for _ in 0..r.u32()? {
            // Version 1 : seulement des flux, sans genre.
            let kind = if version >= 2 { Kind::from_code(r.u8()?)? } else { Kind::Flux };
            let key = (kind, r.str()?, r.str()?);
            let public = match r.u8()? {
                0 => None,
                1 => Some(false),
                _ => Some(true),
            };
            let set = |r: &mut Reader| -> Result<BTreeSet<String>, String> { (0..r.u32()?).map(|_| r.str()).collect() };
            let allow = set(&mut r)?;
            let deny = set(&mut r)?;
            self.overrides.insert(key, Override { public, allow, deny });
        }
        r.finish()
    }

    fn save(&self) -> Result<(), String> {
        let Some(file) = &self.file else { return Ok(()) };
        let mut w = Writer::new().u32(FILE_VERSION).u32(self.next_id).u32(self.apps.len() as u32);
        for app in self.apps.values() {
            w = app.summary.write(w.u32(app.id).str(&app.exe));
            w = w.bytes(app.fingerprint.as_ref().map(|f| f.as_slice()).unwrap_or(&[])).u8(app.installed as u8);
        }
        w = w.u32(self.overrides.len() as u32);
        for ((kind, owner, name), o) in &self.overrides {
            w = w.u8(kind.code()).str(owner).str(name).u8(match o.public {
                None => 0,
                Some(false) => 1,
                Some(true) => 2,
            });
            w = o.allow.iter().fold(w.u32(o.allow.len() as u32), |w, s| w.str(s));
            w = o.deny.iter().fold(w.u32(o.deny.len() as u32), |w, s| w.str(s));
        }
        match &self.vault {
            Some(vault) => vault.write(file, "manager", &w.finish()),
            None => Ok(()),
        }
    }

    pub fn is_admin(&self, exe: &str) -> bool {
        self.admins.iter().any(|admin| admin == exe)
    }

    /// Ids des apps qui sont des tableaux de bord (elles ecoutent l'etat).
    pub fn admin_ids(&self) -> Vec<u32> {
        self.apps.values().filter(|a| self.is_admin(&a.exe)).map(|a| a.id).collect()
    }

    pub fn app(&self, name: &str) -> Option<&AppRecord> {
        self.apps.get(name)
    }

    /// Une app se presente (a chaque lancement). La premiere fois, elle
    /// recoit un id et son nom est lie a son executable : une autre app ne
    /// pourra plus le prendre. Retourne son id.
    ///
    /// Identite : une app installee est reconnue a son empreinte (deplacee :
    /// acceptee ; binaire remplace : refusee). Une app de developpement, a
    /// son chemin ou son empreinte (recompilee ou deplacee : acceptee).
    pub fn register(&mut self, summary: Summary, exe: &str, fingerprint: Option<[u8; 32]>, pid: u32) -> Result<u32, String> {
        check_app_name(&summary.name)?;
        let id = match self.apps.get_mut(&summary.name) {
            Some(app) => {
                let same_binary = fingerprint.is_some() && fingerprint == app.fingerprint;
                let allowed = if app.installed { same_binary } else { same_binary || app.exe == exe };
                if !allowed {
                    return Err(if app.installed {
                        format!("'{}' est installee ({}) : cet executable ({exe}) n'est pas le sien (empreinte differente)", summary.name, app.exe)
                    } else {
                        format!("Le nom '{}' appartient a l'executable {}, pas a {exe} (azure_manager forget {} pour le liberer)", summary.name, app.exe, summary.name)
                    });
                }
                // Installee : le manifeste enregistre a l'installation fait foi.
                let changed = (!app.installed && app.summary != summary) || app.exe != exe || (!app.installed && app.fingerprint != fingerprint);
                if !app.installed {
                    app.summary = summary;
                    app.fingerprint = fingerprint;
                }
                app.exe = exe.to_string();
                app.sessions += 1;
                app.pid = Some(pid);
                let id = app.id;
                if changed {
                    self.save()?;
                }
                id
            }
            None => {
                let id = self.next_id;
                self.next_id += 1;
                self.apps.insert(summary.name.clone(), AppRecord { id, exe: exe.to_string(), fingerprint, installed: false, summary, sessions: 1, pid: Some(pid) });
                self.save()?;
                id
            }
        };
        Ok(id)
    }

    /// `azure install` : enregistre l'app installee (son executable, son
    /// empreinte, son manifeste). Elle garde son id, ses donnees et les
    /// choix du tableau de bord si elle etait deja connue.
    pub fn install(&mut self, summary: Summary, exe: &str, fingerprint: [u8; 32]) -> Result<u32, String> {
        check_app_name(&summary.name)?;
        summary.permissions.check()?;
        let id = match self.apps.get_mut(&summary.name) {
            Some(app) => {
                app.exe = exe.to_string();
                app.fingerprint = Some(fingerprint);
                app.installed = true;
                app.summary = summary;
                app.id
            }
            None => {
                let id = self.next_id;
                self.next_id += 1;
                self.apps.insert(summary.name.clone(), AppRecord { id, exe: exe.to_string(), fingerprint: Some(fingerprint), installed: true, summary, sessions: 0, pid: None });
                id
            }
        };
        self.save()?;
        Ok(id)
    }

    /// L'app qui porte l'id `id` (pour que les autres daemons verifient qui
    /// leur parle).
    pub fn by_id(&self, id: u32) -> Option<&AppRecord> {
        self.apps.values().find(|a| a.id == id)
    }

    /// Une connexion de l'app `name` s'est fermee.
    pub fn disconnected(&mut self, name: &str) {
        if let Some(app) = self.apps.get_mut(name) {
            app.sessions = app.sessions.saturating_sub(1);
            if app.sessions == 0 {
                app.pid = None;
            }
        }
    }

    pub fn resolve(&self, name: &str) -> Result<u32, String> {
        self.apps.get(name).map(|a| a.id).ok_or_else(|| format!("App inconnue : '{name}' (elle ne s'est jamais lancee)"))
    }

    /// Apps autorisees (noms), et si c'est public.
    fn allowed(&self, kind: Kind, owner: &str, name: &str) -> Result<(bool, BTreeSet<String>), String> {
        let app = self.apps.get(owner).ok_or_else(|| format!("App inconnue : '{owner}'"))?;
        let declared = offer(&app.summary, kind, name).ok_or_else(|| format!("'{owner}' ne declare pas [{} {name}] dans son manifeste", kind.section()))?;
        let empty = Override::default();
        let o = self.overrides.get(&(kind, owner.to_string(), name.to_string())).unwrap_or(&empty);
        let mut names: BTreeSet<String> = declared.to.into_iter().collect();
        names.extend(o.allow.iter().cloned());
        names.retain(|n| !o.deny.contains(n));
        Ok((o.public.unwrap_or(declared.public), names))
    }

    /// Qui peut ecouter le flux (ou appeler la methode) `name` de `owner`.
    /// Les apps pas encore lancees une fois (sans id) sont ajoutees quand
    /// elles arrivent (voir `pushes`).
    pub fn access(&self, kind: Kind, owner: &str, name: &str) -> Result<Access, String> {
        let (public, names) = self.allowed(kind, owner, name)?;
        if public {
            return Ok(Access::Public);
        }
        Ok(Access::Apps(names.iter().filter_map(|n| self.apps.get(n)).map(|a| a.id).collect()))
    }

    /// `caller` peut-il ouvrir `target` (`[open target]` dans son manifeste,
    /// `target` installee) ? `Ok(true)` : elle ne tourne pas, a lancer.
    pub fn open_check(&self, caller: &str, target: &str) -> Result<bool, String> {
        let from = self.apps.get(caller).ok_or_else(|| format!("App inconnue : '{caller}'"))?;
        if !from.summary.opens.iter().any(|o| o == target) {
            return Err(format!("'{caller}' ne declare pas [open {target}] dans son manifeste"));
        }
        let app = self.apps.get(target).ok_or_else(|| format!("'{target}' n'est pas installee"))?;
        if !app.installed {
            return Err(format!("'{target}' n'est pas installee (azure install)"));
        }
        Ok(app.sessions == 0)
    }

    /// La tache de fond a lancer pour que `caller` puisse appeler `method`
    /// de `owner` alors que `owner` n'est pas ouverte (`[provide method]
    /// service = ...`) : son nom chez azure-provider, `<owner>-<service>`.
    /// Refuse si `caller` n'a pas le droit d'appeler la methode : on ne
    /// reveille pas une app pour rien.
    pub fn wake_service(&self, caller: &str, owner: &str, method: &str) -> Result<String, String> {
        let app = self.apps.get(owner).ok_or_else(|| format!("App inconnue : '{owner}'"))?;
        let provide = app.summary.provides.iter().find(|p| p.method == method).ok_or_else(|| format!("'{owner}' ne declare pas [provide {method}] dans son manifeste"))?;
        let caller_id = self.resolve(caller)?;
        if !self.access(Kind::Call, owner, method)?.allows(app.id, caller_id) {
            return Err(format!("'{owner}' ne permet pas a '{caller}' d'appeler '{method}'"));
        }
        if provide.service.is_empty() {
            return Err(format!("'{method}' de '{owner}' n'est servie que par l'app ouverte (aucun service dans son manifeste)"));
        }
        Ok(format!("{owner}-{}", provide.service))
    }

    /// Tous les acces a donner a azure-service.
    pub fn pushes(&self) -> Vec<Push> {
        let mut out = Vec::new();
        for app in self.apps.values() {
            for kind in [Kind::Flux, Kind::Call] {
                for name in offers(&app.summary, kind) {
                    if let Ok(access) = self.access(kind, &app.summary.name, &name) {
                        out.push(Push { kind, owner: app.id, name, access });
                    }
                }
            }
        }
        out
    }

    fn edit(&mut self, kind: Kind, owner: &str, name: &str, change: impl FnOnce(&mut Override)) -> Result<(), String> {
        // Verifie que c'est declare.
        self.allowed(kind, owner, name)?;
        let key = (kind, owner.to_string(), name.to_string());
        let entry = self.overrides.entry(key.clone()).or_default();
        change(entry);
        if *entry == Override::default() {
            self.overrides.remove(&key);
        }
        self.save()
    }

    fn declared(&self, kind: Kind, owner: &str, name: &str) -> Option<Offer> {
        self.apps.get(owner).and_then(|a| offer(&a.summary, kind, name))
    }

    /// Autorise `app` (a ecouter le flux / appeler la methode).
    pub fn grant(&mut self, kind: Kind, owner: &str, name: &str, app: &str) -> Result<(), String> {
        self.resolve(app)?;
        let declared = self.declared(kind, owner, name).is_some_and(|o| o.to.iter().any(|t| t == app));
        self.edit(kind, owner, name, |o| {
            o.deny.remove(app);
            if !declared {
                o.allow.insert(app.to_string());
            }
        })
    }

    /// Retire ce droit a `app` (ses ecoutes en cours sont coupees).
    pub fn revoke(&mut self, kind: Kind, owner: &str, name: &str, app: &str) -> Result<(), String> {
        let declared = self.declared(kind, owner, name).is_some_and(|o| o.to.iter().any(|t| t == app));
        self.edit(kind, owner, name, |o| {
            o.allow.remove(app);
            if declared {
                o.deny.insert(app.to_string());
            }
        })
    }

    pub fn set_public(&mut self, kind: Kind, owner: &str, name: &str, public: bool) -> Result<(), String> {
        let declared_public = self.declared(kind, owner, name).map(|o| o.public);
        self.edit(kind, owner, name, |o| o.public = if declared_public == Some(public) { None } else { Some(public) })
    }

    /// Revient a ce que declare le manifeste.
    pub fn reset(&mut self, kind: Kind, owner: &str, name: &str) -> Result<(), String> {
        self.edit(kind, owner, name, |o| *o = Override::default())
    }

    /// Une app signale quelque chose (erreur, plantage, information).
    pub fn report(&mut self, app: &str, level: Level, message: &str) {
        self.events.push_back(Event { at: now(), app: app.to_string(), level, message: message.chars().take(2000).collect() });
        self.activity_dirty = true;
        while self.events.len() > MAX_EVENTS {
            self.events.pop_front();
        }
    }

    pub fn events(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }

    /// Oublie une app (son nom pourra etre pris par un autre executable,
    /// par exemple apres un deplacement). Refuse si elle tourne.
    pub fn forget(&mut self, name: &str) -> Result<(), String> {
        let app = self.apps.get(name).ok_or_else(|| format!("App inconnue : '{name}'"))?;
        if app.sessions > 0 {
            return Err(format!("'{name}' tourne encore"));
        }
        self.apps.remove(name);
        self.overrides.retain(|(_, owner, _), _| owner != name);
        self.save()
    }

    /// Un flux ou une methode d'une app, pour le tableau de bord.
    fn offer_state(&self, kind: Kind, owner: &str, name: &str, description: &str, activity: &Activity) -> Value {
        let text = |s: &str| Value::from(s);
        let owner_id = self.apps.get(owner).map(|a| a.id).unwrap_or(0);
        let names = |set: &BTreeSet<String>| Value::list(set.iter().map(|n| text(n)));
        let (public, allowed) = self.allowed(kind, owner, name).unwrap_or_default();
        let users = declared_users(&self.apps, kind, owner, name);
        let others: Vec<Value> = self.apps.values().filter(|a| a.summary.name != owner && !allowed.contains(&a.summary.name)).map(|a| text(&a.summary.name)).collect();
        let (key, users_key, nb_users_key) = match kind {
            Kind::Flux => ("flux", "ecoutent", "nb_ecoutent"),
            Kind::Call => ("methode", "appelants", "nb_appelants"),
        };
        Value::map([
            (key, text(name)),
            ("description", text(description)),
            ("public", public.into()),
            ("modifie", self.overrides.contains_key(&(kind, owner.to_string(), name.to_string())).into()),
            ("nb_autorises", (allowed.len() as i64).into()),
            ("autorises", names(&allowed)),
            (nb_users_key, (users.len() as i64).into()),
            (users_key, Value::list(users.iter().map(|n| text(n)))),
            ("autres", Value::List(others)),
            ("stats", match kind {
                Kind::Flux => {
                    let f = activity.flux.iter().find(|f| f.owner == owner_id && f.name == name);
                    Value::map([
                        ("actif", f.is_some().into()),
                        ("modifications", (f.map(|f| f.changes).unwrap_or(0) as i64).into()),
                        ("ecoutes", (f.map(|f| f.listeners).unwrap_or(0) as i64).into()),
                        ("persistant", f.is_some_and(|f| f.persist).into()),
                    ])
                }
                Kind::Call => {
                    let m = activity.methods.iter().find(|m| m.owner == owner_id && m.method == name);
                    let calls = m.map(|m| m.calls).unwrap_or(0);
                    Value::map([
                        ("servie", m.is_some_and(|m| m.served).into()),
                        ("appels", (calls as i64).into()),
                        ("erreurs", (m.map(|m| m.errors).unwrap_or(0) as i64).into()),
                        ("delais", (m.map(|m| m.timeouts).unwrap_or(0) as i64).into()),
                        ("duree", (if calls > 0 { m.map(|m| m.total_ms / calls).unwrap_or(0) } else { 0 } as i64).into()),
                        ("derniere_erreur", text(m.map(|m| m.last_error.as_str()).unwrap_or(""))),
                    ])
                }
            }),
        ])
    }

    fn link(&self, kind: Kind, owner: &str, user: &AppRecord, name: &str) -> Value {
        let text = |s: &str| Value::from(s);
        let allowed = self.apps.get(owner).is_some_and(|o| self.access(kind, owner, name).is_ok_and(|a| a.allows(o.id, user.id)));
        let status = if !self.apps.contains_key(owner) {
            "source inconnue"
        } else if self.allowed(kind, owner, name).is_err() {
            "non déclaré"
        } else if allowed {
            "autorisé"
        } else {
            "refusé"
        };
        let (kind_text, verb) = match kind {
            Kind::Flux => ("flux", "écoute"),
            Kind::Call => ("appel", "appelle"),
        };
        Value::map([
            ("type", text(kind_text)),
            ("verbe", text(verb)),
            ("de", text(owner)),
            ("vers", text(&user.summary.name)),
            ("nom", text(name)),
            ("autorise", allowed.into()),
            ("statut", text(status)),
        ])
    }

    /// L'etat montre par le tableau de bord (des listes, pour `<for>` en
    /// rsH) : `apps`, `liens` (qui ecoute ou appelle qui), `services`.
    pub fn state(&self, services: &[ServiceStatus], provider_ok: bool) -> Value {
        self.state_with(services, provider_ok, &Activity::default())
    }

    /// L'etat avec les compteurs d'azure-service et les evenements.
    pub fn state_with(&self, services: &[ServiceStatus], provider_ok: bool, activity: &Activity) -> Value {
        let text = |s: &str| Value::from(s);
        let apps = self.apps.values().map(|app| {
            let owner = app.summary.name.as_str();
            let shares = app.summary.shares.iter().map(|s| self.offer_state(Kind::Flux, owner, &s.flux, "", activity));
            let provides = app.summary.provides.iter().map(|p| self.offer_state(Kind::Call, owner, &p.method, &p.description, activity));
            let errors: Vec<&Event> = self.events.iter().rev().filter(|e| e.app == owner && e.level == Level::Error).collect();
            let listens = app.summary.listens.iter().map(|l| Value::map([("flux", text(&l.flux)), ("source", text(&l.owner))]));
            let uses = app.summary.uses.iter().map(|u| Value::map([("methode", text(&u.method)), ("source", text(&u.owner))]));
            Value::map([
                ("nom", text(owner)),
                ("id", app.id.into()),
                ("titre", text(&app.summary.title)),
                ("version", text(&app.summary.version)),
                ("actif", (app.sessions > 0).into()),
                ("pid", app.pid.map(|p| p as i64).unwrap_or(0).into()),
                ("exe", text(&app.exe)),
                ("admin", self.is_admin(&app.exe).into()),
                ("installee", app.installed.into()),
                ("empreinte", text(&app.fingerprint.map(|f| azure_core::managers::identity::hex(&f[..6])).unwrap_or_default())),
                ("permissions", text(&app.summary.permissions.describe())),
                ("enfermee", app.summary.permissions.sandbox.into()),
                ("nb_partages", (app.summary.shares.len() as i64).into()),
                ("partages", Value::list(shares)),
                ("nb_ecoutes", (app.summary.listens.len() as i64).into()),
                ("ecoutes", Value::list(listens)),
                ("nb_fournit", (app.summary.provides.len() as i64).into()),
                ("fournit", Value::list(provides)),
                ("nb_utilise", (app.summary.uses.len() as i64).into()),
                ("utilise", Value::list(uses)),
                ("nb_erreurs", (errors.len() as i64).into()),
                ("erreurs", Value::list(errors.iter().take(5).map(|e| Value::map([("heure", text(&local_time(e.at))), ("message", text(&e.message))])))),
                ("nb_services", (app.summary.services.len() as i64).into()),
                ("services", Value::list(app.summary.services.iter().map(|s| text(s)))),
            ])
        });
        let mut links = Vec::new();
        for app in self.apps.values() {
            for l in &app.summary.listens {
                links.push(self.link(Kind::Flux, &l.owner, app, &l.flux));
            }
            for u in &app.summary.uses {
                links.push(self.link(Kind::Call, &u.owner, app, &u.method));
            }
        }
        let services = services.iter().map(|s| {
            Value::map([
                ("nom", text(&s.name)),
                ("etat", text(s.state.label())),
                ("pret", s.state.is_ready().into()),
                ("pid", s.pid.map(|p| p as i64).unwrap_or(0).into()),
                ("relances", s.restarts.into()),
                ("message", text(&s.message)),
            ])
        });
        let running = self.apps.values().filter(|a| a.sessions > 0).count();
        Value::map([
            ("apps", Value::list(apps)),
            ("liens", Value::List(links)),
            ("services", Value::list(services)),
            ("evenements", Value::list(self.events.iter().rev().take(200).map(|e| {
                Value::map([
                    ("heure", text(&local_time(e.at))),
                    ("app", text(&e.app)),
                    ("niveau", text(e.level.label())),
                    ("message", text(&e.message)),
                ])
            }))),
            ("nb_evenements", (self.events.len() as i64).into()),
            ("nb_apps", (self.apps.len() as i64).into()),
            ("nb_actives", (running as i64).into()),
            ("provider", provider_ok.into()),
        ])
    }
}
