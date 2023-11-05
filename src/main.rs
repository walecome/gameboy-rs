mod gameboy;
mod common;
mod platform;

use std::{fs, path::PathBuf};

use clap::Parser;
use platform::platform::{Platform, Size, PlatformEvent};

use crate::gameboy::gameboy::Gameboy;
use crate::gameboy::cpu::TraceMode;
use crate::gameboy::reference::get_reference_metadata;
use crate::gameboy::video::{SCREEN_HEIGHT, SCREEN_WIDTH};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    rom: PathBuf,
    #[arg(long)]
    reference: Option<PathBuf>,
    #[arg(long)]
    #[arg(value_enum, default_value_t=TraceMode::Off)]
    trace_mode: TraceMode,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let rom_data = fs::read(args.rom).unwrap();
    let reference_metdata = if let Some(reference) = args.reference {
        Some(get_reference_metadata(&reference))
    } else {
        None
    };

    let mut gameboy = Gameboy::new(
        rom_data,
        reference_metdata,
        args.trace_mode,
    );

    let mut platform = Platform::new(
        Size::new(600, 540),
        Size::new(SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize),
    )?;

    'running: loop {
        let maybe_frame = gameboy.tick();

        if let Some(frame) = maybe_frame {
            let maybe_event = platform.give_new_frame(frame);
            if let Some(event) = maybe_event {
                match event {
                    PlatformEvent::Quit => break 'running,
                }
            }
        }
    }

    return Ok(());
}
