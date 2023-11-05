#[derive(Clone, Copy)]

pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn new_gray(shade: u8) -> Self {
        RgbColor::new(shade, shade, shade)
    }

    pub fn white() -> Self {
        RgbColor::new(0xFF, 0xFF, 0xFF)
    }
}

pub struct FrameBuffer {
    data: Vec<RgbColor>,
    pub width: usize,
    pub height: usize,
}

impl FrameBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let pixel_count = width * height;
        Self {
            data: vec![RgbColor::white(); pixel_count],
            width,
            height,
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> RgbColor {
        let index = y as usize * self.width + x as usize;
        self.data[index]
    }

    pub fn set_pixel(&mut self, x: u8, y: u8, color: RgbColor) {
        let index = y as usize * self.width + x as usize;
        self.data[index] = color;
    }
}
