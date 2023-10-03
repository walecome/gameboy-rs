use super::header::CartridgeType;
use super::address::Address;

pub trait Cartridge {
    fn read(&self, address: Address) -> u8;
    fn write(&self, address: Address, value: u8);
}

struct MBC1 {
    rom_data: Vec<u8>,
}

impl MBC1 {
    fn new(rom_data: Vec<u8>) -> Self {
        Self { rom_data, }
    }
}

impl Cartridge for MBC1 {
    fn read(&self, address: Address) -> u8 {
        match address.value() {
            0x0000..=0x3FFF => self.rom_data[address.index_value()],
            _ => todo!("Read from unmapped or implemented cartridge address: {:#06X}", address.value()),
        }
    }
    fn write(&self, _address: Address, _value: u8) {
        todo!()
    }
}

pub fn create_for_cartridge_type(cartridge_type: CartridgeType, rom_data: Vec<u8>) -> Option<Box<dyn Cartridge>> {
    match cartridge_type {
        CartridgeType::MBC1 => Some(Box::new(MBC1::new(rom_data))),
        _ => None,
    }
}
