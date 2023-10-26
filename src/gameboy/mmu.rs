use super::address::Address;
use super::cartridge::Cartridge;
use super::video::Video;

pub struct Word {
    pub value: u16,
}

const BOOT_ROM: &[u8] = &[
    0x31, 0xFE, 0xFF, 0xAF, 0x21, 0xFF, 0x9F, 0x32, 0xCB, 0x7C, 0x20, 0xFB, 0x21, 0x26, 0xFF, 0x0E,
    0x11, 0x3E, 0x80, 0x32, 0xE2, 0x0C, 0x3E, 0xF3, 0xE2, 0x32, 0x3E, 0x77, 0x77, 0x3E, 0xFC, 0xE0,
    0x47, 0x11, 0x04, 0x01, 0x21, 0x10, 0x80, 0x1A, 0xCD, 0x95, 0x00, 0xCD, 0x96, 0x00, 0x13, 0x7B,
    0xFE, 0x34, 0x20, 0xF3, 0x11, 0xD8, 0x00, 0x06, 0x08, 0x1A, 0x13, 0x22, 0x23, 0x05, 0x20, 0xF9,
    0x3E, 0x19, 0xEA, 0x10, 0x99, 0x21, 0x2F, 0x99, 0x0E, 0x0C, 0x3D, 0x28, 0x08, 0x32, 0x0D, 0x20,
    0xF9, 0x2E, 0x0F, 0x18, 0xF3, 0x67, 0x3E, 0x64, 0x57, 0xE0, 0x42, 0x3E, 0x91, 0xE0, 0x40, 0x04,
    0x1E, 0x02, 0x0E, 0x0C, 0xF0, 0x44, 0xFE, 0x90, 0x20, 0xFA, 0x0D, 0x20, 0xF7, 0x1D, 0x20, 0xF2,
    0x0E, 0x13, 0x24, 0x7C, 0x1E, 0x83, 0xFE, 0x62, 0x28, 0x06, 0x1E, 0xC1, 0xFE, 0x64, 0x20, 0x06,
    0x7B, 0xE2, 0x0C, 0x3E, 0x87, 0xE2, 0xF0, 0x42, 0x90, 0xE0, 0x42, 0x15, 0x20, 0xD2, 0x05, 0x20,
    0x4F, 0x16, 0x20, 0x18, 0xCB, 0x4F, 0x06, 0x04, 0xC5, 0xCB, 0x11, 0x17, 0xC1, 0xCB, 0x11, 0x17,
    0x05, 0x20, 0xF5, 0x22, 0x23, 0x22, 0x23, 0xC9, 0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B,
    0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E,
    0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC,
    0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E, 0x3C, 0x42, 0xB9, 0xA5, 0xB9, 0xA5, 0x42, 0x3C,
    0x21, 0x04, 0x01, 0x11, 0xA8, 0x00, 0x1A, 0x13, 0xBE, 0x00, 0x00, 0x23, 0x7D, 0xFE, 0x34, 0x20,
    0xF5, 0x06, 0x19, 0x78, 0x86, 0x23, 0x05, 0x20, 0xFB, 0x86, 0x00, 0x00, 0x3E, 0x01, 0xE0, 0x50,
];

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
    boot_rom_disabled: u8,
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
            boot_rom_disabled: 0x00,
        }
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

    pub fn video(&mut self) -> &mut Video {
        &mut self.video
    }

    pub fn read(&self, address: Address) -> u8 {
        if address.value() == 0xFF0F {
            return self.interrupt_flags;
        }

        match address.value() {
            0x0000..=0x7FFF => {
                if address.value() <= 0xFF && self.io.boot_rom_disabled == 0x00 {
                    BOOT_ROM[address.index_value()]
                } else {
                    self.cartridge.read(address)
                }
            }
            0x8000..=0x9FFF => self.video.read_vram(Address::new(address.value() - 0x8000)),
            0xA000..=0xBFFF => todo!("Read from cartridge RAM"),
            0xC000..=0xDFFF => self.internal_ram[address.index_value() - 0xC000],
            0xE000..=0xFDFF => panic!("Read access for prohibited memory area"),
            0xFE00..=0xFE9F => self.video.read_oam(Address::new(address.value() - 0xFE00)),
            0xFEA0..=0xFEFF => panic!("Read access for prohibited memory area"),
            0xFF00..=0xFF7F => self.read_io(address),
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
            0xFE00..=0xFE9F => self.video.write_oam(Address::new(address.value() - 0xFE00), value),
            0xFEA0..=0xFEFF => panic!("Write access for prohibited memory area"),
            0xFF00..=0xFF7F => self.write_io(address, value),
            0xFF80..=0xFFFE => self.high_ram[address.index_value() - 0xFF80] = value,
            0xFFFF => self.interrupt_enable = value,
        }
    }

    pub fn write_word(&mut self, address: Address, value: Word) {
        self.write(address, value.low());
        self.write(address.next(), value.high())VRAM
    }


    fn read_io(&self, address: Address) -> u8 {
        let select_byte: u8 = match address.value() {
            0xFF00..=0xFF70 => (address.value() & 0xFF) as u8,
            _ => panic!("Trying to read IO outside mapped area: {:#06X}", address.value()),
        };

        match select_byte {
            0x00 => self.io.joypad_input,
            0x01 => self.io.serial_transfer.0,
            0x02 => self.io.serial_transfer.1,
            0x04..=0x07 => self.io.timer_and_divider[(select_byte - 0x04) as usize],
            0x10..=0x26 => self.io.audio[(select_byte - 0x10) as usize],
            0x30..=0x3F => self.io.wave_pattern[(select_byte - 0x30) as usize],
            0x40..=0x45 => self.video.read_register(select_byte),
            0x46 => panic!("Reading from DMA transfer register"),
            0x47..=0x4B => self.video.read_register(select_byte),
            _ => panic!("Read for unmapped IO address: {:#06X}", address.value()),
        }
    }

    fn write_io(& mut self, address: Address, value: u8) {
        let select_byte: u8 = match address.value() {
            0xFF00..=0xFF70 => (address.value() & 0xFF) as u8,
            _ => panic!("Trying to write IO outside mapped area: {:#06X}", address.value()),
        };

        match select_byte {
            0x00 => self.io.joypad_input = value,
            0x01 => self.io.serial_transfer.0 = value,
            0x02 => self.io.serial_transfer.1 = value,
            0x04..=0x07 => self.io.timer_and_divider[(select_byte - 0x04) as usize] = value,
            0x10..=0x26 => self.io.audio[(select_byte - 0x10) as usize] = value,
            0x30..=0x3F => self.io.wave_pattern[(select_byte - 0x30) as usize] = value,
            0x40..=0x45 => self.video.write_register(select_byte, value),
            0x46 => self.do_dma_transfer(value),
            0x47..=0x4B => self.video.write_register(select_byte, value),
            _ => panic!("Write for unmapped IO address: {:#06X}", address.value()),
        };
    }

    fn do_dma_transfer(&mut self, dma_target: u8) {
        // TODO: The DMA transfer could take 160 cycle for normal speed, do we need to care?
        // https://gbdev.io/pandocs/OAM_DMA_Transfer.html#ff46--dma-oam-dma-source-address--start
        let mut src_addr = Address::new((dma_target as u16) * 0x0100);
        let mut dst_addr = Address::new(0xFE00);
        for _ in 0..=0x9F {

            let value = self.read(src_addr);
            self.write(dst_addr, value);

            src_addr = src_addr.next();
            dst_addr = dst_addr.next();
        }

    }
}
