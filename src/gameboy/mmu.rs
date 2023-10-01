#[derive(Clone, Copy)]
pub struct Address {
    addr: u16,
}

impl Address {
    pub fn new(addr: u16) -> Address {
        Address {
            addr,
        }
    }

    pub fn from_lower(lower_addr: u8) -> Address {
        Address::new(0xFF00 + lower_addr as u16)
    }

    fn next(&self) -> Self {
        Self { addr: self.addr + 1 }
    }
}

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
        let select_byte: u8 = match address.addr {
            0xFF00..=0xFF70 => (address.addr & 0xFF) as u8,
            _ => panic!("Trying to read IO outside mapped area: {:#06X}", address.addr),
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
            _ => panic!("Read for unmapped IO address: {:#06X}", address.addr),
        }
    }

    fn write(& mut self, address: Address, value: u8) {
        let select_byte: u8 = match address.addr {
            0xFF00..=0xFF70 => (address.addr & 0xFF) as u8,
            _ => panic!("Trying to write IO outside mapped area: {:#06X}", address.addr),
        };

        let target: &mut u8 = match select_byte {
            0x00 => &mut self.joypad_input,
            0x01 => &mut self.serial_transfer.0,
            0x02 => &mut self.serial_transfer.1,
            0x04..=0x07 => &mut self.timer_and_divider[(select_byte - 0x04) as usize],
            0x10..=0x26 => &mut self.audio[(select_byte - 0x10) as usize],
            0x30..=0x3F => &mut self.wave_pattern[(select_byte - 0x30) as usize],
            0x40..=0x4B => &mut self.lcd[(select_byte - 0x40) as usize],
            0x4F => &mut self.vram_bank_select,
            0x50 => &mut self.boot_rom_disabled,
            0x51..=0x55 => &mut self.vram_dma[(select_byte - 0x51) as usize],
            0x68..=0x6B => &mut self.bg_obj_palettes[(select_byte - 0x68) as usize],
            0x70 => &mut self.wram_bank_select,
            _ => panic!("Write for unmapped IO address: {:#06X}", address.addr),
        };
        *target = value;
    }
}

pub struct MMU {
    internal_ram: Vec<u8>,
    io: IO,
}

impl MMU {
    pub fn new() -> MMU {
        MMU {
            internal_ram: vec![0x00; 0x3000],
            io: IO::new(),
        }
    }

    pub fn read(&self, address: Address) -> u8 {
        // self.data[address.addr as usize]
        match address.addr {
            0x0000..=0x7FFF => todo!("Read from cartridge"),
            0x8000..=0x9FFF => todo!("Read VRAM"),
            0xA000..=0xBFFF => todo!("Read from cartridge RAM"),
            0xC000..=0xDFFF => self.internal_ram[address.addr as usize - 0xC000],
            0xE000..=0xFDFF => panic!("Read access for prohibited memory area"),
            0xFE00..=0xFE9F => todo!("Read OAM"),
            0xFEA0..=0xFEFF => panic!("Read access for prohibited memory area"),
            0xFF00..=0xFF7F => todo!("Read from I/O registers"),
            0xFF80..=0xFFFE => todo!("Read high RAM"),
            0xFFFF => todo!("Read IE"),
        }
    }

    pub fn read_word(&self, address: Address) -> Word {
        let low = self.read(address);
        let high = self.read(address.next());

        Word::compose_new(high, low)
    }

    pub fn write(&mut self, address: Address, value: u8) {
        match address.addr {
            0x0000..=0x7FFF => todo!("Write to cartridge"),
            0x8000..=0x9FFF => todo!("Write VRAM"),
            0xA000..=0xBFFF => todo!("Write to cartridge RAM"),
            0xC000..=0xDFFF => self.internal_ram[address.addr as usize - 0xC000] = value,
            0xE000..=0xFDFF => panic!("Write access for prohibited memory area"),
            0xFE00..=0xFE9F => todo!("Write OAM"),
            0xFEA0..=0xFEFF => panic!("Write access for prohibited memory area"),
            0xFF00..=0xFF7F => todo!("Write to I/O registers"),
            0xFF80..=0xFFFE => todo!("Write high RAM"),
            0xFFFF => todo!("Write IE"),
        }
    }

    pub fn write_word(&mut self, address: Address, value: Word) {
        self.write(address, value.low());
        self.write(address.next(), value.high());
    }
}
