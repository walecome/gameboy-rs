use core::fmt;

#[derive(Clone, Copy, PartialEq)]
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

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Address").field("addr", &format_args!("{:#06X}", &self.addr)).finish()
    }
}
