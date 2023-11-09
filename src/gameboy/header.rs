use std::str;

#[derive(Debug)]
pub enum FlagCGB {
    WorksWithOld,
    RequiresNew,
}

#[derive(Debug)]
pub struct TitleInfo {
    title: String,
    manufacturer_code: Option<String>,
    flag: FlagCGB,
}

#[derive(Debug)]
pub enum FlagSGB {
    NoSGB,
    SGB,
}

fn read_title_info(data: &Vec<u8>) -> Result<TitleInfo, String> {
    // TODO: Take different size into better consideration:
    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0134-0143--title
    // Title was originally 15 bytes, but support only 11 bytes (due to newer gameboys)
    let mut title_bytes: &[u8] = &data[0x0134..=0x0143];
    let first_zero = title_bytes.iter().position(|&x| x == 0x00);
    title_bytes = match first_zero {
        Some(index) => &title_bytes[0..index],
        None => title_bytes,
    };
    let title = str::from_utf8(title_bytes)
        .map_err(|x| x.to_string())?
        .to_owned();

    let manufacturer_code_bytes: &[u8] = &data[0x013F..=0x0142];
    let has_manufacturer_code = manufacturer_code_bytes
        .iter()
        .find(|&x| x != &0x00)
        .is_some();
    let manufacturer_code = if has_manufacturer_code {
        Some(
            str::from_utf8(manufacturer_code_bytes)
                .map_err(|x| x.to_string())?
                .to_owned(),
        )
    } else {
        None
    };

    let flag_byte: &u8 = &data[0x0143];

    match flag_byte {
        0xC0 => {
            panic!("The cartridge requires CGB functionality");
        },
        _ => (),
    }

    let flag: FlagCGB = match flag_byte {
        // 0x80 => Ok(FlagCGB::WorksWithOld),
        0xC0 => FlagCGB::RequiresNew,
        // _ => Err(format!("Invalid flag: {}", flag_byte)),
        _ => FlagCGB::WorksWithOld,
    };

    Ok(TitleInfo {
        title,
        manufacturer_code,
        flag,
    })
}

#[derive(Debug, Copy, Clone)]
pub enum CartridgeType {
    RomOnly,
    MBC1,
    MBC1Ram,
    MBC1RamBattery,
    MBC2,
    MBC2Battery,
    RomRam,
    RomRamBattery,
    MMM01,
    Mmm01Ram,
    Mmm01RamBattery,
    MBC3TimerBattery,
    MBC3TimerRamBattery,
    MBC3,
    MBC3Ram,
    MBC3RamBattery,
    MBC4,
    MBC4Ram,
    MBC4RamBattery,
    MBC5,
    MBC5Ram,
    MBC5RamBattery,
    MBC5Rumble,
    MBC5RumbleRam,
    MBC5RumbleRamBattery,
    PocketCamera,
    BandaiTama5,
    HuC3,
    HuC1RamBattery,
}

impl CartridgeType {
    fn from_byte(byte: u8) -> Option<CartridgeType> {
        match byte {
            0x00 => Some(CartridgeType::RomOnly),
            0x01 => Some(CartridgeType::MBC1),
            0x02 => Some(CartridgeType::MBC1Ram),
            0x03 => Some(CartridgeType::MBC1RamBattery),
            0x05 => Some(CartridgeType::MBC2),
            0x06 => Some(CartridgeType::MBC2Battery),
            0x08 => Some(CartridgeType::RomRam),
            0x09 => Some(CartridgeType::RomRamBattery),
            0x0B => Some(CartridgeType::MMM01),
            0x0C => Some(CartridgeType::Mmm01Ram),
            0x0D => Some(CartridgeType::Mmm01RamBattery),
            0x0F => Some(CartridgeType::MBC3TimerBattery),
            0x10 => Some(CartridgeType::MBC3TimerRamBattery),
            0x11 => Some(CartridgeType::MBC3),
            0x12 => Some(CartridgeType::MBC3Ram),
            0x13 => Some(CartridgeType::MBC3RamBattery),
            0x15 => Some(CartridgeType::MBC4),
            0x16 => Some(CartridgeType::MBC4Ram),
            0x17 => Some(CartridgeType::MBC4RamBattery),
            0x19 => Some(CartridgeType::MBC5),
            0x1A => Some(CartridgeType::MBC5Ram),
            0x1B => Some(CartridgeType::MBC5RamBattery),
            0x1C => Some(CartridgeType::MBC5Rumble),
            0x1D => Some(CartridgeType::MBC5RumbleRam),
            0x1E => Some(CartridgeType::MBC5RumbleRamBattery),
            0xFC => Some(CartridgeType::PocketCamera),
            0xFD => Some(CartridgeType::BandaiTama5),
            0xFE => Some(CartridgeType::HuC3),
            0xFF => Some(CartridgeType::HuC1RamBattery),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum RomSize {
    NoBanking,
    WithBanking(usize),
}

impl RomSize {
    fn from_byte(byte: u8) -> Option<RomSize> {
        match byte {
            0x00 => Some(RomSize::NoBanking),
            0x01 => Some(RomSize::WithBanking(4)),
            0x02 => Some(RomSize::WithBanking(8)),
            0x03 => Some(RomSize::WithBanking(16)),
            0x04 => Some(RomSize::WithBanking(32)),
            // only 63 banks used by MBC1,
            0x05 => Some(RomSize::WithBanking(64)),
            // only 125 banks used by MBC1
            0x06 => Some(RomSize::WithBanking(128)),
            0x07 => Some(RomSize::WithBanking(256)),
            0x52 => Some(RomSize::WithBanking(72)),
            0x53 => Some(RomSize::WithBanking(80)),
            0x54 => Some(RomSize::WithBanking(96)),
            _ => None,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum RamSize {
    NoBanks,
    Size {
        bank_count: usize,
        bank_size_kb: usize,
    },
}

impl RamSize {
    fn from_byte(byte: u8) -> Option<RamSize> {
        match byte {
            0x00 => Some(RamSize::NoBanks),
            0x01 => Some(RamSize::Size { bank_count: 1, bank_size_kb: 2 }),
            0x02 => Some(RamSize::Size { bank_count: 1, bank_size_kb: 8 }),
            0x03 => Some(RamSize::Size { bank_count: 4, bank_size_kb: 8 }),
            _ => None,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Header {
    pub title: String,
    pub manufacturer_code: Option<String>,
    pub cgb_flag: FlagCGB,
    pub license_code: Option<String>,
    pub sgb_flag: FlagSGB,
    pub cartridge_type: CartridgeType,
    pub rom_size: RomSize,
    pub ram_size: RamSize,
}

impl Header {
    pub fn read_from_rom(rom_data: &Vec<u8>) -> Result<Header, String> {
        if rom_data.len() < 0x0150 {
            return Err(format!(
                "Too little ROM data to read header. Need 0x0150, got {}",
                rom_data.len()
            ));
        }
        let title_info = read_title_info(rom_data)?;
        let license_code = match (rom_data[0x0114], rom_data[0x0145]) {
            (_, 0x00) => None,
            (0x00, _) => None,
            _ => {
                let data = &rom_data[0x0114..=0x0145];
                Some(str::from_utf8(data).unwrap().to_owned())
            }
        };

        let sgb_flag = match rom_data[0x0146] {
            0x00 => FlagSGB::NoSGB,
            0x03 => FlagSGB::SGB,
            _ => panic!("Invalid SGB flag: {}", rom_data[0x0146]),
        };

        let cartridge_type = CartridgeType::from_byte(rom_data[0x147])
            .ok_or(format!("Invalid cartridge type: {}", rom_data[0x147]))?;

        let rom_size = RomSize::from_byte(rom_data[0x148])
            .ok_or(format!("Invalid ROM size: {}", rom_data[0x148]))?;

        let ram_size = RamSize::from_byte(rom_data[0x149])
            .ok_or(format!("Invalid RAM size: {}", rom_data[0x149]))?;


        Ok(Header {
            title: title_info.title,
            manufacturer_code: title_info.manufacturer_code,
            cgb_flag: title_info.flag,
            license_code,
            sgb_flag,
            cartridge_type,
            rom_size,
            ram_size,
        })
    }
}
