#[cfg(test)]
mod tests {
    
    use azure_foundation::navigation::models::route::Route;
    
    use azure_foundation::navigation::models::route_table::*;

    #[test]
    fn resolve_calls_the_handler_registered_for_the_route_path() {
        let table = RouteTable::new().on("/home", |_payload| Vec::new());
        let resolved = table.resolve(&Route::new("/home", ""));
        assert!(resolved.is_some());
    }

    #[test]
    fn resolve_returns_none_for_an_unregistered_path() {
        let table = RouteTable::new().on("/home", |_payload| Vec::new());
        let resolved = table.resolve(&Route::new("/missing", ""));
        assert!(resolved.is_none());
    }
}
