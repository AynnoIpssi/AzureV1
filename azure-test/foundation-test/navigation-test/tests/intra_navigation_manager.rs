#[cfg(test)]
mod tests {
    
    
    use azure_foundation::navigation::models::route_table::RouteTable;
    use azure_foundation::ui::models::ui_node::UiNode;
    
    use azure_rooter::managers::intra_router::IntraRouter;
    
    use azure_foundation::navigation::managers::intra_navigation_manager::*;

    fn screen(_payload: &str) -> Vec<UiNode> {
        Vec::new()
    }

    #[test]
    fn navigate_to_self_then_poll_resolves_the_registered_screen() {
        let shared = IntraRouter::new();
        let intra = connect(&shared, 1);

        navigate(&intra, intra.view_id, "/confirm", "");

        assert!(poll(&intra, &RouteTable::new().on("/confirm", screen)).is_some());
    }

    #[test]
    fn poll_without_a_pending_message_returns_none() {
        let shared = IntraRouter::new();
        let intra = connect(&shared, 1);

        assert!(poll(&intra, &RouteTable::new().on("/confirm", screen)).is_none());
    }

    #[test]
    fn navigate_reaches_a_different_view_in_the_same_process() {
        let shared = IntraRouter::new();
        let sender = connect(&shared, 1);
        let receiver = connect(&shared, 2);

        navigate(&sender, receiver.view_id, "/notify", "salut");

        assert!(poll(&receiver, &RouteTable::new().on("/notify", screen)).is_some());
        assert!(poll(&sender, &RouteTable::new().on("/notify", screen)).is_none());
    }

    #[test]
    fn drain_consumes_every_pending_message_and_keeps_the_last_resolved_one() {
        let shared = IntraRouter::new();
        let view = connect(&shared, 1);
        let routes = RouteTable::new().on("/known", screen);

        // Le premier message ne resout dans aucune route enregistree -
        // `drain` doit quand meme continuer jusqu'au suivant plutot que de
        // s'arreter la (contrairement a une boucle naive sur `poll`).
        navigate(&view, view.view_id, "/unknown", "");
        navigate(&view, view.view_id, "/known", "");

        assert!(drain(&view, &routes).is_some());
        // Tout a ete consomme : plus rien en attente derriere.
        assert!(poll(&view, &routes).is_none());
    }
}
