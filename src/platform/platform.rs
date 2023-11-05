use sdl2::EventPump;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::common::framebuffer::{FrameBuffer, RgbColor};

extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;

pub struct Size {
    width: usize,
    height: usize,
}

impl Size {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }
}


pub enum PlatformEvent {
    Quit,
}

fn write_pixel_to_buffer(buffer: &mut [u8], pitch: usize, x: usize, y: usize, color: RgbColor) {
    let offset = y * pitch + x * 3;
    buffer[offset] = color.r;
    buffer[offset + 1] = color.g;
    buffer[offset + 2] = color.b
}

pub struct Platform {
    event_pump: EventPump,
    canvas: Canvas<Window>,
    texture: Texture,
    buffer_size: Size,
}

impl Platform {
    pub fn new(
        window_size: Size,
        buffer_size: Size,
    ) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window("Gameboy emulator", window_size.width as u32, window_size.height as u32)
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        let texture_creator = canvas.texture_creator();

        let texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, buffer_size.width as u32, buffer_size.height as u32)
            .map_err(|e| e.to_string())?;

        let event_pump = sdl_context.event_pump()?;

        Ok(Self {
            event_pump,
            canvas,
            texture,
            buffer_size,
        })

    }

    pub fn give_new_frame(&mut self, frame: &FrameBuffer) -> Option<PlatformEvent> {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Some(PlatformEvent::Quit),
                _ => {}
            }
        }

        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        self.texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for y in 0..self.buffer_size.height {
                for x in 0..self.buffer_size.width {
                    write_pixel_to_buffer(buffer, pitch, x, y, frame.get_pixel(x, y));
                }
            }
        }).expect("Failed to draw texture");

        self.canvas.copy(&self.texture, None, None).expect("Failed to copy texture to canvas");
        self.canvas.present();

        return None;
    }
}
