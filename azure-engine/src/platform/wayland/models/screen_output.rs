pub struct ScreenOutput {
    pub width: i32,
    pub height: i32,
}

impl  ScreenOutput {
    pub fn new(width: i32, height: i32) -> ScreenOutput {
        ScreenOutput { 
            width,
            height 
        }
    }
}