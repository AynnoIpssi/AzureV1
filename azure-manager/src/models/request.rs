// Protocole du socket d'azure-manager : trames et statut comme azure-service
// (voir `azure_service::flux::protocol`).
pub use azure_service::flux::protocol::{check_status, error, read_frame, response, write_frame, STATUS_OK};

/// Une app se presente avec son manifeste ; recoit son id. La connexion
/// ouverte = l'app tourne.
pub const REGISTER: u32 = 0;
/// Nom d'app -> id.
pub const RESOLVE: u32 = 1;
/// Qui peut ecouter un de mes flux, ou appeler une de mes methodes
/// (manifeste + choix du tableau de bord). Arguments : genre u8 (voir
/// `Kind`), nom.
pub const ACCESS: u32 = 2;
/// Etat complet (apps, liens, services) : ce que montre le tableau de bord.
pub const STATE: u32 = 3;
/// Une app signale un evenement : nom, niveau u8 (0 info, 1 attention,
/// 2 erreur), message. Le nom doit etre celui de l'executable qui parle.
pub const REPORT: u32 = 4;
/// L'app qui parle veut appeler une methode d'une app fermee : le manager
/// verifie son droit et lance la tache de fond qui la sert (voir
/// `Manager::wake_service`). Arguments : app, methode.
pub const WAKE: u32 = 6;
/// Qui est l'app `id` ? (pour azure-stockage et azure-service). Reponse :
/// trouvee u8, nom, executable, installee u8, empreinte (32 octets ou
/// vide), permission stockage u8.
pub const IDENTIFY: u32 = 5;
/// L'app qui parle ouvre l'app `nom` : permis si son manifeste declare
/// `[open nom]` et si `nom` est installee. Le manager la lance si elle ne
/// tourne pas (la page voulue, envoyee avant par le routeur, l'attend dans
/// sa boite aux lettres). Arguments : nom, jeton d'activation Wayland
/// (vide : aucun ; donne a l'app lancee dans `XDG_ACTIVATION_TOKEN`, pour
/// qu'elle s'ouvre au premier plan). Reponse : lancee u8 (0 : tournait deja).
pub const OPEN: u32 = 7;

// Reserve au tableau de bord et a la ligne de commande. GRANT, REVOKE,
// SET_PUBLIC et RESET_ACCESS commencent par le genre (u8, voir `Kind`).
pub const GRANT: u32 = 10;
pub const REVOKE: u32 = 11;
pub const SET_PUBLIC: u32 = 12;
pub const RESET_ACCESS: u32 = 13;
pub const FORGET: u32 = 14;
pub const RESTART_SERVICE: u32 = 15;
/// Les dernieres lignes du journal d'un service ou d'une app : nom, nombre.
pub const LOGS: u32 = 16;
/// `azure install` : resume du manifeste, executable, empreinte. Retourne l'id.
pub const INSTALL: u32 = 17;
/// Le terminal du tableau de bord : lance `azure <arguments>` (voir
/// `managers::terminal`). Arguments : nombre u32 puis chaque argument.
/// Reponse : code de sortie (u32, -1 = arretee), nombre de lignes, lignes.
pub const AZURE_COMMAND: u32 = 18;
