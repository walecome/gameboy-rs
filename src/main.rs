mod gameboy;

use std::{fs, path::PathBuf};

use clap::Parser;

use gameboy::header::Header;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    rom: PathBuf,
}

struct Memory {
    data: Vec<u8>,
}

impl Memory {
    fn set(&mut self, address: u16, value: u8) {
        self.data[address as usize] = value;
    }

    fn get(&self, address: u16) -> u8 {
        self.data[address as usize]
    }
}

enum RegisterU8 {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
}

enum RegisterU16 {
    AF,
    BC,
    DE,
    HL,
}

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
        let high = (value & 0xF0) >> 8;
        let low = value & 0x0F;

        *self.high = high as u8;
        *self.low = low as u8;
    }
}

struct CPU<'a> {
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
}

enum TargetU8 {
    Register(RegisterU8),
    AddressHL,
}

enum Instruction {
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
fn decode(opcode: u8) -> Option<Instruction> {
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

impl CPU<'_> {
    fn tick(&mut self) -> bool {
        let opcode = self.read_u8();
        println!("opcode: {:#04X}", opcode);
        let instruction =
            decode(opcode).expect(format!("Unknown opcode: {:#04X}", opcode).as_str());
        let should_halt = match instruction {
            Instruction::Halt => true,
            _ => false,
        };
        if should_halt {
            return false;
        }

        match instruction {
            Instruction::Noop => {}
            Instruction::Load { dst, src } => {
                let value = self.read_u8_target(src);
                self.write_u8_target(dst, value);
            }

            Instruction::Halt => panic!("Should be caught above"),
            Instruction::JumpImmediate => {
                let address = self.read_u16();
                self.pc = address;
            }
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

    fn resolve_u16_reg(&mut self, reg: RegisterU16) -> RegisterPair {
        let (high, low) = match reg {
            RegisterU16::AF => (&mut self.a, &mut self.f),
            RegisterU16::BC => (&mut self.b, &mut self.c),
            RegisterU16::DE => (&mut self.d, &mut self.e),
            RegisterU16::HL => (&mut self.h, &mut self.l),
        };
        RegisterPair { high, low }
    }

    fn read_u8_target(&mut self, target: TargetU8) -> u8 {
        match target {
            TargetU8::Register(reg) => self.resolve_u8_reg(reg).clone(),
            TargetU8::AddressHL => {
                let value = self.resolve_u16_reg(RegisterU16::HL).get();
                self.memory.get(value)
            }
        }
    }

    fn write_u8_target(&mut self, target: TargetU8, value: u8) {
        match target {
            TargetU8::Register(reg) => {
                *self.resolve_u8_reg(reg) = value;
            }
            TargetU8::AddressHL => {
                let address = self.resolve_u16_reg(RegisterU16::HL).get();
                self.memory.set(address, value);
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    let rom_data = fs::read(args.rom).unwrap();
    let header = Header::read_from_rom(&rom_data).unwrap();
    println!("{:?}", header);

    let mut cpu = CPU {
        rom_data: &rom_data,
        pc: 0x0100,
        sp: 0x0000,
        memory: Memory {
            data: vec![0x00; 0xFFFF],
        },
        a: 0x00,
        b: 0x00,
        c: 0x00,
        d: 0x00,
        e: 0x00,
        f: 0x00,
        h: 0x00,
        l: 0x00,
    };
    loop {
        cpu.tick();
    }
}
