use crate::common::framebuffer::{FrameBuffer, RgbColor};

use super::address::Address;
use super::utils::{get_bit, set_bit_mut};

pub const SCREEN_WIDTH: u8 = 160;
pub const SCREEN_HEIGHT: u8 = 144;

const OAM_START: u16 = 0xFE00;
const TILE_BYTE_COUNT: u16 = 16;
const SPRITE_TILE_START: u16 = 0x8000;

const DOTS_PER_MODE2: usize = 80;
const DOTS_PER_MODE3: usize = 172;
const DOTS_PER_MODE0: usize = 204;
const DOTS_PER_MODE1_ROW: usize = 456;

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

enum ObjectSize {
    Size8x8,
    Size8x16,
}

impl ObjectSize {
    fn height_pixels(&self) -> u8 {
        match self {
            ObjectSize::Size8x8 => 8,
            ObjectSize::Size8x16 => 16,
        }
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

    fn get_object_size(&self) -> ObjectSize {
        if self.get_field(LcdControlBit::ObjSize) {
            ObjectSize::Size8x16
        } else {
            ObjectSize::Size8x8
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum PaletteColor {
    White = 0,
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
        0 => PaletteColor::White,
        1 => PaletteColor::LightGray,
        2 => PaletteColor::DarkGray,
        3 => PaletteColor::Black,
        _ => panic!("Invalid bg color"),
    }
}

impl Palette {
    fn new() -> Self {
        Self {
            id0: PaletteColor::White,
            id1: PaletteColor::White,
            id2: PaletteColor::White,
            id3: PaletteColor::White,
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

    fn resolve_for_bg_from_color_id(&self, color_id: u8) -> PaletteColor {
        match color_id {
            0 => self.id0,
            1 => self.id1,
            2 => self.id2,
            3 => self.id3,
            _ => panic!("Invalid color ID: {}", color_id),
        }
    }

    fn resolve_for_sprite_from_color_id(&self, color_id: u8) -> Option<PaletteColor> {
        match color_id {
            0 => None,
            1 => Some(self.id1),
            2 => Some(self.id2),
            3 => Some(self.id3),
            _ => panic!("Invalid color ID: {}", color_id),
        }
    }
}

fn to_screen_color(palette_color: PaletteColor) -> RgbColor {
    match palette_color {
        PaletteColor::White => RgbColor::new_gray(255),
        PaletteColor::LightGray => RgbColor::new_gray(160),
        PaletteColor::DarkGray => RgbColor::new_gray(90),
        PaletteColor::Black => RgbColor::new_gray(0),
    }
}

struct SpriteObject {
    y_pos: u8,
    x_pos: u8,
    tile_index: u8,
    attributes: u8,
    index: u8,
}

enum SpritePalette {
    OBP0,
    OBP1,
}

impl SpriteObject {
    fn resolve_row_in_sprite(&self, line: u8, size: &ObjectSize) -> Option<u8> {
        // Y = Object’s vertical position on the screen + 16.
        let line_with_offset = line + 16;

        if line_with_offset < self.y_pos {
            return None;
        }

        if line_with_offset >= self.y_pos + size.height_pixels() {
            return None;
        }

        let row_in_sprite = line_with_offset - self.y_pos;

        return Some(if self.y_flip() {
            size.height_pixels() - row_in_sprite
        } else {
            row_in_sprite
        });
    }

    fn priority(&self) -> bool {
        get_bit(self.attributes, 7)
    }

    fn y_flip(&self) -> bool {
        get_bit(self.attributes, 6)
    }

    fn x_flip(&self) -> bool {
        get_bit(self.attributes, 5)
    }

    fn dmg_palette(&self) -> SpritePalette {
        if get_bit(self.attributes, 4) {
            SpritePalette::OBP1
        } else {
            SpritePalette::OBP0
        }
    }
}

pub struct Video {
    vram: Vec<u8>,
    oam: Vec<u8>,
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
    current_line: u8,

    // internal
    dot_in_current_mode: usize,
    frame_buffer: FrameBuffer,
    is_frame_ready: bool,
}

pub enum VideoInterrupt {
    Stat,
    VBlank,
}

impl Video {
    pub fn new() -> Self {
        Self {
            vram: vec![0x00; 0x4000],
            oam: vec![0x00; 0xA0],
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
            current_line: 0,

            dot_in_current_mode: 0,
            frame_buffer: FrameBuffer::new(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize),
            is_frame_ready: true,
        }
    }

    pub fn try_take_frame(&mut self) -> Option<&FrameBuffer> {
        if !self.is_frame_ready {
            return None;
        }
        self.is_frame_ready = false;
        return Some(&self.frame_buffer);
    }

    pub fn tick(&mut self) -> Vec<VideoInterrupt> {
        self.dot_in_current_mode += 1;

        let mut interrupts: Vec<VideoInterrupt> = vec![];

        let maybe_next_mode = match self.lcd_status.get_ppu_mode() {
            VideoMode::Mode2OamScan if self.dot_in_current_mode >= DOTS_PER_MODE2 => {
                self.dot_in_current_mode = 0;
                Some(VideoMode::Mode3DrawPixels)
            }

            VideoMode::Mode3DrawPixels if self.dot_in_current_mode >= DOTS_PER_MODE3 => {
                self.dot_in_current_mode = 0;
                self.draw_scanline(self.current_line);
                Some(VideoMode::Mode0HorizontalBlank)
            }

            VideoMode::Mode0HorizontalBlank if self.dot_in_current_mode >= DOTS_PER_MODE0 => {
                self.dot_in_current_mode = 0;
                self.current_line += 1;

                if self.current_line == self.lyc
                    && self.lcd_status.get_field(LcdStatusBit::LycIntSelect)
                {
                    interrupts.push(VideoInterrupt::Stat);
                }

                if self.current_line > 143 {
                    Some(VideoMode::Mode1VerticalBlank)
                } else {
                    Some(VideoMode::Mode2OamScan)
                }
            }

            VideoMode::Mode1VerticalBlank if self.dot_in_current_mode >= DOTS_PER_MODE1_ROW => {
                self.dot_in_current_mode = 0;
                self.current_line += 1;

                if self.current_line > 153 {
                    self.is_frame_ready = true;
                    self.current_line = 0;
                    Some(VideoMode::Mode2OamScan)
                } else {
                    None
                }
            }

            _ => None,
        };

        self.lcd_status
            .set_lyc_condition(self.current_line == self.lyc);

        if let Some(next_mode) = maybe_next_mode {
            self.lcd_status.set_ppu_mode(next_mode);

            match next_mode {
                VideoMode::Mode2OamScan => {
                    if self.lcd_status.get_field(LcdStatusBit::Mode2IntSelect) {
                        interrupts.push(VideoInterrupt::Stat);
                    }
                }

                VideoMode::Mode3DrawPixels => {
                    // TODO: [1] specifies that VRAM / OAM is inaccessible during certain
                    //       modes, but disallowing access to VRAM (write in this case)
                    //       during Mode 3 breaks the boot rom logo. Figure out if we
                    //       need it.
                    // [1]: https://gbdev.io/pandocs/Rendering.html
                }

                VideoMode::Mode0HorizontalBlank => {
                    if self.lcd_status.get_field(LcdStatusBit::Mode0IntSelect) {
                        interrupts.push(VideoInterrupt::Stat);
                    }
                }

                VideoMode::Mode1VerticalBlank => {
                    interrupts.push(VideoInterrupt::VBlank);
                    if self.lcd_status.get_field(LcdStatusBit::Mode1IntSelect) {
                        interrupts.push(VideoInterrupt::Stat);
                    }
                }
            }
        };

        return interrupts;
    }

    pub fn write_vram(&mut self, address: Address, value: u8) {
        let index = address.index_value() - 0x8000;
        self.vram[index] = value;
    }

    pub fn read_vram(&self, address: Address) -> u8 {
        let index = address.index_value() - 0x8000;
        self.vram[index]
    }

    pub fn write_oam(&mut self, address: Address, value: u8) {
        let index = address.index_value() - 0xFE00;
        self.oam[index] = value;
    }

    pub fn read_oam(&self, address: Address) -> u8 {
        let index = address.index_value() - 0xFE00;
        self.oam[index]
    }

    pub fn read_register(&self, address: Address) -> u8 {
        match address.value() {
            0xFF40 => self.lcd_control.data,
            0xFF41 => self.lcd_status.read_as_byte(),
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => {
                if self.lcd_control.get_field(LcdControlBit::LcdEnable) {
                    self.current_line
                } else {
                    0
                }
            }
            0xFF45 => self.lyc,
            0xFF46 => panic!("Should be handled by MMU"),
            0xFF47 => self.bg_palette.read_as_byte(),
            0xFF48 => self.obj_palette_0.read_as_byte(),
            0xFF49 => self.obj_palette_1.read_as_byte(),
            0xFF4A => self.window_y,
            0xFF4B => self.window_x,
            _ => todo!(),
        }
    }

    pub fn write_register(&mut self, address: Address, value: u8) {
        match address.value() {
            0xFF40 => self.lcd_control.data = value,
            0xFF41 => self.lcd_status.write_as_byte(value),
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => panic!("Trying to write to LY"),
            0xFF45 => self.lyc = value,
            0xFF46 => panic!("Should be handled by MMU"),
            0xFF47 => self.bg_palette.write_as_byte(value),
            0xFF48 => self.obj_palette_0.write_as_byte(value),
            0xFF49 => self.obj_palette_1.write_as_byte(value),
            0xFF4A => self.window_y = value,
            0xFF4B => self.window_x = value,
            _ => todo!(),
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

        if self.lcd_control.get_field(LcdControlBit::ObjEnable) {
            self.draw_sprites_for_current_line(line);
        }
    }

    fn draw_bg_for_current_line(&mut self, line: u8) {
        let y = line;

        for x in 0..SCREEN_WIDTH {
            let tile_index = self.resolve_tile_index(x, y);
            let tile_start_addr = self.resolve_tile_addr(tile_index);

            let x_in_tile = self.scx.wrapping_add(x) % 8;
            let y_in_tile = self.scy.wrapping_add(y) % 8;
            let tile_row_byte_count: u16 = 2;
            let tile_row_addr =
                Address::new(tile_start_addr.value() + (y_in_tile as u16) * tile_row_byte_count);

            let color = self.read_bg_tile_pixel_color(tile_row_addr, x_in_tile, &self.bg_palette);
            self.frame_buffer.set_pixel(x, y, to_screen_color(color));
        }
    }

    fn draw_window_for_current_line(&mut self) {
        todo!("Draw window");
    }

    fn draw_sprites_for_current_line(&mut self, line: u8) {
        // https://gbdev.io/pandocs/OAM.html#object-attribute-memory-oam

        // TODO: Should probably take cancelling into account
        // https://gbdev.io/pandocs/pixel_fifo.html#object-fetch-canceling

        let sprite_size = self.lcd_control.get_object_size();

        let mut visible_sprites_with_row = (0..40)
            .map(|index| self.read_sprite_object(index))
            .filter_map(|sprite| {
                if let Some(row_in_sprite) = sprite.resolve_row_in_sprite(line, &sprite_size) {
                    return Some((sprite, row_in_sprite));
                }
                return None;
            })
            .collect::<Vec<_>>();

        // Sprites with the lowest X position should be drawn first,
        // if the X position is the same then index is used.
        visible_sprites_with_row.sort_by_key(|(sprite, _)| (sprite.x_pos, sprite.index));

        // Because of a limitation of hardware, only ten objects can be displayed per scanline.
        visible_sprites_with_row.truncate(10);

        // TODO: Do this in reverse?
        for (sprite, row_in_sprite) in visible_sprites_with_row {
            let sprite_row_start_addr = self.resolve_sprite_row_addr(&sprite, row_in_sprite);

            // From pandocs:
            // X = Object’s horizontal position on the screen + 8.
            let x_start = sprite.x_pos.wrapping_sub(8);

            for current_pixel in 0..8 {
                let x_on_screen = x_start.wrapping_add(current_pixel);

                if x_on_screen >= SCREEN_WIDTH {
                    continue;
                }

                let index_in_sprite: u8 = if sprite.x_flip() {
                    7 - current_pixel
                } else {
                    current_pixel
                };

                let color_id = self.read_color_id(sprite_row_start_addr, index_in_sprite);

                let palette = match sprite.dmg_palette() {
                    SpritePalette::OBP0 => &self.obj_palette_0,
                    SpritePalette::OBP1 => &self.obj_palette_1,
                };

                let maybe_color = palette.resolve_for_sprite_from_color_id(color_id);
                if maybe_color.is_none() {
                    continue;
                }

                // Pandocs:
                // Priority: 0 = No, 1 = BG and Window colors 1–3 are drawn over this OBJ
                let bg_has_priority = sprite.priority();
                if !bg_has_priority || self.frame_buffer.get_pixel(x_on_screen as usize, line as usize) == to_screen_color(PaletteColor::White) {
                    self.frame_buffer.set_pixel(x_on_screen, line, to_screen_color(maybe_color.unwrap()));
                }
            }
        }
    }

    fn resolve_sprite_row_addr(&self, sprite: &SpriteObject, row: u8) -> Address {
        let row_byte_count = 2;
        Address::new(
            SPRITE_TILE_START
                + (sprite.tile_index as u16) * TILE_BYTE_COUNT
                + (row as u16) * row_byte_count,
        )
    }

    fn read_sprite_object(&self, index: u8) -> SpriteObject {
        // Sprite objects are 4 bytes
        let oam_addr = Address::new(OAM_START + (index as u16 * 4));
        let y_pos = self.read_oam(oam_addr.plus(0));
        let x_pos = self.read_oam(oam_addr.plus(1));
        let tile_index = self.read_oam(oam_addr.plus(2));
        let attributes = self.read_oam(oam_addr.plus(3));

        SpriteObject {
            y_pos,
            x_pos,
            tile_index,
            attributes,
            index,
        }
    }

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
        return if self
            .lcd_control
            .get_field(LcdControlBit::BgAndWindowTileDataArea)
        {
            let tile_data_start = 0x8000;
            Address::new(tile_data_start + TILE_BYTE_COUNT * (tile_index as u16))
        } else {
            let tile_data_base = 0x9000 as i32;
            let signed_tile_index = tile_index as i8 as i32;
            let target = tile_data_base + (TILE_BYTE_COUNT as i32) * signed_tile_index;
            Address::new(target as u16)
        };
    }

    fn read_color_id(&self, tile_row_addr: Address, x_in_tile: u8) -> u8 {
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

        return ms_bit_color_id << 1 | ls_bit_color_id;
    }

    fn read_bg_tile_pixel_color(
        &self,
        tile_row_addr: Address,
        x_in_tile: u8,
        palette: &Palette,
    ) -> PaletteColor {
        let color_id = self.read_color_id(tile_row_addr, x_in_tile);
        return palette.resolve_for_bg_from_color_id(color_id);
    }
}
