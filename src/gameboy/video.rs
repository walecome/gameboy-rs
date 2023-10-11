use super::address::Address;

const DOTS_PER_LINE: usize = 456;
const DOTS_PER_FRAME: usize = 70224;
const DOTS_OAM_SCAN: usize = 80;
const MIN_DOTS_DRAW_PIXELS: usize = 172;

struct Point {
    x: usize,
    y: usize,
}

pub struct Lcd {
    scy: u8,
    scx: u8,
}

impl Lcd {
    fn new() -> Self {
        Self { scy: 0, scx: 0 }
    }

    pub fn read(&self, _select_byte: u8) -> u8 {
        todo!();
    }

    pub fn write(&mut self, _select_byte: u8, _value: u8) {
        todo!();
    }
}

#[derive(Debug, PartialEq)]
enum VideoMode {
    OamScan,
    DrawPixels,
    HorizontalBlank,
    VerticalBlank,
}

pub struct Video {
    vram: Vec<u8>,
    current_mode: VideoMode,
    ly: usize,
    current_dot: usize,
    lcd: Lcd,
}

impl Video {
    pub fn new() -> Self {
        Self {
            vram: vec![0x00; 0x4000],
            current_mode: VideoMode::OamScan,
            ly: 0,
            current_dot: 0,
            lcd: Lcd::new(),
        }
    }

    pub fn tick(&mut self, elapsed_cycles: usize) {
        self.current_dot += elapsed_cycles;

        let next_mode = if let Some(mode) = self.get_mode_transition() {
            mode
        } else {
            return;
        };

        match next_mode {
            VideoMode::OamScan => {
                assert_eq!(self.current_mode, VideoMode::VerticalBlank);
                // TODO: Implement
            },
            VideoMode::DrawPixels => {
                assert_eq!(self.current_mode, VideoMode::OamScan);
                // TODO: Implement
            },
            VideoMode::HorizontalBlank => {
                assert_eq!(self.current_mode, VideoMode::DrawPixels);
                // TODO: Implement
            },
            VideoMode::VerticalBlank => {
                assert_eq!(self.current_mode, VideoMode::HorizontalBlank);
                // TODO: Implement
            },
        }

        self.current_mode = next_mode;
    }

    fn get_mode_transition(&mut self) -> Option<VideoMode> {
        let point = self.current_point();
        Some(match self.current_mode {
            VideoMode::OamScan => {
                if point.x < DOTS_OAM_SCAN {
                    return None;
                }
                VideoMode::OamScan
            },

            VideoMode::DrawPixels => {
                // TODO: Calculate MODE 3 penalty
                let elapsed_draw_pixels = point.x - DOTS_OAM_SCAN;
                if elapsed_draw_pixels < MIN_DOTS_DRAW_PIXELS {
                    return None
                }
                VideoMode::DrawPixels
            },

            VideoMode::HorizontalBlank => {
                assert!(point.y <= 144);
                if point.y != 144 {
                    return None
                }
                VideoMode::VerticalBlank
            },

            VideoMode::VerticalBlank => {
                if self.current_dot < DOTS_PER_FRAME {
                    return None
                }
                VideoMode::OamScan
            },
        })
    }

    pub fn write_vram(&mut self, address: Address, value: u8) {
        self.vram[address.index_value()] = value;
    }

    pub fn read_vram(&self, address: Address) -> u8 {
        self.vram[address.index_value()]
    }

    pub fn lcd_mut(&mut self) -> &mut Lcd {
        &mut self.lcd
    }

    pub fn lcd(&self) -> &Lcd {
        &self.lcd
    }

    fn current_point(&self) -> Point {
        Point {
            x: self.current_dot / DOTS_PER_LINE,
            y: self.current_dot % DOTS_PER_LINE,
        }
    }
}
