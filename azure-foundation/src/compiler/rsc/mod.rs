// La DSL de style "CSS-like" activement maintenue du crate - selecteurs,
// pseudo-classes, flex/grid complets (voir `models::property`). C'est le
// systeme a utiliser pour tout nouveau code ; voir `crate::style` pour la
// comparaison avec l'ancien format texte `.style` (`style::services::stylesheet_parser`,
// legacy, garde pour compatibilite mais bien plus limite).
pub mod models;
pub mod mangers;
pub mod services;

use crate::compiler::rsc::models::property::Property;
use crate::compiler::rsc::models::rule::RscStylesheet;

/// Proprietes reconnues mais pas encore appliquees par le rendu.
pub const IGNORED: &[&str] = &[];

/// Ce qu'une feuille contient d'inutile : proprietes inconnues (faute de
/// frappe ?) ou sans effet pour l'instant. Une ligne par propriete.
pub fn warnings(sheet: &RscStylesheet) -> Vec<String> {
    let mut out: Vec<String> = sheet.skipped.clone();
    for rule in &sheet.rules {
        for decl in &rule.declarations {
            let name = decl.name.as_str();
            let message = if Property::from_name(name).is_none() {
                format!("propriete '{name}' inconnue, ignoree")
            } else if IGNORED.contains(&name) {
                format!("propriete '{name}' sans effet pour l'instant")
            } else {
                continue;
            };
            if !out.contains(&message) {
                out.push(message);
            }
        }
    }
    out
}
