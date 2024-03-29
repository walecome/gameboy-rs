#[derive(Debug, Copy, Clone)]
pub enum RegisterU8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Copy, Clone)]
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
}

#[derive(Debug)]
pub enum LoadDstU16 {
    Register(RegisterU16),
    StackPointer,
    ImmediateAddress,
}

#[derive(Debug)]
pub enum FlagCondition {
    NZ,
    NC,
    Z,
    C,
}

#[derive(Debug)]
pub enum IncDecU8Target {
    RegisterU8(RegisterU8),
    Address(RegisterU16),
}

#[derive(Debug)]
pub enum U16Target {
    RegisterU16(RegisterU16),
    StackPointer,
}

#[derive(Debug)]
pub enum CommonOperand {
    Register(RegisterU8),
    AddressHL,
}

#[derive(Debug)]
pub enum LogicalOpTarget {
    Common(CommonOperand),
    ImmediateU8,
}

#[derive(Debug)]
pub enum Instruction {
    Noop,
    Halt,
    LoadU8 { dst: LoadDstU8, src: LoadSrcU8 },
    LoadU16 { dst: LoadDstU16, src: LoadSrcU16 },
    LoadHlWithOffsetSp,
    JumpImmediate(Option<FlagCondition>),
    JumpAddressHL,
    DisableInterrupts,
    EnableInterrupts,
    Call(Option<FlagCondition>),
    JumpRelative(Option<FlagCondition>),
    Ret(Option<FlagCondition>),
    Reti,
    Push(RegisterU16),
    Pop(RegisterU16),
    IncU8(IncDecU8Target),
    IncU16(U16Target),
    Or(LogicalOpTarget),
    Compare(LogicalOpTarget),
    And(LogicalOpTarget),
    DecU8(IncDecU8Target),
    DecU16(U16Target),
    Xor(LogicalOpTarget),
    AddStackPointer,
    AddU8(LogicalOpTarget),
    AddU16(U16Target),
    Sub(LogicalOpTarget),
    CbSrl(CommonOperand),
    CbRr(CommonOperand),
    CbRl(CommonOperand),
    CbRlc(CommonOperand),
    CbRrc(CommonOperand),
    CbBit { n: u8, target: CommonOperand },
    CbRes { n: u8, target: CommonOperand },
    CbSet { n: u8, target: CommonOperand },
    CbSwap(CommonOperand),
    CbSla(CommonOperand),
    CbSra(CommonOperand),
    Rra,
    Rla,
    Rlca,
    Rrca,
    Adc(LogicalOpTarget),
    Sbc(LogicalOpTarget),
    Cpl,
    Scf,
    Ccf,
    Daa,
    Rst(u16),
    Stop,
}

fn resolve_common_operand_from_col(col: u8) -> CommonOperand {
    let offset_col = if col < 0x8 {
        col
    } else {
        col - 0x8
    };
    match offset_col {
        0x0 => CommonOperand::Register(RegisterU8::B),
        0x1 => CommonOperand::Register(RegisterU8::C),
        0x2 => CommonOperand::Register(RegisterU8::D),
        0x3 => CommonOperand::Register(RegisterU8::E),
        0x4 => CommonOperand::Register(RegisterU8::H),
        0x5 => CommonOperand::Register(RegisterU8::L),
        0x6 => CommonOperand::AddressHL,
        0x7 => CommonOperand::Register(RegisterU8::A),
        _ => panic!("Invalid offset column: {}", offset_col),
    }
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
        0xF8 => Some(Instruction::LoadHlWithOffsetSp),
        0xF9 => Some(Instruction::LoadU16 {
            dst: LoadDstU16::StackPointer,
            src: LoadSrcU16::Register(RegisterU16::HL),
        }),
        _ => None,
    }
}

fn try_decode_call_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xC4 => Instruction::Call(Some(FlagCondition::NZ)),
        0xCC => Instruction::Call(Some(FlagCondition::Z)),
        0xCD => Instruction::Call(None),

        0xD4 => Instruction::Call(Some(FlagCondition::NC)),
        0xDC => Instruction::Call(Some(FlagCondition::C)),
        _ => return None,
    })
}

fn try_decode_relative_jump_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0x18 => Instruction::JumpRelative(None),

        0x20 => Instruction::JumpRelative(Some(FlagCondition::NZ)),
        0x28 => Instruction::JumpRelative(Some(FlagCondition::Z)),

        0x30 => Instruction::JumpRelative(Some(FlagCondition::NC)),
        0x38 => Instruction::JumpRelative(Some(FlagCondition::C)),
        _ => return None,
    })
}

fn try_decode_ret_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xC0 => Instruction::Ret(Some(FlagCondition::NZ)),
        0xC8 => Instruction::Ret(Some(FlagCondition::Z)),
        0xC9 => Instruction::Ret(None),

        0xD0 => Instruction::Ret(Some(FlagCondition::NC)),
        0xD8 => Instruction::Ret(Some(FlagCondition::C)),

        0xD9 => Instruction::Reti,
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

fn try_decode_inc_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0x03 => Instruction::IncU16(U16Target::RegisterU16(RegisterU16::BC)),
        0x13 => Instruction::IncU16(U16Target::RegisterU16(RegisterU16::DE)),
        0x23 => Instruction::IncU16(U16Target::RegisterU16(RegisterU16::HL)),
        0x33 => Instruction::IncU16(U16Target::StackPointer),

        0x04 => Instruction::IncU8(IncDecU8Target::RegisterU8(RegisterU8::B)),
        0x14 => Instruction::IncU8(IncDecU8Target::RegisterU8(RegisterU8::D)),
        0x24 => Instruction::IncU8(IncDecU8Target::RegisterU8(RegisterU8::H)),
        0x34 => Instruction::IncU8(IncDecU8Target::Address(RegisterU16::HL)),

        0x0C => Instruction::IncU8(IncDecU8Target::RegisterU8(RegisterU8::C)),
        0x1C => Instruction::IncU8(IncDecU8Target::RegisterU8(RegisterU8::E)),
        0x2C => Instruction::IncU8(IncDecU8Target::RegisterU8(RegisterU8::L)),
        0x3C => Instruction::IncU8(IncDecU8Target::RegisterU8(RegisterU8::A)),
        _ => return None,
    })
}

fn try_decode_or_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xB0..=0xB7 => {
            let operand = resolve_common_operand_from_col(opcode & 0xF);
            Instruction::Or(LogicalOpTarget::Common(operand))
        },
        0xF6 => Instruction::Or(LogicalOpTarget::ImmediateU8),
        _ => return None,
    })
}

fn try_decode_compare_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xB8..=0xBF => {
            let operand = resolve_common_operand_from_col(opcode & 0xF);
            Instruction::Compare(LogicalOpTarget::Common(operand))
        },
        0xFE => Instruction::Compare(LogicalOpTarget::ImmediateU8),
        _ => return None,
    })
}

fn try_decode_and_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xA0..=0xA7 => {
            let operand = resolve_common_operand_from_col(opcode & 0xF);
            Instruction::And(LogicalOpTarget::Common(operand))
        },
        0xE6 => Instruction::And(LogicalOpTarget::ImmediateU8),
        _ => return None,
    })
}

fn try_decode_dec_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0x0B => Instruction::DecU16(U16Target::RegisterU16(RegisterU16::BC)),
        0x1B => Instruction::DecU16(U16Target::RegisterU16(RegisterU16::DE)),
        0x2B => Instruction::DecU16(U16Target::RegisterU16(RegisterU16::HL)),
        0x3B => Instruction::DecU16(U16Target::StackPointer),

        0x05 => Instruction::DecU8(IncDecU8Target::RegisterU8(RegisterU8::B)),
        0x15 => Instruction::DecU8(IncDecU8Target::RegisterU8(RegisterU8::D)),
        0x25 => Instruction::DecU8(IncDecU8Target::RegisterU8(RegisterU8::H)),
        0x35 => Instruction::DecU8(IncDecU8Target::Address(RegisterU16::HL)),

        0x0D => Instruction::DecU8(IncDecU8Target::RegisterU8(RegisterU8::C)),
        0x1D => Instruction::DecU8(IncDecU8Target::RegisterU8(RegisterU8::E)),
        0x2D => Instruction::DecU8(IncDecU8Target::RegisterU8(RegisterU8::L)),
        0x3D => Instruction::DecU8(IncDecU8Target::RegisterU8(RegisterU8::A)),
        _ => return None,
    })
}


fn try_decode_xor_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xA8..=0xAF => {
            let operand = resolve_common_operand_from_col(opcode & 0xF);
            Instruction::Xor(LogicalOpTarget::Common(operand))
        },
        0xEE => Instruction::Xor(LogicalOpTarget::ImmediateU8),
        _ => return None,
    })
}

fn try_decode_add_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {

        0x09 => Instruction::AddU16(U16Target::RegisterU16(RegisterU16::BC)),
        0x19 => Instruction::AddU16(U16Target::RegisterU16(RegisterU16::DE)),
        0x29 => Instruction::AddU16(U16Target::RegisterU16(RegisterU16::HL)),
        0x39 => Instruction::AddU16(U16Target::StackPointer),

        0xE8 => Instruction::AddStackPointer,

        0x80..=0x87 => {
            let operand = resolve_common_operand_from_col(opcode & 0xF);
            Instruction::AddU8(LogicalOpTarget::Common(operand))
        }

        0xC6 => Instruction::AddU8(LogicalOpTarget::ImmediateU8),
        _ => return None,
    })
}

fn try_decode_sub_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0x90..=0x97 => {
            let operand = resolve_common_operand_from_col(opcode & 0xF);
            Instruction::Sub(LogicalOpTarget::Common(operand))
        },
        0xD6 => Instruction::Sub(LogicalOpTarget::ImmediateU8),
        _ => return None,
    })
}

fn try_decode_adc_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0x88..=0x8F => {
            let operand = resolve_common_operand_from_col(opcode & 0xF);
            Instruction::Adc(LogicalOpTarget::Common(operand))
        },
        0xCE => Instruction::Adc(LogicalOpTarget::ImmediateU8),
        _ => return None,
    })
}

fn try_decode_sbc_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0x98..=0x9F => {
            let operand = resolve_common_operand_from_col(opcode & 0xF);
            Instruction::Sbc(LogicalOpTarget::Common(operand))
        },
        0xDE => Instruction::Sbc(LogicalOpTarget::ImmediateU8),
        _ => return None,
    })
}

fn try_decode_jp_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xC2 => Instruction::JumpImmediate(Some(FlagCondition::NZ)),
        0xD2 => Instruction::JumpImmediate(Some(FlagCondition::NC)),

        0xC3 => Instruction::JumpImmediate(None),

        0xCA => Instruction::JumpImmediate(Some(FlagCondition::Z)),
        0xDA => Instruction::JumpImmediate(Some(FlagCondition::C)),
        _ => return None,
    })
}

fn try_decode_rst_instruction(opcode: u8) -> Option<Instruction> {
    Some(match opcode {
        0xC7 => Instruction::Rst(0x00),
        0xD7 => Instruction::Rst(0x10),
        0xE7 => Instruction::Rst(0x20),
        0xF7 => Instruction::Rst(0x30),

        0xCF => Instruction::Rst(0x08),
        0xDF => Instruction::Rst(0x18),
        0xEF => Instruction::Rst(0x28),
        0xFF => Instruction::Rst(0x38),

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

    if let Some(instruction) = try_decode_inc_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_or_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_compare_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_and_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_dec_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_xor_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_add_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_sub_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_adc_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_sbc_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_jp_instruction(opcode) {
        return Some(instruction);
    }

    if let Some(instruction) = try_decode_rst_instruction(opcode) {
        return Some(instruction);
    }

    match opcode {
        0x00 => Some(Instruction::Noop),
        0x07 => Some(Instruction::Rlca),
        0x0F => Some(Instruction::Rrca),
        0x10 => Some(Instruction::Stop),
        0x17 => Some(Instruction::Rla),
        0x1F => Some(Instruction::Rra),
        0x27 => Some(Instruction::Daa),
        0x2F => Some(Instruction::Cpl),
        0x37 => Some(Instruction::Scf),
        0x3F => Some(Instruction::Ccf),
        0x76 => Some(Instruction::Halt),
        0xE9 => Some(Instruction::JumpAddressHL),
        0xF3 => Some(Instruction::DisableInterrupts),
        0xFB => Some(Instruction::EnableInterrupts),
        _ => None,
    }
}

// https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
pub fn decode_cb(opcode: u8) -> Option<Instruction> {
    let target = resolve_common_operand_from_col(opcode & 0xF);
    Some(match opcode {
        0x00..=0x07 => Instruction::CbRlc(target),
        0x08..=0x0F => Instruction::CbRrc(target),
        0x10..=0x17 => Instruction::CbRl(target),
        0x18..=0x1F => Instruction::CbRr(target),
        0x20..=0x27 => Instruction::CbSla(target),
        0x28..=0x2F => Instruction::CbSra(target),
        0x38..=0x3F => Instruction::CbSrl(target),
        0x30..=0x37 => Instruction::CbSwap(target),
        0x40..=0x47 => Instruction::CbBit { n: 0, target, },
        0x48..=0x4F => Instruction::CbBit { n: 1, target, },
        0x50..=0x57 => Instruction::CbBit { n: 2, target, },
        0x58..=0x5F => Instruction::CbBit { n: 3, target, },
        0x60..=0x67 => Instruction::CbBit { n: 4, target, },
        0x68..=0x6F => Instruction::CbBit { n: 5, target, },
        0x70..=0x77 => Instruction::CbBit { n: 6, target, },
        0x78..=0x7F => Instruction::CbBit { n: 7, target, },
        0x80..=0x87 => Instruction::CbRes { n: 0, target, },
        0x88..=0x8F => Instruction::CbRes { n: 1, target, },
        0x90..=0x97 => Instruction::CbRes { n: 2, target, },
        0x98..=0x9F => Instruction::CbRes { n: 3, target, },
        0xA0..=0xA7 => Instruction::CbRes { n: 4, target, },
        0xA8..=0xAF => Instruction::CbRes { n: 5, target, },
        0xB0..=0xB7 => Instruction::CbRes { n: 6, target, },
        0xB8..=0xBF => Instruction::CbRes { n: 7, target, },
        0xC0..=0xC7 => Instruction::CbSet { n: 0, target, },
        0xC8..=0xCF => Instruction::CbSet { n: 1, target, },
        0xD0..=0xD7 => Instruction::CbSet { n: 2, target, },
        0xD8..=0xDF => Instruction::CbSet { n: 3, target, },
        0xE0..=0xE7 => Instruction::CbSet { n: 4, target, },
        0xE8..=0xEF => Instruction::CbSet { n: 5, target, },
        0xF0..=0xF7 => Instruction::CbSet { n: 6, target, },
        0xF8..=0xFF => Instruction::CbSet { n: 7, target, },
    })
}
