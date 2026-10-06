// Ce qu'une app utilise pour parler au daemon de stockage. Chaque methode
// envoie une requete et attend sa reponse.
use crate::managers::stockage::{Credentials, SharedInfo};
use crate::models::request::*;
use crate::models::wire::{Reader, Writer};
use crate::rss::engine::{Login, RssResult};
use crate::rss::ast::ColumnDef;
use crate::rss::table::IndexDef;
use crate::rss::value::{DataType, Value};
use azure_core::models::storage_model::{Role, ShareAccess};
use std::os::unix::net::UnixStream;

/// Une app vue par l'admin des donnees.
#[derive(Debug, Clone, PartialEq)]
pub struct AdminApp {
    pub id: u32,
    pub name: String,
    pub tables: u32,
    pub keys: u32,
}

/// Une table vue par l'admin des donnees.
#[derive(Debug, Clone, PartialEq)]
pub struct AdminTable {
    pub name: String,
    pub share: Option<ShareAccess>,
    pub rows: u64,
    pub columns: Vec<ColumnDef>,
    pub indexes: Vec<IndexDef>,
}

pub struct StockageClient {
    stream: UnixStream,
    app_id: u32,
}

fn creds(credentials: Option<Credentials<'_>>) -> (&str, &str) {
    credentials.map_or(("", ""), |c| (c.user, c.password))
}

impl StockageClient {
    /// Se connecte au daemon par defaut en tant qu'app `app_id`. Refuse si
    /// cet id appartient deja a un autre executable.
    pub fn connect(app_id: u32) -> Result<StockageClient, String> {
        StockageClient::connect_at(&crate::SOCKET_PATH, app_id)
    }

    pub fn connect_at(socket: &str, app_id: u32) -> Result<StockageClient, String> {
        let stream = UnixStream::connect(socket).map_err(|e| format!("{socket} : {e}"))?;
        let mut client = StockageClient { stream, app_id };
        client.call(Writer::new().u32(HELLO).u32(app_id), |_| Ok(()))?;
        Ok(client)
    }

    pub fn app_id(&self) -> u32 {
        self.app_id
    }

    fn call<T>(&mut self, request: Writer, parse: impl FnOnce(&mut Reader) -> Result<T, String>) -> Result<T, String> {
        write_frame(&mut self.stream, &request.finish())?;
        let response = read_frame(&mut self.stream)?;
        let mut reader = check_status(&response)?;
        let value = parse(&mut reader)?;
        reader.finish()?;
        Ok(value)
    }

    // ---- Prive ----

    pub fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        self.call(Writer::new().u32(PUT).str(key).bytes(value), |_| Ok(()))
    }

    pub fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, String> {
        self.call(Writer::new().u32(GET).str(key), |r| Ok(if r.u8()? == 1 { Some(r.bytes()?.to_vec()) } else { None }))
    }

    /// Raccourci pour du texte.
    pub fn put_text(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.put(key, value.as_bytes())
    }

    pub fn get_text(&mut self, key: &str) -> Result<Option<String>, String> {
        self.get(key)?.map(|v| String::from_utf8(v).map_err(|_| format!("'{key}' n'est pas du texte UTF-8"))).transpose()
    }

    pub fn delete(&mut self, key: &str) -> Result<bool, String> {
        self.call(Writer::new().u32(DELETE).str(key), |r| Ok(r.u8()? == 1))
    }

    pub fn keys(&mut self) -> Result<Vec<String>, String> {
        self.call(Writer::new().u32(KEYS), |r| (0..r.u32()?).map(|_| r.str()).collect())
    }

    /// Deplace l'espace prive de cette app dans `place` (chemin absolu) ;
    /// `None` = retour a l'endroit par defaut.
    pub fn set_location(&mut self, place: Option<&str>) -> Result<(), String> {
        self.call(Writer::new().u32(SET_LOCATION).str(place.unwrap_or("")), |_| Ok(()))
    }

    /// Ou vit l'espace prive de cette app, `None` = endroit par defaut.
    pub fn location(&mut self) -> Result<Option<String>, String> {
        self.call(Writer::new().u32(LOCATION), |r| Ok(Some(r.str()?).filter(|p| !p.is_empty())))
    }

    /// Execute du RsS (une ou plusieurs instructions) : un resultat par
    /// instruction. `logins` : comptes pour les tables protegees des autres apps.
    pub fn rss(&mut self, sql: &str, params: &[Value], logins: &[Login]) -> Result<Vec<RssResult>, String> {
        self.rss_request(Writer::new().u32(RSS), sql, params, logins)
    }

    fn rss_request(&mut self, request: Writer, sql: &str, params: &[Value], logins: &[Login]) -> Result<Vec<RssResult>, String> {
        let mut request = params.iter().fold(request.str(sql).u32(params.len() as u32), |w, v| v.write(w));
        request = logins.iter().fold(request.u32(logins.len() as u32), |w, l| w.u32(l.owner).str(&l.user).str(&l.password));
        self.call(request, |r| {
            (0..r.u32()?)
                .map(|_| {
                    let columns = (0..r.u32()?).map(|_| r.str()).collect::<Result<Vec<_>, _>>()?;
                    let rows = (0..r.u64()?)
                        .map(|_| (0..columns.len()).map(|_| Value::read(r)).collect::<Result<Vec<_>, _>>())
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(RssResult { columns, rows, affected: r.u64()? })
                })
                .collect()
        })
    }

    // ---- Admin des donnees (Azure Data ; refuse aux autres apps) ----

    /// Les apps qui ont un espace de stockage.
    pub fn admin_apps(&mut self) -> Result<Vec<AdminApp>, String> {
        self.call(Writer::new().u32(ADMIN_APPS), |r| (0..r.u32()?).map(|_| Ok(AdminApp { id: r.u32()?, name: r.str()?, tables: r.u32()?, keys: r.u32()? })).collect())
    }

    /// Du RsS au nom de l'app `owner` : ses tables privees, comme elle.
    pub fn admin_rss(&mut self, owner: u32, sql: &str, params: &[Value]) -> Result<Vec<RssResult>, String> {
        self.rss_request(Writer::new().u32(ADMIN_RSS).u32(owner), sql, params, &[])
    }

    /// Les tables de l'app `owner`.
    pub fn admin_schema(&mut self, owner: u32) -> Result<Vec<AdminTable>, String> {
        self.call(Writer::new().u32(ADMIN_SCHEMA).u32(owner), |r| {
            (0..r.u32()?)
                .map(|_| {
                    let name = r.str()?;
                    let share = match r.u32()? {
                        0 => None,
                        code => Some(ShareAccess::from_code(code - 1).ok_or("Acces inconnu")?),
                    };
                    let rows = r.u64()?;
                    let mut columns = Vec::new();
                    for _ in 0..r.u32()? {
                        let col_name = r.str()?;
                        let ty = DataType::from_code(r.u8()?).ok_or("Type de colonne inconnu")?;
                        let flags = r.u8()?;
                        let default = if r.u8()? == 1 { Some(Value::read(r)?) } else { None };
                        columns.push(ColumnDef { name: col_name, ty, primary: flags & 1 != 0, unique: flags & 2 != 0, not_null: flags & 4 != 0, default });
                    }
                    let indexes = (0..r.u32()?).map(|_| Ok(IndexDef { name: r.str()?, column: r.str()?, unique: r.u8()? != 0 })).collect::<Result<Vec<_>, String>>()?;
                    Ok(AdminTable { name, share, rows, columns, indexes })
                })
                .collect()
        })
    }

    /// Les cles privees de l'app `owner` et la taille de leur valeur.
    pub fn admin_keys(&mut self, owner: u32) -> Result<Vec<(String, u64)>, String> {
        self.call(Writer::new().u32(ADMIN_KEYS).u32(owner), |r| (0..r.u32()?).map(|_| Ok((r.str()?, r.u64()?))).collect())
    }

    pub fn admin_get(&mut self, owner: u32, key: &str) -> Result<Option<Vec<u8>>, String> {
        self.call(Writer::new().u32(ADMIN_GET).u32(owner).str(key), |r| Ok(if r.u8()? == 1 { Some(r.bytes()?.to_vec()) } else { None }))
    }

    // ---- Comptes ----

    pub fn add_account(&mut self, user: &str, password: &str, role: Role) -> Result<(), String> {
        self.call(Writer::new().u32(ADD_ACCOUNT).str(user).str(password).u32(role.code()), |_| Ok(()))
    }

    pub fn remove_account(&mut self, user: &str) -> Result<bool, String> {
        self.call(Writer::new().u32(REMOVE_ACCOUNT).str(user), |r| Ok(r.u8()? == 1))
    }

    pub fn accounts(&mut self) -> Result<Vec<(String, Role)>, String> {
        self.call(Writer::new().u32(ACCOUNTS), |r| {
            (0..r.u32()?).map(|_| Ok((r.str()?, Role::from_code(r.u32()?).ok_or("Role inconnu")?))).collect()
        })
    }

    // ---- Partage ----

    pub fn share(&mut self, name: &str, value: &[u8], access: ShareAccess) -> Result<(), String> {
        self.call(Writer::new().u32(SHARE).str(name).bytes(value).u32(access.code()), |_| Ok(()))
    }

    pub fn unshare(&mut self, name: &str) -> Result<bool, String> {
        self.call(Writer::new().u32(UNSHARE).str(name), |r| Ok(r.u8()? == 1))
    }

    pub fn read_shared(&mut self, owner: u32, name: &str, credentials: Option<Credentials>) -> Result<Vec<u8>, String> {
        let (user, password) = creds(credentials);
        self.call(Writer::new().u32(READ_SHARED).u32(owner).str(name).str(user).str(password), |r| Ok(r.bytes()?.to_vec()))
    }

    pub fn write_shared(&mut self, owner: u32, name: &str, value: &[u8], credentials: Option<Credentials>) -> Result<(), String> {
        let (user, password) = creds(credentials);
        self.call(Writer::new().u32(WRITE_SHARED).u32(owner).str(name).bytes(value).str(user).str(password), |_| Ok(()))
    }

    pub fn shared_list(&mut self, owner: u32) -> Result<Vec<SharedInfo>, String> {
        self.call(Writer::new().u32(SHARED_LIST).u32(owner), |r| {
            (0..r.u32()?)
                .map(|_| Ok(SharedInfo { name: r.str()?, access: ShareAccess::from_code(r.u32()?).ok_or("Acces inconnu")? }))
                .collect()
        })
    }
}
