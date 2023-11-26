use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::EventPump;

use crate::common::framebuffer::{FrameBuffer, RgbColor};
use crate::common::joypad_events::{JoypadButton, JoypadEvent};

extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
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
    Joypad(JoypadEvent),
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

fn scancode_to_button(scancode: Scancode) -> Option<JoypadButton> {
    match scancode {
        Scancode::Kp8 => Some(JoypadButton::Up),
        Scancode::Kp2 => Some(JoypadButton::Down),
        Scancode::Kp4 => Some(JoypadButton::Left),
        Scancode::Kp6 => Some(JoypadButton::Right),
        Scancode::Kp7 => Some(JoypadButton::A),
        Scancode::Kp9 => Some(JoypadButton::B),
        Scancode::Kp3 => Some(JoypadButton::Select),
        Scancode::Kp1 => Some(JoypadButton::Start),
        _ => None,
    }
}

impl Platform {
    pub fn new(window_size: Size, buffer_size: Size) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(
                "Gameboy emulator",
                window_size.width as u32,
                window_size.height as u32,
            )
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        let texture_creator = canvas.texture_creator();

        let texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                buffer_size.width as u32,
                buffer_size.height as u32,
            )
            .map_err(|e| e.to_string())?;

        let event_pump = sdl_context.event_pump()?;

        Ok(Self {
            event_pump,
            canvas,
            texture,
            buffer_size,
        })
    }

    pub fn give_new_frame(&mut self, frame: &FrameBuffer) -> Vec<PlatformEvent> {
        let mut platform_events: Vec<PlatformEvent> = vec![];
        for event in self.event_pump.poll_iter() {
            let maybe_platform_event = match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => Some(PlatformEvent::Quit),


                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(button) = scancode_to_button(scancode) {
                        Some(PlatformEvent::Joypad(JoypadEvent::new_down(button)))
                    } else {
                        None
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(button) = scancode_to_button(scancode) {
                        Some(PlatformEvent::Joypad(JoypadEvent::new_up(button)))
                    } else {
                        None
                    }
                }

                _ => None,
            };
            if let Some(platform_event) = maybe_platform_event {
                platform_events.push(platform_event);
            }
        }

        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        self.texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..self.buffer_size.height {
                    for x in 0..self.buffer_size.width {
                        write_pixel_to_buffer(buffer, pitch, x, y, frame.get_pixel(x, y));
                    }
                }
            })
            .expect("Failed to draw texture");

        self.canvas
            .copy(&self.texture, None, None)
            .expect("Failed to copy texture to canvas");
        self.canvas.present();

        return platform_events;
    }
}
