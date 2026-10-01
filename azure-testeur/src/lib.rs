// Azure Testeur : trouve les tests d'un projet relie (ou d'Azure lui-meme),
// les lance, et en ecrit de nouveaux directement dans le code du projet.
//
// - `projet`    : l'environnement Azure + les projets relies ;
// - `langage`   : ce qu'on sait faire d'un langage (trait `Langage`) ;
//                 `langage::rust` pour cargo. Un langage de plus = un module ;
// - `execution` : les commandes de test dans un thread, resultats en direct ;
// - `ecran`     : l'etat et les donnees de ui/testeur.rsh ;
// - `composants`: les composants de l'utilisateur (stockages, dossiers
//                 imbriques), inseres dans un nouveau test ;
// - `clics`     : ce que font les boutons ; l'atelier ecrit par
//                 `langage::Operation` (une facon d'editer de plus = une
//                 variante).
// L'editeur de code est celui d'Azure Note (bloc de code colore,
// `azure_note::code`), dans la fenetre de Testeur.
pub mod clics;
pub mod composants;
pub mod ecran;
pub mod execution;
pub mod langage;
pub mod projet;
