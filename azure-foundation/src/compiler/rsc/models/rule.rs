use crate::compiler::rsc::models::selector::ComplexSelector;
use crate::compiler::rsc::models::value::Value;

/// Une declaration `propriete: valeur;` brute, avant resolution contre un
/// element. `name` est garde tel quel (pas encore mappe vers `Property`) :
/// c'est `services::link` qui fait cette resolution au moment de la
/// cascade, pas le parser.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
    /// `true` si la declaration porte `!important` - elle l'emporte alors
    /// sur toute declaration normale, quelle que soit sa specificite.
    pub important: bool,
}

/// Un bloc de regle : une liste de selecteurs (separes par des virgules,
/// tous equivalents pour ce bloc) et les declarations qu'ils partagent.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<ComplexSelector>,
    pub declarations: Vec<Declaration>,
}

/// L'AST complet d'une feuille de style rsC : l'equivalent de
/// `Vec<AstNode>` cote rsH.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RscStylesheet {
    pub rules: Vec<Rule>,
    /// Declarations illisibles, sautees (comme un navigateur) au lieu de
    /// faire echouer toute la feuille : un message par declaration.
    pub skipped: Vec<String>,
}
