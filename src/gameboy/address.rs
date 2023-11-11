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

    pub fn next(&self) -> Self {
        self.plus(1)
    }

    pub fn plus(&self, offset: u16) -> Self {
        Self { addr: self.addr + offset }
    }

    pub fn value(&self) -> u16 {
        self.addr
    }

    pub fn index_value(&self) -> usize {
        self.addr as usize
    }
}
