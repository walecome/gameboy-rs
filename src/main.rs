mod gameboy;

use std::{fs, path::PathBuf};

use clap::Parser;

use gameboy::header::Header;
use gameboy::instruction_decoder::{
    decode,
    Instruction,
    LoadDstU16,
    LoadDstU8,
    LoadSrcU16,
    LoadSrcU8,
    RegisterU16,
    RegisterU8,
};

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
        // TODO: Handle memory map
        self.data[address as usize] = value;
    }

    fn set_from_u8(&mut self, lower_address: u8, value: u8) {
        let address = 0xFF00 + (lower_address as u16);
        self.data[address as usize] = value;
    }

    fn get(&self, address: u16) -> u8 {
        self.data[address as usize]
    }

    fn get_from_u8(&self, lower_address: u8) -> u8 {
        let address = 0xFF00 + (lower_address as u16);
        self.data[(address) as usize]
    }

    fn set_u16(&mut self, address: u16, value: u16) {
        self.data[address as usize] = ((value & 0xFF00) >> 8) as u8;
        self.data[(address + 1) as usize] = (value & 0x00FF) as u8;
    }

    fn get_u16(&self, address: u16) -> u16 {
        let low = self.data[address as usize];
        let high = self.data[(address + 1) as usize];
        return ((high as u16) << 8) & (low as u16);
    }
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
    interrupts_enabled: bool,
}

impl CPU<'_> {
    fn tick(&mut self) -> bool {
        let pc = self.pc;
        let opcode = self.read_u8();
        println!("{:#06X}: {:#04X}", pc, opcode);
        let instruction =
            decode(opcode).expect(format!("Unknown opcode: {:#04X}", opcode).as_str());

        match instruction {
            Instruction::Noop => {}
            Instruction::LoadU8 { dst, src } => {
                let value = self.read_u8_target(src);
                self.write_u8_target(dst, value);
            }
            Instruction::Halt => {
                return false;
            }
            Instruction::JumpImmediate => {
                let address = self.read_u16();
                self.pc = address;
            }
            Instruction::DisableInterrupts => self.interrupts_enabled = false,
            Instruction::LoadU16 { dst, src } => {
                let value = self.read_u16_target(src);
                self.write_u16_target(dst, value);
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
            LoadSrcU8::Register(reg) => {
                *self.resolve_u8_reg(reg)
            },
            LoadSrcU8::AddressU16(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.memory.get(address)
            },
            LoadSrcU8::AddressU8(reg) => {
                let lower_address = *self.resolve_u8_reg(reg);
                self.memory.get_from_u8(lower_address)
            },
            LoadSrcU8::ImmediateAddressU8 => {
                let lower_address = self.read_u8();
                self.memory.get_from_u8(lower_address)
            },
            LoadSrcU8::ImmediateAddressU16 => {
                let address = self.read_u16();
                self.memory.get(address)
            },
            LoadSrcU8::ImmediateU8 => {
                self.read_u8()
            }
            LoadSrcU8::AddressU16Increment(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.resolve_u16_reg(&reg).set(address + 1);
                self.memory.get(address)
            },
            LoadSrcU8::AddressU16Decrement(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.resolve_u16_reg(&reg).set(address - 1);
                self.memory.get(address)
            },
        }
    }

    fn write_u8_target(&mut self, target: LoadDstU8, value: u8) {
        match target {
            LoadDstU8::Register(reg) => {
                *self.resolve_u8_reg(reg) = value;
            },
            LoadDstU8::AddressU8(reg) => {
                let lower_address = *self.resolve_u8_reg(reg);
                self.memory.set_from_u8(lower_address, value);
            },
            LoadDstU8::AddressU16(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.memory.set(address, value);
            },
            LoadDstU8::AddressU16Increment(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.resolve_u16_reg(&reg).set(address + 1);
                self.memory.set(address, value);
            },
            LoadDstU8::AddressU16Decrement(reg) => {
                let address = self.resolve_u16_reg(&reg).get();
                self.resolve_u16_reg(&reg).set(address - 1);
                self.memory.set(address, value);
            },
            LoadDstU8::ImmediateAddressU8 => {
                let lower_address = self.read_u8();
                self.memory.set_from_u8(lower_address, value);
            },
            LoadDstU8::ImmediateAddressU16 => {
                let address = self.read_u16();
                self.memory.set(address, value);
            },
        }
    }

    fn read_u16_target(&mut self, target: LoadSrcU16) -> u16 {
        match target {
            LoadSrcU16::Register(reg) => {
                self.resolve_u16_reg(&reg).get()
            },
            LoadSrcU16::ImmediateU16 => {
                self.read_u16()
            },
            LoadSrcU16::StackPointer => {
                self.sp
            },
            LoadSrcU16::StackPointerWithOffset => {
                let offset = self.read_u8() as i32;
                let signed_sp = self.sp as i32;
                (signed_sp + offset) as u16
            }
        }
    }

    fn write_u16_target(&mut self, target: LoadDstU16, value: u16) {
        match target {
            LoadDstU16::Register(reg) => {
                self.resolve_u16_reg(&reg).set(value);
            },
            LoadDstU16::StackPointer => {
                self.sp = value;
            },
            LoadDstU16::ImmediateAddress => {
                let address = self.read_u16();
                self.memory.set_u16(address, value);
            },
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
            data: vec![0x00; 0xFFFF + 1],
        },
        a: 0x00,
        b: 0x00,
        c: 0x00,
        d: 0x00,
        e: 0x00,
        f: 0x00,
        h: 0x00,
        l: 0x00,
        interrupts_enabled: false,
    };
    loop {
        cpu.tick();
    }
}
