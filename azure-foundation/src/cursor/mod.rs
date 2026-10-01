// Curseur souris : genere lui-meme ses icones (aucun theme de curseur
// systeme lu, meme philosophie que le reste d'Azure - voir
// `services::cursor_icon`) et choisit laquelle afficher selon ce que la
// souris survole (voir `models::cursor_kind::CursorKind`) :
// - main (`Pointer`) au-dessus d'un element cliquable (bouton, boutons de
//   la barre d'en-tete) ;
// - barre de texte (`Text`) UNIQUEMENT au-dessus d'une zone ou l'on peut
//   taper (`TextArea`) - jamais au-dessus d'un texte simplement affiche
//   (`Label`, titres...) ;
// - fleche (`Default`) partout ailleurs.
//
// `AzureWindow::run` l'utilise tel quel ; une app avec sa propre boucle
// Wayland peut faire de meme via `managers::cursor_manager::CursorSet`.
pub mod managers;
pub mod models;
pub mod services;
