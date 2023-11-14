use super::header::CartridgeType;
use super::address::Address;

pub trait Cartridge {
    fn read(&self, address: Address) -> u8;
    fn write(&mut self, address: Address, value: u8);
}

struct RomOnly {
    rom_data: Vec<u8>,
}

impl RomOnly {
    fn new(rom_data: Vec<u8>) -> Self {
        Self { rom_data }
    }
}

impl Cartridge for RomOnly {
    fn read(&self, address: Address) -> u8 {
        return self.rom_data[address.index_value()];
    }

    fn write(&mut self, _address: Address, _value: u8) {
        // panic!("Attempt to write to RomOnly data");
    }
}

struct MBC1 {
    rom_data: Vec<u8>,
    rom_bank: u8,
}

impl MBC1 {
    fn new(rom_data: Vec<u8>) -> Self {
        Self {
            rom_data,
            // Zero is not valid number, should be 1 initially
            rom_bank: 0x01,
        }
    }
}

impl Cartridge for MBC1 {
    fn read(&self, address: Address) -> u8 {
        match address.value() {
            0x0000..=0x3FFF => self.rom_data[address.index_value()],
            0x4000..=0x7FFF => {
                let normalized_addr = address.value() - 0x4000;
                let bank_offset_addr = 0x4000 * (self.rom_bank as u16);
                let addr = bank_offset_addr + normalized_addr;
                self.rom_data[addr as usize]
            },
            _ => todo!("Read from unmapped or unimplemented cartridge address: {:#06X}", address.value()),
        }
    }

    fn write(&mut self, address: Address, value: u8) {
        match address.value() {
            0x2000..=0x3FFF => {
                if value > 0b0001_1111 {
                    panic!("Invalid BANK1 register value '{:04X}'. Should we allow this?", value);
                }

                let fixed_value = match value {
                    0x0 | 0x20 | 0x40 | 0x60 => value + 1,
                    _ => value,
                };

                self.rom_bank = fixed_value;
            }
            _ => todo!("Write to unmapped or unimplemented cartridge address: {:#06X} = {:#04X}", address.value(), value)
        }
    }
}

pub fn create_for_cartridge_type(cartridge_type: CartridgeType, rom_data: Vec<u8>) -> Option<Box<dyn Cartridge>> {
    match cartridge_type {
        CartridgeType::RomOnly => Some(Box::new(RomOnly::new(rom_data))),
        CartridgeType::MBC1 => Some(Box::new(MBC1::new(rom_data))),
        _ => None,
    }
}
