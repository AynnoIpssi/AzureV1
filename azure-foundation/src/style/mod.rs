// `models::{Style, Stylesheet}` sont le format RESOLU commun aux deux
// systemes de style du crate - la seule chose qui compte pour
// `compiler::services::{interpreter, codegen}` en aval, quelle que soit
// l'origine :
// - `compiler::rsc` (recommande) : la DSL "CSS-like" activement maintenue -
//   selecteurs (balise/classe/id/imbrication/combinateurs), pseudo-classes
//   (`:hover`/`:focus`), ET flex/grid complets (voir `rsc::models::property`).
//   Convertit son propre format interne vers `models::style::Style` via
//   `rsc::services::link::to_legacy_style`.
// - `services::stylesheet_parser` (LEGACY) : l'ancien format texte
//   `.nom { propriete: valeur; }`, plus simple mais fige a un sous-ensemble
//   de 11 proprietes - ni selecteurs, ni pseudo-classes, ni flex/grid. Garde
//   pour compatibilite (voir sa propre doc), pas pour du nouveau code : voir
//   `compiler::rsc` pour tout ce qu'il ne sait pas exprimer.
pub mod models;
pub mod services;
