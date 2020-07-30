use std::fs::File;
use std::io;
use std::io::{BufReader, Read};
use std::str;

pub mod record;
pub use record::*;
use super::common::*;

pub struct Plugin {
    version: f32,
    pub is_master: bool,
    author: String,
    description: String,
    masters: Vec<(String, u64)>,
    records: Vec<Record>,
}

const HEADER_LENGTH: usize = 300;
const FLAG_MASTER: u32 = 0x1;

impl Plugin {
    pub fn read<T: Read>(f: &mut T) -> io::Result<Plugin> {
        let header = Record::read(f)?;
        if header.name() != b"TES3" {
            return Err(io_error(&format!("Expected TES3 record, got {}", header.display_name())));
        }

        let mut fields = header.into_iter();
        // TODO: does the game require the HEDR field to come first or is that just convention?
        let header = match fields.next() {
            Some(field) if field.name() == b"HEDR" => field,
            _ => return Err(io_error("Missing HEDR field")),
        };

        let header_data = header.consume();
        if header_data.len() != HEADER_LENGTH {
            return Err(io_error("Invalid HEDR field"));
        }

        // decode header structure
        let mut head_reader: &[u8] = header_data.as_ref();
        let version = extract!(head_reader as f32)?;
        let flags = extract!(head_reader as u32)?;
        let author = extract_str(32, &mut head_reader)?;
        let description = extract_str(256, &mut head_reader)?;
        let num_records = extract!(head_reader as u32)?;

        let mut masters = vec![];
        let mut master_name = None;
        while let Some(field) = fields.next() {
            match field.name() {
                b"MAST" => {
                    if let Some(name) = master_name {
                        return Err(io_error(&format!("Missing size for master {}", name)));
                    }

                    let string_name = field.get_zstring().map_err(|e| io_error(&format!("Could not decode master name: {}", e)))?;
                    master_name = Some(String::from(string_name));
                },
                b"DATA" => {
                    if let Some(name) = master_name {
                        let size = field.get_u64().ok_or(io_error("Invalid master size"))?;
                        masters.push((name, size));
                        master_name = None;
                    } else {
                        return Err(io_error("Data field without master"));
                    }
                },
                _ => return Err(io_error(&format!("Unexpected field in header: {}", field.display_name()))),
            }
        }

        if let Some(name) = master_name {
            return Err(io_error(&format!("Missing size for master {}", name)));
        }

        let mut records = Vec::with_capacity(num_records as usize);
        for _ in 0..num_records {
            records.push(Record::read(f)?);
        }

        Ok(Plugin {
            version,
            is_master: flags & FLAG_MASTER != 0,
            author,
            description,
            masters,
            records,
        })
    }

    pub fn load_file(path: &str) -> io::Result<Plugin> {
        let f = File::open(path)?;
        let mut reader = BufReader::new(f);
        Plugin::read(&mut reader)
    }
}

#[cfg(test)]
mod tests {

}