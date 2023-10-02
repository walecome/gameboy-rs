use std::fmt;

use crate::gameboy::instruction_decoder::decode_cb;

use super::instruction_decoder::{
    decode, FlagCondition, IncDecU8Target, Instruction, LoadDstU16, LoadDstU8, LoadSrcU16,
    LoadSrcU8, LogicalOpTarget, RegisterU16, RegisterU8, U16Target, CbTarget,
};

use super::mmu::{MMU, Word};
use super::address::Address;

use super::reference::ReferenceMetadata;

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

#[derive(Debug)]
pub struct FlagChange {
    z: Option<bool>,
    n: Option<bool>,
    h: Option<bool>,
    c: Option<bool>,
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
    pc: u16,
    sp: u16,
    mmu: MMU<'a>,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    interrupts_enabled: bool,
    // TODO: This should probably be register 'f'
    flags: Flags,

    // Debug
    depth: usize,
}

impl fmt::Debug for CPU<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CPU")
            .field("pc", &format_args!("{:#06X}", &self.pc))
            .field("sp", &format_args!("{:#06X}", &self.sp))
            .field("mmu", &"<omitted>".to_owned())
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
    } else { None };

    if let Some(message) = maybe_error_message {
        println!("CPU (tick {}): {:#?}", i, cpu);
        panic!("{}", message);
    }
}

impl CPU<'_> {
    pub fn new<'a>(rom_data: &'a Vec<u8>) -> CPU<'a> {
        CPU {
            pc: 0x0100,
            sp: 0x0FFFE,
            mmu: MMU::new(rom_data),
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
        let instruction = if opcode == 0xCB {
            let cb_opcode = self.read_u8();
            decode_cb(cb_opcode).expect(format!("Unknown CB opcode: {:#06X}: {:#04X}", pc, cb_opcode).as_str())
        } else {
            decode(opcode).expect(format!("Unknown opcode: {:#06X}: {:#04X}", pc, opcode).as_str())
        };
        print!("{:.<1$}", "", 1 * self.depth);
        println!("{:#06X}: {:#04X} ({:?})", pc, opcode, instruction);

        verify_state(self, maybe_metadata, i, pc);

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
            Instruction::CbSrl(target) => self.srl(target),
            Instruction::CbRr(target) => self.rr(target),
            Instruction::Rra => self.rra(),
            Instruction::CbBit { n, target } => self.bit(n, target),
        }

        return true;
    }

    fn next_pc(&mut self) -> u16 {
        let tmp = self.pc;
        self.pc += 1;
        return tmp;
    }

    fn read_u8(&mut self) -> u8 {
        let address = Address::new(self.next_pc());
        self.mmu.read_rom(address)
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
                let addr = self.resolve_u16_reg(&reg).get();
                self.mmu.read(Address::new(addr))
            }
            LoadSrcU8::AddressU8(reg) => {
                let lower_addr = *self.resolve_u8_reg(reg);
                self.mmu.read(Address::from_lower(lower_addr))
            }
            LoadSrcU8::ImmediateAddressU8 => {
                let lower_addr = self.read_u8();
                self.mmu.read(Address::from_lower(lower_addr))
            }
            LoadSrcU8::ImmediateAddressU16 => {
                let addr = self.read_u16();
                self.mmu.read(Address::new(addr))
            }
            LoadSrcU8::ImmediateU8 => self.read_u8(),
            LoadSrcU8::AddressU16Increment(reg) => {
                let addr = self.resolve_u16_reg(&reg).get();
                let value = self.mmu.read(Address::new(addr));
                self.resolve_u16_reg(&reg).set(addr + 1);
                value
            }
            LoadSrcU8::AddressU16Decrement(reg) => {
                let addr = self.resolve_u16_reg(&reg).get();
                let value = self.mmu.read(Address::new(addr));
                self.resolve_u16_reg(&reg).set(addr - 1);
                value
            }
        }
    }

    fn write_u8_target(&mut self, target: LoadDstU8, value: u8) {
        match target {
            LoadDstU8::Register(reg) => {
                *self.resolve_u8_reg(reg) = value;
            }
            LoadDstU8::AddressU8(reg) => {
                let lower_addr = *self.resolve_u8_reg(reg);
                self.mmu.write(Address::from_lower(lower_addr), value);
            }
            LoadDstU8::AddressU16(reg) => {
                let addr = self.resolve_u16_reg(&reg).get();
                self.mmu.write(Address::new(addr), value)
            }
            LoadDstU8::AddressU16Increment(reg) => {
                let addr = self.resolve_u16_reg(&reg).get();
                self.mmu.write(Address::new(addr), value);
                self.resolve_u16_reg(&reg).set(addr + 1);
            }
            LoadDstU8::AddressU16Decrement(reg) => {
                let addr = self.resolve_u16_reg(&reg).get();
                self.mmu.write(Address::new(addr), value);
                self.resolve_u16_reg(&reg).set(addr - 1);
            }
            LoadDstU8::ImmediateAddressU8 => {
                let lower_addr = self.read_u8();
                self.mmu.write(Address::from_lower(lower_addr), value);
            }
            LoadDstU8::ImmediateAddressU16 => {
                let addr = self.read_u16();
                self.mmu.write(Address::new(addr), value);
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
                let addr = self.read_u16();
                self.mmu.write_word(Address::new(addr), Word::new(value));
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
        self.mmu.write_word(Address::new(self.sp), Word::new(value));
    }

    fn stack_pop(&mut self) -> u16 {
        let word = self.mmu.read_word(Address::new(self.sp));
        self.sp += 2;
        word.value
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
                let address = Address::new(self.resolve_u16_reg(&reg).get());
                let value = self.mmu.read(address).wrapping_add(1);
                self.mmu.write(address, value);
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
                let address = Address::new(self.resolve_u16_reg(&reg).get());
                let value = self.mmu.read(address).wrapping_sub(1);
                self.mmu.write(address, value);
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

    fn rra(&mut self) {
        self.rr(CbTarget::Register(RegisterU8::A));
        self.flags.z = false;
    }

    fn srl(&mut self, target: CbTarget) {
        self.apply_cb_target(target, |value| {
            let carry = value & 0x1 != 0;

            let result = value >> 1;

            return (Some(result), FlagChange {
                z: Some(result == 0),
                n: Some(false),
                h: Some(false),
                c: Some(carry),
            });
        });
    }

    fn rr(&mut self, target: CbTarget) {
        let old_carry = self.flags.c;

        self.apply_cb_target(target, |value| {
            let new_carry = value & 0x1 != 0;

            let result = if old_carry {
                (value >> 1) | (1 << 7)
            } else {
                value >> 1
            };

            return (Some(result), FlagChange {
                z: Some(result == 0),
                n: Some(false),
                h: Some(false),
                c: Some(new_carry),
            });
        });
    }

    fn bit(&mut self, n: u8, target: CbTarget) {
        self.apply_cb_target(target, |value| {
            let z = value & (1 << n) != 0;

            return (None, FlagChange {
                z: Some(z),
                n: Some(false),
                h: Some(true),
                c: None,
            });
        });
    }

    fn apply_cb_target(&mut self, target: CbTarget, applier: impl Fn(u8) -> (Option<u8>, FlagChange)) {
        let value: u8 = match target {
            CbTarget::Register(reg) => {
                *self.resolve_u8_reg(reg)
            },
            CbTarget::AddressHL => {
                let address = Address::new(self.hl());
                self.mmu.read(address)
            }
        };

        let (maybe_result, flag_change) = applier(value);

        self.apply_flag_change(flag_change);

        if let Some(result) = maybe_result {
            match target {
                CbTarget::Register(reg) => {
                    *self.resolve_u8_reg(reg) = result;
                },
                CbTarget::AddressHL => {
                    let address = Address::new(self.hl());
                    self.mmu.write(address, result);
                }
            };
        }
    }

    fn apply_flag_change(&mut self, flag_change: FlagChange) {
        if let Some(z) = flag_change.z {
            self.flags.z = z;
        }

        if let Some(n) = flag_change.n {
            self.flags.n = n;
        }

        if let Some(h) = flag_change.h {
            self.flags.h = h;
        }

        if let Some(c) = flag_change.c {
            self.flags.c = c;
        }
    }

    fn resolve_logical_op_target(&mut self, target: LogicalOpTarget) -> u8 {
        match target {
            LogicalOpTarget::Register(reg) => *self.resolve_u8_reg(reg),
            LogicalOpTarget::AddressHL => {
                let addr = self.resolve_u16_reg(&RegisterU16::HL).get();
                self.mmu.read(Address::new(addr))
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

    fn hl(&mut self) -> u16 {
        self.resolve_u16_reg(&RegisterU16::HL).get()
    }
}
