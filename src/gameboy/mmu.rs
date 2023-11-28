use std::io::{self, Write};

use crate::common::joypad_events::{JoypadEvent, JoypadButton};

use super::address::Address;
use super::cartridge::Cartridge;
use super::video::Video;
use super::utils::{get_bit, set_bit_mut};

pub struct Word {
    pub value: u16,
}

const BOOT_ROM: &[u8] = &[
    0x31, 0xFE, 0xFF, 0xAF, 0x21, 0xFF, 0x9F, 0x32, 0xCB, 0x7C, 0x20, 0xFB, 0x21, 0x26, 0xFF, 0x0E,
    0x11, 0x3E, 0x80, 0x32, 0xE2, 0x0C, 0x3E, 0xF3, 0xE2, 0x32, 0x3E, 0x77, 0x77, 0x3E, 0xFC, 0xE0,
    0x47, 0x11, 0x04, 0x01, 0x21, 0x10, 0x80, 0x1A, 0xCD, 0x95, 0x00, 0xCD, 0x96, 0x00, 0x13, 0x7B,
    0xFE, 0x34, 0x20, 0xF3, 0x11, 0xD8, 0x00, 0x06, 0x08, 0x1A, 0x13, 0x22, 0x23, 0x05, 0x20, 0xF9,
    0x3E, 0x19, 0xEA, 0x10, 0x99, 0x21, 0x2F, 0x99, 0x0E, 0x0C, 0x3D, 0x28, 0x08, 0x32, 0x0D, 0x20,
    0xF9, 0x2E, 0x0F, 0x18, 0xF3, 0x67, 0x3E, 0x64, 0x57, 0xE0, 0x42, 0x3E, 0x91, 0xE0, 0x40, 0x04,
    0x1E, 0x02, 0x0E, 0x0C, 0xF0, 0x44, 0xFE, 0x90, 0x20, 0xFA, 0x0D, 0x20, 0xF7, 0x1D, 0x20, 0xF2,
    0x0E, 0x13, 0x24, 0x7C, 0x1E, 0x83, 0xFE, 0x62, 0x28, 0x06, 0x1E, 0xC1, 0xFE, 0x64, 0x20, 0x06,
    0x7B, 0xE2, 0x0C, 0x3E, 0x87, 0xE2, 0xF0, 0x42, 0x90, 0xE0, 0x42, 0x15, 0x20, 0xD2, 0x05, 0x20,
    0x4F, 0x16, 0x20, 0x18, 0xCB, 0x4F, 0x06, 0x04, 0xC5, 0xCB, 0x11, 0x17, 0xC1, 0xCB, 0x11, 0x17,
    0x05, 0x20, 0xF5, 0x22, 0x23, 0x22, 0x23, 0xC9, 0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B,
    0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E,
    0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC,
    0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E, 0x3C, 0x42, 0xB9, 0xA5, 0xB9, 0xA5, 0x42, 0x3C,
    0x21, 0x04, 0x01, 0x11, 0xA8, 0x00, 0x1A, 0x13, 0xBE, 0x00, 0x00, 0x23, 0x7D, 0xFE, 0x34, 0x20,
    0xF5, 0x06, 0x19, 0x78, 0x86, 0x23, 0x05, 0x20, 0xFB, 0x86, 0x00, 0x00, 0x3E, 0x01, 0xE0, 0x50,
];

impl Word {
    pub fn new(value: u16) -> Self {
        Self { value }
    }

    fn compose_new(high: u8, low: u8) -> Self {
        Self {
            value: ((high as u16) << 8) | low as u16,
        }

    }

    fn low(&self) -> u8 {
        (self.value & 0xFF) as u8
    }

    fn high(&self) -> u8 {
        ((self.value & 0xFF00) >> 8) as u8
    }
}

pub struct IO {
    joypad_input: Joypad,
    serial: Serial,
    timer: Timer,
    audio: Vec<u8>,
    wave_pattern: Vec<u8>,
    boot_rom_disabled: u8,
}

fn byte_vec_for_range(
    range_inclusive_start: u16,
    range_inclusive_end: u16,
) -> Vec<u8> {
    let size = (range_inclusive_end - range_inclusive_start + 1)  as usize;
    vec![0x00; size]
}

impl IO {
    fn new(print_serial: bool) -> Self {
        Self {
            joypad_input: Joypad::new(),
            serial: Serial::new(print_serial),
            timer: Timer::new(),
            audio: byte_vec_for_range(0xFF10, 0xFF26),
            wave_pattern: byte_vec_for_range(0xFF30, 0xFF3F),
            boot_rom_disabled: 0x00,
        }
    }
}

pub struct MMU {
    cartridge: Box<dyn Cartridge>,
    video: Video,
    internal_ram: Vec<u8>,
    io: IO,
    high_ram: Vec<u8>,
    interrupt_enable: u8,
    interrupt_flags: u8,
    consumed_read_write_cycles: u8,
}

#[derive(Copy, Clone)]
pub enum InterruptSource {
    VBlank = 0,
    Lcd = 1,
    Timer = 2,
    Serial = 3,
    Joypad = 4,
}

pub fn interrupt_vector(interrupt: InterruptSource) -> u8 {
    match interrupt {
        InterruptSource::VBlank => 0x40,
        InterruptSource::Lcd => 0x48,
        InterruptSource::Timer => 0x50,
        InterruptSource::Serial => 0x58,
        InterruptSource::Joypad => 0x60,
    }
}

struct Timer {
    divider: u16,
    timer_counter: u8,
    timer_modulo: u8,
    timer_control: u8,

    // Internal
    clock_counter: usize,
}

#[derive(Copy, Clone)]
enum ClockSelect {
    Div1024 = 1024,
    Div16 = 16,
    Div64 = 64,
    Div256 = 256,
}

impl Timer {
    fn new() -> Self {
        Self {
            divider: 0,
            timer_counter: 0,
            timer_modulo: 0,
            timer_control: 0,
            clock_counter: 0,
        }
    }

    fn read(&self, address: Address) -> u8 {
        match address.value() {
            // The divider ticks at CPU clock / 256, so use
            // the lower 8 bits (256) of the 16 bits to cover that.
            0xFF04 => (self.divider >> 8) as u8,
            0xFF05 => self.timer_counter,
            0xFF06 => self.timer_modulo,
            0xFF07 => self.timer_control,
            _ => panic!("Invalid timer address: {:#06X}", address.value()),
        }
    }

    fn write(&mut self, address: Address, value: u8) {
        match address.value() {
            // Writing any value to this register resets it to $00.
            // https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register
            0xFF04 => self.divider = 0,
            0xFF05 => self.timer_counter = value,
            0xFF06 => self.timer_modulo = value,
            0xFF07 => {
                self.timer_control = value;
            }
            _ => panic!("Invalid timer address: {:#06X}", address.value()),
        }
    }

    fn maybe_tick_cycles(&mut self, elapsed_cycles: u8) -> bool {
        let mut fire_interrupt = false;
        for _ in 0..(elapsed_cycles * 4) {
            self.divider = self.divider.wrapping_add(1);
            if self.is_timer_enabled() {
                fire_interrupt |= self.tick_clock();
            }
        }
        return fire_interrupt;
    }

    fn tick_clock(&mut self) -> bool {
        self.clock_counter += 1;

        let clock_select_div = self.get_clock_select() as usize;

        if self.clock_counter < clock_select_div {
            return false;
        }

        self.clock_counter -= clock_select_div;
        return self.increment_timer_counter();
    }

    fn get_clock_select(&self) -> ClockSelect {
        match self.timer_control & 0b11 {
            0b00 => ClockSelect::Div1024,
            0b01 => ClockSelect::Div16,
            0b10 => ClockSelect::Div64,
            0b11 => ClockSelect::Div256,
            _ => panic!(),
        }
    }

    fn increment_timer_counter(&mut self) -> bool{
        self.timer_counter = self.timer_counter.wrapping_add(1);

        if self.timer_counter == 0x00 {
            self.timer_counter = self.timer_modulo;
            return true;
        }

        return false;
    }

    fn is_timer_enabled(&self) -> bool {
        get_bit(self.timer_control, 2)
    }
}

struct Serial {
    transfer_data: u8,
    print_serial: bool,
}

impl Serial {
    fn new(print_serial: bool) -> Self {
        Self {
            transfer_data: 0,
            print_serial,
        }
    }
    fn read(&self, address: Address) -> u8 {
        match address.value() {
            0xFF01 => self.transfer_data,
            0xFF02 => todo!("Read for serial control"),
            _ => panic!("Invalid serial address: {:#06X}", address.value()),
        }
    }

    fn write(&mut self, address: Address, value: u8) {
        match address.value() {
            0xFF01 => self.transfer_data = value,
            // TODO: Fire interrupt?
            0xFF02 => {
                if self.print_serial && get_bit(value, 7) {
                    print!("{}", self.transfer_data as char);
                    io::stdout().flush().unwrap();
                }
            },
            _ => panic!("Invalid serial address: {:#06X}", address.value()),
        }
    }
}

#[derive(Debug)]
pub struct Joypad {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    a: bool,
    b: bool,
    select: bool,
    start: bool,

    select_buttons: bool,
    direction_buttons: bool,
}

impl Joypad {
    fn new() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
            a: false,
            b: false,
            select: false,
            start: false,
            select_buttons: false,
            direction_buttons: false,
        }
    }

    pub fn consume_platform_event(&mut self, event: JoypadEvent) {
        let field: &mut bool = match event.button {
            JoypadButton::Up => &mut self.up,
            JoypadButton::Down => &mut self.down,
            JoypadButton::Left => &mut self.left,
            JoypadButton::Right => &mut self.right,
            JoypadButton::A => &mut self.a,
            JoypadButton::B => &mut self.b,
            JoypadButton::Select => &mut self.select,
            JoypadButton::Start => &mut self.start,
        };
        *field = event.is_down;
    }

    fn read(&self) -> u8 {
        let mut base: u8 = 0xF;

        if self.direction_buttons {
            set_bit_mut(&mut base, 0, !self.right);
            set_bit_mut(&mut base, 1, !self.left);
            set_bit_mut(&mut base, 2, !self.up);
            set_bit_mut(&mut base, 3, !self.down);
        }

        if self.select_buttons {
            set_bit_mut(&mut base, 0, !self.a);
            set_bit_mut(&mut base, 1, !self.b);
            set_bit_mut(&mut base, 2, !self.select);
            set_bit_mut(&mut base, 3, !self.start);
        }

        set_bit_mut(&mut base, 4, !self.direction_buttons);
        set_bit_mut(&mut base, 5, !self.select_buttons);

        return base;
    }

    fn write(&mut self, value: u8) {
        self.direction_buttons = !get_bit(value, 4);
        self.select_buttons = !get_bit(value, 5);
    }
}

impl MMU {
    pub fn new(cartridge: Box<dyn Cartridge>, print_serial: bool) -> MMU {
        MMU {
            cartridge,
            video: Video::new(),
            internal_ram: vec![0x00; 0x3000],
            io: IO::new(print_serial),
            high_ram: vec![0x00; 0x80],
            interrupt_enable: 0x00,
            interrupt_flags: 0x00,
            consumed_read_write_cycles: 0x00,
        }
    }

    pub fn take_consumed_cycles(&mut self) -> u8 {
        let ret = self.consumed_read_write_cycles;
        self.consumed_read_write_cycles = 0;
        return ret;
    }

    pub fn video(&mut self) -> &mut Video {
        &mut self.video
    }

    pub fn joypad(&mut self) -> &mut Joypad {
        &mut self.io.joypad_input
    }

    pub fn read(&mut self, address: Address) -> u8 {
        self.consume_cycle();
        self.read_no_consume_cycles(address)
    }

    fn read_no_consume_cycles(&self, address: Address) -> u8 {
        if address.value() == 0xFF0F {
            return self.interrupt_flags;
        }

        match address.value() {
            0x0000..=0x7FFF => {
                if address.value() <= 0xFF && self.io.boot_rom_disabled == 0x00 {
                    BOOT_ROM[address.index_value()]
                } else {
                    self.cartridge.read(address)
                }
            }
            0x8000..=0x9FFF => self.video.read_vram(address),
            0xA000..=0xBFFF => self.cartridge.read(address),
            0xC000..=0xDFFF => self.internal_ram[address.index_value() - 0xC000],
            0xE000..=0xFDFF => panic!("Read access for prohibited memory area"),
            0xFE00..=0xFE9F => self.video.read_oam(address),
            0xFEA0..=0xFEFF => panic!("Read access for prohibited memory area"),
            0xFF00..=0xFF7F => self.read_io(address),
            0xFF80..=0xFFFE => self.high_ram[address.index_value() - 0xFF80],
            0xFFFF => self.interrupt_enable,
        }
    }

    pub fn read_word(&mut self, address: Address) -> Word {
        let low = self.read(address);
        let high = self.read(address.next());

        Word::compose_new(high, low)
    }

    pub fn write(&mut self, address: Address, value: u8) {
        self.consume_cycle();
        self.write_no_consume_cycles(address, value);
    }

    fn write_no_consume_cycles(&mut self, address: Address, value: u8) {
        if address.value() == 0xFF0F {
            self.interrupt_flags = value;
            return;
        }

        match address.value() {
            0x0000..=0x7FFF => self.cartridge.write(address, value),
            0x8000..=0x9FFF => self.video.write_vram(address, value),
            0xA000..=0xBFFF => self.cartridge.write(address, value),
            0xC000..=0xDFFF => self.internal_ram[address.index_value() - 0xC000] = value,
            0xE000..=0xFDFF => panic!("Write access for prohibited memory area"),
            0xFE00..=0xFE9F => self.video.write_oam(address, value),
            0xFEA0..=0xFEFF => println!("Write access for prohibited memory area: {:#06X}", address.value()),
            0xFF00..=0xFF7F => self.write_io(address, value),
            0xFF80..=0xFFFE => self.high_ram[address.index_value() - 0xFF80] = value,
            0xFFFF => self.interrupt_enable = value,
        }
    }

    pub fn write_word(&mut self, address: Address, value: Word) {
        self.write(address, value.low());
        self.write(address.next(), value.high());
    }

    pub fn is_interrupt_enabled(&self, interrupt: InterruptSource) -> bool {
        get_bit(self.interrupt_enable, interrupt as u8)
    }

    pub fn has_interrupt_flag(&self, interrupt: InterruptSource) -> bool {
        get_bit(self.interrupt_flags, interrupt as u8)
    }

    pub fn set_interrupt_flag(&mut self, interrupt: InterruptSource, enabled: bool) {
        set_bit_mut(&mut self.interrupt_flags, interrupt as u8, enabled);
    }

    pub fn maybe_tick_timers(&mut self, elapsed_cycles: u8) {
        if self.io.timer.maybe_tick_cycles(elapsed_cycles) {
            self.set_interrupt_flag(InterruptSource::Timer, true);
        }
    }

    pub fn disable_boot_rom(&mut self) {
        self.io.boot_rom_disabled = 1
    }

    pub fn boot_rom_disabled(&self) -> bool {
        self.io.boot_rom_disabled != 0
    }

    fn read_io(&self, address: Address) -> u8 {
        match address.value() {
            0xFF00 => self.io.joypad_input.read(),
            0xFF01..=0xFF02 => self.io.serial.read(address),
            0xFF04..=0xFF07 => self.io.timer.read(address),
            0xFF10..=0xFF26 => self.io.audio[address.index_value() - 0xFF10],
            0xFF30..=0xFF3F => self.io.wave_pattern[address.index_value() - 0xFF30],
            0xFF40..=0xFF45 => self.video.read_register(address),
            0xFF46 => panic!("Reading from DMA transfer register"),
            0xFF47..=0xFF4B => self.video.read_register(address),
            0xFF4D => {
                // TODO: This is for CGB, but still used by some roms. Log?
                0x00
            },
            0xFF50 => self.io.boot_rom_disabled,
            _ => panic!("Read for unmapped IO address: {:#06X}", address.value()),
        }
    }

    fn write_io(& mut self, address: Address, value: u8) {
        match address.value() {
            0xFF00 => self.io.joypad_input.write(value),
            0xFF01..=0xFF02 => self.io.serial.write(address, value),
            0xFF04..=0xFF07 => self.io.timer.write(address, value),
            0xFF10..=0xFF26 => self.io.audio[address.index_value() - 0xFF10] = value,
            0xFF30..=0xFF3F => self.io.wave_pattern[address.index_value() - 0xFF30] = value,
            0xFF40..=0xFF45 => self.video.write_register(address, value),
            0xFF46 => self.do_dma_transfer(value),
            0xFF47..=0xFF4B => self.video.write_register(address, value),
            0xFF4D => {
                // TODO: This is for CGB, but still used by some roms. Log?
            },
            0xFF50 => self.io.boot_rom_disabled = value,
            // Undocumented but used
            0xFF7F => println!("Write to undocumented IO address: {:?} = {}", address, value),
            _ => panic!("Write for unmapped IO address: {:#06X}", address.value()),
        };
    }

    fn do_dma_transfer(&mut self, dma_target: u8) {
        // TODO: The DMA transfer could take 160 cycle for normal speed, do we need to care?
        // https://gbdev.io/pandocs/OAM_DMA_Transfer.html#ff46--dma-oam-dma-source-address--start
        let mut src_addr = Address::new((dma_target as u16) * 0x0100);
        let mut dst_addr = Address::new(0xFE00);
        for _ in 0..=0x9F {

            // TODO: Check if we need to care about cycles
            let value = self.read_no_consume_cycles(src_addr);
            self.write_no_consume_cycles(dst_addr, value);

            src_addr = src_addr.next();
            dst_addr = dst_addr.next();
        }
    }

    fn consume_cycle(&mut self) {
        self.consumed_read_write_cycles += 1;
        self.maybe_tick_timers(1);
    }
}
