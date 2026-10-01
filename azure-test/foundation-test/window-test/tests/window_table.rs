#[cfg(test)]
mod tests {
    
    use azure_foundation::window::models::window::AzureWindow;
    use azure_foundation::window::models::window_table::*;

    #[test]
    fn resolve_calls_the_handler_registered_for_the_path() {
        let table = WindowTable::new().on("/settings", |_payload| AzureWindow::new("Reglages"));
        assert!(table.resolve("/settings", "").is_some());
    }

    #[test]
    fn resolve_returns_none_for_an_unregistered_path() {
        let table = WindowTable::new().on("/settings", |_payload| AzureWindow::new("Reglages"));
        assert!(table.resolve("/missing", "").is_none());
    }
}
