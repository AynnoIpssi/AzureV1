use crate::rules::window_event::WindowEvent;

pub trait AzureWindowProvider {
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn render(&mut self, pixels: &[u8]) -> Result<(), String>;
    fn poll_event(&mut self) -> Option<WindowEvent>;

}