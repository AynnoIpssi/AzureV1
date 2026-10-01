// Routage INTRA-app : memes concepts (register/send/receive,
// subscribe/publish/unsubscribe/unregister) que `managers::router` (le
// routeur inter-app par socket Unix), mais 100% en memoire dans le process
// de l'app - aucune IPC, aucun `/tmp/azure-router.sock`, aucune serialisation
// binaire. Sert deux usages : naviguer entre les ecrans d'UNE MEME fenetre
// (voir `navigate`/`IntraRoute`) et faire communiquer plusieurs vues
// independantes qui tournent dans le MEME process (`send`/`publish` bruts).
//
// A ne jamais confondre avec `router.rs` : celui-ci reste le seul point
// d'entree pour parler a une AUTRE app (un AUTRE process) - personne n'y
// touche ici, et ce fichier ne l'importe pas.
//
// Implemente `AzureRouterProvider` (azure-core) pour respecter l'interface
// unifiee documentee dans le README d'azure-core : intra-app et inter-app
// exposent la meme forme (`register`/`send`/`receive`), seul le transport
// derriere differe.
use crate::models::intra_route::IntraRoute;
use azure_core::rules::root_provider::AzureRouterProvider;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Poignee partageable (voir `Clone`) vers un routeur intra-app : toutes les
/// clones d'un meme `IntraRouter::new()` voient les memes vues enregistrees
/// et les memes abonnements (l'`Arc<Mutex<...>>` interne est partage, pas
/// duplique) - a construire UNE FOIS par process/app et a cloner vers chaque
/// vue qui doit pouvoir router localement, exactement comme
/// `NavigationClient` est obtenu une fois puis reutilise cote inter-app.
#[derive(Clone, Default)]
pub struct IntraRouter {
    inboxes: Arc<Mutex<HashMap<u32, VecDeque<String>>>>,
    subscriptions: Arc<Mutex<HashMap<String, Vec<u32>>>>,
}

impl IntraRouter {
    pub fn new() -> IntraRouter {
        IntraRouter {
            inboxes: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Abonne `view_id` a `event` - sans effet si `view_id` n'est pas (ou
    /// plus) enregistree (voir `register`), comme le routeur inter-app.
    pub fn subscribe(&self, view_id: u32, event: &str) {
        let registered = self.inboxes.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&view_id);
        if !registered {
            return;
        }
        self.subscriptions.lock().unwrap_or_else(|e| e.into_inner()).entry(event.to_string()).or_default().push(view_id);
    }

    pub fn unsubscribe(&self, view_id: u32, event: &str) {
        if let Some(ids) = self.subscriptions.lock().unwrap_or_else(|e| e.into_inner()).get_mut(event) {
            ids.retain(|&id| id != view_id);
        }
    }

    /// Depose `content` dans la boite de chaque vue actuellement abonnee a
    /// `event` (voir `subscribe`) - sans effet si personne n'y est abonne.
    /// `sender_id` n'est pas transmis aux abonnes, comme cote inter-app
    /// (voir `router::handle_connection`, bras opcode 3) : seul l'evenement
    /// et son contenu comptent pour un abonnement.
    pub fn publish(&self, event: &str, content: &str) {
        let targets = self.subscriptions.lock().unwrap_or_else(|e| e.into_inner()).get(event).cloned();
        let Some(targets) = targets else { return };
        let mut inboxes = self.inboxes.lock().unwrap_or_else(|e| e.into_inner());
        for id in targets {
            if let Some(inbox) = inboxes.get_mut(&id) {
                inbox.push_back(content.to_string());
            }
        }
    }

    /// Libere `view_id` : retire sa boite de reception et la sort de tous
    /// les abonnements. A appeler quand une vue se ferme, pour ne pas
    /// accumuler des boites mortes ni republier vers une vue disparue.
    pub fn unregister(&self, view_id: u32) {
        self.inboxes.lock().unwrap_or_else(|e| e.into_inner()).remove(&view_id);
        for ids in self.subscriptions.lock().unwrap_or_else(|e| e.into_inner()).values_mut() {
            ids.retain(|&id| id != view_id);
        }
    }

    /// Envoie une demande de navigation vers l'ecran `path` de la vue
    /// `receiver_id`, avec `payload` comme donnee libre pour cet ecran (voir
    /// `IntraRoute`) - l'equivalent local de
    /// `navigation_manager::navigate` cote inter-app, sans passer par le
    /// socket. Cote receveur, `receive` + `IntraRoute::decode` (puis
    /// `RouteTable::resolve` si la vue en tient un) transforment ce message
    /// en nouvel ecran.
    pub fn navigate(&self, sender_id: u32, receiver_id: u32, path: &str, payload: &str) {
        let route = IntraRoute::new(path, payload);
        self.send(sender_id, receiver_id, &route.encode());
    }
}

impl AzureRouterProvider for IntraRouter {
    /// Enregistre `app_id` (ici l'id d'une vue locale) aupres du routeur -
    /// idempotent, comme une simple insertion dans la boite si elle
    /// n'existe pas deja (ne vide pas une boite existante).
    fn register(&mut self, app_id: u32) {
        self.inboxes.lock().unwrap_or_else(|e| e.into_inner()).entry(app_id).or_default();
    }

    /// Depose `message` dans la boite de `receiver_id` - silencieusement
    /// perdu si elle n'est pas (ou plus) enregistree, comme cote inter-app.
    fn send(&self, _sender_id: u32, receiver_id: u32, message: &str) {
        if let Some(inbox) = self.inboxes.lock().unwrap_or_else(|e| e.into_inner()).get_mut(&receiver_id) {
            inbox.push_back(message.to_string());
        }
    }

    /// Retire et retourne le plus ancien message en attente pour
    /// `receiver_id`, ou `None` si sa boite est vide/inexistante - contrairement
    /// au `receive` bloquant du client inter-app (lecture socket), celui-ci
    /// est un sondage immediat : a appeler depuis une boucle d'evenements
    /// deja existante (ex: `AzureWindow::run`, le bras `on_tick`), pas dans
    /// un thread dedie.
    fn receive(&self, receiver_id: u32) -> Option<String> {
        self.inboxes.lock().unwrap_or_else(|e| e.into_inner()).get_mut(&receiver_id).and_then(|inbox| inbox.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_then_receive_delivers_the_message_in_order() {
        let mut router = IntraRouter::new();
        router.register(1);
        router.register(2);

        router.send(1, 2, "hello");
        router.send(1, 2, "world");

        assert_eq!(router.receive(2), Some("hello".to_string()));
        assert_eq!(router.receive(2), Some("world".to_string()));
        assert_eq!(router.receive(2), None);
    }

    #[test]
    fn send_to_an_unregistered_view_is_silently_dropped() {
        let mut router = IntraRouter::new();
        router.register(1);

        router.send(1, 99, "lost");

        assert_eq!(router.receive(99), None);
    }

    #[test]
    fn publish_reaches_every_subscriber_but_not_others() {
        let mut router = IntraRouter::new();
        router.register(1);
        router.register(2);
        router.register(3);

        router.subscribe(2, "notification");
        router.subscribe(3, "notification");
        router.publish("notification", "salut");

        assert_eq!(router.receive(2), Some("salut".to_string()));
        assert_eq!(router.receive(3), Some("salut".to_string()));
        assert_eq!(router.receive(1), None);
    }

    #[test]
    fn subscribe_before_register_is_a_no_op() {
        let router = IntraRouter::new();
        router.subscribe(1, "notification");
        router.publish("notification", "salut");

        assert_eq!(router.receive(1), None);
    }

    #[test]
    fn unsubscribe_stops_future_publishes_from_reaching_the_view() {
        let mut router = IntraRouter::new();
        router.register(1);
        router.subscribe(1, "notification");
        router.unsubscribe(1, "notification");

        router.publish("notification", "salut");

        assert_eq!(router.receive(1), None);
    }

    #[test]
    fn unregister_removes_the_inbox_and_all_subscriptions() {
        let mut router = IntraRouter::new();
        router.register(1);
        router.subscribe(1, "notification");

        router.unregister(1);
        router.publish("notification", "salut");

        // La boite n'existe plus du tout : un send direct est aussi perdu.
        router.send(0, 1, "direct");
        assert_eq!(router.receive(1), None);
    }

    #[test]
    fn navigate_encodes_a_route_that_the_receiver_can_decode() {
        let mut router = IntraRouter::new();
        router.register(1);
        router.register(2);

        router.navigate(1, 2, "/settings", "tab=audio");

        let raw = router.receive(2).expect("un message doit etre en attente");
        let route = IntraRoute::decode(&raw);
        assert_eq!(route.path, "/settings");
        assert_eq!(route.payload, "tab=audio");
    }

    #[test]
    fn cloned_routers_share_the_same_underlying_state() {
        let mut router = IntraRouter::new();
        let clone = router.clone();
        router.register(1);

        clone.send(0, 1, "via clone");

        assert_eq!(router.receive(1), Some("via clone".to_string()));
    }
}
