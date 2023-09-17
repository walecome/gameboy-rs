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

struct Register {
    high: u8,
    low: u8,
}

impl Register {
    fn new() -> Register {
        Register { high: 0, low: 0 }
    }

    fn value(&self) -> u16 {
        ((self.high as u16) << 8) | (self.low as u16)
    }
}

struct Registers {
    af: Register,
    bc: Register,
    de: Register,
    hl: Register,
}

impl Registers {
    fn new() -> Registers {
        Registers {
            af: Register::new(),
            bc: Register::new(),
            de: Register::new(),
            hl: Register::new(),
        }
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

struct CPU<'a> {
    rom_data: &'a Vec<u8>,
    pc: u16,
    sp: u16,
    memory: Memory,
    registers: Registers,
}


enum Target {
    Register(RegisterU8),
    AddressHL,
}

enum Instruction {
    Noop,
    Halt,
    Load {
        dst: Target,
        src: Target,
    }
}

fn decode_load_source(mask: u8) -> Target {
    match mask {
        0x0 | 0x8 => Target::Register(RegisterU8::B),
        0x1 | 0x9 => Target::Register(RegisterU8::C),
        0x2 | 0xA => Target::Register(RegisterU8::D),
        0x3 | 0xB => Target::Register(RegisterU8::E),
        0x4 | 0xC => Target::Register(RegisterU8::H),
        0x5 | 0xD => Target::Register(RegisterU8::L),
        0x7 | 0xF => Target::Register(RegisterU8::A),
        0x6 | 0xE => Target::AddressHL,
        _ => panic!("Unknown LD destination mask: {}", mask),
    }
}

fn decode_load_destination(row_mask: u8, col_mask: u8) -> Target {
    if col_mask <= 0x7 {
        match row_mask {
            0x4 => Target::Register(RegisterU8::B),
            0x5 => Target::Register(RegisterU8::D),
            0x6 => Target::Register(RegisterU8::H),
            0x7 => Target::AddressHL,
            _ => panic!("Unknown LD src mask: ({}, {}", row_mask, col_mask),
        }
    } else {
        match row_mask {
            0x4 => Target::Register(RegisterU8::C),
            0x5 => Target::Register(RegisterU8::E),
            0x6 => Target::Register(RegisterU8::L),
            0x7 => Target::Register(RegisterU8::A),
            _ => panic!("Unknown LD src mask: ({}, {}", row_mask, col_mask),
        }
    }
}

// https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
fn decode(opcode: u8) -> Option<Instruction> {
    let row_mask = (opcode & 0xF0) >> 4;
    let col_mask = opcode & 0x0F;

    match (opcode, row_mask, col_mask) {
        (0x00, _, _) => Some(Instruction::Noop),
        (_, 0x4, _) | (_, 0x5, _) | (_, 0x6, _) | (_, 0x7, _) => {
            if opcode == 0x76 {
                return Some(Instruction::Halt);
            }
            let src = decode_load_source(col_mask);
            let dst = decode_load_destination(row_mask, col_mask);
            Some(Instruction::Load { src, dst })
        }
        _ => None,
    }
}

impl CPU<'_> {
    fn tick(&mut self) -> bool {
        let opcode = self.read_u8();
        println!("opcode: {:#04X}", opcode);
        let instruction = decode(opcode).expect(format!("Unknown opcode: {:#04X}", opcode).as_str());
        let should_halt = match instruction {
            Instruction::Halt => true,
            _ => false,
        };
        if should_halt {
            return false;
        }

        match instruction {
            Instruction::Noop => {},
            Instruction::Load { dst, src } => {
                let value: u8 = match src {
                    Target::Register(reg) => self.resolve_u8_reg(reg).clone(),
                    Target::AddressHL => self.memory.get(self.registers.hl.value()),
                };
                match dst {
                    Target::Register(reg) => {
                        *self.resolve_u8_reg(reg) = value;
                    },
                    Target::AddressHL => {
                        self.memory.set(self.registers.hl.value(), value);
                    },
                }
            },

            Instruction::Halt => panic!("Should be caught above"),
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
            RegisterU8::A => &mut self.registers.af.high,
            RegisterU8::B => todo!(),
            RegisterU8::C => todo!(),
            RegisterU8::D => todo!(),
            RegisterU8::E => todo!(),
            RegisterU8::F => todo!(),
            RegisterU8::H => todo!(),
            RegisterU8::L => todo!(),
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
        registers: Registers::new(),
        memory: Memory {
            data: vec![0x00; 0xFFFF],
        },
    };
    loop {
        cpu.tick();
    }
}
