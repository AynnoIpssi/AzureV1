// Les deux langages du systeme, cote a cote : rsH (balisage, l'equivalent
// HTML - lexer/parser vers un AST d'elements) et rsC (feuilles de style,
// l'equivalent CSS - lexer/parser vers des regles/selecteurs, plus le
// systeme de lien qui les fait correspondre a l'arbre rsH). `services`
// contient ce qui consomme les deux a la fois pour produire la sortie
// finale (code Rust genere ou arbre `UiNode` construit directement) - voir
// `services::codegen`/`services::interpreter`.
pub mod rsh;
pub mod rsc;
pub mod services;
pub mod components;
