// Cote app (et tableau de bord) : parler a azure-manager.
use crate::managers::manager::Kind;
use crate::models::manifest::Summary;
use crate::models::request::*;
use azure_core::models::wire::Writer;
use azure_service::flux::{Access, Value};
use std::os::unix::net::UnixStream;

pub struct ManagerClient {
    stream: UnixStream,
}

impl ManagerClient {
    /// Se connecte au daemon par defaut, lance par azure-provider s'il ne
    /// tourne pas.
    pub fn connect() -> Result<ManagerClient, String> {
        let ensured = azure_provider::Provider::ensure("manager");
        ManagerClient::connect_at(&crate::SOCKET_PATH).map_err(|e| match ensured {
            Err(provider) => format!("{e} (azure-provider : {provider})"),
            Ok(()) => e,
        })
    }

    pub fn connect_at(socket: &str) -> Result<ManagerClient, String> {
        let stream = UnixStream::connect(socket).map_err(|e| format!("azure-manager injoignable ({socket}) : {e}"))?;
        Ok(ManagerClient { stream })
    }

    fn call(&mut self, request: Writer) -> Result<Vec<u8>, String> {
        write_frame(&mut self.stream, &request.finish())?;
        let response = read_frame(&mut self.stream)?;
        check_status(&response)?;
        Ok(response)
    }

    /// Presente l'app (son manifeste) ; retourne son id. Tant que cette
    /// connexion reste ouverte, l'app est vue comme active.
    pub fn register(&mut self, summary: &Summary) -> Result<u32, String> {
        let response = self.call(summary.write(Writer::new().u32(REGISTER)))?;
        check_status(&response)?.u32()
    }

    /// `azure install` : enregistre l'app installee ; retourne son id.
    pub fn install(&mut self, summary: &Summary, exe: &str, fingerprint: &[u8; 32]) -> Result<u32, String> {
        let response = self.call(summary.write(Writer::new().u32(INSTALL)).str(exe).bytes(fingerprint))?;
        check_status(&response)?.u32()
    }

    /// Id de l'app `name`.
    /// Lance la tache de fond qui sert `method` de `app` (fermee), si
    /// l'app qui parle a le droit de l'appeler. Revient quand elle tourne.
    pub fn wake(&mut self, app: &str, method: &str) -> Result<(), String> {
        self.call(Writer::new().u32(WAKE).str(app).str(method)).map(|_| ())
    }

    /// Ouvre l'app `app` (voir `OPEN`) : la lance si elle ne tourne pas,
    /// avec le jeton d'activation `activation` (vide : aucun). `true` : lancee.
    pub fn open(&mut self, app: &str, activation: &str) -> Result<bool, String> {
        let response = self.call(Writer::new().u32(OPEN).str(app).str(activation))?;
        Ok(check_status(&response)?.u8()? != 0)
    }

    pub fn resolve(&mut self, name: &str) -> Result<u32, String> {
        let response = self.call(Writer::new().u32(RESOLVE).str(name))?;
        check_status(&response)?.u32()
    }

    /// Qui peut ecouter mon flux / appeler ma methode `name`.
    pub fn access(&mut self, kind: Kind, name: &str) -> Result<Access, String> {
        let response = self.call(Writer::new().u32(ACCESS).u8(kind.code()).str(name))?;
        Access::read(&mut check_status(&response)?)
    }

    /// Etat complet (tableau de bord et ligne de commande seulement).
    pub fn state(&mut self) -> Result<Value, String> {
        let response = self.call(Writer::new().u32(STATE))?;
        Value::read(&mut check_status(&response)?)
    }

    /// Autorise `app` a ecouter le flux / appeler la methode `name` de `owner`.
    pub fn grant(&mut self, kind: Kind, owner: &str, name: &str, app: &str) -> Result<(), String> {
        self.call(Writer::new().u32(GRANT).u8(kind.code()).str(owner).str(name).str(app)).map(|_| ())
    }

    pub fn revoke(&mut self, kind: Kind, owner: &str, name: &str, app: &str) -> Result<(), String> {
        self.call(Writer::new().u32(REVOKE).u8(kind.code()).str(owner).str(name).str(app)).map(|_| ())
    }

    pub fn set_public(&mut self, kind: Kind, owner: &str, name: &str, public: bool) -> Result<(), String> {
        self.call(Writer::new().u32(SET_PUBLIC).u8(kind.code()).str(owner).str(name).u8(public as u8)).map(|_| ())
    }

    /// Revient a ce que declare le manifeste.
    pub fn reset(&mut self, kind: Kind, owner: &str, name: &str) -> Result<(), String> {
        self.call(Writer::new().u32(RESET_ACCESS).u8(kind.code()).str(owner).str(name)).map(|_| ())
    }

    pub fn forget(&mut self, name: &str) -> Result<(), String> {
        self.call(Writer::new().u32(FORGET).str(name)).map(|_| ())
    }

    /// Signale un evenement de l'app `name` (qui doit etre celle qui parle).
    pub fn report(&mut self, name: &str, level: crate::managers::manager::Level, message: &str) -> Result<(), String> {
        self.call(Writer::new().u32(REPORT).str(name).u8(level.code()).str(message)).map(|_| ())
    }

    /// Les `lines` dernieres lignes du journal d'un service ou d'une app.
    pub fn logs(&mut self, name: &str, lines: u32) -> Result<Vec<String>, String> {
        let response = self.call(Writer::new().u32(LOGS).str(name).u32(lines))?;
        let mut r = check_status(&response)?;
        (0..r.u32()?).map(|_| r.str()).collect()
    }

    /// Lance `azure <args>` (terminal du tableau de bord) : code de sortie
    /// et lignes affichees.
    pub fn azure_command(&mut self, args: &[String]) -> Result<(i32, Vec<String>), String> {
        let request = args.iter().fold(Writer::new().u32(AZURE_COMMAND).u32(args.len() as u32), |w, a| w.str(a));
        let response = self.call(request)?;
        let mut r = check_status(&response)?;
        let code = r.u32()? as i32;
        let lines = (0..r.u32()?).map(|_| r.str()).collect::<Result<Vec<_>, _>>()?;
        Ok((code, lines))
    }

    pub fn restart_service(&mut self, name: &str) -> Result<(), String> {
        self.call(Writer::new().u32(RESTART_SERVICE).str(name)).map(|_| ())
    }
}
