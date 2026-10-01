// Routeur inter-app : les apps s'y enregistrent, s'envoient des messages,
// s'abonnent a des evenements ou a d'autres apps, et partagent des fenetres.
// Opcodes et format : voir `models::request`.
use crate::models::mailbox::Mailbox;
use crate::models::request::*;
use azure_core::models::frame::write_frame;
use azure_core::models::window_model::WindowScope;
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex, MutexGuard};

/// Etat partage par toutes les connexions (un thread chacune).
#[derive(Default)]
struct Router {
    /// App enregistree -> la connexion ou lui livrer ses messages.
    connections: Mutex<HashMap<u32, UnixStream>>,
    /// Evenement -> apps abonnees.
    subscriptions: Mutex<HashMap<String, Vec<u32>>>,
    /// Qui suit qui : app suivie -> ses abonnes (FOLLOW/UNFOLLOW), utilise
    /// pour distribuer les fenetres `WindowScope::Followers`.
    follows: Mutex<HashMap<u32, Vec<u32>>>,
    /// Messages pour les apps qui ne tournent pas (voir `models::mailbox`).
    mailbox: Mutex<Mailbox>,
}

/// Un thread qui a plante en tenant un verrou ne bloque pas les autres.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Demarre le routeur sur le chemin par defaut (voir `crate::SOCKET_PATH`) -
/// c'est ce que `bin/routeur_daemon` appelle. Voir `start_router_at` pour un
/// autre chemin (tests d'integration notamment).
pub fn start_router() -> Result<(), String> {
    start_router_with(&crate::SOCKET_PATH, Some(&crate::default_data_dir()))
}

/// Messages en attente gardes en memoire seulement (tests).
pub fn start_router_at(path: &str) -> Result<(), String> {
    start_router_with(path, None)
}

/// `data` : dossier ou garder les messages destines aux apps qui ne
/// tournent pas (voir `models::mailbox`) ; `None` = en memoire.
pub fn start_router_with(path: &str, data: Option<&std::path::Path>) -> Result<(), String> {
    let mailbox = match data {
        Some(dir) => Mailbox::open(dir)?,
        None => Mailbox::in_memory(),
    };
    // Un routeur qui repond deja garde son socket : un second demarrage
    // echoue au lieu de le lui voler.
    let listener = azure_core::daemon::bind(path)?;
    let router = Arc::new(Router { mailbox: Mutex::new(mailbox), ..Router::default() });
    // Chaque connexion tourne dans son propre thread (voir `daemon::serve`) :
    // App A et App B sont lues en parallele sans se bloquer.
    azure_core::daemon::serve(listener, move |connection| {
        if let Err(e) = handle_connection(connection, &router) {
            println!("Erreur sur une connexion: {}", e);
        }
    });
    Ok(())
}

static MANAGER_SOCKET: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Socket d'azure-manager a interroger sur l'identite des apps.
pub fn set_manager_socket(socket: &str) {
    let _ = MANAGER_SOCKET.set(socket.to_string());
}

fn manager_socket() -> String {
    MANAGER_SOCKET.get().cloned().unwrap_or_else(|| azure_core::security::registry::MANAGER_SOCKET.to_string())
}

static OWNERS: std::sync::OnceLock<Mutex<HashMap<u32, String>>> = std::sync::OnceLock::new();

fn claim_id(app_id: u32, connection: &UnixStream) -> Result<(), String> {
    let exe = azure_core::managers::identity::peer_exe(connection)?;
    let mut owners = lock(OWNERS.get_or_init(|| Mutex::new(HashMap::new())));
    match owners.get(&app_id) {
        Some(owner) if *owner != exe => Err(format!("l'id {app_id} appartient deja a {owner}")),
        _ => {
            owners.insert(app_id, exe);
            Ok(())
        }
    }
}

/// Identifiant du socket (meme valeur pour une connexion et ses clones).
fn socket_id(stream: &UnixStream) -> Option<std::path::PathBuf> {
    use std::os::fd::AsRawFd;
    std::fs::read_link(format!("/proc/self/fd/{}", stream.as_raw_fd())).ok()
}

fn handle_connection(connection: UnixStream, router: &Router) -> Result<(), String> {
    let id = socket_id(&connection);
    let mut session = Session { router, connection: connection.try_clone().map_err(|e| e.to_string())?, registered: None };
    let result = session.run();
    // Fin de la connexion (depart, erreur, client fautif) : on la ferme pour
    // de bon, et l'app n'est plus joignable par elle (ses messages iront dans
    // la boite aux lettres) - sauf si elle s'est deja reconnectee ailleurs.
    let _ = connection.shutdown(std::net::Shutdown::Both);
    if let Some(app_id) = session.registered {
        let mut map = lock(&router.connections);
        if map.get(&app_id).is_some_and(|stored| socket_id(stored) == id) {
            map.remove(&app_id);
        }
    }
    result
}

/// Une connexion d'app.
struct Session<'a> {
    router: &'a Router,
    connection: UnixStream,
    /// L'app que CETTE connexion represente (apres REGISTER) : elle seule
    /// peut s'abonner, suivre ou se desinscrire en son nom.
    registered: Option<u32>,
}

impl Session<'_> {
    /// Lit les requetes jusqu'a la fermeture de la connexion (ou UNREGISTER).
    /// Une requete tronquee ou demesuree coupe la connexion (`Err`).
    fn run(&mut self) -> Result<(), String> {
        // Plus d'opcode : l'app a ferme la connexion.
        while let Ok(opcode) = read_u32(&mut self.connection) {
            match opcode {
                REGISTER => {
                    let app_id = read_u32(&mut self.connection)?;
                    self.register(app_id)?;
                }
                SEND => {
                    let _sender_id = read_u32(&mut self.connection)?;
                    let receiver_id = read_u32(&mut self.connection)?;
                    let content = read_str(&mut self.connection)?;
                    self.send(receiver_id, content);
                }
                SUBSCRIBE => {
                    let app_id = read_u32(&mut self.connection)?;
                    let event = read_str(&mut self.connection)?;
                    self.subscribe(app_id, event);
                }
                PUBLISH => {
                    let _app_id = read_u32(&mut self.connection)?;
                    let event = read_str(&mut self.connection)?;
                    let content = read_str(&mut self.connection)?;
                    self.publish(&event, &content);
                }
                UNSUBSCRIBE => {
                    let app_id = read_u32(&mut self.connection)?;
                    let event = read_str(&mut self.connection)?;
                    self.unsubscribe(app_id, &event);
                }
                UNREGISTER => {
                    let app_id = read_u32(&mut self.connection)?;
                    // Seulement soi-meme (pas une autre app).
                    if self.registered == Some(app_id) {
                        self.unregister(app_id);
                        break;
                    }
                }
                FOLLOW => {
                    let follower_id = read_u32(&mut self.connection)?;
                    let followed_id = read_u32(&mut self.connection)?;
                    self.follow(follower_id, followed_id);
                }
                UNFOLLOW => {
                    let follower_id = read_u32(&mut self.connection)?;
                    let followed_id = read_u32(&mut self.connection)?;
                    self.unfollow(follower_id, followed_id);
                }
                SEND_WINDOW => {
                    let sender_id = read_u32(&mut self.connection)?;
                    let scope_code = read_u32(&mut self.connection)?;
                    let content = read_bytes(&mut self.connection)?;
                    self.send_window(sender_id, scope_code, &content);
                }
                _ => println!("Opcode inconnu: {}", opcode),
            }
        }
        Ok(())
    }

    fn owns(&self, app_id: u32) -> bool {
        self.registered.is_none_or(|r| r == app_id)
    }

    fn register(&mut self, app_id: u32) -> Result<(), String> {
        // App connue d'azure-manager : ce doit etre elle (sinon un processus
        // pourrait recevoir les messages d'une autre). Id hors manager (ou
        // manager absent) : le premier executable arrive garde l'id pour la
        // vie du routeur.
        let checked = azure_core::security::registry::verify(&manager_socket(), app_id, &self.connection).and_then(|_| claim_id(app_id, &self.connection));
        if let Err(e) = checked {
            eprintln!("routeur : enregistrement refuse : {e}");
            return Err(e);
        }
        self.registered = Some(app_id);

        // Une COPIE de la connexion pour livrer les messages, l'originale
        // continuant d'etre lue par cette session.
        let mut stored = self.connection.try_clone().map_err(|e| e.to_string())?;
        // Un destinataire qui ne lit plus ne bloque pas les autres.
        let _ = stored.set_write_timeout(Some(std::time::Duration::from_secs(2)));
        let mut map = lock(&self.router.connections);
        // Messages arrives pendant son absence : livres tout de suite.
        for content in lock(&self.router.mailbox).take(app_id) {
            write_frame(&mut stored, content.as_bytes())?;
        }
        map.insert(app_id, stored);
        println!("App {} enregistrée", app_id);
        Ok(())
    }

    fn send(&self, receiver_id: u32, content: String) {
        let mut map = lock(&self.router.connections);
        match map.get_mut(&receiver_id) {
            Some(conn) => {
                // Destinataire parti entre-temps : le message l'attendra.
                if write_frame(conn, content.as_bytes()).is_err() {
                    map.remove(&receiver_id);
                    lock(&self.router.mailbox).push(receiver_id, content);
                }
            }
            // Pas la : garde pour son prochain lancement.
            None => lock(&self.router.mailbox).push(receiver_id, content),
        }
    }

    fn subscribe(&self, app_id: u32, event: String) {
        let known = lock(&self.router.connections).contains_key(&app_id);
        if known && self.owns(app_id) {
            println!("App {} abonnée à '{}'", app_id, event);
            lock(&self.router.subscriptions).entry(event).or_default().push(app_id);
        } else {
            println!("App non enregistrée");
        }
    }

    fn publish(&self, event: &str, content: &str) {
        let subscribers = lock(&self.router.subscriptions).get(event).cloned().unwrap_or_default();
        let mut map = lock(&self.router.connections);
        for app_id in &subscribers {
            if let Some(conn) = map.get_mut(app_id) {
                // Un abonne mort ne coupe pas l'expediteur.
                let _ = write_frame(conn, content.as_bytes());
            }
        }
    }

    fn unsubscribe(&self, app_id: u32, event: &str) {
        if !self.owns(app_id) {
            return;
        }
        if let Some(app_ids) = lock(&self.router.subscriptions).get_mut(event) {
            app_ids.retain(|&id| id != app_id);
        }
    }

    fn unregister(&self, app_id: u32) {
        lock(&self.router.connections).remove(&app_id);
        for app_ids in lock(&self.router.subscriptions).values_mut() {
            app_ids.retain(|&id| id != app_id);
        }
        let mut follows = lock(&self.router.follows);
        follows.remove(&app_id);
        for followers in follows.values_mut() {
            followers.retain(|&id| id != app_id);
        }
    }

    fn follow(&self, follower_id: u32, followed_id: u32) {
        let known = lock(&self.router.connections).contains_key(&follower_id);
        if known && self.owns(follower_id) && follower_id != followed_id {
            let mut follows = lock(&self.router.follows);
            let followers = follows.entry(followed_id).or_default();
            if !followers.contains(&follower_id) {
                followers.push(follower_id);
            }
            println!("App {} suit l'app {}", follower_id, followed_id);
        } else {
            println!("Follow refuse ({} -> {})", follower_id, followed_id);
        }
    }

    fn unfollow(&self, follower_id: u32, followed_id: u32) {
        if !self.owns(follower_id) {
            return;
        }
        if let Some(followers) = lock(&self.router.follows).get_mut(&followed_id) {
            followers.retain(|&id| id != follower_id);
        }
    }

    fn send_window(&self, sender_id: u32, scope_code: u32, content: &[u8]) {
        if !self.owns(sender_id) {
            return;
        }
        // Destinataires selon le scope ; l'expediteur n'est jamais inclus (il
        // a deja sa fenetre, voir WindowKind::InterApp).
        let mut map = lock(&self.router.connections);
        let receivers: Vec<u32> = match WindowScope::from_code(scope_code) {
            Some(WindowScope::Followers) => lock(&self.router.follows).get(&sender_id).cloned().unwrap_or_default(),
            Some(WindowScope::All) => map.keys().copied().filter(|&id| id != sender_id).collect(),
            _ => {
                println!("Fenetre de l'app {} refusee : scope {} invalide", sender_id, scope_code);
                Vec::new()
            }
        };
        for receiver_id in receivers {
            // Un destinataire mort ne doit pas couper la connexion de
            // l'expediteur : on l'ignore.
            if let Some(conn) = map.get_mut(&receiver_id)
                && write_frame(conn, content).is_err()
            {
                println!("Fenetre non livree a l'app {}", receiver_id);
            }
        }
    }
}
