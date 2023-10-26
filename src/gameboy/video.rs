use super::address::Address;
use super::utils::{get_bit, set_bit_mut};

const DOTS_PER_LINE: usize = 456;
const DOTS_PER_FRAME: usize = 70224;
const DOTS_OAM_SCAN: usize = 80;
const MIN_DOTS_DRAW_PIXELS: usize = 172;

struct Point {
    x: usize,
    y: usize,
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum VideoMode {
    Mode2OamScan = 2,
    Mode3DrawPixels = 3,
    Mode0HorizontalBlank = 0,
    Mode1VerticalBlank = 1,
}

struct LcdStatus {
    data: u8,
    ppu_mode: VideoMode,
}

enum LcdStatusBit {
    LyCompare = 2,
    Mode0IntSelect = 3,
    Mode1IntSelect = 4,
    Mode2IntSelect = 5,
    LycIntSelect = 6,
}

impl LcdStatus {
    fn new() -> Self {
        Self {
            data: 0,
            ppu_mode: VideoMode::Mode2OamScan,
        }
    }

    fn get_field(&mut self, bit: LcdStatusBit) -> bool {
        get_bit(self.data, bit as u8)
    }

    fn get_ppu_mode(&self) -> VideoMode {
        self.ppu_mode
    }

    fn set_ppu_mode(&mut self, mode: VideoMode) {
        self.ppu_mode = mode;
    }

    fn set_lyc_condition(&mut self, lyc_is_ly: bool) {
        set_bit_mut(&mut self.data, LcdStatusBit::LyCompare as u8, lyc_is_ly)
    }

    fn read_as_byte(&self) -> u8 {
        return self.data | self.ppu_mode as u8
    }

    fn write_as_byte(&mut self, value: u8) {
        // Only bits 3 to 6 are writable
        let masked_value = value & 0b0111_1000;
        self.data = masked_value;
    }
}

struct LcdControl {
    data: u8,
}

enum LcdControlBit {
    BgWindowEnablePriority = 0,
    ObjEnable = 1,
    ObjSize = 2,
    BgTileMap = 3,
    BgAndWindowTiles = 4,
    WindowEnable = 5,
    WindowTileMap = 6,
    LcdAndPpuEnable = 7,
}

impl LcdControl {
    fn new() -> Self {
        Self {
            data: 0,
        }
    }

    fn get_field(&self, field: LcdControlBit) -> bool {
        get_bit(self.data, field as u8)
    }
}

#[derive(Copy, Clone)]
enum BgColor {
    White = 0,
    LightGray = 1,
    DarkGray = 2,
    Black = 3,
}

struct BgPalette {
    id0: BgColor,
    id1: BgColor,
    id2: BgColor,
    id3: BgColor,
}

fn map_bg_color(value: u8) -> BgColor {
    match value {
        0 => BgColor::White,
        1 => BgColor::LightGray,
        2 => BgColor::DarkGray,
        3 => BgColor::Black,
        _ => panic!("Invalid bg color"),
    }
}

impl BgPalette {
    fn new() -> Self {
        Self {
            id0: BgColor::White,
            id1: BgColor::White,
            id2: BgColor::White,
            id3: BgColor::White,
        }
    }

    fn write_as_byte(&mut self, value: u8) {
        self.id0 = map_bg_color((value & 0b0000_0011) >> 0);
        self.id1 = map_bg_color((value & 0b0000_1100) >> 2);
        self.id2 = map_bg_color((value & 0b0011_0000) >> 4);
        self.id3 = map_bg_color((value & 0b1100_0000) >> 6);
    }

    fn read_as_byte(&self) -> u8 {
        ((self.id3 as u8) << 6) | ((self.id2 as u8) << 4) | ((self.id1 as u8) << 2) | self.id0 as u8
    }
}

pub struct Video {
    vram: Vec<u8>,
    lyc: u8,

    lcd_status: LcdStatus,
    lcd_control: LcdControl,
    scy: u8,
    scx: u8,
    bg_palette: BgPalette,

    // internal
    current_dot: usize,
}

impl Video {
    pub fn new() -> Self {
        Self {
            vram: vec![0x00; 0x4000],
            lcd_status: LcdStatus::new(),
            lcd_control: LcdControl::new(),
            lyc: 0,
            current_dot: 0,
            scy: 0,
            scx: 0,
            bg_palette: BgPalette::new(),
        }
    }

    pub fn tick(&mut self, elapsed_cycles: usize) {
        self.current_dot += elapsed_cycles;

        if !self.is_current_mode_ending() {
            return;
        }

        let point = self.current_point();

        // TODO: Should this only be set after drawing pixels?
        let lyc_is_ly = point.y as u8 == self.lyc;
        self.lcd_status.set_lyc_condition(lyc_is_ly);
        if lyc_is_ly && self.lcd_status.get_field(LcdStatusBit::LycIntSelect) {
            todo!("Trigger STAT interrupt");
        }

        let previous_mode = self.lcd_status.get_ppu_mode();

        let next_mode = match previous_mode {
            VideoMode::Mode2OamScan => VideoMode::Mode3DrawPixels,
            VideoMode::Mode3DrawPixels => VideoMode::Mode0HorizontalBlank,
            VideoMode::Mode0HorizontalBlank => {
                if point.y >= 144 {
                    VideoMode::Mode1VerticalBlank
                } else {
                    VideoMode::Mode2OamScan
                }
            }
            VideoMode::Mode1VerticalBlank => VideoMode::Mode2OamScan,
        };

        self.lcd_status.set_ppu_mode(next_mode);

        match next_mode {
            VideoMode::Mode2OamScan => {
                if self.lcd_status.get_field(LcdStatusBit::Mode2IntSelect) {
                    todo!("Trigger STAT interrupt");
                }
            },

            VideoMode::Mode3DrawPixels => {

            },

            VideoMode::Mode0HorizontalBlank => {
                if self.lcd_status.get_field(LcdStatusBit::Mode0IntSelect) {
                    todo!("Trigger STAT interrupt");
                }
            },

            VideoMode::Mode1VerticalBlank => {
                if self.lcd_status.get_field(LcdStatusBit::Mode1IntSelect) {
                    todo!("Trigger STAT interrupt");
                }
            },
        }
    }

    fn is_current_mode_ending(&self) -> bool {
        let point = self.current_point();

        return match self.lcd_status.get_ppu_mode() {
            VideoMode::Mode2OamScan => point.x >= DOTS_OAM_SCAN,

            VideoMode::Mode3DrawPixels => {
                // TODO: Calculate MODE 3 penalty
                let elapsed_draw_pixels = point.x - DOTS_OAM_SCAN;
                elapsed_draw_pixels >= MIN_DOTS_DRAW_PIXELS
            },

            VideoMode::Mode0HorizontalBlank => {
                assert!(point.y <= 144);
                point.y >= 144
            },

            VideoMode::Mode1VerticalBlank => {
                self.current_dot >= DOTS_PER_FRAME
            },
        }
    }

    pub fn write_vram(&mut self, address: Address, value: u8) {
        self.vram[address.index_value()] = value;
    }

    pub fn read_vram(&self, address: Address) -> u8 {
        self.vram[address.index_value()]
    }

    pub fn read_register(&self, select_byte: u8) -> u8 {
        match select_byte {
            0x40 => self.lcd_control.data,
            0x41 => self.lcd_status.read_as_byte(),
            0x42 => self.scy,
            0x43 => self.scx,
            0x44 => self.current_point().y as u8,
            0x45 => self.lyc,
            0x46 => panic!("Should be handled by MMU"),
            0x47 => self.bg_palette.read_as_byte(),
            _ => todo!()
        }
    }

    pub fn write_register(&mut self, select_byte: u8, value: u8) {
        match select_byte {
            0x40 => self.lcd_control.data = value,
            0x41 => self.lcd_status.write_as_byte(value),
            0x42 => self.scy = value,
            0x43 => self.scx = value,
            0x44 => panic!("Trying to write to LY"),
            0x45 => self.lyc = value,
            0x46 => panic!("Should be handled by MMU"),
            0x47 => self.bg_palette.write_as_byte(value),
            _ => todo!(),
        }
    }

    fn current_point(&self) -> Point {
        Point {
            x: self.current_dot / DOTS_PER_LINE,
            y: self.current_dot % DOTS_PER_LINE,
        }
    }
}
