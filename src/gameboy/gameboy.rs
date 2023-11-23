use crate::common::framebuffer::FrameBuffer;

use super::cartridge::create_for_cartridge_type;
use super::cpu::CPU;
use super::cpu::TraceMode;
use super::header::{Header, FlagCGB};
use super::mmu::InterruptSource;
use super::reference::ReferenceMetadata;
use super::video::VideoInterrupt;

pub struct Gameboy {
    cpu: CPU,

    // Internal / debug
    index: usize,
    maybe_reference_metadata: Option<Vec<ReferenceMetadata>>,
}

impl Gameboy {
    pub fn new(
        rom_data: Vec<u8>,
        reference_metadata: Option<Vec<ReferenceMetadata>>,
        trace_mode: TraceMode,
        skip_boot_rom: bool,
    ) -> Self {
        let header = Header::read_from_rom(&rom_data).unwrap();
        println!("{:#?}", header);

        if !matches!(header.cgb_flag, FlagCGB::WorksWithOld) {
            panic!("Only DMG ROMs support for now");
        }

        match header.sgb_flag {
            crate::gameboy::header::FlagSGB::NoSGB => (),
            crate::gameboy::header::FlagSGB::SGB => panic!("SGB features are currently not supported"),
        }

        let cartridge = match create_for_cartridge_type(header.cartridge_type, rom_data) {
            Some(cartridge) => cartridge,
            None => todo!(
                "Cartridge not implemented for type: {:?}",
                header.cartridge_type
            ),
        };

        Self {
            cpu: if skip_boot_rom {
                let mut tmp = CPU::new_without_boot_rom(cartridge, trace_mode);
                tmp.mmu().disable_boot_rom();
                tmp
            } else {
                CPU::new(cartridge, trace_mode)
            },

            index: 0,
            maybe_reference_metadata: reference_metadata,
        }
    }

    pub fn tick(&mut self) -> Option<&FrameBuffer> {
        let current_metadata = if let Some(reference_metadata) = &self.maybe_reference_metadata {
            if self.index >= reference_metadata.len() {
                panic!("Ran out of reference data");
            }
            Some(&reference_metadata[self.index])
        } else {
            None
        };

        let cycles = self.cpu.tick(current_metadata, self.index);
        for _ in 0..cycles {
            // TODO: Should we tick cycles * 4 here?
            let video_interrupts = self.cpu.mmu().video().tick();
            for interrupt in video_interrupts {
                let interrupt_flag = match interrupt {
                    VideoInterrupt::Stat => InterruptSource::Lcd,
                    VideoInterrupt::VBlank => InterruptSource::VBlank,
                };
                self.cpu.mmu().set_interrupt_flag(interrupt_flag, true);
            }
        }
        let consumed_memory_cycles = self.cpu.mmu().take_consumed_cycles();
        self.cpu.mmu().maybe_tick_timers(cycles - consumed_memory_cycles);

        self.index += 1;

        return self.cpu.mmu().video().try_take_frame();
    }
}
