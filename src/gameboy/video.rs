use super::address::Address;

pub struct Video {
    vram: Vec<u8>,
}

impl Video {
    pub fn new() -> Self {
        Self {
            vram: vec![0x00; 0x4000],
        }
    }

    pub fn write_vram(&mut self, address: Address, value: u8) {
        self.vram[address.index_value()] = value;
    }

    pub fn read_vram(&self, address: Address) -> u8 {
        self.vram[address.index_value()]
    }
}
