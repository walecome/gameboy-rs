use super::header::CartridgeType;
use super::address::Address;

pub trait Cartridge {
    fn read(&self, address: Address) -> u8;
    fn write(&mut self, address: Address, value: u8);
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
            _ => todo!("Read from unmapped or unimplemented cartridge address: {:#06X}", address.value()),
        }
    }
    fn write(&mut self, address: Address, value: u8) {
        match address.value() {
            0x2000..=0x3FFF => {
                if value > 0b0001_1111 {
                    todo!("Invalid BANK1 register value '{:04X}'. Should we allow this?", value);
                }
                let fixed_value = if value & 0x1 == 0 {
                    value + 1
                } else {
                    value
                };
                self.rom_bank = fixed_value;
            }
            _ => todo!("Write to unmapped or unimplemented cartridge address: {:#06X} = {:#04X}", address.value(), value)
        }
    }
}

pub fn create_for_cartridge_type(cartridge_type: CartridgeType, rom_data: Vec<u8>) -> Option<Box<dyn Cartridge>> {
    match cartridge_type {
        CartridgeType::MBC1 => Some(Box::new(MBC1::new(rom_data))),
        _ => None,
    }
}
