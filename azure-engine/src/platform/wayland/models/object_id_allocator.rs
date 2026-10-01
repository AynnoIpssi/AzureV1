pub struct ObjectIdAllocator {
    next: u32,
}

impl Default for ObjectIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectIdAllocator {
    pub fn new() -> Self { ObjectIdAllocator { next: 4 } }
    pub fn next_id(&mut self) -> u32 {
        let id = self.next;
        self.next += 1;
        id
    }
}