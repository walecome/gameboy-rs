use std::fmt;

use crate::gameboy::instruction_decoder::decode_cb;

use super::cartridge::Cartridge;
use super::instruction_decoder::{
    decode, FlagCondition, IncDecU8Target, Instruction, LoadDstU16, LoadDstU8, LoadSrcU16,
    LoadSrcU8, LogicalOpTarget, RegisterU16, RegisterU8, U16Target, CommonOperand,
};

use super::mmu::{MMU, Word};
use super::address::Address;
use super::utils::{get_bit, set_bit};

use super::reference::ReferenceMetadata;

use super::cycles;

struct RegisterPair<'a> {
    high: &'a mut u8,
    low: &'a mut u8,
}

struct ImmutableRegisterPair<'a> {
    high: &'a u8,
    low: &'a u8,
}

impl RegisterPair<'_> {
    fn get(&self) -> u16 {
        ImmutableRegisterPair { high: self.high, low: self.low, }.get()
    }

    fn set(&mut self, value: u16) {
        let high = (value & 0xFF00) >> 8;
        let low = value & 0x00FF;

        *self.high = high as u8;
        *self.low = low as u8;
    }
}

impl ImmutableRegisterPair<'_> {
    fn get(&self) -> u16 {
        let high = self.high.clone() as u16;
        let low = self.low.clone() as u16;
        return high << 8 | low;
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct FlagDebug {
    z: bool,
    n: bool,
    h: bool,
    c: bool,
}

pub struct FlagRegister {
    value: u8,
}

impl FlagRegister {
    fn new() -> Self {
        Self { value: 0x00, }
    }

    fn get_z(&self) -> bool {
        get_bit(self.value, 7)
    }

    fn get_n(&self) -> bool {
        get_bit(self.value, 6)
    }

    fn get_h(&self) -> bool {
        get_bit(self.value, 5)
    }

    fn get_c(&self) -> bool {
        get_bit(self.value, 4)
    }

    fn set_z(&mut self, bit_value: bool) {
        self.value = set_bit(self.value, 7, bit_value);
    }

    fn set_n(&mut self, bit_value: bool) {
        self.value = set_bit(self.value, 6, bit_value);
    }

    fn set_h(&mut self, bit_value: bool) {
        self.value = set_bit(self.value, 5, bit_value);
    }

    fn set_c(&mut self, bit_value: bool) {
        self.value = set_bit(self.value, 4, bit_value);
    }

    fn debug_obj(&self) -> FlagDebug {
        FlagDebug {
            z: self.get_z(),
            n: self.get_n(),
            h: self.get_h(),
            c: self.get_c(),
        }
    }
}

#[derive(Debug)]
pub struct FlagChange {
    z: Option<bool>,
    n: Option<bool>,
    h: Option<bool>,
    c: Option<bool>,
}

pub struct CPU {
    pc: u16,
    sp: u16,
    mmu: MMU,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    interrupts_enabled: bool,
    flag_register: FlagRegister,
    did_take_conditional_branch: bool,

    // Debug
    depth: usize,
    trace_cpu: bool,
}

impl fmt::Debug for CPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CPU")
            .field("pc", &format_args!("{:#06X}", &self.pc))
            .field("sp", &format_args!("{:#06X}", &self.sp))
            .field("mmu", &"<omitted>".to_owned())
            .field("AF", &format_args!("{:#06X}", &self.resolve_u16_reg_immutable(&RegisterU16::AF).get()))
            .field("BC", &format_args!("{:#06X}", &self.resolve_u16_reg_immutable(&RegisterU16::BC).get()))
            .field("DE", &format_args!("{:#06X}", &self.resolve_u16_reg_immutable(&RegisterU16::DE).get()))
            .field("HL", &format_args!("{:#06X}", &self.resolve_u16_reg_immutable(&RegisterU16::HL).get()))
            .field("interrupts_enabled", &self.interrupts_enabled)
            .field("flags", &self.flag_register.debug_obj())
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

enum OpcodeType {
    Normal,
    Cb,
}

impl CPU {
    pub fn new(cartridge: Box<dyn Cartridge>, trace_cpu: bool) -> CPU {
        CPU {
            pc: 0x0000,
            sp: 0x0FFFE,
            mmu: MMU::new(cartridge),
            a: 0x00,
            b: 0x00,
            c: 0x00,
            d: 0x00,
            e: 0x00,
            h: 0x00,
            l: 0x00,
            interrupts_enabled: false,
            flag_register: FlagRegister::new(),
            did_take_conditional_branch: false,
            depth: 0,
            trace_cpu,
        }
    }

    pub fn tick(&mut self, maybe_metadata: Option<&ReferenceMetadata>, i: usize) -> Option<u8> {
        self.did_take_conditional_branch = false;

        let pc = self.pc;
        let (instruction, opcode_type, opcode) = self.next_instruction();

        if self.trace_cpu {
            print!("{:.<1$}", "", 1 * self.depth);
            println!("{:#06X}: {:#04X} ({:?})", pc, opcode, instruction);
        }

        verify_state(self, maybe_metadata, i, pc);

        match instruction {
            Instruction::Noop => {}
            Instruction::LoadU8 { dst, src } => {
                let value = self.read_u8_target(src);
                self.write_u8_target(dst, value);
            }
            Instruction::Halt => return None,
            Instruction::JumpImmediate(condition) => self.jump_immediate(condition),
            Instruction::DisableInterrupts => self.interrupts_enabled = false,
            Instruction::EnableInterrupts => self.interrupts_enabled = true,
            Instruction::LoadU16 { dst, src } => {
                let value = self.read_u16_target(src);
                self.write_u16_target(dst, value);
            }
            Instruction::Call(condition) => self.call(condition),
            Instruction::JumpRelative(condition) => self.relative_jump(condition),
            Instruction::Ret(condition) => self.ret(condition),
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
            Instruction::CbRl(target) => self.rl(target),
            Instruction::CbRlc(target) => self.rlc(target),
            Instruction::Rra => self.rra(),
            Instruction::Rla => self.rla(),
            Instruction::Rlca => self.rlca(),
            Instruction::CbBit { n, target } => self.bit(n, target),
            Instruction::Adc(target) => self.adc(target),
            Instruction::Sbc(target) => self.sbc(target),
            Instruction::JumpAddressHL => {
                // NOTE: This instruction has conflicting documentation.
                //       It's specified as `JP (HL)`, so "PC = memory value for address HL".
                //       But some docs specify to just do `PC = HL`. The timing is also 4
                //       cycles, with one byte being read, so I believe it's the latter.
                self.pc = self.hl();
            }
            Instruction::CbSwap(target) => self.swap(target),
            Instruction::Cpl => self.cpl(),
            Instruction::Scf => self.scf(),
            Instruction::Ccf => self.ccf(),
        }

        return Some(match (self.did_take_conditional_branch, opcode_type) {
            (false, OpcodeType::Normal) => cycles::NORMAL_OPCODE_CYCLES[opcode as usize],
            (false, OpcodeType::Cb) => cycles::CB_OPCODE_CYCLES[opcode as usize],
            (true, OpcodeType::Normal) => cycles::NORMAL_OPCODE_CYCLES_BRANCED[opcode as usize],
            (true, OpcodeType::Cb) => unreachable!("CB opcodes shouldn't branch"),
        });
    }

    pub fn mmu(&mut self) -> &mut MMU {
        &mut self.mmu
    }

    fn next_instruction(&mut self) -> (Instruction, OpcodeType, u8) {
        let pc = self.pc;
        let opcode = self.read_u8();
        let is_cb_opcode = opcode == 0xCB;
        if is_cb_opcode {
            let cb_opcode = self.read_u8();
            let decoded = decode_cb(cb_opcode).expect(format!("Unknown CB opcode: {:#06X}: {:#04X}", pc, cb_opcode).as_str());
            return (decoded, OpcodeType::Cb, cb_opcode);
        }

        let decoded = decode(opcode).expect(format!("Unknown opcode: {:#06X}: {:#04X}", pc, opcode).as_str());
        return (decoded, OpcodeType::Normal, opcode);
    }

    fn next_pc(&mut self) -> u16 {
        let tmp = self.pc;
        self.pc += 1;
        return tmp;
    }

    fn read_u8(&mut self) -> u8 {
        let address = Address::new(self.next_pc());
        self.mmu.read(address)
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
            RegisterU8::H => &mut self.h,
            RegisterU8::L => &mut self.l,
        }
    }

    fn resolve_u16_reg(&mut self, reg: &RegisterU16) -> RegisterPair {
        let (high, low) = match reg {
            RegisterU16::AF => (&mut self.a, &mut self.flag_register.value),
            RegisterU16::BC => (&mut self.b, &mut self.c),
            RegisterU16::DE => (&mut self.d, &mut self.e),
            RegisterU16::HL => (&mut self.h, &mut self.l),
        };
        RegisterPair { high, low }
    }

    fn resolve_u16_reg_immutable(&self, reg: &RegisterU16) -> ImmutableRegisterPair {
        let (high, low) = match reg {
            RegisterU16::AF => (&self.a, &self.flag_register.value),
            RegisterU16::BC => (&self.b, &self.c),
            RegisterU16::DE => (&self.d, &self.e),
            RegisterU16::HL => (&self.h, &self.l),
        };
        ImmutableRegisterPair { high, low }
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
            self.depth += 1;
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

    fn jump_immediate(&mut self, condition: Option<FlagCondition>) {
        let address = self.read_u16();
        if self.is_flag_condition_true(condition) {
            self.pc = address;
        }
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
            assert!(self.depth != 0);
            self.depth -= 1;
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

        self.apply_flag_change(FlagChange {
            z: Some(result == 0),
            n: Some(false),
            h: Some((result & 0x0F) == 0x00),
            c: None,
        });
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

        self.apply_flag_change(FlagChange {
            z: Some(result == 0),
            n: Some(true),
            h: Some((result & 0x0F) == 0x0F),
            c: None,
        });
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

        self.apply_flag_change(FlagChange {
            z: Some(self.a == 0),
            n: Some(false),
            h: Some(false),
            c: Some(false),
        })
    }

    fn and(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        self.a = self.a & value;

        self.apply_flag_change(FlagChange {
            z: Some(self.a == 0),
            n: Some(false),
            h: Some(true),
            c: Some(false),
        })
    }

    fn xor(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        self.a = self.a ^ value;

        self.apply_flag_change(FlagChange {
            z: Some(self.a == 0),
            n: Some(false),
            h: Some(true),
            c: Some(false),
        })
    }

    fn add_u8(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        let half_carry = (self.a & 0xF) + (value & 0xF) > 0xF;

        let result = (self.a as u16) + (value as u16);

        self.a = result as u8;

        self.apply_flag_change(FlagChange {
            z: Some(self.a == 0),
            n: Some(false),
            h: Some(half_carry),
            c: Some(result > 0xFF),
        });
    }

    fn add_u16(&mut self, target: U16Target) {
        let rhs = match target {
            U16Target::RegisterU16(reg) => self.resolve_u16_reg(&reg).get(),
            U16Target::StackPointer => self.sp,
        };
        let hl = self.resolve_u16_reg(&RegisterU16::HL).get();
        let result = (hl as u32) + (rhs as u32);

        self.resolve_u16_reg(&RegisterU16::HL).set(result as u16);

        self.apply_flag_change(FlagChange {
            z: None,
            n: Some(false),
            h: Some((hl & 0xFFF) + (rhs & 0xFFF) > 0xFFF),
            c: Some(result > 0xFFFF),
        });
    }

    fn adc(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);
        let carry_value: u8 = if self.flag_register.get_c() { 1 } else { 0 };

        let half_carry = (self.a & 0xF) + (value & 0xF) + carry_value > 0xF;
        let result = (self.a as u16) + (value as u16) + (carry_value as u16);

        self.a = result as u8;

        self.apply_flag_change(FlagChange {
            z: Some(self.a == 0),
            n: Some(false),
            h: Some(half_carry),
            c: Some(result > 0xFF),
        });
    }

    fn sbc(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);
        let carry_value: u8 = if self.flag_register.get_c() { 1 } else { 0 };

        let new_carry = (self.a as u16) < (value as u16) + (carry_value as u16);
        let half_carry = (self.a & 0xF) < ((value & 0xF) + carry_value);

        self.a = self.a.wrapping_sub(value).wrapping_sub(carry_value);

        self.apply_flag_change(FlagChange {
            z: Some(self.a == 0),
            n: Some(true),
            h: Some(half_carry),
            c: Some(new_carry),
        });
    }

    fn sub(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        let half_carry = (self.a & 0xF) < (value & 0xF);
        let carry = self.a < value;

        self.a = self.a.wrapping_sub(value);

        self.apply_flag_change(FlagChange {
            z: Some(self.a == 0),
            n: Some(true),
            h: Some(half_carry),
            c: Some(carry),
        });
    }

    fn add_stackpointer_immediate(&mut self) {
        let offset = self.read_u8() as i8 as i16;
        let signed_sp = self.sp as i16;
        let result = signed_sp + offset;

        self.sp = result as u16;

        let signed_mask = 0xFFFF as u16 as i16;

        self.apply_flag_change(FlagChange {
            z: Some(false),
            n: Some(false),
            h: Some(((signed_sp ^ offset ^ (result & signed_mask)) & 0x10) == 0x10),
            c: Some(((signed_sp ^ offset ^ (result & signed_mask)) & 0x100) == 0x100),
        });
    }

    fn compare(&mut self, target: LogicalOpTarget) {
        let value = self.resolve_logical_op_target(target);

        let result = self.a.wrapping_sub(value);

        let nibble_a = self.a & 0xF;
        let nibble_value = self.a & 0xF;

        self.apply_flag_change(FlagChange {
            z: Some(result == 0),
            n: Some(true),
            h: Some(nibble_a < nibble_value),
            c: Some(self.a < value),
        });
    }

    fn rra(&mut self) {
        self.rr(CommonOperand::Register(RegisterU8::A));
        self.flag_register.set_z(false);
    }

    fn rla(&mut self) {
        self.rl(CommonOperand::Register(RegisterU8::A));
        self.flag_register.set_z(false);
    }

    fn rlca(&mut self) {
        self.rlc(CommonOperand::Register(RegisterU8::A));
        self.flag_register.set_z(false);
    }

    fn srl(&mut self, target: CommonOperand) {
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

    fn rr(&mut self, target: CommonOperand) {
        let old_carry = self.flag_register.get_c();

        self.apply_cb_target(target, |value| {
            let new_carry = get_bit(value, 0);

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

    fn rl(&mut self, target: CommonOperand) {
        let old_carry = self.flag_register.get_c();

        self.apply_cb_target(target, |value| {
            let new_carry = get_bit(value, 7);

            let result = if old_carry {
                (value << 1) | 1
            } else {
                value << 1
            };

            return (Some(result), FlagChange {
                z: Some(result == 0),
                n: Some(false),
                h: Some(false),
                c: Some(new_carry),
            });
        });
    }

    fn rlc(&mut self, target: CommonOperand) {
        self.apply_cb_target(target, |value| {
            let carry = get_bit(value, 7);

            let result = (value << 1) | if carry { 1 } else { 0 };

            return (Some(result), FlagChange {
                z: Some(result == 0),
                n: Some(false),
                h: Some(false),
                c: Some(carry),
            });
        });
    }

    fn bit(&mut self, n: u8, target: CommonOperand) {
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

    fn swap(&mut self, target: CommonOperand) {
        self.apply_cb_target(target, |value| {
            let result = swap_nibbles(value);

            return (Some(result), FlagChange {
                z: Some(result == 0),
                n: Some(false),
                h: Some(false),
                c: Some(false),
            })
        });
    }

    fn cpl(&mut self) {
        self.a = !self.a;

        self.apply_flag_change(FlagChange {
            z: None,
            n: Some(true),
            h: Some(true),
            c: None,
        });
    }

    fn scf(&mut self) {
        self.apply_flag_change(FlagChange {
            z: None,
            n: Some(false),
            h: Some(false),
            c: Some(true),
        });
    }

    fn ccf(&mut self) {
        self.apply_flag_change(FlagChange {
            z: None,
            n: Some(false),
            h: Some(false),
            c: Some(!self.flag_register.get_c()),
        });
    }

    fn apply_cb_target(&mut self, target: CommonOperand, applier: impl Fn(u8) -> (Option<u8>, FlagChange)) {
        let value: u8 = match target {
            CommonOperand::Register(reg) => {
                *self.resolve_u8_reg(reg)
            },
            CommonOperand::AddressHL => {
                let address = Address::new(self.hl());
                self.mmu.read(address)
            }
        };

        let (maybe_result, flag_change) = applier(value);

        self.apply_flag_change(flag_change);

        if let Some(result) = maybe_result {
            match target {
                CommonOperand::Register(reg) => {
                    *self.resolve_u8_reg(reg) = result;
                },
                CommonOperand::AddressHL => {
                    let address = Address::new(self.hl());
                    self.mmu.write(address, result);
                }
            };
        }
    }

    fn apply_flag_change(&mut self, flag_change: FlagChange) {
        if let Some(z) = flag_change.z {
            self.flag_register.set_z(z);
        }

        if let Some(n) = flag_change.n {
            self.flag_register.set_n(n);
        }

        if let Some(h) = flag_change.h {
            self.flag_register.set_h(h);
        }

        if let Some(c) = flag_change.c {
            self.flag_register.set_c(c);
        }
    }

    fn resolve_logical_op_target(&mut self, target: LogicalOpTarget) -> u8 {
        match target {
            LogicalOpTarget::Common(operand) => match operand {
                CommonOperand::Register(reg) => *self.resolve_u8_reg(reg),
                CommonOperand::AddressHL => {
                    let addr = self.resolve_u16_reg(&RegisterU16::HL).get();
                    self.mmu.read(Address::new(addr))
                }
            }
            LogicalOpTarget::ImmediateU8 => self.read_u8(),
        }
    }

    fn is_flag_condition_true(&mut self, condition: Option<FlagCondition>) -> bool {
        // No condition is always true
        if condition.is_none() {
            return true;
        }
        let is_condition_true = match condition.unwrap() {
            FlagCondition::Z => self.flag_register.get_z(),
            FlagCondition::NZ => !self.flag_register.get_z(),
            FlagCondition::C => self.flag_register.get_c(),
            FlagCondition::NC => !self.flag_register.get_c(),
        };
        self.did_take_conditional_branch = is_condition_true;
        return is_condition_true;
    }

    fn hl(&mut self) -> u16 {
        self.resolve_u16_reg(&RegisterU16::HL).get()
    }
}

fn swap_nibbles(value: u8) -> u8 {
   ((value & 0x0F) << 4) | ((value & 0xF0) >> 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_nibbles() {
        assert_eq!(swap_nibbles(0xAB), 0xBA);
        assert_eq!(swap_nibbles(0x0F), 0xF0);
        assert_eq!(swap_nibbles(0xF0), 0x0F);
    }

    #[test]
    fn test_get_bit() {
        assert_eq!(get_bit(0b1011_0010, 0), false);
        assert_eq!(get_bit(0b1011_0010, 1), true);
        assert_eq!(get_bit(0b1011_0010, 2), false);
        assert_eq!(get_bit(0b1011_0010, 3), false);
        assert_eq!(get_bit(0b1011_0010, 4), true);
        assert_eq!(get_bit(0b1011_0010, 5), true);
        assert_eq!(get_bit(0b1011_0010, 6), false);
        assert_eq!(get_bit(0b1011_0010, 7), true);
    }

    #[test]
    fn test_set_bit() {
        assert_eq!(set_bit(0b1011_0010, 0, true), 0b1011_0011);
        assert_eq!(set_bit(0b1011_0010, 1, false), 0b1011_0000);
        assert_eq!(set_bit(0b1011_0010, 2, false), 0b1011_0010);
        assert_eq!(set_bit(0b1011_0010, 3, true), 0b1011_1010);
        assert_eq!(set_bit(0b1011_0010, 4, false), 0b1010_0010);
        assert_eq!(set_bit(0b1011_0010, 5, true), 0b1011_0010);
        assert_eq!(set_bit(0b1011_0010, 6, true), 0b1111_0010);
        assert_eq!(set_bit(0b1011_0010, 7, false), 0b0011_0010);
    }
}
