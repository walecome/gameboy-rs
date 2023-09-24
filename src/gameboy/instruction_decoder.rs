#[derive(Debug)]
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

#[derive(Debug)]
pub enum RegisterU16 {
    AF,
    BC,
    DE,
    HL,
}

#[derive(Debug)]
pub enum LoadSrcU8 {
    Register(RegisterU8),
    AddressU16(RegisterU16),
    AddressU8(RegisterU8),
    ImmediateAddressU8,
    ImmediateAddressU16,
    ImmediateU8,
    AddressU16Increment(RegisterU16),
    AddressU16Decrement(RegisterU16),
}

#[derive(Debug)]
pub enum LoadDstU8 {
    Register(RegisterU8),
    AddressU8(RegisterU8),
    AddressU16(RegisterU16),
    AddressU16Increment(RegisterU16),
    AddressU16Decrement(RegisterU16),
    ImmediateAddressU8,
    ImmediateAddressU16,
}

#[derive(Debug)]
pub enum LoadSrcU16 {
    Register(RegisterU16),
    ImmediateU16,
    StackPointer,
    StackPointerWithOffset,
}

#[derive(Debug)]
pub enum LoadDstU16 {
    Register(RegisterU16),
    StackPointer,
    ImmediateAddress,
}

pub enum Condition {
    NZ,
    NC,
    Z,
    C,
}

pub enum Instruction {
    Noop,
    Halt,
    LoadU8 { dst: LoadDstU8, src: LoadSrcU8 },
    LoadU16 { dst: LoadDstU16, src: LoadSrcU16 },
    JumpImmediate,
    DisableInterrupts,
    Call(Option<Condition>),
    JumpRelative(Option<Condition>),
    Ret(Option<Condition>),
    Push(RegisterU16),
    Pop(RegisterU16),
}

fn try_decode_u8_load_src(row_mask: u8, col_mask: u8) -> Option<LoadSrcU8> {
    Some(match (row_mask, col_mask) {
        (0x0..=0x3, 0x2) => LoadSrcU8::Register(RegisterU8::A),

        (0x0..=0x3, 0x6) => LoadSrcU8::ImmediateU8,

        (0x0, 0xA) => LoadSrcU8::AddressU16(RegisterU16::BC),
        (0x1, 0xA) => LoadSrcU8::AddressU16(RegisterU16::DE),
        (0x2, 0xA) => LoadSrcU8::AddressU16Increment(RegisterU16::HL),
        (0x3, 0xA) => LoadSrcU8::AddressU16Decrement(RegisterU16::HL),

        (0x0..=0x3, 0xE) => LoadSrcU8::ImmediateU8,

        (0x4..=0x7, 0x0 | 0x8) => LoadSrcU8::Register(RegisterU8::B),
        (0x4..=0x7, 0x1 | 0x9) => LoadSrcU8::Register(RegisterU8::C),
        (0x4..=0x7, 0x2 | 0xA) => LoadSrcU8::Register(RegisterU8::D),
        (0x4..=0x7, 0x3 | 0xB) => LoadSrcU8::Register(RegisterU8::E),
        (0x4..=0x7, 0x4 | 0xC) => LoadSrcU8::Register(RegisterU8::H),
        (0x4..=0x7, 0x5 | 0xD) => LoadSrcU8::Register(RegisterU8::L),
        (0x4..=0x6, 0x6) => LoadSrcU8::AddressU16(RegisterU16::HL),
        (0x4..=0x7, 0xE) => LoadSrcU8::AddressU16(RegisterU16::HL),
        (0x4..=0x7, 0x7 | 0xF) => LoadSrcU8::Register(RegisterU8::A),


        (0xE, 0x0) => LoadSrcU8::Register(RegisterU8::A),
        (0xE, 0x2) => LoadSrcU8::Register(RegisterU8::A),
        (0xE, 0xA) => LoadSrcU8::Register(RegisterU8::A),

        (0xF, 0x0) => LoadSrcU8::ImmediateAddressU8,
        (0xF, 0x2) => LoadSrcU8::AddressU8(RegisterU8::C),
        (0xF, 0xA) => LoadSrcU8::ImmediateAddressU16,

        _ => return None
    })
}

fn try_decode_u8_load_dst(row_mask: u8, col_mask: u8) -> Option<LoadDstU8> {
    Some(match (row_mask, col_mask) {
        (0x0, 0x2) => LoadDstU8::AddressU16(RegisterU16::BC),
        (0x1, 0x2) => LoadDstU8::AddressU16(RegisterU16::DE),
        (0x2, 0x2) => LoadDstU8::AddressU16Increment(RegisterU16::HL),
        (0x3, 0x2) => LoadDstU8::AddressU16Decrement(RegisterU16::HL),

        (0x0, 0x6) => LoadDstU8::Register(RegisterU8::B),
        (0x1, 0x6) => LoadDstU8::Register(RegisterU8::D),
        (0x2, 0x6) => LoadDstU8::Register(RegisterU8::H),
        (0x3, 0x6) => LoadDstU8::AddressU16(RegisterU16::HL),

        (0x0..=0x3, 0xA) => LoadDstU8::Register(RegisterU8::A),

        (0x0, 0xE) => LoadDstU8::Register(RegisterU8::C),
        (0x1, 0xE) => LoadDstU8::Register(RegisterU8::E),
        (0x2, 0xE) => LoadDstU8::Register(RegisterU8::L),
        (0x3, 0xE) => LoadDstU8::Register(RegisterU8::A),

        (0x4, 0x0..=0x7) => LoadDstU8::Register(RegisterU8::B),
        (0x5, 0x0..=0x7) => LoadDstU8::Register(RegisterU8::D),
        (0x6, 0x0..=0x7) => LoadDstU8::Register(RegisterU8::H),
        (0x7, 0x0..=0x7) => LoadDstU8::AddressU16(RegisterU16::HL),

        (0x4, 0x8..=0xF) => LoadDstU8::Register(RegisterU8::C),
        (0x5, 0x8..=0xF) => LoadDstU8::Register(RegisterU8::E),
        (0x6, 0x8..=0xF) => LoadDstU8::Register(RegisterU8::L),
        (0x7, 0x8..=0xF) => LoadDstU8::Register(RegisterU8::A),

        (0xE, 0x0) => LoadDstU8::ImmediateAddressU8,
        (0xE, 0x2) => LoadDstU8::AddressU8(RegisterU8::C),
        (0xE, 0xA) => LoadDstU8::ImmediateAddressU16,

        (0xF, 0x0) => LoadDstU8::Register(RegisterU8::A),
        (0xF, 0x2) => LoadDstU8::Register(RegisterU8::A),
        (0xF, 0xA) => LoadDstU8::Register(RegisterU8::A),

        _ => return None
    })
}

fn try_decode_u8_load_instruction(opcode: u8) -> Option<Instruction> {
    let row_mask = (opcode & 0xF0) >> 4;
    let col_mask = opcode & 0x0F;

    let src = try_decode_u8_load_src(row_mask, col_mask)?;
    let dst = try_decode_u8_load_dst(row_mask, col_mask)?;

    return Some(Instruction::LoadU8 { src, dst });
}

fn try_decode_u16_load_instruction(opcode: u8) -> Option<Instruction> {
    match opcode {
        0x01 => Some(Instruction::LoadU16 {
            dst: LoadDstU16::Register(RegisterU16::BC),
            src: LoadSrcU16::ImmediateU16,
        }),
        0x11 => Some(Instruction::LoadU16 {
            dst: LoadDstU16::Register(RegisterU16::DE),
            src: LoadSrcU16::ImmediateU16,
        }),
        0x21 => Some(Instruction::LoadU16 {
            dst: LoadDstU16::Register(RegisterU16::HL),
            src: LoadSrcU16::ImmediateU16,
        }),
        0x31 => Some(Instruction::LoadU16 {
            dst: LoadDstU16::StackPointer,
            src: LoadSrcU16::ImmediateU16,
        }),
        0x08 => Some(Instruction::LoadU16 {
            dst: LoadDstU16::ImmediateAddress,
            src: LoadSrcU16::StackPointer,
        }),
        0xF8 => Some(Instruction::LoadU16 {
            dst: LoadDstU16::Register(RegisterU16::HL),
            src: LoadSrcU16::StackPointerWithOffset,
        }),
        0xF9 => Some(Instruction::LoadU16 {
            dst: LoadDstU16::StackPointer,
            src: LoadSrcU16::Register(RegisterU16::HL),
        }),
        _ => None,
    }
}

fn try_decode_call_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xC4 => Instruction::Call(Some(Condition::NZ)),
        0xCC => Instruction::Call(Some(Condition::Z)),
        0xCD => Instruction::Call(None),

        0xD4 => Instruction::Call(Some(Condition::NC)),
        0xDC => Instruction::Call(Some(Condition::C)),
        _ => return None,
    })
}

fn try_decode_relative_jump_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0x18 => Instruction::JumpRelative(None),

        0x20 => Instruction::JumpRelative(Some(Condition::NZ)),
        0x28 => Instruction::JumpRelative(Some(Condition::Z)),

        0x30 => Instruction::JumpRelative(Some(Condition::NC)),
        0x38 => Instruction::JumpRelative(Some(Condition::C)),
        _ => return None,
    })
}

fn try_decode_ret_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xC0 => Instruction::Ret(Some(Condition::NZ)),
        0xC8 => Instruction::Ret(Some(Condition::Z)),
        0xC9 => Instruction::Ret(None),

        0xD0 => Instruction::Ret(Some(Condition::NC)),
        0xD8 => Instruction::Ret(Some(Condition::C)),

        0xD9 => todo!("RETI"),
        _ => return None,
    })
}

fn try_decode_push_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xC5 => Instruction::Push(RegisterU16::BC),
        0xD5 => Instruction::Push(RegisterU16::DE),
        0xE5 => Instruction::Push(RegisterU16::HL),
        0xF5 => Instruction::Push(RegisterU16::AF),
        _ => return None,
    })
}

fn try_decode_pop_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xC1 => Instruction::Pop(RegisterU16::BC),
        0xD1 => Instruction::Pop(RegisterU16::DE),
        0xE1 => Instruction::Pop(RegisterU16::HL),
        0xF1 => Instruction::Pop(RegisterU16::AF),
        _ => return None,
    })
}

// https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
pub fn decode(opcode: u8) -> Option<Instruction> {
    if let Some(instruction) = try_decode_u8_load_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_u16_load_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_call_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_relative_jump_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_ret_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_push_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_pop_instruction(opcode) {
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
