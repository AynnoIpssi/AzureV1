#[cfg(test)]
mod tests {
    
    
    
    use azure_foundation::window::models::window_context::*;
    use azure_foundation::navigation::managers::intra_navigation_manager;
    use azure_foundation::navigation::models::route_table::RouteTable;
    use azure_rooter::managers::intra_router::IntraRouter;

    #[test]
    fn goto_queues_a_self_addressed_navigation_when_an_intra_router_is_attached() {
        let shared = IntraRouter::new();
        let intra = intra_navigation_manager::connect(&shared, 1);
        let mut ctx = WindowContext { intra: Some(&intra), nav: None, windows: None, routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };

        assert!(ctx.goto("/confirm", ""));

        let routes = RouteTable::new().on("/confirm", |_payload| Vec::new());
        assert!(intra_navigation_manager::poll(&intra, &routes).is_some());
    }

    #[test]
    fn goto_without_an_intra_router_does_nothing() {
        let mut ctx = WindowContext { intra: None, nav: None, windows: None, routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(!ctx.goto("/confirm", ""));
    }

    #[test]
    fn navigate_to_without_a_navigation_client_returns_an_error() {
        let mut ctx = WindowContext { intra: None, nav: None, windows: None, routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.navigate_to(2, "/update-text", "salut").is_err());
    }

    #[test]
    fn open_window_without_a_window_table_does_nothing() {
        let mut ctx = WindowContext { intra: None, nav: None, windows: None, routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.open_window("/settings", "").is_err());
    }

    #[test]
    fn open_window_with_an_unregistered_path_does_nothing() {
        use azure_foundation::window::models::window::AzureWindow;
        use azure_foundation::window::models::window_table::WindowTable;

        let windows = WindowTable::new().on("/settings", |_payload| AzureWindow::new("Reglages"));
        let mut ctx = WindowContext { intra: None, nav: None, windows: Some(&windows), routes: None, stockage: None, app_id: Some(1), clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.open_window("/missing", "").is_err());
    }

    // Les tests ci-dessous ne lancent jamais de vraie fenetre : chaque cas
    // est refuse AVANT le `thread::spawn` de `open_window`.
    use azure_core::models::window_model::*;
    use azure_foundation::window::models::window::AzureWindow;
    use azure_foundation::window::models::window_table::WindowTable;

    fn size() -> WindowSize {
        WindowSize::new(400, 300).unwrap()
    }

    #[test]
    fn open_window_from_a_window_without_spec_is_refused() {
        let windows = WindowTable::new().on("/settings", |_payload| AzureWindow::new("Reglages").spec(WindowSpec::internal(1, size())));
        let mut ctx = WindowContext { intra: None, nav: None, windows: Some(&windows), routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.open_window("/settings", "").is_err());
    }

    #[test]
    fn open_window_refuses_a_window_without_spec() {
        let windows = WindowTable::new().on("/settings", |_payload| AzureWindow::new("Reglages"));
        let mut ctx = WindowContext { intra: None, nav: None, windows: Some(&windows), routes: None, stockage: None, app_id: Some(1), clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.open_window("/settings", "").is_err());
    }

    #[test]
    fn open_window_refuses_a_window_owned_by_another_app() {
        let windows = WindowTable::new().on("/settings", |_payload| AzureWindow::new("Reglages").spec(WindowSpec::internal(2, size())));
        let mut ctx = WindowContext { intra: None, nav: None, windows: Some(&windows), routes: None, stockage: None, app_id: Some(1), clicked: None, values: None, scroll_request: None, effects: Default::default() };
        let err = ctx.open_window("/settings", "").unwrap_err();
        assert!(err.contains("app 2"), "{err}");
    }

    #[test]
    fn open_window_refuses_an_external_window() {
        let windows = WindowTable::new().on("/share", |_payload| {
            let spec = WindowSpec::new(1, size(), WindowState::Active, WindowScope::Followers, WindowKind::External).unwrap();
            AzureWindow::new("Partage").spec(spec)
        });
        let mut ctx = WindowContext { intra: None, nav: None, windows: Some(&windows), routes: None, stockage: None, app_id: Some(1), clicked: None, values: None, scroll_request: None, effects: Default::default() };
        let err = ctx.open_window("/share", "").unwrap_err();
        assert!(err.contains("External"), "{err}");
    }

    #[test]
    fn spec_size_wins_over_fullscreen() {
        let window = AzureWindow::new("Reglages").spec(WindowSpec::internal(1, size())).fullscreen();
        assert_eq!(window.window_spec().unwrap().size(), size());
    }

    #[test]
    fn goto_route_navigates_to_the_named_route_with_its_params() {
        let shared = IntraRouter::new();
        let intra = intra_navigation_manager::connect(&shared, 1);
        let routes = RouteTable::new()
            .route("/user/{id}", |r| {
                assert_eq!(r.param("id"), Some("42"));
                assert_eq!(r.payload, "menu");
                Vec::new()
            })
            .name("user.show");
        let mut ctx = WindowContext { intra: Some(&intra), nav: None, windows: None, routes: Some(&routes), stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };

        assert_eq!(ctx.goto_route("user.show", &[("id", "42")], "menu"), Ok(()));
        assert!(intra_navigation_manager::poll(&intra, &routes).is_some());
    }

    #[test]
    fn goto_route_refuses_unknown_names_missing_params_and_missing_tables() {
        let shared = IntraRouter::new();
        let intra = intra_navigation_manager::connect(&shared, 1);
        let routes = RouteTable::new().route("/user/{id}", |_| Vec::new()).name("user.show");

        let mut ctx = WindowContext { intra: Some(&intra), nav: None, windows: None, routes: Some(&routes), stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.goto_route("inconnue", &[], "").is_err());
        assert!(ctx.goto_route("user.show", &[], "").is_err());

        let mut ctx = WindowContext { intra: Some(&intra), nav: None, windows: None, routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.goto_route("user.show", &[("id", "1")], "").is_err());

        let mut ctx = WindowContext { intra: None, nav: None, windows: None, routes: Some(&routes), stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.goto_route("user.show", &[("id", "1")], "").is_err());

        assert!(intra_navigation_manager::poll(&intra, &routes).is_none(), "aucune navigation ne doit partir apres un refus");
    }

    #[test]
    fn goto_view_route_is_resolved_by_the_target_view() {
        let shared = IntraRouter::new();
        let view_a = intra_navigation_manager::connect(&shared, 1);
        let view_b = intra_navigation_manager::connect(&shared, 2);
        // A n'a AUCUNE route : seul B connait "user.show".
        let mut ctx = WindowContext { intra: Some(&view_a), nav: None, windows: None, routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        ctx.goto_view_route(2, "user.show", &[("id", "7")], "depuis A").unwrap();

        let routes_b = RouteTable::new()
            .route("/user/{id}", |r| {
                assert_eq!((r.path.as_str(), r.param("id"), r.payload.as_str()), ("/user/7", Some("7"), "depuis A"));
                Vec::new()
            })
            .name("user.show");
        assert!(intra_navigation_manager::poll(&view_b, &routes_b).is_some());
        assert!(intra_navigation_manager::poll(&view_a, &routes_b).is_none(), "A ne recoit rien");
    }

    #[test]
    fn goto_view_route_refuses_reserved_characters_and_missing_router() {
        let shared = IntraRouter::new();
        let view_a = intra_navigation_manager::connect(&shared, 1);
        let mut ctx = WindowContext { intra: Some(&view_a), nav: None, windows: None, routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.goto_view_route(2, "user\u{1F}show", &[], "").is_err());
        assert!(ctx.goto_view_route(2, "user.show", &[("i=d", "7")], "").is_err());

        let mut ctx = WindowContext { intra: None, nav: None, windows: None, routes: None, stockage: None, app_id: None, clicked: None, values: None, scroll_request: None, effects: Default::default() };
        assert!(ctx.goto_view_route(2, "user.show", &[], "").is_err());
        assert!(ctx.navigate_to_route(2, "user.show", &[], "").is_err(), "pas de routeur inter-app branche");
    }
}
