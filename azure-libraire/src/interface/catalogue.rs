// Le catalogue des modules d'interface : leur nom (la balise rsH), leur
// categorie, ce qu'ils font, et leurs deux sources.
use std::sync::OnceLock;

/// Un module d'interface : la balise `<nom>` d'une page rsH.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Module {
    /// Le nom de la balise (`card` pour `<card>`).
    pub nom: &'static str,
    /// Son dossier dans `modules/`.
    pub categorie: &'static str,
    /// Ses attributs et ce qu'il affiche, en une ligne.
    pub resume: &'static str,
    /// Son modele rsH.
    pub rsh: &'static str,
    /// Ses styles rsC, avec les jetons du theme (`$accent`...).
    pub rsc: &'static str,
}

macro_rules! module {
    ($categorie:literal, $nom:literal, $resume:literal) => {
        Module {
            nom: $nom,
            categorie: $categorie,
            resume: $resume,
            rsh: include_str!(concat!("modules/", $categorie, "/", $nom, ".rsh")),
            rsc: include_str!(concat!("modules/", $categorie, "/", $nom, ".rsc")),
        }
    };
}

/// Styles qui ne sont pas ceux d'un module : les champs dessines par la
/// fondation (`input`, `checkbox`, `select`...).
const BASE: &str = include_str!("modules/base/champs.rsc");

/// Tous les modules. L'ordre est celui des styles : a selecteur egal, la
/// regle du module place plus bas l'emporte.
const MODULES: &[Module] = &[
    // ------------------------------------------------------ application
    module!("application", "app", "racine d'une app, pleine hauteur, fond du theme ; .colonne pour empiler"),
    module!("application", "sidebar", "titre ; barre laterale (menu de l'app)"),
    module!("application", "sidebar-group", "titre ; groupe d'entrees dans la barre laterale"),
    module!("application", "sidebar-item", "id actif=\"true\" ; entree de la barre laterale (bouton #id)"),
    module!("application", "main", "zone principale a cote de la barre laterale, qui defile"),
    module!("application", "toolbar", "titre ; barre d'outils en haut d'une zone"),
    module!("application", "statusbar", "barre d'etat en bas de l'app"),
    // ------------------------------------------------------ mise en page
    module!("mise_en_page", "row", "rangee : enfants cote a cote"),
    module!("mise_en_page", "column", "colonne serree"),
    module!("mise_en_page", "stack", "colonne aeree"),
    module!("mise_en_page", "grid", "cols=\"1|2|3|4\" ; grille"),
    module!("mise_en_page", "center", "centre son contenu"),
    module!("mise_en_page", "spacer", "pousse ce qui suit au bout de la rangee"),
    module!("mise_en_page", "divider", "trait de separation"),
    module!("mise_en_page", "section", "titre description ; bloc titre d'une page"),
    module!("mise_en_page", "page", "titre description ; page avec marges"),
    module!("mise_en_page", "navbar", "titre ; barre de navigation du haut"),
    module!("mise_en_page", "footer", "pied de page"),
    module!("mise_en_page", "header", "titre description ; en-tete de page, actions a droite (contenu)"),
    module!("mise_en_page", "panel", "titre ; panneau borde avec barre de titre ; .plat sans marge interieure"),
    module!("mise_en_page", "split", "zones cote a cote sur toute la hauteur (contient des <pane>)"),
    module!("mise_en_page", "pane", "zone d'un <split> ; .etroit .large pour une largeur fixe"),
    module!("mise_en_page", "scroll", "zone qui defile et prend la place restante"),
    module!("mise_en_page", "box", "boite simple ; .creux .borde .serre"),
    // ----------------------------------------------------------- contenu
    module!("contenu", "card", "titre description pied ; carte"),
    module!("contenu", "stat", "label valeur detail tendance=\"hausse|baisse\" ; chiffre cle"),
    module!("contenu", "feature", "titre description ; argument avec pastille"),
    module!("contenu", "price", "nom prix periode items=\"a, b\" ; offre"),
    module!("contenu", "hero", "titre description ; bandeau d'accueil, actions dans le contenu"),
    module!("contenu", "list", "liste bordee de <list-item>"),
    module!("contenu", "list-item", "titre description ; ligne de liste, contenu a droite"),
    module!("contenu", "table", "tableau : <tr> de <th> / <td>"),
    module!("contenu", "tr", "ligne de tableau"),
    module!("contenu", "th", "cellule d'en-tete"),
    module!("contenu", "td", "cellule"),
    module!("contenu", "info", "terme valeur ; ligne terme / valeur"),
    module!("contenu", "timeline", "frise de <timeline-item>"),
    module!("contenu", "timeline-item", "titre date ; etape de la frise"),
    module!("contenu", "accordion", "id titre ouvert=\"true\" ; bloc repliable (bouton #id)"),
    module!("contenu", "quote", "auteur ; citation"),
    module!("contenu", "code", "bloc de code"),
    module!("contenu", "kbd", "touche du clavier"),
    module!("contenu", "avatar", "nom ; pastille aux initiales"),
    module!("contenu", "empty", "titre description ; etat vide"),
    module!("contenu", "skeleton", "lignes grises en attendant le contenu"),
    module!("contenu", "media", "nom titre description ; avatar + deux lignes, contenu a droite"),
    module!("contenu", "shortcut", "label touches=\"Ctrl, K\" ; raccourci clavier"),
    module!("contenu", "caption", "petit intitule en capitales"),
    module!("contenu", "lead", "paragraphe d'introduction"),
    module!("contenu", "muted", "texte discret"),
    // ---------------------------------------------------------- messages
    module!("messages", "badge", "etiquette ronde ; .succes .danger .attention .neutre"),
    module!("messages", "tag", "etiquette carree"),
    module!("messages", "chip", "id ; etiquette avec bouton de retrait #id"),
    module!("messages", "alert", "titre ; message ; .succes .attention .danger"),
    module!("messages", "toast", "titre ; notification ; .succes .danger"),
    module!("messages", "meter", "label texte value max ; jauge legendee"),
    module!("messages", "banner", "id ; bandeau (bouton #id-fermer)"),
    module!("messages", "status", "point de couleur + texte ; .succes .danger .attention .neutre"),
    module!("messages", "count", "pastille de compteur ; .accent .danger"),
    // -------------------------------------------------------- navigation
    module!("navigation", "btn", "id ; bouton ; .primary .danger .ghost .small"),
    module!("navigation", "link", "id ; lien"),
    module!("navigation", "ancre", "vers=\"id\" ; lien qui fait defiler jusqu'a #id"),
    module!("navigation", "menu", "menu vertical de <menu-item>"),
    module!("navigation", "menu-item", "id actif=\"true\" ; entree de menu"),
    module!("navigation", "tabs", "id items=\"A, B\" actif=\"A\" ; onglets (#id-0, #id-1...)"),
    module!("navigation", "breadcrumb", "items=\"A, B\" ; fil d'Ariane"),
    module!("navigation", "steps", "items=\"A, B\" courant=\"2\" ; etapes"),
    module!("navigation", "pagination", "id pages=\"6\" page=\"2\" ; pages (#id-1...)"),
    module!("navigation", "btn-group", "boutons colles en un seul bloc"),
    module!("navigation", "icon-btn", "id ; bouton carre pour un signe ; .small .primary"),
    module!("navigation", "tree-item", "id niveau=\"0..6\" ouvert=\"true|false\" actif=\"true\" ; ligne d'arbre"),
    // -------------------------------------------------------- formulaire
    module!("formulaire", "form", "formulaire : colonne de <field>"),
    module!("formulaire", "field", "label aide erreur ; champ legende"),
    module!("formulaire", "richbar", "pour=\"id\" ; barre d'outils d'un <richtext>"),
    module!("formulaire", "label", "intitule d'un champ"),
    module!("formulaire", "help", "aide sous un champ ; .erreur"),
    module!("formulaire", "fieldset", "titre description ; groupe de champs borde"),
    module!("formulaire", "setting", "titre description ; reglage : texte a gauche, champ a droite"),
    module!("formulaire", "search-bar", "id placeholder valeur bouton ; recherche (champ #id, bouton #id-ok)"),
    module!("formulaire", "actions", "rangee de boutons en bas d'un formulaire ; .gauche"),
    // ----------------------------------------------------------- graphes
    // Donnees communes : valeurs="1, 4, 2" ou series="CPU: 1 4 2; RAM: 3 3 5", etiquettes="Lun, Mar".
    module!("graphes", "chart", "titre detail valeur aide ; carte d'un graphe, pour y mettre son <graphe> ; .petit .grand .nu"),
    module!("graphes", "chart-line", "titre detail valeur aide + valeurs|series etiquettes min max unite legende ; courbe"),
    module!("graphes", "chart-area", "comme chart-line ; courbe remplie dessous"),
    module!("graphes", "chart-bars", "comme chart-line ; barres verticales (cote a cote par serie)"),
    module!("graphes", "chart-stack", "series etiquettes ; barres empilees"),
    module!("graphes", "chart-points", "comme chart-line ; nuage de points"),
    module!("graphes", "chart-hbars", "valeurs etiquettes unite max ; classement en barres horizontales"),
    module!("graphes", "chart-donut", "valeurs etiquettes unite centre ; parts en anneau, total au centre"),
    module!("graphes", "chart-pie", "valeurs etiquettes ; parts en disque"),
    module!("graphes", "chart-gauge", "valeurs=\"72\" min max unite ; jauge en demi-cercle"),
    module!("graphes", "chart-radar", "series etiquettes max legende ; toile d'araignee"),
    module!("graphes", "spark", "valeurs ; mini-courbe sans axes ; .succes .attention .danger"),
    module!("graphes", "stat-spark", "label valeur detail tendance valeurs ton ; chiffre cle avec sa mini-courbe"),
    // ----------------------------------------------------------- couches
    module!("couches", "modal", "id titre ouvert=\"true\" ; fenetre modale (#id-fermer)"),
    module!("couches", "drawer", "id titre ouvert=\"true\" ; panneau lateral (#id-fermer)"),
    module!("couches", "toasts", "pile de <toast> en bas a droite"),
    module!("couches", "confirm", "id titre ouvert=\"true\" oui non ; question (#id-oui, #id-fermer) ; .danger"),
];

/// Tous les modules d'interface.
pub fn modules() -> &'static [Module] {
    MODULES
}

/// Le module `<nom>`.
pub fn module(nom: &str) -> Option<&'static Module> {
    MODULES.iter().find(|m| m.nom == nom)
}

/// Les categories, dans l'ordre du catalogue.
pub fn categories() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for m in MODULES {
        if !out.contains(&m.categorie) {
            out.push(m.categorie);
        }
    }
    out
}

/// Les styles rsC de tous les modules, jetons non remplaces : a passer
/// par `Theme::appliquer`.
pub fn styles() -> &'static str {
    static STYLES: OnceLock<String> = OnceLock::new();
    STYLES.get_or_init(|| {
        let mut out = String::from(BASE);
        for m in MODULES {
            out.push_str(m.rsc);
            out.push('\n');
        }
        out
    })
}
