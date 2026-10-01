// Azure Note : un bloc-notes, relie a la documentation d'Azure.
//
// - `carnet` : les notes v1 (lues une fois pour la migration) ;
// - `ecrans` : la route de l'ecran principal (ui/note.rsh) ;
// - `page`   : les donnees de l'ecran ; `clics` : ce que font les clics ;
// - `proprietes` : valeurs (pastilles, calendrier) et carte d'une propriete ;
// - `code`   : la coloration des blocs de code ;
// - `doc`    : la petite fenetre Doc, dont le contenu vient d'Azure Docs ;
// - `formule`: les proprietes calculees (v2), `moyenne(enfants.avancement)` ;
// - `modele` : les pages v2 en memoire (bases, vues, proprietes, blocs) ;
// - `riche`  : le texte stylé d'un bloc, range en texte ;
// - `classeur`: les pages v2 dans la base RsS, migration des notes v1.
pub mod carnet;
pub mod classeur;
pub mod clics;
pub mod code;
pub mod riche;
pub mod doc;
pub mod ecrans;
pub mod formule;
pub mod modele;
pub mod page;
pub mod proprietes;

pub use carnet::{Carnet, Note};
