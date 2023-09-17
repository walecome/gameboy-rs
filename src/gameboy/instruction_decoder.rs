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
    AddressU16(RegisterU16),
    AddressU8(RegisterU8),
    ImmediateAddressU8,
    ImmediateAddressU16,
    ImmediateValue,
    AddressU16Increment(RegisterU16),
    AddressU16Decrement(RegisterU16),
}

pub enum TargetU16 {
    Register(RegisterU16),
    ImmediateU16,
    StackPointer,
    Address,
    StackPointerWithOffset,
}

pub enum Instruction {
    Noop,
    Halt,
    LoadU8 { dst: TargetU8, src: TargetU8 },
    LoadU16 { dst: TargetU16, src: TargetU16 },
    JumpImmediate,
    DisableInterrupts,
}

fn decode_u8_load_src(opcode: u8, row_mask: u8, col_mask: u8) -> TargetU8 {
    match (row_mask, col_mask) {
        (0x4..=0x7, 0x0 | 0x8) => TargetU8::Register(RegisterU8::B),
        (0x4..=0x7, 0x1 | 0x9) => TargetU8::Register(RegisterU8::C),
        (0x4..=0x7, 0x2 | 0xA) => TargetU8::Register(RegisterU8::D),
        (0x4..=0x7, 0x3 | 0xB) => TargetU8::Register(RegisterU8::E),
        (0x4..=0x7, 0x4 | 0xC) => TargetU8::Register(RegisterU8::H),
        (0x4..=0x7, 0x5 | 0xD) => TargetU8::Register(RegisterU8::L),
        (0x4..=0x7, 0x6 | 0xE) => TargetU8::AddressU16(RegisterU16::HL),
        (0x4..=0x7, 0x7 | 0xF) => TargetU8::Register(RegisterU8::A),

        (0x0, 0xE) => TargetU8::Register(RegisterU8::A),
        (0x0, 0xF) => TargetU8::ImmediateAddressU8,

        (0x0..=0x3 | 0xE, 0x2) => TargetU8::Register(RegisterU8::A),
        (0xF, 0x2) => TargetU8::AddressU8(RegisterU8::C),

        (0x0..=0x3, 0x6) => TargetU8::ImmediateValue,

        (0x0, 0xA) => TargetU8::AddressU16(RegisterU16::BC),
        (0x1, 0xA) => TargetU8::AddressU16(RegisterU16::DE),
        (0x2, 0xA) => TargetU8::AddressU16Increment(RegisterU16::HL),
        (0x3, 0xA) => TargetU8::AddressU16Decrement(RegisterU16::HL),
        (0xE, 0xA) => TargetU8::Register(RegisterU8::A),
        (0xF, 0xA) => TargetU8::ImmediateAddressU16,

        _ => panic!("Unknown src for LD opcode: {}, {}", row_mask, col_mask),
    }
}

fn decode_u8_load_dst(row_mask: u8, col_mask: u8) -> TargetU8 {
    if col_mask <= 0x7 {
        match row_mask {
            0x4 => TargetU8::Register(RegisterU8::B),
            0x5 => TargetU8::Register(RegisterU8::D),
            0x6 => TargetU8::Register(RegisterU8::H),
            0x7 => TargetU8::AddressU16(RegisterU16::HL),
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

fn try_decode_u8_load_instruction(opcode: u8) -> Option<Instruction> {
    if !is_u8_load_instruction(opcode) {
        return None;
    }
    let row_mask = (opcode & 0xF0) >> 4;
    let col_mask = opcode & 0x0F;

    let src = decode_u8_load_src(col_mask);
    let dst = decode_u8_load_dst(row_mask, col_mask);

    return Some(Instruction::LoadU8 { src, dst });
}

fn try_decode_u16_load_instruction(opcode: u8) -> Option<Instruction> {
    match opcode {
        0x01 => Some(Instruction::LoadU16 {
            dst: TargetU16::Register(RegisterU16::BC),
            src: TargetU16::ImmediateU16,
        }),
        0x11 => Some(Instruction::LoadU16 {
            dst: TargetU16::Register(RegisterU16::DE),
            src: TargetU16::ImmediateU16,
        }),
        0x21 => Some(Instruction::LoadU16 {
            dst: TargetU16::Register(RegisterU16::HL),
            src: TargetU16::ImmediateU16,
        }),
        0x31 => Some(Instruction::LoadU16 {
            dst: TargetU16::StackPointer,
            src: TargetU16::ImmediateU16,
        }),
        0x08 => Some(Instruction::LoadU16 {
            dst: TargetU16::Address,
            src: TargetU16::StackPointer,
        }),
        0xF8 => Some(Instruction::LoadU16 {
            dst: TargetU16::Register(RegisterU16::HL),
            src: TargetU16::StackPointerWithOffset,
        }),
        0xF9 => Some(Instruction::LoadU16 {
            dst: TargetU16::StackPointer,
            src: TargetU16::Register(RegisterU16::HL),
        }),
        _ => None,
    }
}

// https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
pub fn decode(opcode: u8) -> Option<Instruction> {
    if let Some(instruction) = try_decode_u8_load_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_u16_load_instruction(opcode) {
        return Some(instruction);
    }

    match opcode {
        0x00 => Some(Instruction::Noop),
        0x76 => Some(Instruction::Halt),
        0xC3 => Some(Instruction::JumpImmediate),
        0xF3 => Some(Instruction::DisableInterrupts),
        _ => None,
    }
}
