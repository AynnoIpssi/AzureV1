pub trait AzureRouterProvider {
    fn send(&self, sender_id: u32, receiver_id: u32, message: &str);
    fn receive(&self, receiver_id: u32) -> Option<String>;
    fn register(&mut self, app_id: u32);
}