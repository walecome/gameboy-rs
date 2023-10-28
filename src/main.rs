mod gameboy;

use std::{fs, path::PathBuf};

use clap::Parser;

use gameboy::header::Header;
use gameboy::video::FrameBuffer;

use crate::gameboy::cartridge::create_for_cartridge_type;
use crate::gameboy::cpu::CPU;
use crate::gameboy::reference::{get_reference_metadata, ReferenceMetadata};
use crate::gameboy::video::{RgbColor, SCREEN_HEIGHT, SCREEN_WIDTH};

extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    rom: PathBuf,
    #[arg(long)]
    reference: Option<PathBuf>,
    #[arg(long)]
    trace_cpu: bool,
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
            None => todo!(
                "Cartridge not implemented for type: {:?}",
                header.cartridge_type
            ),
        };

        Self {
            cpu: CPU::new(cartridge, args.trace_cpu),

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
            }
            None => {
                println!("HALT!");
                return false;
            }
        }

        self.index += 1;

        return true;
    }

    fn try_take_frame(&mut self) -> Option<&FrameBuffer> {
        self.cpu.mmu().video().try_take_frame()
    }
}

fn main() -> Result<(), String> {
    let mut gameboy = Gameboy::new(Args::parse());

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Gameboy emulator", 600, 540)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    fn write_pixel(buffer: &mut [u8], pitch: usize, x: usize, y: usize, color: RgbColor) {
        let offset = y * pitch + x * 3;
        buffer[offset] = color.r;
        buffer[offset + 1] = color.g;
        buffer[offset + 2] = color.b
    }

    let width = SCREEN_WIDTH as usize;
    let height = SCREEN_HEIGHT as usize;

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, width as u32, height as u32)
        .map_err(|e| e.to_string())?;

    // Initial texture
    if let Some(frame_buffer) = gameboy.try_take_frame() {
        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for y in 0..height {
                for x in 0..width {
                    write_pixel(buffer, pitch, x, y, frame_buffer.get_pixel(x, y));
                }
            }
        })?;
    }

    canvas.clear();
    canvas.copy(&texture, None, None)?;
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }
        if !gameboy.tick() {
            break 'running;
        }

        if let Some(frame_buffer) = gameboy.try_take_frame() {
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();
            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..height {
                    for x in 0..width {
                        write_pixel(buffer, pitch, x, y, frame_buffer.get_pixel(x, y));
                    }
                }
            })?;

            canvas.copy(&texture, None, None)?;
            canvas.present();
        }
    }

    return Ok(());
}
