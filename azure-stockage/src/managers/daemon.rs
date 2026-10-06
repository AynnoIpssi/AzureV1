// Le daemon de stockage : SEUL process qui touche au disque (voir
// `AzureStockage`). Une connexion = une app : elle commence par HELLO avec
// son id, verifie contre son executable reel (voir `identity`), et toutes
// ses requetes suivantes agissent en son nom.
use crate::managers::identity::peer_exe;
use crate::managers::stockage::{AzureStockage, Credentials};
use crate::models::request::*;
use crate::models::wire::{Reader, Writer};
use crate::rss::engine::{execute, Login, RssResult, Session};
use crate::rss::value::Value;
use azure_core::models::storage_model::{Role, ShareAccess};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{Arc, Mutex};

static MANAGER_SOCKET: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Socket d'azure-manager a interroger sur l'identite des apps (defaut :
/// `azure_core::security::registry::MANAGER_SOCKET`).
pub fn set_manager_socket(socket: &str) {
    let _ = MANAGER_SOCKET.set(socket.to_string());
}

fn manager_socket() -> String {
    MANAGER_SOCKET.get().cloned().unwrap_or_else(|| azure_core::security::registry::MANAGER_SOCKET.to_string())
}

/// Nom (attribue par azure-manager) de l'app admin des donnees : installee
/// sous ce nom, elle peut ouvrir l'espace de toutes les apps (opcodes
/// `ADMIN_*`). Voir SECURITE.md.
pub const ADMIN_APP: &str = "data";

/// Reglages du daemon.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Executables admin en plus de l'app `ADMIN_APP` installee (essais,
    /// developpement) : `--admin <exe>` ou `AZURE_STOCKAGE_ADMIN=a:b`.
    pub admins: Vec<String>,
}

impl Options {
    /// Depuis les arguments du daemon et son environnement.
    pub fn from_args(args: &[String]) -> Options {
        let mut admins: Vec<String> = args.windows(2).filter(|w| w[0] == "--admin").map(|w| w[1].clone()).collect();
        if let Ok(list) = std::env::var("AZURE_STOCKAGE_ADMIN") {
            admins.extend(list.split(':').filter(|e| !e.is_empty()).map(str::to_string));
        }
        Options { admins }
    }
}

/// Une connexion apres son HELLO.
#[derive(Clone, Copy)]
struct Peer {
    app: u32,
    admin: bool,
}

/// Demarre le daemon sur le socket par defaut, dans le dossier choisi par
/// `resolve_root` (variable d'environnement, config, sinon par defaut).
pub fn start_daemon() -> Result<(), String> {
    start_daemon_at(&crate::SOCKET_PATH, &crate::resolve_root(&[])?)
}

/// Comme `start_daemon`, avec un autre socket et un autre dossier (tests).
/// Bloque tant que le daemon tourne.
pub fn start_daemon_at(socket: &str, root: &Path) -> Result<(), String> {
    start_daemon_with(socket, root, Options::default())
}

/// Comme `start_daemon_at`, avec des reglages (executables admin).
pub fn start_daemon_with(socket: &str, root: &Path, options: Options) -> Result<(), String> {
    let stockage = Arc::new(Mutex::new(AzureStockage::open(root)?));
    let listener = azure_core::daemon::bind(socket)?;
    let options = Arc::new(options);
    azure_core::daemon::serve(listener, move |stream| handle_connection(stream, Arc::clone(&stockage), Arc::clone(&options)));
    Ok(())
}

fn handle_connection(mut stream: UnixStream, stockage: Arc<Mutex<AzureStockage>>, options: Arc<Options>) {
    let mut app: Option<Peer> = None;
    // Transaction RsS ouverte par cette connexion (abandonnee si elle se ferme).
    let mut session = Session::new();
    // Une trame illisible ou une connexion fermee termine la connexion.
    while let Ok(request) = read_frame(&mut stream) {
        let response = response(handle_request(&request, &mut app, &mut session, &stream, &stockage, &options));
        if write_frame(&mut stream, &response).is_err() {
            break;
        }
    }
}

fn credentials<'a>(user: &'a str, password: &'a str) -> Option<Credentials<'a>> {
    (!user.is_empty()).then_some(Credentials { user, password })
}

fn write_results(mut out: Writer, results: &[RssResult]) -> Writer {
    out = out.u32(results.len() as u32);
    for result in results {
        out = result.columns.iter().fold(out.u32(result.columns.len() as u32), |o, c| o.str(c));
        out = out.u64(result.rows.len() as u64);
        for row in &result.rows {
            out = row.iter().fold(out, |o, v| v.write(o));
        }
        out = out.u64(result.affected);
    }
    out
}

fn read_rss<'a>(r: &mut Reader<'a>) -> Result<(String, Vec<Value>, Vec<Login>), String> {
    let sql = r.str()?;
    let params = (0..r.u32()?).map(|_| Value::read(r)).collect::<Result<Vec<_>, _>>()?;
    let logins = (0..r.u32()?).map(|_| Ok(Login { owner: r.u32()?, user: r.str()?, password: r.str()? })).collect::<Result<Vec<_>, String>>()?;
    Ok((sql, params, logins))
}

// Les apps a montrer a l'admin : celles d'azure-manager (ids a la suite a
// partir de 1000, avec des trous quand une app a ete oubliee) et celles
// qui se sont presentees sans lui.
fn known_apps(stockage: &AzureStockage) -> Result<Vec<(u32, String)>, String> {
    use azure_core::security::registry::{identify, FIRST_MANAGED_ID};
    let mut apps: Vec<(u32, String)> = stockage.bound_apps()?.into_iter().map(|(id, exe)| (id, exe.rsplit('/').next().unwrap_or("").to_string())).collect();
    let socket = manager_socket();
    let mut missing = 0;
    let mut id = FIRST_MANAGED_ID;
    while missing < 32 {
        match identify(&socket, id) {
            Ok(Some(identity)) => {
                missing = 0;
                apps.retain(|(known, _)| *known != id);
                apps.push((id, identity.name));
            }
            Ok(None) => missing += 1,
            Err(_) => break,
        }
        id += 1;
    }
    apps.sort();
    Ok(apps)
}

fn handle_request(request: &[u8], app: &mut Option<Peer>, session: &mut Session, stream: &UnixStream, stockage: &Mutex<AzureStockage>, options: &Options) -> Result<Vec<u8>, String> {
    let mut r = Reader::new(request);
    let opcode = r.u32()?;
    let mut stockage = stockage.lock().unwrap_or_else(|e| e.into_inner());

    if opcode == HELLO {
        let id = r.u32()?;
        r.finish()?;
        if app.is_some() {
            return Err("HELLO deja fait sur cette connexion".to_string());
        }
        // App connue d'azure-manager : c'est lui qui dit qui elle est (meme
        // empreinte) et si elle a droit au stockage. Sinon : premier
        // executable arrive.
        let exe = peer_exe(stream)?;
        let mut admin = options.admins.iter().any(|a| *a == exe);
        match azure_core::security::registry::verify(&manager_socket(), id, stream)? {
            Some(identity) if !identity.storage => {
                return Err(format!("L'app '{}' n'a pas la permission stockage ([permissions] stockage = false)", identity.name));
            }
            // Installee (empreinte verifiee par le manager) sous le nom reserve.
            Some(identity) => admin |= identity.installed && identity.name == ADMIN_APP,
            None => stockage.bind_app(id, &exe)?,
        }
        *app = Some(Peer { app: id, admin });
        return Ok(Vec::new());
    }
    let peer = app.ok_or("HELLO attendu avant toute requete")?;
    let app = peer.app;
    if (ADMIN_APPS..=ADMIN_GET).contains(&opcode) && !peer.admin {
        return Err("Reserve a Azure Data (app admin des donnees)".to_string());
    }

    let out = Writer::new();
    let out = match opcode {
        PUT => {
            let (key, value) = (r.str()?, r.bytes()?);
            r.finish()?;
            stockage.put(app, &key, value)?;
            out
        }
        GET => {
            let key = r.str()?;
            r.finish()?;
            match stockage.get(app, &key)? {
                Some(value) => out.u8(1).bytes(&value),
                None => out.u8(0),
            }
        }
        DELETE => {
            let key = r.str()?;
            r.finish()?;
            out.u8(stockage.delete(app, &key)? as u8)
        }
        KEYS => {
            r.finish()?;
            let keys = stockage.keys(app)?;
            keys.iter().fold(out.u32(keys.len() as u32), |out, key| out.str(key))
        }
        SET_LOCATION => {
            let place = r.str()?;
            r.finish()?;
            stockage.set_location(app, (!place.is_empty()).then(|| Path::new(&place)))?;
            out
        }
        LOCATION => {
            r.finish()?;
            out.str(&stockage.location(app).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default())
        }
        ADD_ACCOUNT => {
            let (user, password, role) = (r.str()?, r.str()?, r.u32()?);
            r.finish()?;
            stockage.add_account(app, &user, &password, Role::from_code(role).ok_or("Role inconnu")?)?;
            out
        }
        REMOVE_ACCOUNT => {
            let user = r.str()?;
            r.finish()?;
            out.u8(stockage.remove_account(app, &user)? as u8)
        }
        ACCOUNTS => {
            r.finish()?;
            let accounts = stockage.accounts(app)?;
            accounts.iter().fold(out.u32(accounts.len() as u32), |out, (user, role)| out.str(user).u32(role.code()))
        }
        SHARE => {
            let (name, value, access) = (r.str()?, r.bytes()?, r.u32()?);
            r.finish()?;
            stockage.share(app, &name, value, ShareAccess::from_code(access).ok_or("Acces inconnu")?)?;
            out
        }
        UNSHARE => {
            let name = r.str()?;
            r.finish()?;
            out.u8(stockage.unshare(app, &name)? as u8)
        }
        READ_SHARED => {
            let (owner, name, user, password) = (r.u32()?, r.str()?, r.str()?, r.str()?);
            r.finish()?;
            out.bytes(&stockage.read_shared(app, owner, &name, credentials(&user, &password))?)
        }
        WRITE_SHARED => {
            let (owner, name, value, user, password) = (r.u32()?, r.str()?, r.bytes()?, r.str()?, r.str()?);
            r.finish()?;
            stockage.write_shared(app, owner, &name, value, credentials(&user, &password))?;
            out
        }
        SHARED_LIST => {
            let owner = r.u32()?;
            r.finish()?;
            let list = stockage.shared_list(owner)?;
            list.iter().fold(out.u32(list.len() as u32), |out, info| out.str(&info.name).u32(info.access.code()))
        }
        RSS => {
            let (sql, params, logins) = read_rss(&mut r)?;
            r.finish()?;
            write_results(out, &execute(&mut *stockage, session, app, &sql, &params, &logins)?)
        }
        ADMIN_APPS => {
            r.finish()?;
            let apps = known_apps(&stockage)?;
            let mut out = out.u32(apps.len() as u32);
            for (id, name) in apps {
                out = out.u32(id).str(&name).u32(stockage.rss_catalog(id)?.len() as u32).u32(stockage.keys(id)?.len() as u32);
            }
            out
        }
        ADMIN_RSS => {
            let owner = r.u32()?;
            let (sql, params, logins) = read_rss(&mut r)?;
            r.finish()?;
            write_results(out, &execute(&mut *stockage, session, owner, &sql, &params, &logins)?)
        }
        ADMIN_SCHEMA => {
            let owner = r.u32()?;
            r.finish()?;
            let names = stockage.rss_catalog(owner)?;
            let mut out = out.u32(names.len() as u32);
            for name in &names {
                let table = stockage.rss_table(owner, name)?.ok_or_else(|| format!("Table '{name}' introuvable"))?;
                out = out.str(name).u32(table.share.map_or(0, |a| a.code() + 1)).u64(table.rows.len() as u64).u32(table.columns.len() as u32);
                for c in &table.columns {
                    let flags = c.primary as u8 | (c.unique as u8) << 1 | (c.not_null as u8) << 2;
                    out = out.str(&c.name).u8(c.ty.code()).u8(flags);
                    out = match &c.default {
                        Some(v) => v.write(out.u8(1)),
                        None => out.u8(0),
                    };
                }
                out = table.indexes.iter().fold(out.u32(table.indexes.len() as u32), |o, i| o.str(&i.name).str(&i.column).u8(i.unique as u8));
            }
            out
        }
        ADMIN_KEYS => {
            let owner = r.u32()?;
            r.finish()?;
            let keys = stockage.keys(owner)?;
            let mut out = out.u32(keys.len() as u32);
            for key in &keys {
                out = out.str(key).u64(stockage.get(owner, key)?.map_or(0, |v| v.len() as u64));
            }
            out
        }
        ADMIN_GET => {
            let (owner, key) = (r.u32()?, r.str()?);
            r.finish()?;
            match stockage.get(owner, &key)? {
                Some(value) => out.u8(1).bytes(&value),
                None => out.u8(0),
            }
        }
        other => return Err(format!("Opcode inconnu : {other}")),
    };
    Ok(out.finish())
}
