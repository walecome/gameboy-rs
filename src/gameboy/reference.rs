use std::path::PathBuf;
use std::fs;

pub enum ReferenceOpcode {
    Plain(u8),
    CB(u8),
}

pub struct ReferenceMetadata {
    pub pc: u16,
    pub instruction: String,
    pub opcode: ReferenceOpcode,
}

pub fn get_reference_metadata(reference: &PathBuf) -> Vec<ReferenceMetadata> {
    fs::read_to_string(reference)
        .unwrap()
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let parts: Vec<&str> = line.split(": ").collect();
            let pc = u16::from_str_radix(
                parts[0]
                    .strip_prefix("0x")
                    .expect(format!("{}: {}", i, line).as_str()),
                16,
            );
            if pc.is_err() {
                panic!("{} could not be made to hex", parts[0]);
            }
            let instruction = parts[1].to_owned();
            ReferenceMetadata {
                pc: pc.unwrap(),
                instruction,
                opcode: read_opcode(line),
            }
        })
        .collect()
}

fn read_opcode(part: &str) -> ReferenceOpcode {
    let mut tmp = part.rsplit_once("(").unwrap().1.to_owned();
    tmp.pop();
    let (raw, is_cb) = if tmp.starts_with("CB") {
        (
            tmp.strip_prefix("CB ").unwrap().strip_prefix("0x").unwrap(),
            true,
        )
    } else {
        (tmp.strip_prefix("0x").unwrap(), false)
    };
    let value =
        u8::from_str_radix(raw, 16).expect(format!("Could not make '{}' to hex", raw).as_str());
    return if is_cb {
        ReferenceOpcode::CB(value)
    } else {
        ReferenceOpcode::Plain(value)
    };
}
