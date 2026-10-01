// Sujets rsC - chaque recette applique une feuille rsC differente sur un
// petit fixture rsH, et l'apercu est le vrai resultat du moteur de
// selecteurs/cascade (compiler::rsc::services::link), pas une description.
use super::recipe::Recipe;

pub fn recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            title: "Selecteurs et combinateurs",
            goal: "Cibler par classe, par tag imbrique (descendant) ou par enfant direct (>).",
            rsh: Some("<container.card>\n    <title>Carte<!title>\n    <container>\n        <button>Action<!button>\n    <!container>\n<!container>"),
            rsc: Some(".card { background-color: #1e1e2e; padding: 4px; }\n.card title { color: #7c9cff; }\n.card > container { padding: 2px; }"),
            ctx: None,
            snippet: None,
            note: Some(".card title (espace = descendant) matche a n'importe quelle profondeur sous .card ; .card > container (enfant direct) matche seulement un enfant immediat."),
        },
        Recipe {
            title: "Cascade, specificite, !important",
            goal: "Predire quelle regle gagne quand plusieurs ciblent la meme propriete du meme element.",
            rsh: Some("<container>\n    <text.info#status>Statut<!text>\n<!container>"),
            rsc: Some("text { color: #9a9ab0; }\n.info { color: #4ade80; }\n#status { color: #f5a623 !important; }"),
            ctx: None,
            snippet: None,
            note: Some("Ordre normal de specificite : id > classe > tag. Mais !important sur #status l'emporte sur tout le reste, quelle que soit la specificite des autres regles."),
        },
        Recipe {
            title: "Modele de boite (box model)",
            goal: "Dimensionner et espacer un element avec width/height/padding/margin.",
            rsh: Some("<container.boxed>\n    <text>Contenu<!text>\n<!container>"),
            rsc: Some(".boxed { width: 60%; height: 40%; padding: 4px; margin: 2px; background-color: #1e1e2e; }"),
            ctx: None,
            snippet: None,
            note: Some("border-width/border-color/border-radius sont reconnus par le parseur rsC (voir compiler::rsc::models::property) mais ne sont pas encore dessines par le moteur de rendu - ce n'est pas une erreur dans une feuille qui les utilise, juste une limitation actuelle, sans effet visuel."),
        },
        Recipe {
            title: "Flexbox",
            goal: "Repartir des enfants sur un axe avec flex-direction/justify-content/align-items/gap.",
            rsh: Some("<container.row>\n    <button>A<!button>\n    <button>B<!button>\n    <button>C<!button>\n<!container>"),
            rsc: Some(".row { display: flex; flex-direction: row; justify-content: space-between; align-items: center; gap: 4px; height: 20%; }"),
            ctx: None,
            snippet: None,
            note: None,
        },
        Recipe {
            title: "Grid",
            goal: "Disposer des enfants sur une grille 2D avec grid-template-columns/rows en pistes fr/%.",
            rsh: Some(
                "<container.grid>\n    <container><text>1<!text><!container>\n    <container><text>2<!text><!container>\n    <container><text>3<!text><!container>\n    <container><text>4<!text><!container>\n<!container>",
            ),
            rsc: Some(".grid { display: grid; grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr; gap: 4px; }\n.grid > container { background-color: #1e1e2e; }"),
            ctx: None,
            snippet: None,
            note: Some("Pas de raccourci grid-column: 2 / 4 - il faut grid-column + grid-column-span separement."),
        },
        Recipe {
            title: "Couleurs et unites",
            goal: "Utiliser hex/rgba() pour les couleurs, et connaitre la vraie portee de px/%/em/rem/vw/vh.",
            rsh: Some("<container.units>\n    <text>Unites<!text>\n<!container>"),
            rsc: Some(".units { width: 50%; height: 30%; padding: 2em; background-color: rgba(124, 156, 255, 0.35); }"),
            ctx: None,
            snippet: None,
            note: Some("px/em/rem/vw/vh sont reconnus par le parseur (voir Value::as_bare_number) mais LayoutProps n'a pas de vrai systeme d'unites : le nombre brut est toujours interprete comme un pourcentage du parent, quelle que soit l'unite ecrite - 2em ci-dessus se comporte donc comme 2%, pas comme une vraie taille de police relative."),
        },
        Recipe {
            title: "Pseudo-classes :hover / :focus",
            goal: "Changer l'apparence d'un element au survol ou au focus - seuls ces deux etats sont reellement geres.",
            rsh: Some("<container>\n    <button.cta>Survolez-moi<!button>\n<!container>"),
            rsc: Some(".cta { background-color: #33334a; }\n.cta:hover { background-color: #7c9cff; }\n.cta:focus { background-color: #4ade80; }"),
            ctx: None,
            snippet: None,
            note: Some("Seuls :hover et :focus sont reellement evalues par le moteur (voir resolve_pseudo_background). D'autres pseudo-classes CSS standard (:active, :first-child, :nth-child...) sont acceptees par le parseur rsC mais n'ont aucun effet visuel."),
        },
    ]
}
