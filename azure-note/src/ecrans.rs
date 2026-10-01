// L'ecran principal d'Azure Note (ui/note.rsh + ui/note.rsc) : une seule
// route, `/`. Ce qu'elle montre vient de l'espace des pages (`Classeur`) et
// de l'etat de l'ecran (`page::Etat`) ; les clics sont dans `clics`.
use crate::classeur::Classeur;
use crate::page::{ecran, Etat};
use azure_foundation::navigation::models::route_table::RouteTable;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// La table de routes de la fenetre principale.
pub fn routes(ui: &Path, classeur: &Classeur, etat: Arc<Mutex<Etat>>) -> RouteTable {
    let rsh = ui.join("note.rsh").to_string_lossy().into_owned();
    let rsc = ui.join("note.rsc").to_string_lossy().into_owned();
    let classeur = classeur.clone();
    RouteTable::new().view_with("/", &rsh, &rsc, move |_| {
        let etat = etat.lock().unwrap_or_else(|e| e.into_inner()).clone();
        classeur.lire(|e| ecran(e, &etat))
    })
}
