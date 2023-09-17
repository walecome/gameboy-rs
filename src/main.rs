mod gameboy;

use std::{fs, path::PathBuf};

use clap::Parser;

use gameboy::header::Header;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    rom: PathBuf,
}

fn main() {
    let args = Args::parse();
    let rom_data = fs::read(args.rom).unwrap();
    let header = Header::read_from_rom(&rom_data);
    println!("{:?}", header);
}
