mod gameboy;

use std::{fs, path::PathBuf};

use clap::Parser;

use gameboy::header::Header;

use crate::gameboy::cartridge::create_for_cartridge_type;
use crate::gameboy::cpu::CPU;
use crate::gameboy::reference::{ReferenceMetadata, get_reference_metadata};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    rom: PathBuf,
    #[arg(long)]
    reference: Option<PathBuf>,
}

struct Gameboy {
    cpu: CPU,

    // Internal / debug
    index: usize,
    maybe_reference_metadata: Option<Vec<ReferenceMetadata>>,
}

impl Gameboy {
    fn new(args: Args) -> Self {
        let rom_data = fs::read(args.rom).unwrap();
        let header = Header::read_from_rom(&rom_data).unwrap();
        println!("{:?}", header);

        let cartridge = match create_for_cartridge_type(header.cartridge_type, rom_data) {
            Some(cartridge) => cartridge,
            None => todo!("Cartridge not implemented for type: {:?}", header.cartridge_type),
        };

        let mut index = 0;

        Self {
            cpu: CPU::new(cartridge),

            index: 0,
            maybe_reference_metadata: if let Some(reference) = args.reference {
                Some(get_reference_metadata(&reference))
            } else {
                None
            },
        }
    }

    fn tick(&mut self) -> bool {
        let current_metadata = if let Some(reference_metadata) = &self.maybe_reference_metadata {
            if self.index >= reference_metadata.len() {
                panic!("Ran out of reference data");
            }
            Some(&reference_metadata[self.index])
        } else {
            None
        };
        let maybe_cycles = self.cpu.tick(current_metadata, self.index);
        match maybe_cycles {
            Some(cycles) => {
                self.cpu.mmu().video().tick(cycles as usize);
            },
            None => {
                println!("HALT!");
                return false;
            }
        }

        self.index += 1;

        return true;
    }
}

fn main() {
    let mut gameboy = Gameboy::new(Args::parse());

    loop {
        if !gameboy.tick() {
            return;
        }
    }
}
