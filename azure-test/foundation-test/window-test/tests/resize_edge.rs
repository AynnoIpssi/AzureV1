#[cfg(test)]
mod tests {
    use azure_foundation::window::models::resize_edge::*;

    #[test]
    fn corners_take_priority_over_single_edges() {
        assert_eq!(resize_edge_at(800, 600, 0, 0), Some(ResizeEdge::TopLeft));
        assert_eq!(resize_edge_at(800, 600, 799, 0), Some(ResizeEdge::TopRight));
        assert_eq!(resize_edge_at(800, 600, 0, 599), Some(ResizeEdge::BottomLeft));
        assert_eq!(resize_edge_at(800, 600, 799, 599), Some(ResizeEdge::BottomRight));
    }

    #[test]
    fn single_edges_are_found_away_from_corners() {
        assert_eq!(resize_edge_at(800, 600, 400, 0), Some(ResizeEdge::Top));
        assert_eq!(resize_edge_at(800, 600, 400, 599), Some(ResizeEdge::Bottom));
        assert_eq!(resize_edge_at(800, 600, 0, 300), Some(ResizeEdge::Left));
        assert_eq!(resize_edge_at(800, 600, 799, 300), Some(ResizeEdge::Right));
    }

    #[test]
    fn the_middle_of_the_window_is_not_a_resize_edge() {
        assert_eq!(resize_edge_at(800, 600, 400, 300), None);
    }

    #[test]
    fn points_outside_the_window_are_ignored() {
        assert_eq!(resize_edge_at(800, 600, -1, 300), None);
        assert_eq!(resize_edge_at(800, 600, 800, 300), None);
        assert_eq!(resize_edge_at(800, 600, 400, 600), None);
    }
}
