use crate::rendering::models::color::Color;

pub fn get_pixel_index(x: u32, y: u32, width: u32) -> usize {
    ((y * width + x) * 4) as usize
}

    pub fn set_pixel(buffer: &mut [u8], x: u32, y: u32, width: u32, height: u32, color: &Color) {

        if x >= width || y >= height {
            return;
        }

        let index = get_pixel_index(x, y, width);
        // Wayland's wl_shm buffer is ARGB8888, stored little-endian as B,G,R,A
        buffer[index] = color.b;
        buffer[index + 1] = color.g;
        buffer[index +2] = color.r;
        buffer[index + 3] = color.a;
    }