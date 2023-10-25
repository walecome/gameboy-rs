pub fn get_bit(value: u8, bit: u8) -> bool {
    value & (1 << bit) != 0
}

pub fn set_bit_mut(value: &mut u8, bit: u8, bit_value: bool) {
    if bit_value {
        *value |= 1 << bit
    } else {
        *value &= !1 << bit
    }
}

pub fn set_bit(value: u8, bit: u8, bit_value: bool) -> u8 {
    if bit_value {
        value | (1 << bit)
    } else {
        value & !(1 << bit)
    }
}
