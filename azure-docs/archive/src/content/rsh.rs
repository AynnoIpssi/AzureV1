// Sujets rsH, du plus simple au plus subtil - chaque recette avec un `rsh`
// est reellement interpretee par `crate::ui::preview` (vrai pipeline
// azure-foundation), pas une capture d'ecran.
use super::recipe::Recipe;
use azure_foundation::compiler::services::condition::{Context, ConditionValue};

fn ctx_roles() -> Context {
    Context::new().with_bool("est_admin", true).with_bool("est_invite", false)
}

fn ctx_match() -> Context {
    Context::new().with_text("role", "invite")
}

fn ctx_notifications() -> Context {
    Context::new().with_list(
        "notifications",
        vec![ConditionValue::Bool(true), ConditionValue::Bool(true), ConditionValue::Bool(true)],
    )
}

fn ctx_while() -> Context {
    Context::new().with_bool("a_des_notifications", true)
}

pub fn recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            title: "Tags et attributs",
            goal: "Composer un ecran avec <container>, <title>, <text> et <button> - class/id optionnels, fermeture par <!tag>.",
            rsh: Some(
                "<container>\n    <title>Azure Docs<!title>\n    <text>Un conteneur peut recevoir des enfants de types differents.<!text>\n    <button.primary#cta>Cliquer ici<!button>\n<!container>",
            ),
            rsc: None,
            ctx: None,
            snippet: None,
            note: None,
        },
        Recipe {
            title: "if / elseif / else",
            goal: "Afficher une branche differente selon des variables de contexte fournies a build_ui_with_context.",
            rsh: Some(
                "<container>\n    <if.est_admin>Bonjour Admin<!if>\n    <elseif.est_invite>Bonjour Invite<!elseif>\n    <else>Connecte-toi pour continuer<!else>\n<!container>",
            ),
            rsc: None,
            ctx: Some(ctx_roles),
            snippet: None,
            note: Some(
                "build_ui() (sans contexte) utilise un Context::new() vide : aucun if/elseif SANS else ne peut alors etre vrai. Cet apercu utilise build_ui_with_context avec un vrai contexte (est_admin = true).",
            ),
        },
        Recipe {
            title: "match / arm",
            goal: "Choisir UNE branche parmi plusieurs motifs, avec _ comme motif de secours (catch-all).",
            rsh: Some(
                "<container>\n    <match.role>\n        <arm.\"admin\">Panneau admin<!arm>\n        <arm.\"invite\">Acces limite<!arm>\n        <arm._>Role inconnu<!arm>\n    <!match>\n<!container>",
            ),
            rsc: None,
            ctx: Some(ctx_match),
            snippet: None,
            note: Some("Comme un match Rust : une seule branche s'affiche, jamais plusieurs empilees, jamais aucune si le sujet n'est pas evaluable et qu'aucun _ n'est present."),
        },
        Recipe {
            title: "for / foreach",
            goal: "Repeter un bloc une fois par element d'une liste fournie en contexte.",
            rsh: Some("<container>\n    <for.item in notifications>Nouvelle notification<!for>\n<!container>"),
            rsc: None,
            ctx: Some(ctx_notifications),
            snippet: None,
            note: Some(
                "rsH n'a pas d'interpolation de texte : chaque iteration reaffiche exactement le meme contenu statique, rien ne distingue une iteration de la suivante (voir interpreter::build_for).",
            ),
        },
        Recipe {
            title: "while (limitation documentee)",
            goal: "Comprendre ce que <while> fait REELLEMENT dans un apercu live, avant de s'y fier pour une vraie boucle.",
            rsh: Some("<container>\n    <while.a_des_notifications>Notification<!while>\n<!container>"),
            rsc: None,
            ctx: Some(ctx_while),
            snippet: None,
            note: Some(
                "L'interpreteur (celui que cette app utilise pour l'apercu ci-dessus) evalue <while> UNE SEULE FOIS - au mieux un aperçu a un tour, pas une vraie boucle. Seul compiler::services::codegen, qui genere du code Rust source a COMPILER separement, produit un vrai while qui se repete a l'execution.",
            ),
        },
        Recipe {
            title: "Accrocher du style (class + id)",
            goal: "Cibler le meme element par sa classe ET son id depuis une feuille rsC - les deux s'appliquent si elles portent sur des proprietes differentes.",
            rsh: Some("<container>\n    <text.subtitle#tagline>Style applique via class ET id<!text>\n<!container>"),
            rsc: Some(".subtitle { color: #9a9ab0; font-size: 16px; }\n#tagline { font-weight: 700; }"),
            ctx: None,
            snippet: None,
            note: Some("Voir la categorie rsC pour la regle de cascade complete quand class et id portent sur la MEME propriete."),
        },
        Recipe {
            title: "Interpreteur vs codegen",
            goal: "Savoir quel chemin du compilateur on utilise avant d'ecrire une demo live.",
            rsh: None,
            rsc: None,
            ctx: None,
            snippet: Some(
                "interpreter::build_ui(...)        -> Vec<UiNode> directement, en memoire, sans recompilation\n                                       (chemin utilise par CETTE app pour chaque apercu ci-dessus)\n\ncodegen::generate_with_rsc(...)   -> une String de code Rust source, a ecrire dans un\n                                       fichier PUIS compiler separement avant de pouvoir l'executer",
            ),
            note: Some("Chaque apercu de cette fenetre est le vrai Vec<UiNode> produit par build_ui_with_context - pas une simulation ni une capture d'ecran."),
        },
    ]
}
