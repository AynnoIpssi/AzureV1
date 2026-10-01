// Ecran partage par les fenetres rsH et rsC : sommaire scrollable a gauche
// (juste une liste de titres, non cliquable - voir la contrainte
// `AzureWindow::on_click` documentee dans le plan : une fenetre ne peut
// distinguer QUEL widget est clique, donc pas de "cliquer pour sauter a un
// sujet") + panneau de contenu scrollable a droite, toutes les recettes de
// la categorie empilees dans l'ordre - un "livre" continu plutot qu'un menu
// a onglets, mais un VRAI menu scrollable (le sommaire) au sens ou
// `layout.content_height` + `Container::scroll_offset` (voir
// `azure_foundation::ui::services::interact::scroll_at`) le rendent
// effectivement defilable a la molette, comme le contenu.
use crate::content::recipe::Recipe;
use crate::theme;
use crate::ui::recipe_block;
use azure_foundation::layout::models::layout_props::{AlignItems, DisplayMode, FlexDirection, LayoutProps};
use azure_foundation::ui::models::container::Container;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::ui_node::UiNode;

const SIDEBAR_WIDTH_PCT: f32 = 24.0;
const CONTENT_WIDTH_PCT: f32 = 76.0;

// Hauteur virtuelle du sommaire, en % de sa propre hauteur reelle - fixe et
// genereux plutot que calcule a partir du nombre de recettes : meme avec
// peu de recettes, ca garantit un vrai debordement (donc un vrai
// defilement a demontrer), et avec beaucoup de recettes, chaque titre garde
// une hauteur confortable a une ligne plutot que de retrecir a mesure
// qu'on en ajoute.
const SIDEBAR_VIRTUAL_PCT: f32 = 160.0;
const SIDEBAR_HEADING_BASIS: f32 = 14.0;

pub fn page(category_title: &str, recipes: &[Recipe]) -> Vec<UiNode> {
    let mut root = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, theme::SPACE_SM);
    root.display = DisplayMode::Flex;
    root.flex_direction = FlexDirection::Row;
    root.gap = theme::SPACE_SM;
    root.align_items = AlignItems::Stretch;

    let children = vec![sidebar(category_title, recipes), content_pane(recipes)];
    vec![UiNode::Container(Container::new(root, theme::BACKGROUND, children))]
}

pub fn sidebar(category_title: &str, recipes: &[Recipe]) -> UiNode {
    let mut layout = LayoutProps::new(0.0, 0.0, SIDEBAR_WIDTH_PCT, 100.0, 0.0, theme::SPACE_SM);
    layout.flex_basis = Some(SIDEBAR_WIDTH_PCT);
    layout.flex_shrink = 0.0;
    layout.display = DisplayMode::Flex;
    layout.flex_direction = FlexDirection::Column;
    layout.gap = theme::SPACE_XS;
    layout.content_height = Some(SIDEBAR_VIRTUAL_PCT);

    let mut heading_layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
    heading_layout.flex_basis = Some(SIDEBAR_HEADING_BASIS);
    heading_layout.flex_shrink = 0.0;

    let mut children = vec![UiNode::Label(Label::new(
        heading_layout,
        category_title.to_string(),
        theme::TEXT_PRIMARY,
        theme::TITLE_SIZE,
        theme::TITLE_WEIGHT,
    ))];

    let item_basis = (100.0 - SIDEBAR_HEADING_BASIS) / recipes.len().max(1) as f32;
    for recipe in recipes {
        let mut item_layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
        item_layout.flex_basis = Some(item_basis);
        item_layout.flex_shrink = 0.0;
        children.push(UiNode::Label(Label::new(item_layout, recipe.title.to_string(), theme::TEXT_SECONDARY, theme::TOC_SIZE, theme::TOC_WEIGHT)));
    }

    UiNode::Container(Container::new(layout, theme::SURFACE, children))
}

pub fn content_pane(recipes: &[Recipe]) -> UiNode {
    let mut layout = LayoutProps::new(0.0, 0.0, CONTENT_WIDTH_PCT, 100.0, 0.0, theme::SPACE_SM);
    layout.flex_basis = Some(CONTENT_WIDTH_PCT);
    layout.flex_grow = 1.0;
    layout.display = DisplayMode::Flex;
    layout.flex_direction = FlexDirection::Column;
    layout.gap = theme::SPACE_SM;

    // Poids total de toutes les recettes (voir `recipe_block::weight`) : la
    // hauteur virtuelle du panneau est dimensionnee dessus, et chaque bloc
    // recoit ensuite `son_poids / poids_total * 100.0` comme `flex_basis` -
    // voir la doc de `recipe_block::weight` pour pourquoi ca fait tenir
    // exactement chaque bloc dans la part d'ecran REELLE qu'il merite.
    let weights: Vec<f32> = recipes.iter().map(recipe_block::weight).collect();
    let total_weight: f32 = weights.iter().sum::<f32>().max(1.0);

    // Chaque unite de poids vaut cette fraction de la hauteur REELLE du
    // panneau - choisie pour qu'un bloc "moyen" occupe une part confortable
    // de l'ecran sans que la liste entiere ne devienne un mur minuscule ni
    // un unique ecran gigantesque, quel que soit le nombre de recettes.
    const REAL_PCT_PER_UNIT: f32 = 5.5;
    layout.content_height = Some((total_weight * REAL_PCT_PER_UNIT).max(100.0));

    let children: Vec<UiNode> = recipes
        .iter()
        .zip(weights)
        .map(|(recipe, w)| recipe_block::block(recipe, w / total_weight * 100.0))
        .collect();

    UiNode::Container(Container::new(layout, theme::BACKGROUND, children))
}
