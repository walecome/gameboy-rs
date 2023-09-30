pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            data: vec![0x00; 0xFFFF + 1],
        }
    }
    pub fn get(&self, address: u16) -> u8 {
        let value = self.data[address as usize];
        println!("mem[{:#06X}] -> {:#04X}", address, value);
        value
    }

    pub fn get_from_u8(&self, lower_address: u8) -> u8 {
        let address = 0xFF00 + (lower_address as u16);
        let value = self.data[(address) as usize];
        println!("mem[{:#06X}] -> {:#04X}", address, value);
        value
    }

    pub fn get_u16(&self, address: u16) -> u16 {
        let low = self.data[address as usize];
        let high = self.data[(address + 1) as usize];
        let value = ((high as u16) << 8) | (low as u16);
        println!("mem[{:#06X}] -> {:#06X}", address, value);
        value
    }

    pub fn set(&mut self, address: u16, value: u8) {
        // TODO: Handle memory map
        self.set_internal(address as usize, value);
        println!("mem[{:#06X}]={:#04X}", address, value);
    }

    pub fn set_from_u8(&mut self, lower_address: u8, value: u8) {
        let address = 0xFF00 + (lower_address as u16);
        self.set_internal(address as usize, value);
        println!("mem[{:#06X}]={:#04X}", address, value);
    }

    pub fn set_u16(&mut self, address: u16, value: u16) {
        println!("mem[{:#06X}]={:#06X}", address, value);
        let low = (value & 0x00FF) as u8;
        let high = ((value & 0xFF00) >> 8) as u8;
        self.set_internal(address as usize, low);
        self.set_internal((address + 1) as usize, high);
    }

    fn set_internal(&mut self, address: usize, value: u8) {
        self.data[address] = value;
    }
}
