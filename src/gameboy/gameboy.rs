use crate::common::framebuffer::FrameBuffer;

use super::cartridge::create_for_cartridge_type;
use super::cpu::CPU;
use super::cpu::TraceMode;
use super::header::{Header, FlagCGB};
use super::reference::ReferenceMetadata;

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
        self.cpu.mmu().video().tick(cycles as usize);
        self.cpu.mmu().maybe_tick_timers(cycles);

        self.index += 1;

        return self.cpu.mmu().video().try_take_frame();
    }
}
