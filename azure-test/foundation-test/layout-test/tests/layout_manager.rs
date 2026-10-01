#[cfg(test)]
mod tests {
    use azure_foundation::layout::models::layout_props::{AlignItems, DisplayMode, FlexDirection, JustifyContent, LayoutProps, Track};
    use azure_foundation::ui::models::ui_node::UiNode;
    use azure_foundation::layout::managers::layout_manager::*;
    use azure_foundation::ui::models::button::Button;
    use azure_foundation::ui::models::container::Container;
    use azure_engine::rendering::models::color::Color;

    fn props(x: f32, y: f32, width: f32, height: f32, margin: f32, padding: f32) -> LayoutProps {
        LayoutProps::new(x, y, width, height, margin, padding)
    }

    fn button(layout: LayoutProps) -> UiNode {
        UiNode::Button(Button::new(layout, Color::new(0, 0, 0, 255), false, String::new()))
    }

    fn container(layout: LayoutProps, children: Vec<UiNode>) -> Container {
        Container::new(layout, Color::new(0, 0, 0, 255), children)
    }

    #[test]
    fn root_element_resolves_as_a_plain_percentage_of_the_window() {
        let window = (0, 0, 200, 100);
        let got = resolve(&props(25.0, 0.0, 50.0, 100.0, 0.0, 0.0), window);
        assert_eq!(got, (50, 0, 100, 100));
    }

    #[test]
    fn nested_percentages_are_relative_to_the_parent_content_box_not_the_window() {
        // Fenetre 200x100. Conteneur occupant la moitie gauche -> boite
        // (0,0,100,100). Un enfant a "x:50%, width:50%" doit occuper la
        // moitie DROITE DU CONTENEUR (donc [50,100) en absolu), pas 50% de
        // la fenetre entiere (qui donnerait [100,150)).
        let window = (0, 0, 200, 100);
        let container = props(0.0, 0.0, 50.0, 100.0, 0.0, 0.0);
        let container_box = resolve(&container, window);
        assert_eq!(container_box, (0, 0, 100, 100));

        let child = props(50.0, 0.0, 50.0, 100.0, 0.0, 0.0);
        let child_box = resolve(&child, content_box(&container, container_box));
        assert_eq!(child_box, (50, 0, 50, 100));
    }

    #[test]
    fn margin_shifts_the_element_without_changing_its_size() {
        let parent = (0, 0, 200, 100);
        let got = resolve(&props(0.0, 0.0, 50.0, 50.0, 10.0, 0.0), parent);
        // margin 10% -> 20px sur l'axe x (10% de 200), 10px sur l'axe y
        // (10% de 100) ; la taille (100x50) est inchangee.
        assert_eq!(got, (20, 10, 100, 50));
    }

    #[test]
    fn padding_shrinks_the_content_box_and_offsets_it() {
        let element = props(0.0, 0.0, 0.0, 0.0, 0.0, 20.0);
        let resolved = (10, 10, 100, 50);
        let got = content_box(&element, resolved);
        // padding 20% -> 20px sur l'axe x (20% de 100), 10px sur l'axe y
        // (20% de 50), retire des DEUX cotes.
        assert_eq!(got, (30, 20, 60, 30));
    }

    #[test]
    fn oversized_padding_yields_an_empty_content_box_instead_of_underflowing() {
        let element = props(0.0, 0.0, 0.0, 0.0, 0.0, 60.0);
        let got = content_box(&element, (0, 0, 10, 10));
        assert_eq!(got, (6, 6, 0, 0));
    }

    #[test]
    fn zero_margin_and_padding_behave_exactly_like_the_previous_window_relative_layout() {
        let window = (0, 0, 800, 600);
        let got = resolve(&props(10.0, 20.0, 30.0, 40.0, 0.0, 0.0), window);
        assert_eq!(got, (80, 120, 240, 240));
    }

    // ---------------------------------------------------------------
    // `resolve_children` - mode `Block` (regression : doit rester
    // identique a resoudre chaque enfant independamment via `resolve`).
    // ---------------------------------------------------------------

    #[test]
    fn block_mode_resolves_each_child_independently_like_before() {
        let parent_layout = props(0.0, 0.0, 100.0, 100.0, 0.0, 0.0); // display: Block par defaut
        let children = vec![button(props(0.0, 0.0, 50.0, 100.0, 0.0, 0.0)), button(props(50.0, 0.0, 50.0, 100.0, 0.0, 0.0))];

        let got = resolve_children(&parent_layout, &children, (0, 0, 200, 100), 0);
        assert_eq!(got, vec![(0, 0, 100, 100), (100, 0, 100, 100)]);
    }

    // ---------------------------------------------------------------
    // Flex
    // ---------------------------------------------------------------

    fn flex_container() -> LayoutProps {
        let mut l = props(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
        l.display = DisplayMode::Flex;
        l
    }

    #[test]
    fn flex_row_shares_free_space_by_flex_grow() {
        let container = flex_container();
        let mut a = props(0.0, 0.0, 0.0, 100.0, 0.0, 0.0);
        a.flex_grow = 1.0;
        let mut b = props(0.0, 0.0, 0.0, 100.0, 0.0, 0.0);
        b.flex_grow = 1.0;

        let got = resolve_children(&container, &[button(a), button(b)], (0, 0, 300, 100), 0);
        assert_eq!(got, vec![(0, 0, 150, 100), (150, 0, 150, 100)]);
    }

    #[test]
    fn flex_justify_content_space_between_pushes_items_to_the_edges() {
        let mut container = flex_container();
        container.justify_content = JustifyContent::SpaceBetween;
        let a = props(0.0, 0.0, 20.0, 100.0, 0.0, 0.0);
        let b = props(0.0, 0.0, 20.0, 100.0, 0.0, 0.0);

        let got = resolve_children(&container, &[button(a), button(b)], (0, 0, 300, 100), 0);
        assert_eq!(got, vec![(0, 0, 60, 100), (240, 0, 60, 100)]);
    }

    #[test]
    fn flex_justify_content_center_centers_the_leftover_space() {
        let mut container = flex_container();
        container.justify_content = JustifyContent::Center;
        let a = props(0.0, 0.0, 50.0, 100.0, 0.0, 0.0);

        let got = resolve_children(&container, &[button(a)], (0, 0, 300, 100), 0);
        assert_eq!(got, vec![(75, 0, 150, 100)]);
    }

    #[test]
    fn flex_shrinks_overflowing_items_proportionally_to_flex_shrink_and_basis() {
        let container = flex_container();
        // 2 elements a 100% chacun (base 200px chacun) dans une boite de
        // 200px de large au total : ils doivent se partager la place a
        // parts egales (memes shrink, memes bases) plutot que de deborder.
        let a = props(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
        let b = props(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);

        let got = resolve_children(&container, &[button(a), button(b)], (0, 0, 200, 100), 0);
        assert_eq!(got, vec![(0, 0, 100, 100), (100, 0, 100, 100)]);
    }

    #[test]
    fn flex_column_direction_stacks_along_the_vertical_axis() {
        let mut container = flex_container();
        container.flex_direction = FlexDirection::Column;
        let mut a = props(0.0, 0.0, 100.0, 0.0, 0.0, 0.0);
        a.flex_grow = 1.0;
        let mut b = props(0.0, 0.0, 100.0, 0.0, 0.0, 0.0);
        b.flex_grow = 1.0;

        let got = resolve_children(&container, &[button(a), button(b)], (0, 0, 100, 200), 0);
        assert_eq!(got, vec![(0, 0, 100, 100), (0, 100, 100, 100)]);
    }

    #[test]
    fn flex_wrap_stacks_overflowing_lines_along_the_cross_axis() {
        let mut container = flex_container();
        container.flex_wrap = true;
        let item = || props(0.0, 0.0, 40.0, 100.0, 0.0, 0.0);

        // 3 elements a 40% (donc 40px sur un axe principal de 100px) : les
        // 2 premiers tiennent sur la meme ligne (80 <= 100), le 3e deborde
        // et passe a la ligne suivante.
        let got = resolve_children(&container, &[button(item()), button(item()), button(item())], (0, 0, 100, 50), 0);
        assert_eq!(got, vec![(0, 0, 40, 50), (40, 0, 40, 50), (0, 50, 40, 50)]);
    }

    #[test]
    fn flex_align_items_center_centers_a_shorter_item_on_the_cross_axis() {
        let mut container = flex_container();
        container.align_items = AlignItems::Center;
        let a = props(0.0, 0.0, 100.0, 50.0, 0.0, 0.0);

        let got = resolve_children(&container, &[button(a)], (0, 0, 100, 100), 0);
        assert_eq!(got, vec![(0, 25, 100, 50)]);
    }

    #[test]
    fn flex_item_margin_insets_the_item_within_its_own_cell() {
        let container = flex_container();
        let mut a = props(0.0, 0.0, 100.0, 100.0, 10.0, 0.0);
        a.flex_grow = 0.0;
        // margin 10% d'une cellule de 100x100 -> 10px retires de chaque cote.
        let got = resolve_children(&container, &[button(a)], (0, 0, 100, 100), 0);
        assert_eq!(got, vec![(10, 10, 80, 80)]);
    }

    // ---------------------------------------------------------------
    // Grid
    // ---------------------------------------------------------------

    fn grid_container(columns: Vec<Track>, rows: Vec<Track>) -> LayoutProps {
        let mut l = props(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
        l.display = DisplayMode::Grid;
        l.grid_template_columns = columns;
        l.grid_template_rows = rows;
        l
    }

    fn cell() -> UiNode {
        button(props(0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
    }

    #[test]
    fn grid_splits_equal_fr_columns_and_auto_places_children_in_order() {
        let container = grid_container(vec![Track::Fr(1.0), Track::Fr(1.0), Track::Fr(1.0)], vec![]);
        let children = vec![cell(), cell(), cell()];

        let got = resolve_children(&container, &children, (0, 0, 300, 90), 0);
        assert_eq!(got, vec![(0, 0, 100, 90), (100, 0, 100, 90), (200, 0, 100, 90)]);
    }

    #[test]
    fn grid_mixes_fixed_percent_and_flexible_fr_columns() {
        let container = grid_container(vec![Track::Percent(20.0), Track::Fr(1.0), Track::Fr(1.0)], vec![]);
        let children = vec![cell(), cell(), cell()];

        let got = resolve_children(&container, &children, (0, 0, 300, 90), 0);
        assert_eq!(got, vec![(0, 0, 60, 90), (60, 0, 120, 90), (180, 0, 120, 90)]);
    }

    #[test]
    fn grid_explicit_column_and_span_places_the_item_across_several_tracks() {
        let container = grid_container(vec![Track::Fr(1.0); 4], vec![]);
        let mut item = props(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        item.grid_column = Some(2);
        item.grid_column_span = 2;

        let got = resolve_children(&container, &[button(item)], (0, 0, 400, 50), 0);
        assert_eq!(got, vec![(100, 0, 200, 50)]);
    }

    #[test]
    fn grid_explicit_column_beyond_the_template_extends_the_grid_instead_of_overlapping() {
        // Gabarit a 2 colonnes egales, mais un enfant demande explicitement
        // la 3e colonne (grid_column: 3) - avant le correctif, elle etait
        // ecrasee dans la 2e colonne (superposee au premier enfant) ; elle
        // doit desormais etendre la grille a 3 colonnes, chacune 1/3 de la
        // largeur (comme la ligne implicite deja geree par `max_row_end`).
        let container = grid_container(vec![Track::Fr(1.0), Track::Fr(1.0)], vec![]);
        let mut first = props(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        first.grid_column = Some(1);
        let mut overflowing = props(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        overflowing.grid_column = Some(3);

        let got = resolve_children(&container, &[button(first), button(overflowing)], (0, 0, 300, 50), 0);
        assert_eq!(got, vec![(0, 0, 100, 50), (200, 0, 100, 50)], "la 3e colonne implicite doit exister, pas se superposer a la 2e");
    }

    #[test]
    fn grid_auto_flow_still_wraps_at_the_template_width_even_when_another_child_overflows_it() {
        // Le flux automatique (aucun placement explicite) doit continuer a
        // passer a la ligne a la largeur du GABARIT (2 colonnes), meme si un
        // AUTRE enfant, place explicitement, a etendu la grille a 3 colonnes -
        // `template_cols` (fixe) ne doit jamais etre confondu avec `n_cols`
        // (etendu) pour cette decision.
        let container = grid_container(vec![Track::Fr(1.0), Track::Fr(1.0)], vec![]);
        let mut explicit = props(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        explicit.grid_column = Some(3);

        let got = resolve_children(&container, &[cell(), cell(), cell(), button(explicit)], (0, 0, 300, 100), 0);
        // 3 cellules en flux automatique sur un gabarit a 2 colonnes -> la
        // 3e passe a la ligne suivante, quelle que soit l'extension causee
        // par le 4e enfant (explicite en colonne 3).
        assert_eq!(got[2].1, 50, "la 3e cellule en flux auto doit passer a la ligne, pas rester sur la 1ere");
    }

    #[test]
    fn grid_auto_flow_wraps_to_the_next_implicit_row_when_columns_run_out() {
        let container = grid_container(vec![Track::Fr(1.0), Track::Fr(1.0)], vec![]);
        let children = vec![cell(), cell(), cell()];

        let got = resolve_children(&container, &children, (0, 0, 200, 100), 0);
        assert_eq!(got, vec![(0, 0, 100, 50), (100, 0, 100, 50), (0, 50, 100, 50)]);
    }

    #[test]
    fn nested_containers_inside_a_flex_row_still_use_their_own_content_box() {
        // Verifie que le conteneur imbrique dans une ligne flex garde son
        // propre systeme de coordonnees pour SES enfants (via `content_box`),
        // exactement comme en mode Block - la boite qu'il recoit de son
        // parent flex n'est qu'un point de depart, pas une fin en soi.
        let mut outer = flex_container();
        outer.gap = 0.0;
        let mut left = props(0.0, 0.0, 0.0, 100.0, 0.0, 0.0);
        left.flex_grow = 1.0;
        let inner = container(props(0.0, 0.0, 50.0, 50.0, 0.0, 0.0), vec![]);

        let outer_children = vec![button(left), UiNode::Container(inner)];
        let boxes = resolve_children(&outer, &outer_children, (0, 0, 200, 100), 0);
        // Le `Container` (2e enfant) recoit toute la moitie droite (100px) -
        // son propre contenu (non teste ici, deja couvert par les tests
        // `Block` existants) continuera de se positionner relativement a
        // CETTE boite, pas a celle du conteneur flex externe.
        assert_eq!(boxes[1].2, 100);
    }
}
