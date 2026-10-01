pub struct Seat {
    pub seat_id: u32,
}

impl Seat {
    pub fn new(seat_id: u32) -> Seat{
        Seat{
            seat_id
        }
    }
}