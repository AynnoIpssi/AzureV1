// Une "recette" documentaire : objectif -> code -> resultat. `rsh`/`rsc`
// presents ensemble declenchent un aperçu VRAIMENT interprete (voir
// `crate::ui::preview`, qui appelle le vrai pipeline
// `compiler::services::interpreter::build_ui_with_context` d'azure-foundation)
// plutot qu'une capture d'ecran ou une maquette dessinee a la main - c'est
// le coeur de la valeur de cette app : montrer le resultat REEL d'un
// snippet, pas une prose qui le decrit.
use azure_foundation::compiler::services::condition::Context;

pub struct Recipe {
    pub title: &'static str,
    pub goal: &'static str,
    /// Source rsH a afficher ET interpreter en direct - `None` pour une
    /// recette purement textuelle (ex: tableau de format de trame du
    /// routeur, comparatif interpreteur/codegen).
    pub rsh: Option<&'static str>,
    /// Feuille rsC compagnon, appliquee par-dessus la feuille par defaut de
    /// `crate::ui::preview` - `None` = juste la feuille par defaut.
    pub rsc: Option<&'static str>,
    /// Variables de conditions (voir `condition::Context`) necessaires a
    /// l'apercu live d'un `if`/`elseif`/`for`/`foreach`/`match` - un pointeur
    /// de fonction (sans capture, donc `'static`) plutot qu'un `Context` deja
    /// construit, `Context` n'etant pas `const`-constructible. `None` =
    /// `Context::new()` vide (suffisant pour un `if`/`elseif` SANS `else`
    /// affiche comme "aucune branche", voir sa doc).
    pub ctx: Option<fn() -> Context>,
    /// Bloc de code affiche tel quel, sans interpretation - utilise pour un
    /// sujet qui n'est pas du rsH/rsC (ex: sequence d'appels
    /// `azure_rooter::services::client`).
    pub snippet: Option<&'static str>,
    /// Encart d'avertissement/precision - une limitation documentee du
    /// framework (ex: `<while>` non reellement boucle par l'interpreteur),
    /// pas juste une note generique.
    pub note: Option<&'static str>,
}
