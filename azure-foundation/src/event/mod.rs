// Systeme d'evenements reutilisable : interprete les evenements bas niveau
// (clic, deplacement de souris, molette, touche pressee/relachee) en
// actions sur un arbre de `UiNode` - focus, saisie/edition de texte,
// selection a la souris ou au clavier, defilement, presse-papiers interne,
// raccourcis... independamment de tout systeme de fenetrage ou de rendu
// concret.
//
// `window::models::window::AzureWindow` s'appuie dessus pour sa propre
// boucle Wayland, mais c'est ce module qu'il faut reutiliser directement
// si vous ecrivez votre propre application avec votre propre boucle
// d'evenements/votre propre rendu : construisez un `models::app_state::EventState`,
// et appelez `services::dispatch::handle_event`/`handle_tick` a chaque
// evenement recu/chaque "tic" - toute la logique d'interaction (comment un
// clic, une frappe ou un defilement doit modifier l'arbre de widgets) est
// deja la, sans rien connaitre de Wayland ni d'un `Canvas`.
pub mod models;
pub mod services;
