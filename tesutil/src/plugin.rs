use std::cell::{Ref, RefMut, RefCell};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufReader, Read, Write, BufWriter};
use std::rc::Rc;
use std::str;

mod record;
pub use record::*;

use crate::*;

#[derive(Debug)]
pub enum PluginError {
    DuplicateId(String),
    DuplicateMaster(String),
    LimitExceeded { description: String, max_size: usize, actual_size: usize },
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PluginError::DuplicateId(id) => write!(f, "ID {} already in use", id),
            PluginError::DuplicateMaster(name) => write!(f, "Master {} already present", name),
            PluginError::LimitExceeded {
                description, max_size, actual_size
            } => write!(f, "Limit exceeded: {}. Max size {}, actual size {}", description, max_size, actual_size),
        }
    }
}

impl error::Error for PluginError {}

#[derive(Debug)]
pub struct Plugin {
    version: f32,
    pub is_master: bool,
    author: String,
    description: String,
    masters: Vec<(String, u64)>,
    records: Vec<Rc<RefCell<Record>>>,
    id_map: HashMap<String, Rc<RefCell<Record>>>,
}

const HEADER_LENGTH: usize = 300;
const AUTHOR_LENGTH: usize = 32;
const DESCRIPTION_LENGTH: usize = 256;
const FLAG_MASTER: u32 = 0x1;

pub const VERSION_1_2: f32 = 1.20000004768371582031;
pub const VERSION_1_3: f32 = 1.29999995231628417969;

// FIXME: figure out how to handle it if the ID of a record changes after being put in the map
impl Plugin {
    pub fn new(author: String, description: String) -> Plugin {
        Plugin {
            version: VERSION_1_3,
            is_master: false,
            author,
            description,
            masters: vec![],
            records: vec![],
            id_map: HashMap::new(),
        }
    }

    pub fn read<T: Read>(mut f: T) -> io::Result<Plugin> {
        let header = Record::read(&mut f)?;
        if header.name() != b"TES3" {
            return Err(io_error(format!("Expected TES3 record, got {}", header.display_name())));
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
        // TODO: I honestly don't understand why I have to use as_ref for the reader but I can just do &mut buf for the writer
        let mut head_reader: &[u8] = header_data.as_ref();
        let version = extract!(head_reader as f32)?;
        let flags = extract!(head_reader as u32)?;
        let author = extract_string(AUTHOR_LENGTH, &mut head_reader)?;
        let description = extract_string(DESCRIPTION_LENGTH, &mut head_reader)?;
        let num_records = extract!(head_reader as u32)? as usize;

        let mut plugin = Plugin {
            version,
            is_master: flags & FLAG_MASTER != 0,
            author,
            description,
            masters: vec![],
            records: Vec::with_capacity(num_records),
            id_map: HashMap::with_capacity(num_records),
        };

        let mut master_name = None;
        while let Some(field) = fields.next() {
            match field.name() {
                b"MAST" => {
                    if let Some(name) = master_name {
                        return Err(io_error(format!("Missing size for master {}", name)));
                    }

                    let string_name = field.get_zstring().map_err(|e| io_error(format!("Could not decode master name: {}", e)))?;
                    master_name = Some(String::from(string_name));
                },
                b"DATA" => {
                    if let Some(name) = master_name {
                        let size = field.get_u64().ok_or(io_error("Invalid master size"))?;
                        plugin.add_master(name, size).map_err(|e| io_error(format!("Duplicate masters: {}", e)))?;
                        master_name = None;
                    } else {
                        return Err(io_error("Data field without master"));
                    }
                },
                _ => return Err(io_error(format!("Unexpected field in header: {}", field.display_name()))),
            }
        }

        if let Some(name) = master_name {
            return Err(io_error(format!("Missing size for master {}", name)));
        }

        for _ in 0..num_records {
            plugin.add_record(Record::read(&mut f)?).map_err(|e| io_error(format!("Duplicate ID: {}", e)))?;
        }

        Ok(plugin)
    }

    pub fn load_file(path: &str) -> io::Result<Plugin> {
        let f = File::open(path)?;
        let mut reader = BufReader::new(f);
        Plugin::read(&mut reader)
    }

    pub fn add_master(&mut self, name: String, size: u64) -> Result<(), PluginError> {
        // don't add it if it's already in the list
        if !self.masters.iter().any(|m| m.0 == name) {
            self.masters.push((name, size));
            Ok(())
        } else {
            Err(PluginError::DuplicateMaster(name))
        }
    }

    pub fn add_record(&mut self, record: Record) -> Result<(), PluginError> {
        let r = Rc::new(RefCell::new(record));
        if let Some(id) = r.borrow().id() {
            let key = String::from(id);
            if self.id_map.contains_key(id) {
                return Err(PluginError::DuplicateId(key));
            }

            self.id_map.insert(key, Rc::clone(&r));
        }
        self.records.push(r);
        Ok(())
    }

    pub fn get_record(&self, id: &str) -> Option<Ref<Record>> {
        self.id_map.get(id).map(|r| r.borrow())
    }

    pub fn get_record_mut(&self, id: &str) -> Option<RefMut<Record>> {
        self.id_map.get(id).map(|r| r.borrow_mut())
    }

    pub fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
        let mut header = Record::new(b"TES3");
        let mut buf: Vec<u8> = Vec::with_capacity(HEADER_LENGTH);
        let mut buf_writer = &mut buf;

        serialize!(self.version => buf_writer)?;
        serialize!(if self.is_master { FLAG_MASTER } else { 0 } => buf_writer)?;
        serialize_str(&self.author, AUTHOR_LENGTH, &mut buf_writer)?;
        serialize_str(&self.description, DESCRIPTION_LENGTH, &mut buf_writer)?;
        serialize!(self.records.len() as u32 => buf_writer)?;

        header.add_field(Field::new(b"HEDR", buf).unwrap());

        for (name, size) in self.masters.iter() {
            let mast = Field::new_zstring(b"MAST", name.clone()).map_err(|e| io_error(format!("Failed to encode master file name: {}", e)))?;
            header.add_field(mast);
            header.add_field(Field::new_u64(b"DATA", *size));
        }

        header.write(&mut f)?;

        for record in self.records.iter() {
            record.borrow().write(&mut f)?;
        }

        Ok(())
    }

    pub fn save_file(&self, path: &str) -> io::Result<()> {
        let f = File::create(path)?;
        let mut writer = BufWriter::new(f);
        self.write(&mut writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_PLUGIN: &[u8] = include_bytes!("test/multipatch.esp");
    static EXPECTED_PLUGIN: &[u8] = include_bytes!("test/expected.esp");

    #[test]
    fn read_plugin() {
        let plugin = Plugin::read(&mut TEST_PLUGIN.as_ref()).unwrap();
        assert_eq!(plugin.version, VERSION_1_3);
        assert!(!plugin.is_master);
        assert_eq!(plugin.author, "tes3cmd multipatch");
        assert_eq!(plugin.description, "options: cellnames,fogbug,merge_lists,summons_persist");
        assert_eq!(plugin.masters.len(), 0);
        assert_eq!(plugin.records.len(), 8);
    }

    #[test]
    fn write_plugin() {
        let mut buf: Vec<u8> = Vec::with_capacity(EXPECTED_PLUGIN.len());
        let mut plugin = Plugin::new(String::from("test"), String::from("This is an empty test plugin"));
        plugin.is_master = true;
        plugin.add_master(String::from("Morrowind.esm"), 79837557).unwrap();

        let mut test_record = Record::new(b"GMST");
        test_record.add_field(Field::new_string(b"NAME", String::from("iDispKilling")).unwrap());
        test_record.add_field(Field::new_i32(b"INTV", -50));
        plugin.add_record(test_record).unwrap();

        plugin.write(&mut &mut buf).unwrap();

        assert_eq!(buf, EXPECTED_PLUGIN);
    }

    #[test]
    fn fetch_record() {
        let plugin = Plugin::read(&mut TEST_PLUGIN.as_ref()).unwrap();
        let record = plugin.get_record("BM_wolf_grey_summon").unwrap();
        assert_eq!(record.len(), 9);
        for field in record.iter() {
            if let Some(expected) = match field.name() {
                b"NAME" => Some("BM_wolf_grey_summon"),
                b"MODL" => Some("r\\Wolf_Black.NIF"),
                b"CNAM" => Some("BM_wolf_grey"),
                b"FNAM" => Some("Wolf"),
                _ => None,
            } {
                let value = field.get_zstring().unwrap();
                assert_eq!(value, expected);
            }
        }
    }
}