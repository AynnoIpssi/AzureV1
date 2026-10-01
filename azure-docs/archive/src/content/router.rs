// Sujets routeur : contenu statique (pas de pipeline rsH/rsC a interpreter
// ici) - la demo VRAIMENT live (aller-retour reel sur le socket) est un bloc
// a part construit par `crate::router_demo`, pas une "recette" parmi
// d'autres.
use super::recipe::Recipe;

pub fn recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            title: "Prerequis : lancer le daemon",
            goal: "Demarrer le routeur AVANT toute app qui s'y connecte, sinon register()/register_at() echouent immediatement.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some("cargo run -p azure-rooter --bin routeur_daemon\n\n# Dans un AUTRE terminal, ensuite seulement :\ncargo run -p azure-docs"),
            note: Some("register()/register_at() font juste un UnixStream::connect(SOCKET_PATH) - aucun demarrage automatique du daemon depuis le client."),
        },
        Recipe {
            title: "register",
            goal: "S'enregistrer aupres du routeur sous un app_id unique, prealable a tout send/subscribe/publish.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some("use azure_rooter::services::client::register;\n\nlet mut connection = register(42)?; // 42 = app_id, doit etre unique parmi les apps connectees"),
            note: Some("Le routeur ne verifie pas les collisions d'app_id : deux apps enregistrees sous le meme id se marchent silencieusement dessus."),
        },
        Recipe {
            title: "send - point a point",
            goal: "Envoyer un message directement a un app_id precis, recu via receive() (bloquant) cote destinataire.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some("use azure_rooter::services::client::{send, receive};\n\nsend(&mut connection, sender_id, receiver_id, \"contenu\")?;\n\n// Cote destinataire, dans SA propre boucle :\nlet message = receive(&mut connection)?; // bloque jusqu'a reception"),
            note: None,
        },
        Recipe {
            title: "subscribe / publish - diffusion",
            goal: "S'abonner a un evenement nomme, puis recevoir automatiquement tout publish() fait sur ce nom par n'importe quelle app.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some("use azure_rooter::services::client::{subscribe, publish};\n\nsubscribe(&mut connection, app_id, \"notification\")?;\n\n// Une AUTRE app, plus tard :\npublish(&mut other_connection, other_app_id, \"notification\", \"contenu\")?;"),
            note: Some("publish sans abonne n'a aucun effet - la diffusion ne conserve pas les messages pour un abonne qui arriverait plus tard."),
        },
        Recipe {
            title: "unsubscribe / unregister",
            goal: "Se desabonner d'un evenement, puis liberer proprement son app_id avant de fermer.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some("use azure_rooter::services::client::{unsubscribe, unregister};\n\nunsubscribe(&mut connection, app_id, \"notification\")?;\nunregister(&mut connection, app_id)?;"),
            note: None,
        },
        Recipe {
            title: "Format de trame (wire format)",
            goal: "Savoir exactement ce qui circule sur le socket pour chaque appel - utile pour deboguer ou reimplementer un client dans un autre langage.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some(
                "Toutes les valeurs numeriques : u32 little-endian.\n\nOpcode 0  Register     : [opcode:4][app_id:4]\nOpcode 1  Send         : [opcode:4][sender:4][receiver:4][len:4][contenu]\nOpcode 2  Subscribe    : [opcode:4][app_id:4][len:4][evenement]\nOpcode 3  Publish      : [opcode:4][app_id:4][len_evt:4][evenement][len:4][contenu]\nOpcode 4  Unsubscribe  : [opcode:4][app_id:4][len:4][evenement]\nOpcode 5  Unregister   : [opcode:4][app_id:4]\n\nReception (receive()) : [len:4][contenu] - pas d'opcode, pas d'id expediteur.",
            ),
            note: None,
        },
        Recipe {
            title: "navigation_manager (recommande)",
            goal: "Utiliser le routeur SANS importer azure_rooter directement dans une app azure-foundation.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some(
                "use azure_foundation::navigation::managers::navigation_manager;\n\nlet nav = navigation_manager::connect(app_id)?;\nnavigation_manager::navigate(&mut nav, target_app_id, \"/ecran\", \"payload\")?;",
            ),
            note: Some("navigation_manager delegue directement a azure_rooter::services::client, sans logique en plus - c'est la couche que app-a/app-b (azure-test) utilisent reellement, a preferer aux appels bruts."),
        },
        Recipe {
            title: "Aucune authentification",
            goal: "Comprendre ce que le routeur NE garantit PAS avant de s'y fier pour autre chose qu'une demo locale.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some("// N'importe quel processus local peut faire :\nregister(app_id_de_quelqu_un_d_autre)?;\n// -> aucune verification, la connexion existante de la victime est silencieusement remplacee"),
            note: Some("Le routeur n'a aucune authentification : tout processus local capable d'ouvrir /tmp/azure-router.sock peut s'enregistrer sous n'importe quel app_id, y compris celui d'une autre app deja connectee. A ne jamais exposer a des processus non fiables."),
        },
    ]
}
