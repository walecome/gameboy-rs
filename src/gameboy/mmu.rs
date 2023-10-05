use super::address::Address;
use super::cartridge::Cartridge;
use super::video::Video;

pub struct Word {
    pub value: u16,
}

impl Word {
    pub fn new(value: u16) -> Self {
        Self { value }
    }

    fn compose_new(high: u8, low: u8) -> Self {
        Self {
            value: ((high as u16) << 8) | low as u16,
        }

    }

    fn low(&self) -> u8 {
        (self.value & 0xFF) as u8
    }

    fn high(&self) -> u8 {
        ((self.value & 0xFF00) >> 8) as u8
    }
}

pub struct IO {
    joypad_input: u8,
    serial_transfer: (u8, u8),
    timer_and_divider: Vec<u8>,
    audio: Vec<u8>,
    wave_pattern: Vec<u8>,
    lcd: Vec<u8>,
    vram_bank_select: u8,
    boot_rom_disabled: u8,
    vram_dma: Vec<u8>,
    bg_obj_palettes: Vec<u8>,
    wram_bank_select: u8,
}

fn byte_vec_for_range(
    range_inclusive_start: u16,
    range_inclusive_end: u16,
) -> Vec<u8> {
    let size = (range_inclusive_end - range_inclusive_start + 1)  as usize;
    vec![0x00; size]
}

impl IO {
    fn new() -> Self {
        Self {
            joypad_input: 0x00,
            serial_transfer: (0x00, 0x00),
            timer_and_divider: byte_vec_for_range(0xFF00, 0xFF07),
            audio: byte_vec_for_range(0xFF10, 0xFF26),
            wave_pattern: byte_vec_for_range(0xFF30, 0xFF3F),
            lcd: byte_vec_for_range(0xFF40, 0xFF4B),
            vram_bank_select: 0x00,
            boot_rom_disabled: 0x00,
            vram_dma: byte_vec_for_range(0xFF51, 0xFF55),
            bg_obj_palettes: byte_vec_for_range(0xFF68, 0xFF6B),
            wram_bank_select: 0x00,
        }
    }

    fn read(&self, address: Address) -> u8 {
        let select_byte: u8 = match address.value() {
            0xFF00..=0xFF70 => (address.value() & 0xFF) as u8,
            _ => panic!("Trying to read IO outside mapped area: {:#06X}", address.value()),
        };

        match select_byte {
            0x00 => self.joypad_input,
            0x01 => self.serial_transfer.0,
            0x02 => self.serial_transfer.1,
            0x04..=0x07 => self.timer_and_divider[(select_byte - 0x04) as usize],
            0x10..=0x26 => self.audio[(select_byte - 0x10) as usize],
            0x30..=0x3F => self.wave_pattern[(select_byte - 0x30) as usize],
            0x40..=0x4B => self.lcd[(select_byte - 0x40) as usize],
            0x4F => self.vram_bank_select,
            0x50 => self.boot_rom_disabled,
            0x51..=0x55 => self.vram_dma[(select_byte - 0x51) as usize],
            0x68..=0x6B => self.bg_obj_palettes[(select_byte - 0x68) as usize],
            0x70 => self.wram_bank_select,
            _ => panic!("Read for unmapped IO address: {:#06X}", address.value()),
        }
    }

    fn write(& mut self, address: Address, value: u8) {
        let select_byte: u8 = match address.value() {
            0xFF00..=0xFF70 => (address.value() & 0xFF) as u8,
            _ => panic!("Trying to write IO outside mapped area: {:#06X}", address.value()),
        };

        let target: &mut u8 = &mut match select_byte {
            0x00 => self.joypad_input,
            0x01 => self.serial_transfer.0,
            0x02 => self.serial_transfer.1,
            0x04..=0x07 => self.timer_and_divider[(select_byte - 0x04) as usize],
            0x10..=0x26 => self.audio[(select_byte - 0x10) as usize],
            0x30..=0x3F => self.wave_pattern[(select_byte - 0x30) as usize],
            0x40..=0x4B => self.lcd[(select_byte - 0x40) as usize],
            0x4F => self.vram_bank_select,
            0x50 => self.boot_rom_disabled,
            0x51..=0x55 => self.vram_dma[(select_byte - 0x51) as usize],
            0x68..=0x6B => self.bg_obj_palettes[(select_byte - 0x68) as usize],
            0x70 => self.wram_bank_select,
            _ => panic!("Write for unmapped IO address: {:#06X}", address.value()),
        };

        *target = value;
    }
}

pub struct MMU {
    cartridge: Box<dyn Cartridge>,
    video: Video,
    internal_ram: Vec<u8>,
    io: IO,
    high_ram: Vec<u8>,
    interrupt_enable: u8,
    interrupt_flags: u8,
}

impl MMU {
    pub fn new(cartridge: Box<dyn Cartridge>) -> MMU {
        MMU {
            cartridge,
            video: Video::new(),
            internal_ram: vec![0x00; 0x3000],
            io: IO::new(),
            high_ram: vec![0x00; 0x80],
            interrupt_enable: 0x00,
            interrupt_flags: 0x00,
        }
    }

    pub fn read(&self, address: Address) -> u8 {
        if address.value() == 0xFF0F {
            return self.interrupt_flags;
        }

        match address.value() {
            0x0000..=0x7FFF => self.cartridge.read(address),
            0x8000..=0x9FFF => todo!("Read VRAM"),
            0xA000..=0xBFFF => todo!("Read from cartridge RAM"),
            0xC000..=0xDFFF => self.internal_ram[address.index_value() - 0xC000],
            0xE000..=0xFDFF => panic!("Read access for prohibited memory area"),
            0xFE00..=0xFE9F => todo!("Read OAM"),
            0xFEA0..=0xFEFF => panic!("Read access for prohibited memory area"),
            0xFF00..=0xFF7F => self.io.read(address),
            0xFF80..=0xFFFE => self.high_ram[address.index_value() - 0xFF80],
            0xFFFF => self.interrupt_enable,
        }
    }

    pub fn read_word(&self, address: Address) -> Word {
        let low = self.read(address);
        let high = self.read(address.next());

        Word::compose_new(high, low)
    }

    pub fn write(&mut self, address: Address, value: u8) {
        if address.value() == 0xFF0F {
            self.interrupt_flags = value;
            return;
        }

        match address.value() {
            0x0000..=0x3FFF => self.cartridge.write(address, value),
            0x4000..=0x7FFF => todo!("Write to cartridge (switchable bank)"),
            0x8000..=0x9FFF => self.video.write_vram(Address::new(address.value() - 0x8000), value),
            0xA000..=0xBFFF => todo!("Write to cartridge RAM"),
            0xC000..=0xDFFF => self.internal_ram[address.index_value() - 0xC000] = value,
            0xE000..=0xFDFF => panic!("Write access for prohibited memory area"),
            0xFE00..=0xFE9F => todo!("Write OAM"),
            0xFEA0..=0xFEFF => panic!("Write access for prohibited memory area"),
            0xFF00..=0xFF7F => self.io.write(address, value),
            0xFF80..=0xFFFE => self.high_ram[address.index_value() - 0xFF80] = value,
            0xFFFF => self.interrupt_enable = value,
        }
    }

    pub fn write_word(&mut self, address: Address, value: Word) {
        self.write(address, value.low());
        self.write(address.next(), value.high());
    }
}
