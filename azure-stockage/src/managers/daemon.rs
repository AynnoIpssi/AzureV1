// Le daemon de stockage : SEUL process qui touche au disque (voir
// `AzureStockage`). Une connexion = une app : elle commence par HELLO avec
// son id, verifie contre son executable reel (voir `identity`), et toutes
// ses requetes suivantes agissent en son nom.
use crate::managers::identity::peer_exe;
use crate::managers::stockage::{AzureStockage, Credentials};
use crate::models::request::*;
use crate::models::wire::{Reader, Writer};
use crate::rss::engine::{execute, Login, Session};
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

/// Demarre le daemon sur le socket par defaut, dans le dossier choisi par
/// `resolve_root` (variable d'environnement, config, sinon par defaut).
pub fn start_daemon() -> Result<(), String> {
    start_daemon_at(&crate::SOCKET_PATH, &crate::resolve_root(&[])?)
}

/// Comme `start_daemon`, avec un autre socket et un autre dossier (tests).
/// Bloque tant que le daemon tourne.
pub fn start_daemon_at(socket: &str, root: &Path) -> Result<(), String> {
    let stockage = Arc::new(Mutex::new(AzureStockage::open(root)?));
    let listener = azure_core::daemon::bind(socket)?;
    azure_core::daemon::serve(listener, move |stream| handle_connection(stream, Arc::clone(&stockage)));
    Ok(())
}

fn handle_connection(mut stream: UnixStream, stockage: Arc<Mutex<AzureStockage>>) {
    let mut app: Option<u32> = None;
    // Transaction RsS ouverte par cette connexion (abandonnee si elle se ferme).
    let mut session = Session::new();
    // Une trame illisible ou une connexion fermee termine la connexion.
    while let Ok(request) = read_frame(&mut stream) {
        let response = response(handle_request(&request, &mut app, &mut session, &stream, &stockage));
        if write_frame(&mut stream, &response).is_err() {
            break;
        }
    }
}

fn credentials<'a>(user: &'a str, password: &'a str) -> Option<Credentials<'a>> {
    (!user.is_empty()).then_some(Credentials { user, password })
}

fn handle_request(request: &[u8], app: &mut Option<u32>, session: &mut Session, stream: &UnixStream, stockage: &Mutex<AzureStockage>) -> Result<Vec<u8>, String> {
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
        match azure_core::security::registry::verify(&manager_socket(), id, stream)? {
            Some(identity) if !identity.storage => {
                return Err(format!("L'app '{}' n'a pas la permission stockage ([permissions] stockage = false)", identity.name));
            }
            Some(_) => {}
            None => stockage.bind_app(id, &peer_exe(stream)?)?,
        }
        *app = Some(id);
        return Ok(Vec::new());
    }
    let app = app.ok_or("HELLO attendu avant toute requete")?;

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
            let sql = r.str()?;
            let params = (0..r.u32()?).map(|_| Value::read(&mut r)).collect::<Result<Vec<_>, _>>()?;
            let logins = (0..r.u32()?).map(|_| Ok(Login { owner: r.u32()?, user: r.str()?, password: r.str()? })).collect::<Result<Vec<_>, String>>()?;
            r.finish()?;
            let results = execute(&mut stockage, session, app, &sql, &params, &logins)?;
            let mut out = out.u32(results.len() as u32);
            for result in &results {
                out = result.columns.iter().fold(out.u32(result.columns.len() as u32), |o, c| o.str(c));
                out = out.u64(result.rows.len() as u64);
                for row in &result.rows {
                    out = row.iter().fold(out, |o, v| v.write(o));
                }
                out = out.u64(result.affected);
            }
            out
        }
        other => return Err(format!("Opcode inconnu : {other}")),
    };
    Ok(out.finish())
}
