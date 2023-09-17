pub enum RegisterU8 {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
}

pub enum RegisterU16 {
    AF,
    BC,
    DE,
    HL,
}

pub enum TargetU8 {
    Register(RegisterU8),
    AddressHL,
}

pub enum Instruction {
    Noop,
    Halt,
    Load { dst: TargetU8, src: TargetU8 },
    JumpImmediate,
}

fn decode_load_source(mask: u8) -> TargetU8 {
    match mask {
        0x0 | 0x8 => TargetU8::Register(RegisterU8::B),
        0x1 | 0x9 => TargetU8::Register(RegisterU8::C),
        0x2 | 0xA => TargetU8::Register(RegisterU8::D),
        0x3 | 0xB => TargetU8::Register(RegisterU8::E),
        0x4 | 0xC => TargetU8::Register(RegisterU8::H),
        0x5 | 0xD => TargetU8::Register(RegisterU8::L),
        0x7 | 0xF => TargetU8::Register(RegisterU8::A),
        0x6 | 0xE => TargetU8::AddressHL,
        _ => panic!("Unknown LD destination mask: {}", mask),
    }
}

fn decode_load_destination(row_mask: u8, col_mask: u8) -> TargetU8 {
    if col_mask <= 0x7 {
        match row_mask {
            0x4 => TargetU8::Register(RegisterU8::B),
            0x5 => TargetU8::Register(RegisterU8::D),
            0x6 => TargetU8::Register(RegisterU8::H),
            0x7 => TargetU8::AddressHL,
            _ => panic!("Unknown LD src mask: ({}, {}", row_mask, col_mask),
        }
    } else {
        match row_mask {
            0x4 => TargetU8::Register(RegisterU8::C),
            0x5 => TargetU8::Register(RegisterU8::E),
            0x6 => TargetU8::Register(RegisterU8::L),
            0x7 => TargetU8::Register(RegisterU8::A),
            _ => panic!("Unknown LD src mask: ({}, {}", row_mask, col_mask),
        }
    }
}

fn is_u8_load_instruction(opcode: u8) -> bool {
    let row_mask = (opcode & 0xF0) >> 4;
    let col_mask = opcode & 0x0F;

    match col_mask {
        0x2 | 0x6 | 0xA | 0xE => {
            if (0x0..=0x3).contains(&row_mask) {
                return true;
            }
        }
        _ => (),
    }

    match col_mask {
        0x1 | 0x2 | 0xA => {
            if (0xE..=0xF).contains(&row_mask) {
                return true;
            }
        }
        _ => (),
    }

    match row_mask {
        0x4 | 0x5 | 0x6 | 0x7 => {
            // HALT
            if opcode == 0x76 {
                return false;
            }
            return true;
        }
        _ => false,
    }
}

// https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
pub fn decode(opcode: u8) -> Option<Instruction> {
    let row_mask = (opcode & 0xF0) >> 4;
    let col_mask = opcode & 0x0F;

    if is_u8_load_instruction(opcode) {
        let src = decode_load_source(col_mask);
        let dst = decode_load_destination(row_mask, col_mask);
        return Some(Instruction::Load { src, dst });
    }

    match opcode {
        0x00 => Some(Instruction::Noop),
        0x76 => Some(Instruction::Halt),
        0xC3 => Some(Instruction::JumpImmediate),
        _ => None,
    }
}
