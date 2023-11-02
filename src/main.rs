mod gameboy;

use std::{fs, path::PathBuf};

use clap::Parser;

use crate::gameboy::gameboy::Gameboy;
use crate::gameboy::cpu::TraceMode;
use crate::gameboy::reference::get_reference_metadata;
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

    canvas.clear();
    canvas.copy(&texture, None, None)?;
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        let maybe_frame = gameboy.tick();

        if let Some(frame_buffer) = maybe_frame {
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
