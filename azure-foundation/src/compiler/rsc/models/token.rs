// Tokens du langage rsC - le "CSS" du systeme, en pendant de TokenRsH.
//
// On reutilise `Position`/`Spanned` de rsH (compiler::models::token) : ce
// sont des types generiques (une ligne/colonne, une valeur rattachee a une
// position) qui n'ont rien de specifique au HTML, pas de raison de les
// dupliquer.
pub use crate::compiler::rsh::models::token::{Position, Spanned};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenRsC {
    // -------------------- Identifiants et selecteurs --------------------
    /// Nom de balise, mot-cle, nom de propriete ou de fonction
    /// (`container`, `flex`, `width`, `rgba`...).
    Ident(String),
    /// Selecteur de classe `.nom` - le '.' et le nom sont captures ensemble
    /// (meme logique que le lexer rsH qui fusionne '.'+nom dans le token de
    /// balise), ce qui evite l'ambiguite avec le '.' decimal d'un nombre
    /// comme `.5`.
    ClassSel(String),
    /// `#` suivi de caracteres alphanumeriques/'-'/'_'. Volontairement
    /// ambigu a ce stade : selon le contexte ou le parser le rencontre,
    /// c'est soit un selecteur d'id (`#header { ... }`), soit une couleur
    /// hexadecimale (`color: #fff;`). C'est le parser qui choisit, pas le
    /// lexer - comme en CSS reel, ou le tokenizer produit un seul
    /// `<hash-token>` pour les deux usages.
    Hash(String),
    /// `*` - selecteur universel.
    Star,

    // -------------------- Symboles --------------------
    Colon,        // :
    SemiColon,    // ;
    Comma,        // ,
    OpenBrace,    // {
    CloseBrace,   // }
    OpenParen,    // (
    CloseParen,   // )
    GreaterThan,  // > (combinateur enfant)
    Plus,         // + (combinateur frere adjacent)
    Tilde,        // ~ (combinateur frere general)
    Bang,         // ! (pour !important)
    /// Un ou plusieurs espaces/commentaires consecutifs, condenses en un
    /// seul token. Significatif en position de selecteur (combinateur
    /// "descendant" implicite) ; simple separateur ailleurs (entre deux
    /// composants d'une valeur comme `10px 20px`).
    Combinator,

    // -------------------- Valeurs --------------------
    Number(f32),
    /// Nombre immediatement suivi de `%` (`50%`).
    Percentage(f32),
    /// Nombre immediatement suivi d'une unite alphabetique (`10px`, `1.5em`).
    /// L'unite est gardee brute ; sa validite est verifiee plus haut
    /// (`models::value::Unit::parse`).
    Dimension(f32, String),
    /// Chaine entre guillemets simples ou doubles (echappement minimal du
    /// guillemet de fermeture uniquement).
    StringLiteral(String),
}
