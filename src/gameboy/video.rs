use super::address::Address;
use super::utils::{get_bit, set_bit_mut};

pub const SCREEN_WIDTH: u8 = 160;
pub const SCREEN_HEIGHT: u8 = 144;

const DOTS_PER_LINE: usize = 456;
const DOTS_PER_FRAME: usize = 70224;
const DOTS_OAM_SCAN: usize = 80;
const MIN_DOTS_DRAW_PIXELS: usize = 172;

#[derive(Debug)]
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
        return self.data | self.ppu_mode as u8;
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
    BgWindowEnable = 0,
    ObjEnable = 1,
    ObjSize = 2,
    BgTileMapArea = 3,
    BgAndWindowTileDataArea = 4,
    WindowEnable = 5,
    WindowTileMapArea = 6,
    LcdEnable = 7,
}

impl LcdControl {
    fn new() -> Self {
        Self { data: 0 }
    }

    fn get_field(&self, field: LcdControlBit) -> bool {
        get_bit(self.data, field as u8)
    }
}

#[derive(Copy, Clone)]
enum PaletteColor {
    // TODO: Not sure if this correct. Pandocs specifies:
    // "[OBP] They work exactly like BGP, except that the
    // lower two bits are ignored because color index 0
    // is transparent for OBJs."
    // What's not clear to me is what should be ignored?
    WhiteOrTransparent = 0,
    LightGray = 1,
    DarkGray = 2,
    Black = 3,
}

struct Palette {
    id0: PaletteColor,
    id1: PaletteColor,
    id2: PaletteColor,
    id3: PaletteColor,
}

fn map_palette_color(value: u8) -> PaletteColor {
    match value {
        0 => PaletteColor::WhiteOrTransparent,
        1 => PaletteColor::LightGray,
        2 => PaletteColor::DarkGray,
        3 => PaletteColor::Black,
        _ => panic!("Invalid bg color"),
    }
}

impl Palette {
    fn new() -> Self {
        Self {
            id0: PaletteColor::WhiteOrTransparent,
            id1: PaletteColor::WhiteOrTransparent,
            id2: PaletteColor::WhiteOrTransparent,
            id3: PaletteColor::WhiteOrTransparent,
        }
    }

    fn write_as_byte(&mut self, value: u8) {
        self.id0 = map_palette_color((value & 0b0000_0011) >> 0);
        self.id1 = map_palette_color((value & 0b0000_1100) >> 2);
        self.id2 = map_palette_color((value & 0b0011_0000) >> 4);
        self.id3 = map_palette_color((value & 0b1100_0000) >> 6);
    }

    fn read_as_byte(&self) -> u8 {
        ((self.id3 as u8) << 6) | ((self.id2 as u8) << 4) | ((self.id1 as u8) << 2) | self.id0 as u8
    }

    fn read_color_id(&self, color_id: u8) -> PaletteColor {
        match color_id {
            0 => self.id0,
            1 => self.id1,
            2 => self.id2,
            3 => self.id3,
            _ => panic!("Invalid color ID: {}", color_id),
        }
    }
}

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

    fn new_gray(shade: u8) -> Self {
        RgbColor::new(shade, shade, shade)
    }
}

pub struct FrameBuffer {
    data: Vec<RgbColor>,
    pub width: usize,
    pub height: usize,
}

fn to_screen_color(palette_color: PaletteColor) -> RgbColor {
    match palette_color {
        PaletteColor::WhiteOrTransparent => RgbColor::new_gray(255),
        PaletteColor::LightGray => RgbColor::new_gray(160),
        PaletteColor::DarkGray => RgbColor::new_gray(90),
        PaletteColor::Black => RgbColor::new_gray(0),
    }
}

impl FrameBuffer {
    fn new(width: usize, height: usize) -> Self {
        let pixel_count = width * height;
        Self {
            data: vec![to_screen_color(PaletteColor::WhiteOrTransparent); pixel_count],
            width,
            height,
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> RgbColor {
        let index = y as usize * self.width + x as usize;
        self.data[index]
    }

    fn set_pixel(&mut self, x: u8, y: u8, color: PaletteColor) {
        let index = y as usize * self.width + x as usize;
        self.data[index] = to_screen_color(color);
    }
}

pub struct Video {
    vram: Vec<u8>,
    lyc: u8,

    lcd_status: LcdStatus,
    lcd_control: LcdControl,
    scy: u8,
    scx: u8,
    bg_palette: Palette,
    obj_palette_0: Palette,
    obj_palette_1: Palette,
    window_y: u8,
    window_x: u8,

    // internal
    current_dot: usize,
    oam_access_allowed: bool,
    vram_access_allowed: bool,
    frame_buffer: FrameBuffer,
    is_frame_ready: bool,
    last_line: usize,
}

impl Video {
    pub fn new() -> Self {
        Self {
            vram: vec![0x00; 0x4000],
            lcd_status: LcdStatus::new(),
            lcd_control: LcdControl::new(),
            lyc: 0,
            scy: 0,
            scx: 0,
            bg_palette: Palette::new(),
            obj_palette_0: Palette::new(),
            obj_palette_1: Palette::new(),
            window_y: 0,
            window_x: 0,

            current_dot: 0,
            oam_access_allowed: true,
            vram_access_allowed: true,
            frame_buffer: FrameBuffer::new(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize),
            is_frame_ready: true,
            last_line: 0,
        }
    }

    pub fn try_take_frame(&mut self) -> Option<&FrameBuffer> {
        if !self.is_frame_ready {
            return None;
        }
        self.is_frame_ready = false;
        return Some(&self.frame_buffer);
    }

    pub fn tick(&mut self, elapsed_cycles: usize) {
        self.current_dot += elapsed_cycles;

        let should_do_work = self.is_current_mode_ending();
        let point = self.current_point();

        self.last_line = point.y;

        if !should_do_work {
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
                // As we're exiting HBLANK, it means we're at the next line already.
                self.draw_scanline((point.y - 1) as u8);
                if point.y >= (SCREEN_HEIGHT as usize) {
                    VideoMode::Mode1VerticalBlank
                } else {
                    VideoMode::Mode2OamScan
                }
            }
            VideoMode::Mode1VerticalBlank => {
                self.is_frame_ready = true;
                VideoMode::Mode2OamScan
            }
        };

        self.lcd_status.set_ppu_mode(next_mode);

        match next_mode {
            VideoMode::Mode2OamScan => {
                if self.lcd_status.get_field(LcdStatusBit::Mode2IntSelect) {
                    todo!("Trigger STAT interrupt");
                }
                self.oam_access_allowed = false;
            }

            VideoMode::Mode3DrawPixels => {
                self.vram_access_allowed = false;
            }

            VideoMode::Mode0HorizontalBlank => {
                if self.lcd_status.get_field(LcdStatusBit::Mode0IntSelect) {
                    todo!("Trigger STAT interrupt");
                }
                self.oam_access_allowed = true;
                self.vram_access_allowed = true;
            }

            VideoMode::Mode1VerticalBlank => {
                self.current_dot = self.current_dot % DOTS_PER_FRAME;
                if self.lcd_status.get_field(LcdStatusBit::Mode1IntSelect) {
                    todo!("Trigger STAT interrupt");
                }
            }
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
            }

            VideoMode::Mode0HorizontalBlank => point.y > self.last_line,

            VideoMode::Mode1VerticalBlank => self.current_dot >= DOTS_PER_FRAME,
        };
    }

    pub fn write_vram(&mut self, address: Address, value: u8) {
        if !self.vram_access_allowed {
            // TODO: Log?
            return;
        }
        let index = address.index_value() - 0x8000;
        self.vram[index] = value;
    }

    pub fn read_vram(&self, address: Address) -> u8 {
        if !self.vram_access_allowed {
            // TODO: Log?
            return 0xFF;
        }
        let index = address.index_value() - 0x8000;
        self.vram[index]
    }

    pub fn write_oam(&mut self, address: Address, value: u8) {
        if !self.oam_access_allowed {
            // TODO: Log?
            return;
        }
        let _index = address.index_value() - 0xFE00;
        todo!("Write to OAM");
    }

    pub fn read_oam(&self, address: Address) -> u8 {
        if !self.oam_access_allowed {
            // TODO: Log?
            return 0xFF;
        }
        let _index = address.index_value() - 0xFE00;
        todo!("Read from OAM");
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
            0x48 => self.obj_palette_0.read_as_byte(),
            0x49 => self.obj_palette_1.read_as_byte(),
            0x4A => self.window_y,
            0x4B => self.window_x,
            _ => todo!(),
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
            0x48 => self.obj_palette_0.write_as_byte(value),
            0x49 => self.obj_palette_1.write_as_byte(value),
            0x4A => self.window_y = value,
            0x4B => self.window_x = value,
            _ => todo!(),
        }
    }

    fn current_point(&self) -> Point {
        Point {
            x: self.current_dot % DOTS_PER_LINE,
            y: ((self.current_dot % DOTS_PER_FRAME) / DOTS_PER_LINE),
        }
    }

    fn draw_scanline(&mut self, line: u8) {
        if !self.lcd_control.get_field(LcdControlBit::LcdEnable) {
            return;
        }

        if self.lcd_control.get_field(LcdControlBit::BgWindowEnable) {
            self.draw_bg_for_current_line(line);
            if self.lcd_control.get_field(LcdControlBit::WindowEnable) {
                self.draw_window_for_current_line();
            }
        }
    }

    fn draw_bg_for_current_line(&mut self, line: u8) {
        let y = line;

        for x in 0..SCREEN_WIDTH {
            let tile_index = self.resolve_tile_index(x, y);
            let tile_start_addr = self.resolve_tile_addr(tile_index);

            let x_in_tile = self.scx.wrapping_add(x) % 8;
            let y_in_tile = self.scy.wrapping_add(y) % 8;
            let tile_row_addr = Address::new(tile_start_addr.value() + y_in_tile as u16);

            let color = self.read_tile_pixel_color(tile_row_addr, x_in_tile, &self.bg_palette);
            self.frame_buffer.set_pixel(x, y, color);
        }
    }

    fn draw_window_for_current_line(&mut self) {}

    fn resolve_tile_index(&self, x: u8, y: u8) -> u8 {
        // Background map is 256x256 pixels, i.e. 32x32 tiles (tiles are 8x8 pixel)

        let scrolled_x = self.scx.wrapping_add(x);
        let scrolled_y = self.scy.wrapping_add(y);

        let tile_x = scrolled_x / 8;
        let tile_y = scrolled_y / 8;

        let tile_addr_offset = (tile_y as u16) * 32 + tile_x as u16;

        let tile_map_start_addr: u16 = if self.lcd_control.get_field(LcdControlBit::BgTileMapArea) {
            0x9C00
        } else {
            0x9800
        };

        let tile_index_addr = Address::new(tile_map_start_addr + tile_addr_offset);
        return self.read_vram(tile_index_addr);
    }

    fn resolve_tile_addr(&self, tile_index: u8) -> Address {
        let tile_byte_count: u16 = 16;

        return if self
            .lcd_control
            .get_field(LcdControlBit::BgAndWindowTileDataArea)
        {
            let tile_data_start = 0x8000;
            Address::new(tile_data_start + tile_byte_count * (tile_index as u16))
        } else {
            let tile_data_base = 0x9000 as i32;
            let signed_tile_index = tile_index as i8 as i32;
            let target = tile_data_base + (tile_byte_count as i32) * signed_tile_index;
            Address::new(target as u16)
        };
    }

    fn read_tile_pixel_color(
        &self,
        tile_row_addr: Address,
        x_in_tile: u8,
        palette: &Palette,
    ) -> PaletteColor {
        assert!(x_in_tile < 8);
        let first_byte = self.read_vram(tile_row_addr);
        let second_byte = self.read_vram(tile_row_addr.next());

        let ls_bit_color_id = if get_bit(first_byte, 7 - x_in_tile) {
            1
        } else {
            0
        };
        let ms_bit_color_id = if get_bit(second_byte, 7 - x_in_tile) {
            1
        } else {
            0
        };

        let color_id = ms_bit_color_id << 1 | ls_bit_color_id;

        return palette.read_color_id(color_id);
    }
}
