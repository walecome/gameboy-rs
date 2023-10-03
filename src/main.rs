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

fn main() -> ! {
    let args = Args::parse();

    let maybe_reference_metadata: Option<Vec<ReferenceMetadata>> =
        if let Some(reference) = args.reference {
            Some(get_reference_metadata(&reference))
        } else {
            None
        };

    let rom_data = fs::read(args.rom).unwrap();
    let header = Header::read_from_rom(&rom_data).unwrap();
    println!("{:?}", header);

    let cartridge = match create_for_cartridge_type(header.cartridge_type, rom_data) {
        Some(cartridge) => cartridge,
        None => todo!("Cartridge not implemented for type: {:?}", header.cartridge_type),
    };

    let mut index = 0;

    let mut cpu = CPU::new(cartridge);
    loop {
        let current_metadata = if let Some(reference_metadata) = &maybe_reference_metadata {
            if index >= reference_metadata.len() {
                panic!("Ran out of reference data");
            }
            Some(&reference_metadata[index])
        } else {
            None
        };
        let should_continue = cpu.tick(current_metadata, index);
        if !should_continue {
            todo!("HALT!")
        }
        index += 1;
    }
}
