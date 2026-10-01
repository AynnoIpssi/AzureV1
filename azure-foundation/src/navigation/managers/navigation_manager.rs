// Seul endroit du module `navigation` qui parle a `azure_rooter` -
// `NavigationClient` (voir `models::navigation_client`) transporte la
// connexion socket, chaque fonction ici l'emprunte et delegue directement a
// `azure_rooter::services::client`, sans logique en plus : ce fichier existe
// pour que les apps foundation n'aient jamais a importer `azure_rooter`
// elles-memes (voir `mod.rs`), pas pour changer le protocole du routeur.
use crate::navigation::models::navigation_client::NavigationClient;
use crate::navigation::models::incoming::Incoming;
use crate::navigation::models::route::Route;
use crate::window::models::shared_window::SharedWindow;
use azure_core::models::window_model::WindowKind;
use azure_rooter::services::client;
use std::sync::mpsc;
use std::thread;

/// Enregistre cette app aupres du routeur par defaut (voir
/// `azure_rooter::SOCKET_PATH`) sous `app_id` (doit etre unique parmi les
/// apps connectees en meme temps - le routeur ne verifie pas les collisions,
/// voir `azure_rooter::managers::router`) et retourne la poignee a
/// reutiliser pour tous les appels suivants. Voir `connect_at` pour un autre
/// chemin (tests d'integration notamment). Le routeur est lance par
/// azure-provider s'il ne tourne pas (voir `crate::provider`).
pub fn connect(app_id: u32) -> Result<NavigationClient, String> {
    crate::provider::with_service("rooter", || connect_at(&azure_rooter::SOCKET_PATH, app_id))
}

pub fn connect_at(path: &str, app_id: u32) -> Result<NavigationClient, String> {
    let connection = client::register_at(path, app_id)?;
    Ok(NavigationClient { router_id: app_id, connection })
}

/// Envoie `content` a l'app `receiver_id`, silencieusement perdu si elle
/// n'est pas (ou plus) enregistree aupres du routeur.
pub fn send(nav: &mut NavigationClient, receiver_id: u32, content: &str) -> Result<(), String> {
    client::send(&mut nav.connection, nav.router_id, receiver_id, content)
}

/// Abonne cette app a l'evenement `event` - `publish` sur ce meme nom par
/// n'importe quelle app lui sera ensuite relaye (voir `receive`).
pub fn subscribe(nav: &mut NavigationClient, event: &str) -> Result<(), String> {
    client::subscribe(&mut nav.connection, nav.router_id, event)
}

/// Publie `content` sous l'evenement `event` a tous les abonnes actuels
/// (voir `subscribe`) - sans effet si personne n'y est abonne.
pub fn publish(nav: &mut NavigationClient, event: &str, content: &str) -> Result<(), String> {
    client::publish(&mut nav.connection, nav.router_id, event, content)
}

/// Bloque jusqu'a recevoir le prochain message (envoye via `send` ou
/// `publish` sur un evenement suivi) destine a cette app.
pub fn receive(nav: &mut NavigationClient) -> Result<String, String> {
    client::receive(&mut nav.connection)
}

/// Cette app s'abonne a l'app `app_id` : elle recevra ses fenetres
/// `WindowScope::Followers` (voir `send_window`), qui s'ouvriront toutes
/// seules si une `AzureWindow` de cette app est branchee au routeur.
pub fn follow(nav: &mut NavigationClient, app_id: u32) -> Result<(), String> {
    client::follow(&mut nav.connection, nav.router_id, app_id)
}

pub fn unfollow(nav: &mut NavigationClient, app_id: u32) -> Result<(), String> {
    client::unfollow(&mut nav.connection, nav.router_id, app_id)
}

/// Envoie `window` aux apps de son scope : les abonnes de cette app
/// (`Followers`) ou toutes les apps (`All`). La fenetre doit etre externe
/// ou inter-app et creee par CETTE app (`router_id`). Ne l'ouvre pas chez
/// l'expediteur : voir `WindowContext::send_window` pour ca.
pub fn send_window(nav: &mut NavigationClient, window: &SharedWindow) -> Result<(), String> {
    let spec = &window.spec;
    if spec.kind() == WindowKind::Internal {
        return Err("Une fenetre interne ne peut pas etre envoyee a une autre app".to_string());
    }
    if spec.owner_app_id() != nav.router_id {
        return Err(format!("Cette fenetre appartient a l'app {}, pas a l'app {}", spec.owner_app_id(), nav.router_id));
    }
    client::send_window(&mut nav.connection, nav.router_id, spec.scope().code(), &window.encode()?)
}

pub fn unsubscribe(nav: &mut NavigationClient, event: &str) -> Result<(), String> {
    client::unsubscribe(&mut nav.connection, nav.router_id, event)
}

/// Desenregistre cette app aupres du routeur - a appeler avant de fermer la
/// fenetre si l'app veut liberer son identifiant proprement plutot que de
/// laisser le routeur le decouvrir a la fermeture de la connexion (voir
/// `AzureWindow::run`, qui appelle desormais ceci automatiquement en fin de
/// boucle quand une `NavigationClient` est branchee).
pub fn disconnect(nav: &mut NavigationClient) -> Result<(), String> {
    client::unregister(&mut nav.connection, nav.router_id)
}

/// Envoie une demande de navigation vers l'ecran `path` de l'app
/// `target_app_id`, avec `payload` comme donnee libre pour cet ecran (voir
/// `Route`). Cote receveur, `listen` + `RouteTable::resolve` (voir
/// `models::route_table`) transforment ce message en nouvel arbre
/// `ui_nodes` - c'est le point d'entree du systeme de routes, `send` reste
/// disponible pour un message qui n'est pas une navigation.
pub fn navigate(nav: &mut NavigationClient, target_app_id: u32, path: &str, payload: &str) -> Result<(), String> {
    navigate_activated(nav, target_app_id, path, payload, "")
}

/// Comme `navigate`, avec un jeton d'activation (vide : aucun) : la fenetre
/// de l'app cible passe au premier plan.
pub fn navigate_activated(nav: &mut NavigationClient, target_app_id: u32, path: &str, payload: &str, activation: &str) -> Result<(), String> {
    let route = Route::new(path, payload).with_activation(activation);
    client::send(&mut nav.connection, nav.router_id, target_app_id, &route.encode())
}

/// Demarre un thread qui bloque sur `receive` en boucle sur une COPIE de la
/// connexion de `nav` (voir `UnixStream::try_clone` - le meme choix que
/// fait deja `azure_rooter::managers::router::handle_connection` pour
/// stocker une connexion tout en continuant a lire sur l'originale) et
/// decode chaque message recu (une `Route` ou une fenetre envoyee par une
/// autre app, voir `Incoming`) avant de le deposer dans le canal retourne. Le thread s'arrete de lui-meme des que la connexion se ferme
/// (erreur de lecture) - rien a arreter explicitement.
///
/// A appeler AU PLUS UNE FOIS par `NavigationClient` : `receive` directement
/// sur `nav` ET ce thread liraient tous les deux la meme connexion, se
/// disputant les messages entre eux.
pub fn listen(nav: &NavigationClient) -> Result<mpsc::Receiver<Incoming>, String> {
    let mut listener_connection = nav.connection.try_clone().map_err(|e| e.to_string())?;
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        while let Ok(raw) = client::receive(&mut listener_connection) {
            let Some(incoming) = Incoming::decode(&raw) else { continue };
            if sender.send(incoming).is_err() {
                break; // plus personne n'ecoute le canal, inutile de continuer
            }
        }
    });

    Ok(receiver)
}
