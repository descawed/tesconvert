use std::fs::File;
use std::io;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::mem::size_of;
use std::str;

// have to use a macro instead of a generic because from_le_bytes isn't a trait method
// IMPORTANT: this took me a minute to figure out, but this macro definition MUST come before the
// import of the record module below, because the record module uses this macro.
macro_rules! extract {
    ($t:ty from $f:ident) => {
        {
            let mut buf = [0u8; size_of::<$t>()];
            $f.read_exact(&mut buf).map(move |_| <$t>::from_le_bytes(buf))
        }
    }
}

pub mod record;
pub use record::*;

pub struct Plugin {
    version: f32,
    pub is_master: bool,
    author: String,
    description: String,
    masters: Vec<(String, u64)>,
    records: Vec<Record>,
}

fn read_error<T>(msg: &str) -> io::Result<T> {
    Err(Error::new(ErrorKind::InvalidData, msg))
}

const HEADER_LENGTH: usize = 300;
const FLAG_MASTER: u32 = 0x1;

impl Plugin {
    pub fn read<T: Read>(f: &mut T) -> io::Result<Plugin> {
        let header = Record::read(f)?;
        if header.name() != b"TES3" {
            return read_error(&format!("Expected TES3 record, got {}", header.display_name()));
        }

        let mut fields = header.into_iter();
        // TODO: does the game require the HEDR field to come first or is that just convention?
        let header = match fields.next() {
            Some(field) if field.name() == b"HEDR" => field,
            _ => return read_error("Missing HEDR field"),
        };

        let header_data = header.consume();
        if header_data.len() != HEADER_LENGTH {
            return read_error("Invalid HEDR field");
        }

        // decode header structure
        let mut head_reader = header_data.as_ref();
        let version = extract!(f32 from head_reader)?;
        let flags = extract!(u32 from head_reader)?;

        let mut raw_author = [0u8; 32];
        f.read_exact(&mut raw_author)?;

        let num_records = extract!(u32 from head_reader)?;

        let mut masters = vec![];
        let mut master_name = None;
        while let Some(field) = fields.next() {
            match field.name() {
                b"MAST" => {
                    if let Some(name) = master_name {
                        return read_error(&format!("Missing size for master {}", name));
                    }

                    let string_name = field.get_zstring().map_err(|e| read_error(&format!("Could not decode master name: {}", e)))?;
                    master_name = Some(String::from(string_name));
                },
                b"DATA" => {
                    if let Some(name) = master_name {
                        let size = field.get_u64().ok_or_else(Error::new(ErrorKind::InvalidData, "Invalid master size"))?;
                        masters.push((name, size));
                        master_name = None;
                    } else {
                        return read_error("Data field without master");
                    }
                },
                _ => return read_error(&format!("Unexpected field in header: {}", field.display_name())),
            }
        }

        if let Some(name) = master_name {
            return read_error(&format!("Missing size for master {}", name));
        }

        Ok()
    }

    pub fn load_file(path: &str) -> io::Result<Plugin> {
        let f = File::open(path)?;
        let mut reader = BufReader::new(f);
        Plugin::read(&mut reader)
    }
}