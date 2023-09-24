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

pub enum LoadDstU8 {
    Register(RegisterU8),
    AddressHL,
    AddressU8(RegisterU8),
    AddressU16(RegisterU16),
    AddressU16Increment(RegisterU16),
    AddressU16Decrement(RegisterU16),
    ImmediateAddressU8,
    ImmediateAddressU16,
}

pub enum LoadSrcU16 {
    Register(RegisterU16),
    ImmediateU16,
    StackPointer,
    StackPointerWithOffset,
}

pub enum LoadDstU16 {
    Register(RegisterU16),
    StackPointer,
    Address,
}

pub enum Instruction {
    Noop,
    Halt,
    LoadU8 { dst: LoadDstU8, src: LoadSrcU8 },
    LoadU16 { dst: LoadDstU16, src: LoadSrcU16 },
    JumpImmediate,
    DisableInterrupts,
}

fn try_decode_u8_load_src(row_mask: u8, col_mask: u8) -> Option<LoadSrcU8> {
    if (row_mask, col_mask) == (0x7, 0x6) {
        // HALT
        return None;
    }

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
        (0x4..=0x7, 0x6 | 0xE) => LoadSrcU8::AddressU16(RegisterU16::HL),
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
        (0x3, 0x6) => LoadDstU8::AddressHL,

        (0x0..=0x3, 0xA) => LoadDstU8::Register(RegisterU8::A),

        (0x0, 0xE) => LoadDstU8::Register(RegisterU8::C),
        (0x1, 0xE) => LoadDstU8::Register(RegisterU8::E),
        (0x2, 0xE) => LoadDstU8::Register(RegisterU8::L),
        (0x3, 0xE) => LoadDstU8::Register(RegisterU8::A),

        (0x4, 0x0..=0x7) => LoadDstU8::Register(RegisterU8::B),
        (0x5, 0x0..=0x7) => LoadDstU8::Register(RegisterU8::D),
        (0x6, 0x0..=0x7) => LoadDstU8::Register(RegisterU8::H),
        (0x7, 0x0..=0x7) => LoadDstU8::AddressHL,

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
            dst: LoadDstU16::Address,
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
