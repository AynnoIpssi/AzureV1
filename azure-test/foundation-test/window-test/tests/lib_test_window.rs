#[cfg(test)]
mod tests {
    use azure_foundation::window::models::window::AzureWindow;

    #[test]
    fn test_window() {
        AzureWindow::new("Mon App")
            .size(800, 600)
            .run();
    }
}