use super::header::CartridgeType;
use super::address::Address;
use super::utils::{set_bit_mut, get_bit};

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

    fn write(&mut self, address: Address, value: u8) {
        println!("Attempt to write to RomOnly cartridge: {:?} = {}", address, value);
    }
}

enum BankingMode {
    UseRom,
    UseRam,
}

struct MBC1 {
    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    rom_bank: u8,
    ram_bank: u8,
    ram_enabled: bool,
    banking_mode: BankingMode,
}

impl MBC1 {
    fn new(rom_data: Vec<u8>) -> Self {
        Self {
            rom_data,
            ram_data: vec![0x00; 0x2000 * 4],
            // Zero is not valid number, should be 1 initially
            rom_bank: 0x01,
            ram_bank: 0x00,
            ram_enabled: false,
            banking_mode: BankingMode::UseRom,
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
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return 0xFF;
                }
                let normalized_addr = address.index_value() - 0xA000;
                let bank_offset_addr = 0x4000 * self.ram_bank as usize;
                let addr = bank_offset_addr + normalized_addr;
                self.ram_data[addr]
            }
            _ => todo!("Read from unmapped or unimplemented cartridge address: {:#06X}", address.value()),
        }
    }

    fn write(&mut self, address: Address, value: u8) {
        match address.value() {
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0xF == 0xA;
            },
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
            0x4000..=0x5FFF => {
                match self.banking_mode {
                    BankingMode::UseRom => {
                        set_bit_mut(&mut self.rom_bank, 5, get_bit(value, 0));
                        set_bit_mut(&mut self.rom_bank, 6, get_bit(value, 1));

                    },
                    BankingMode::UseRam => self.ram_bank = value & 0b11,
                }
            },
            0x6000..=0x7FFF => {
                self.banking_mode = if value == 0 {
                    BankingMode::UseRom
                } else {
                    BankingMode::UseRam
                };
            },
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return;
                }
                let normalized_addr = address.index_value() - 0xA000;
                let bank_offset_addr = 0x4000 * self.ram_bank as usize;
                let addr = bank_offset_addr + normalized_addr;
                self.ram_data[addr] = value;
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
