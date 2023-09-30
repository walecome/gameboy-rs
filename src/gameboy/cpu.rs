use std::fmt;

use super::instruction_decoder::{
    decode, FlagCondition, IncDecU8Target, Instruction, LoadDstU16, LoadDstU8, LoadSrcU16,
    LoadSrcU8, LogicalOpTarget, RegisterU16, RegisterU8, U16Target,
};

use super::memory::Memory;

use super::reference::{ReferenceMetadata, ReferenceOpcode};

struct RegisterPair<'a> {
    high: &'a mut u8,
    low: &'a mut u8,
}

impl RegisterPair<'_> {
    fn get(&self) -> u16 {
        let high = self.high.clone() as u16;
        let low = self.low.clone() as u16;
        return high << 8 | low;
    }

    fn set(&mut self, value: u16) {
        let high = (value & 0xFF00) >> 8;
        let low = value & 0x00FF;

        *self.high = high as u8;
        *self.low = low as u8;
    }
}

#[derive(Debug)]
pub struct Flags {
    z: bool,
    n: bool,
    h: bool,
    c: bool,
}

impl Flags {
    pub fn new() -> Flags {
        Flags {
            z: false,
            n: false,
            h: false,
            c: false,
        }
    }
}

pub struct CPU<'a> {
    rom_data: &'a Vec<u8>,
    pc: u16,
    sp: u16,
    memory: Memory,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    interrupts_enabled: bool,
    flags: Flags,

    // Debug
    depth: usize,
}

impl fmt::Debug for CPU<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CPU")
            .field("rom_data", &"<omitted>".to_owned())
            .field("pc", &format_args!("{:#06X}", &self.pc))
            .field("sp", &format_args!("{:#06X}", &self.sp))
            .field("memory", &"<omitted>".to_owned())
            .field("a", &format_args!("{:#04X}", &self.a))
            .field("b", &format_args!("{:#04X}", &self.b))
            .field("c", &format_args!("{:#04X}", &self.c))
            .field("d", &format_args!("{:#04X}", &self.d))
            .field("e", &format_args!("{:#04X}", &self.e))
            .field("f", &format_args!("{:#04X}", &self.f))
            .field("h", &format_args!("{:#04X}", &self.h))
            .field("l", &format_args!("{:#04X}", &self.l))
            .field("interrupts_enabled", &self.interrupts_enabled)
            .field("flags", &self.flags)
            .finish()
    }
}

fn verify_state(
    cpu: &CPU,
    maybe_metadata: Option<&ReferenceMetadata>,
    i: usize,
    pc: u16,
    opcode: u8,
) {
    if maybe_metadata.is_none() {
        return;
    }
    let metadata = maybe_metadata.unwrap();

    let maybe_error_message = if pc != metadata.pc {
        Some(format!(
            "PC({:#06X}) != reference PC ({:#06X}). Metadata: {}",
            pc, metadata.pc, metadata.instruction
        ))
    } else {
        match metadata.opcode {
            ReferenceOpcode::Plain(reference_opcode) => {
                if opcode != reference_opcode {
                    Some(format!(
                        "opcode({:#04X}) != reference opcode ({:#04X}). Metadata: {}",
                        opcode, reference_opcode, metadata.instruction
                    ))
                } else {
                    None
                }
            }
            ReferenceOpcode::CB(_) => todo!(),
        }
    };

    if let Some(message) = maybe_error_message {
        println!("CPU (tick {}): {:#?}", i, cpu);
        panic!("{}", message);
    }
}

impl CPU<'_> {
    pub fn new<'a>(rom_data: &'a Vec<u8>) -> CPU<'a> {
        CPU {
            rom_data: &rom_data,
            pc: 0x0100,
            sp: 0x0FFFE,
            memory: Memory::new(),
            a: 0x00,
            b: 0x00,
            c: 0x00,
            d: 0x00,
            e: 0x00,
            f: 0x00,
            h: 0x00,
            l: 0x00,
            interrupts_enabled: false,
            flags: Flags::new(),
            depth: 0,
        }
    }
    pub fn tick(&mut self, maybe_metadata: Option<&ReferenceMetadata>, i: usize) -> bool {
        let pc = self.pc;
        let opcode = self.read_u8();
        if opcode == 0xCB {
            todo!("Implement CB instructions");
        }
        let instruction =
            decode(opcode).expect(format!("Unknown opcode: {:#06X}: {:#04X}", pc, opcode).as_str());
        print!("{:.<1$}", "", 1 * self.depth);
        println!("{:#06X}: {:#04X} ({:?})", pc, opcode, instruction);

        verify_state(self, maybe_metadata, i, pc, opcode);

        match instruction {
            Instruction::Noop => {}
            Instruction::LoadU8 { dst, src } => {
                let value = self.read_u8_target(src);
                self.write_u8_target(dst, value);
            }
            Instruction::Halt => return false,
            Instruction::JumpImmediate => {
                let address = self.read_u16();
                self.pc = address;
            }
            Instruction::DisableInterrupts => self.interrupts_enabled = false,
            Instruction::LoadU16 { dst, src } => {
                let value = self.read_u16_target(src);
                self.write_u16_target(dst, value);
            }
            Instruction::Call(condition) => {
                self.call(condition);
                self.depth += 1;
            }
            Instruction::JumpRelative(condition) => self.relative_jump(condition),
            Instruction::Ret(condition) => {
                self.depth -= 1;
                self.ret(condition);
            }
            Instruction::Push(reg) => self.push(reg),
            Instruction::Pop(reg) => self.pop(reg),
            Instruction::Or(target) => self.or(target),
            Instruction::IncU8(target) => self.inc_u8(target),
            Instruction::IncU16(target) => self.inc_u16(target),
            Instruction::Compare(target) => self.compare(target),
            Instruction::And(target) => self.and(target),
            Instruction::DecU8(target) => self.dec_u8(target),
            Instruction::DecU16(target) => self.dec_u16(target),
            Instruction::Xor(target) => self.xor(target),
            Instruction::AddStackPointer => self.add_stackpointer_immediate(),
            Instruction::AddU8(target) => self.add_u8(target),
            Instruction::AddU16(target) => self.add_u16(target),
            Instruction::Sub(target) => self.sub(target),
        }

        return true;
    }

    fn next_pc(&mut self) -> u16 {
        let tmp = self.pc;
        self.pc += 1;
        return tmp;
    }

    fn read_u8(&mut self) -> u8 {
        self.rom_data[self.next_pc() as usize]
    }

    fn read_u16(&mut self) -> u16 {
        let a = self.read_u8() as u16;
        let b = self.read_u8() as u16;
        return b << 8 | a;
    }

    fn resolve_u8_reg(&mut self, reg: RegisterU8) -> &mut u8 {
        match reg {
            RegisterU8::A => &mut self.a,
            RegisterU8::B => &mut self.b,
            RegisterU8::C => &mut self.c,
            RegisterU8::D => &mut self.d,
            RegisterU8::E => &mut self.e,
            RegisterU8::F => &mut self.f,
            RegisterU8::H => &mut self.h,
            RegisterU8::L => &mut self.l,
        }
    }

    fn resolve_u16_reg(&mut self, reg: &RegisterU16) -> RegisterPair {
        let (high, low) = match reg {
            RegisterU16::AF => (&mut self.a, &mut self.f),
            RegisterU16::BC => (&mut self.b, &mut self.c),
            RegisterU16::DE => (&mut self.d, &mut self.e),
            RegisterU16::HL => (&mut self.h, &mut self.l),
        };
        RegisterPair { high, low }
    }

    fn read_u8_target(&mut self, target: LoadSrcU8) -> u8 {
        match target {
            LoadSrcU8::Register(reg) => *self.resolve_u8_reg(reg),
            LoadSrcU8::AddressU16(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.memory.get(address)
            }
            LoadSrcU8::AddressU8(reg) => {
                let lower_address = *self.resolve_u8_reg(reg);
                self.memory.get_from_u8(lower_address)
            }
            LoadSrcU8::ImmediateAddressU8 => {
                let lower_address = self.read_u8();
                self.memory.get_from_u8(lower_address)
            }
            LoadSrcU8::ImmediateAddressU16 => {
                let address = self.read_u16();
                self.memory.get(address)
            }
            LoadSrcU8::ImmediateU8 => self.read_u8(),
            LoadSrcU8::AddressU16Increment(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.resolve_u16_reg(&reg).set(address + 1);
                self.memory.get(address)
            }
            LoadSrcU8::AddressU16Decrement(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.resolve_u16_reg(&reg).set(address - 1);
                self.memory.get(address)
            }
        }
    }

    fn write_u8_target(&mut self, target: LoadDstU8, value: u8) {
        match target {
            LoadDstU8::Register(reg) => {
                *self.resolve_u8_reg(reg) = value;
            }
            LoadDstU8::AddressU8(reg) => {
                let lower_address = *self.resolve_u8_reg(reg);
                self.memory.set_from_u8(lower_address, value);
            }
            LoadDstU8::AddressU16(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.memory.set(address, value);
            }
            LoadDstU8::AddressU16Increment(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.resolve_u16_reg(&reg).set(address + 1);
                self.memory.set(address, value);
            }
            LoadDstU8::AddressU16Decrement(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.resolve_u16_reg(&reg).set(address - 1);
                self.memory.set(address, value);
            }
            LoadDstU8::ImmediateAddressU8 => {
                let lower_address = self.read_u8();
                self.memory.set_from_u8(lower_address, value);
            }
            LoadDstU8::ImmediateAddressU16 => {
                let address = self.read_u16();
                self.memory.set(address, value);
            }
        }
    }

    fn read_u16_target(&mut self, target: LoadSrcU16) -> u16 {
        match target {
            LoadSrcU16::Register(reg) => self.resolve_u16_reg(&reg).get(),
            LoadSrcU16::ImmediateU16 => self.read_u16(),
            LoadSrcU16::StackPointer => self.sp,
            LoadSrcU16::StackPointerWithOffset => {
                let offset = self.read_u8() as i8 as i16;
                let signed_sp = self.sp as i16;
                (signed_sp + offset) as u16
            }
        }
    }

    fn write_u16_target(&mut self, target: LoadDstU16, value: u16) {
        match target {
            LoadDstU16::Register(reg) => {
                self.resolve_u16_reg(&reg).set(value);
            }
            LoadDstU16::StackPointer => {
                self.sp = value;
            }
            LoadDstU16::ImmediateAddress => {
                let address = self.read_u16();
                self.memory.set_u16(address, value);
            }
        }
    }

    fn call(&mut self, condition: Option<FlagCondition>) {
        let target_address = self.read_u16();
        if self.is_flag_condition_true(condition) {
            self.stack_push(self.pc);
            self.pc = target_address;
        }
    }

    fn stack_push(&mut self, value: u16) {
        self.sp -= 2;
        self.memory.set_u16(self.sp, value);
    }

    fn stack_pop(&mut self) -> u16 {
        let value = self.memory.get_u16(self.sp);
        self.sp += 2;
        value
    }

    fn relative_jump(&mut self, condition: Option<FlagCondition>) {
        let offset = self.read_u8() as i8 as i16;
        let signed_pc = self.pc as i16;
        let new_pc = signed_pc + offset;

        if self.is_flag_condition_true(condition) {
            self.pc = new_pc as u16;
        }
    }

    fn ret(&mut self, condition: Option<FlagCondition>) {
        if self.is_flag_condition_true(condition) {
            let new_pc = self.stack_pop();
            self.pc = new_pc;
        }
    }

    fn push(&mut self, reg: RegisterU16) {
        let value = self.resolve_u16_reg(&reg).get();
        self.stack_push(value);
    }

    fn pop(&mut self, reg: RegisterU16) {
        let value = self.stack_pop();
        self.resolve_u16_reg(&reg).set(value);
    }

    fn inc_u8(&mut self, target: IncDecU8Target) {
        let result = match target {
            IncDecU8Target::RegisterU8(reg) => {
                let current = self.resolve_u8_reg(reg);
                *current = current.wrapping_add(1);
                *current
            }
            IncDecU8Target::Address(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                let value = self.memory.get(address).wrapping_add(1);
                self.memory.set(address, value);
                value
            }
        };

        self.flags.z = result == 0;
        self.flags.n = false;
        self.flags.h = (result & 0x0F) == 0x00;
    }

    fn inc_u16(&mut self, target: U16Target) {
        match target {
            U16Target::RegisterU16(reg) => {
                let current = self.resolve_u16_reg(&reg).get();
                let value = current.wrapping_add(1);
                self.resolve_u16_reg(&reg).set(value);
            }
            U16Target::StackPointer => {
                self.sp += 1;
            }
        };
    }

    fn dec_u8(&mut self, target: IncDecU8Target) {
        let result = match target {
            IncDecU8Target::RegisterU8(reg) => {
                let current = self.resolve_u8_reg(reg);
                *current = current.wrapping_sub(1);
                *current
            }
            IncDecU8Target::Address(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                let value = self.memory.get(address).wrapping_sub(1);
                self.memory.set(address, value);
                value
            }
        };

        self.flags.z = result == 0;
        self.flags.n = true;
        self.flags.h = (result & 0x0F) == 0x0F;
    }

    fn dec_u16(&mut self, target: U16Target) {
        match target {
            U16Target::RegisterU16(reg) => {
                let current = self.resolve_u16_reg(&reg).get();
                let value = current.wrapping_sub(1);
                self.resolve_u16_reg(&reg).set(value);
            }
            U16Target::StackPointer => {
                self.sp -= 1;
            }
        };
    }

    fn or(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        self.a = self.a | value;

        self.flags.z = self.a == 0;
        self.flags.n = false;
        self.flags.h = false;
        self.flags.c = false;
    }

    fn and(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        self.a = self.a & value;

        self.flags.z = self.a == 0;
        self.flags.n = false;
        self.flags.h = true;
        self.flags.c = false;
    }

    fn xor(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        self.a = self.a ^ value;

        self.flags.z = self.a == 0;
        self.flags.n = false;
        self.flags.h = false;
        self.flags.c = false;
    }

    fn add_u8(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        let half_carry = (self.a & 0xF) + (value & 0xF) > 0xF;

        let result = (self.a as u16) + (value as u16);

        self.a = result as u8;

        self.flags.z = self.a == 0;
        self.flags.n = false;
        self.flags.h = half_carry;
        self.flags.c = result > 0xFF;
    }

    fn add_u16(&mut self, target: U16Target) {
        let rhs = match target {
            U16Target::RegisterU16(reg) => self.resolve_u16_reg(&reg).get(),
            U16Target::StackPointer => self.sp,
        };
        let hl = self.resolve_u16_reg(&RegisterU16::HL).get();
        let result = (hl as u32) + (rhs as u32);

        self.resolve_u16_reg(&RegisterU16::HL).set(result as u16);

        self.flags.n = false;
        self.flags.h = (hl & 0xFFF) + (rhs & 0xFFF) > 0xFFF;
        self.flags.c = result > 0xFFFF;
    }

    fn sub(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        let half_carry = (self.a & 0xF) < (value & 0xF);
        let carry = self.a < value;

        self.a = self.a.wrapping_sub(value);

        self.flags.z = self.a == 0;
        self.flags.n = true;
        self.flags.h = half_carry;
        self.flags.c = carry;
    }

    fn add_stackpointer_immediate(&mut self) {
        let offset = self.read_u8() as i8 as i16;
        let signed_sp = self.sp as i16;
        let result = signed_sp + offset;

        self.sp = result as u16;

        let signed_mask = 0xFFFF as u16 as i16;

        self.flags.z = false;
        self.flags.n = false;
        self.flags.h = ((signed_sp ^ offset ^ (result & signed_mask)) & 0x10) == 0x10;
        self.flags.c = ((signed_sp ^ offset ^ (result & signed_mask)) & 0x100) == 0x100;
    }

    fn compare(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        let result = self.a.wrapping_sub(value);

        let nibble_a = self.a & 0xF;
        let nibble_value = self.a & 0xF;

        self.flags.z = result == 0;
        self.flags.n = true;
        self.flags.h = nibble_a < nibble_value;
        self.flags.c = self.a < value;
    }

    fn resolve_logical_op_target(&mut self, target: LogicalOpTarget) -> u8 {
        match target {
            LogicalOpTarget::Register(reg) => *self.resolve_u8_reg(reg),
            LogicalOpTarget::AddressHL => {
                let address = self.resolve_u16_reg(&RegisterU16::HL).get();
                self.memory.get(address)
            }
            LogicalOpTarget::ImmediateU8 => self.read_u8(),
        }
    }

    fn is_flag_condition_true(&self, condition: Option<FlagCondition>) -> bool {
        // No condition is always true
        if condition.is_none() {
            return true;
        }
        match condition.unwrap() {
            FlagCondition::Z => self.flags.z,
            FlagCondition::NZ => !self.flags.z,
            FlagCondition::C => self.flags.c,
            FlagCondition::NC => !self.flags.c,
        }
    }
}
