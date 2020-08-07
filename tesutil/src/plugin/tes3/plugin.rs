use std::cell::{Ref, RefMut, RefCell};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufReader, Error, ErrorKind, Read, Write, BufWriter};
use std::rc::Rc;
use std::str;

use crate::*;
use crate::plugin::*;
use super::record::*;
use super::field::*;

/// Represents a plugin file
///
/// This type can be used to read and write plugin (mod) files. These include .esm files (masters,
/// on which other plugins may depend), .esp files (regular plugin files), and .ess files (saves).
/// Note that the `is_master` flag determines whether a plugin is a master, not the file extension;
/// using .esm for masters and .esp for non-masters is merely convention.
///
/// Plugins consist of a series of records which represent all the different objects in the game
/// world. Each record consists of one or more fields containing the data and attributes of the
/// object.
///
/// Plugins can be read from a file with [`load_file`] and saved to a file with [`save_file`]. You
/// can also use [`read`] and [`write`] directly to read a plugin from/write a plugin to a buffer
/// or any other type implementing [`Read`] or [`Write`], respectively. A new, empty plugin may be
/// created with [`new`].
///
/// [`load_file`]: #method.load_file
/// [`save_file`]: #method.save_file
/// [`read`]: #method.read
/// [`write`]: #method.write
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`new`]: #method.new
#[derive(Debug)]
pub struct Plugin {
    version: f32,
    /// Indicates whether this plugin is a master
    pub is_master: bool,
    author: String,
    description: String,
    masters: Vec<(String, u64)>,
    records: Vec<Rc<RefCell<Record>>>,
    id_map: HashMap<String, Rc<RefCell<Record>>>,
}

const HEADER_LENGTH: usize = 300;
const FLAG_MASTER: u32 = 0x1;

/// Maximum length of the plugin author string
pub const AUTHOR_LENGTH: usize = 32;
/// Maximum length of the plugin description string
pub const DESCRIPTION_LENGTH: usize = 256;

/// Morrowind version 1.2
///
/// Use this instead of a literal `1.2f32` to ensure the correct binary representation.
pub const VERSION_1_2: f32 = 1.20000004768371582031;
/// Morrowind version 1.3
///
/// Use this instead of a literal `1.3f32` to ensure the correct binary representation.
pub const VERSION_1_3: f32 = 1.29999995231628417969;

// FIXME: figure out how to handle the ID of a record changing after being put in the map
impl Plugin {
    /// Create a new, empty plugin
    ///
    /// Add masters and records to the empty plugin with [`add_master`] and [`add_record`]
    /// respectively. The version of new plugins is always 1.3.
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if the length of the author string exceeds
    /// [`AUTHOR_LENGTH`] or the length of the description string exceeds [`DESCRIPTION_LENGTH`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Plugin::new(String::from("test"), String::from("sample plugin"))?;
    /// plugin.is_master = true;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`add_master`]: #method.add_master
    /// [`add_record`]: #method.add_record
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`AUTHOR_LENGTH`]: constant.AUTHOR_LENGTH.html
    /// [`DESCRIPTION_LENGTH`]: constant.DESCRIPTION_LENGTH.html
    pub fn new(author: String, description: String) -> Result<Plugin, TesError> {
        check_size(&author, AUTHOR_LENGTH, "author value too long")?;
        check_size(&description, DESCRIPTION_LENGTH, "description value too long")?;
        Ok(Plugin {
            version: VERSION_1_3,
            is_master: false,
            author,
            description,
            masters: vec![],
            records: vec![],
            id_map: HashMap::new(),
        })
    }

    /// Read a plugin file from the provided reader
    ///
    /// Reads a plugin from any type that implements [`Read`] or a mutable reference to such a type.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if the format of the plugin data is invalid or if an I/O error
    /// occurs while reading the plugin data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let buf: Vec<u8> = vec![/* raw plugin data */];
    /// let plugin = Plugin::read(&mut &buf[..])?;
    /// println!("Plugin info: author {}, description {}", plugin.author(), plugin.description());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    pub fn read<T: Read>(mut f: T) -> io::Result<Plugin> {
        let header = Record::read(&mut f)?.ok_or(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"))?;
        if header.name() != b"TES3" {
            return Err(io_error(format!("Expected TES3 record, got {}", header.display_name())));
        }

        let mut fields = header.into_iter();
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

                    let string_name = field.get_zstring().map_err(|e| io_error(e))?;
                    master_name = Some(String::from(string_name));
                },
                b"DATA" => {
                    if let Some(name) = master_name {
                        let size = field.get_u64().map_err(|e| io_error(e))?;
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

        // num_records is actually not guaranteed to be correct, so we ignore it and just read until we hit EOF
        loop {
            if let Some(record) = Record::read(&mut f)? {
                plugin.add_record(record).map_err(|e| io_error(e))?;
            } else {
                break;
            }
        }

        Ok(plugin)
    }

    /// Reads a plugin from a file
    ///
    /// Reads a plugin from the file at `path`. The entire plugin is read into memory and retains
    /// no reference to the file once the read is complete.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if the file cannot be opened or if [`Plugin::read`] fails;
    /// refer to that method for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::plugin::tes3::*;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Plugin::load_file("Morrowind.esm")?;
    /// assert!(plugin.is_master);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [`Plugin::read`]: #method.read
    pub fn load_file(path: &str) -> io::Result<Plugin> {
        let f = File::open(path)?;
        let mut reader = BufReader::new(f);
        Plugin::read(reader)
    }

    /// Returns the plugin file version
    ///
    /// This should always return one of the version constants ([`VERSION_1_2`] or [`VERSION_1_3`]),
    /// but could return something else if a plugin file is read that contains an unusual version
    /// value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let plugin = Plugin::new(String::from("test"), String::from("sample plugin"))?;
    /// assert_eq!(plugin.version(), VERSION_1_3);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`VERSION_1_2`]: constant.VERSION_1_2.html
    /// [`VERSION_1_3`]: constant.VERSION_1_3.html
    pub fn version(&self) -> f32 {
        self.version
    }

    /// Returns the plugin author string
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes3::*;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Plugin::load_file("Morrowind.esm")?;
    /// assert_eq!(plugin.author(), "Bethesda Softworks");
    /// # Ok(())
    /// # }
    /// ```
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Sets the plugin author string
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if the length of the string exceeds [`AUTHOR_LENGTH`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Plugin::new(String::from("wrong author"), String::from("some description"))?;
    /// plugin.set_author(String::from("correct author"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`AUTHOR_LENGTH`]: constant.AUTHOR_LENGTH.html
    pub fn set_author(&mut self, author: String) -> Result<(), TesError> {
        check_size(&author, AUTHOR_LENGTH, "author value too long")?;
        self.author = author;
        Ok(())
    }

    /// Returns the plugin description string
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::plugin::tes3::*;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Plugin::load_file("Bloodmoon.esm")?;
    /// assert_eq!(plugin.description(), "The main data file for BloodMoon.\r\n(requires Morrowind.esm to run)");
    /// # Ok(())
    /// # }
    /// ```
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Sets the plugin description string
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if the length of the string exceeds [`DESCRIPTION_LENGTH`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Plugin::new(String::from("author"), String::from("some description"))?;
    /// plugin.set_description(String::from("updated description"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`DESCRIPTION_LENGTH`]: constant.AUTHOR_LENGTH.html
    pub fn set_description(&mut self, description: String) -> Result<(), TesError> {
        check_size(&description, DESCRIPTION_LENGTH, "description value too long")?;
        self.description = description;
        Ok(())
    }

    /// Adds a new master to this plugin
    ///
    /// Masters are other plugins that this plugin depends on. `name` should be the filename of the
    /// master file (without path, including extension) and `size` should be the size in bytes of
    /// the master file (this is used by the game to check whether a master has changed since the
    /// last time this plugin was updated).
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::DuplicateMaster`] if `name` is already in the list of masters
    /// (case insensitive).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Plugin::new(String::from("author"), String::from("some description"))?;
    /// plugin.add_master(String::from("Morrowind.esm"), 79837557)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::DuplicateMaster`]: enum.PluginError.html#variant.DuplicateMaster
    pub fn add_master(&mut self, name: String, size: u64) -> Result<(), TesError> {
        // don't add it if it's already in the list
        // TODO: find out if doing a case-insensitive comparison is appropriate for OpenMW running
        //  on Linux
        if !self.masters.iter().any(|m| m.0.to_lowercase() == name.to_lowercase()) {
            self.masters.push((name, size));
            Ok(())
        } else {
            Err(TesError::DuplicateMaster(name))
        }
    }

    /// Adds a new record to this plugin
    ///
    /// Note: you should not explicitly add a TES3/TES4 record; this will be added automatically
    /// based on the metadata from the `Plugin` struct.
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::DuplicateId`] if the record has the same ID as an existing record
    /// in this plugin.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Plugin::new(String::from("test"), String::from("sample plugin"))?;
    /// let mut record = Record::new(b"GMST");
    /// record.add_field(Field::new_string(b"NAME", String::from("iDispKilling"))?);
    /// record.add_field(Field::new_i32(b"INTV", -50));
    /// plugin.add_record(record)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`PluginError::DuplicateId`]: enum.PluginError.html#variant.DuplicateId
    pub fn add_record(&mut self, record: Record) -> Result<(), TesError> {
        let r = Rc::new(RefCell::new(record));
        if let Some(id) = r.borrow().id() {
            let key = String::from(id);
            if self.id_map.contains_key(id) {
                return Err(TesError::DuplicateId(key));
            }

            self.id_map.insert(key, Rc::clone(&r));
        }
        self.records.push(r);
        Ok(())
    }

    /// Finds a record by ID
    ///
    /// If no record exists with the given ID, the return value will be `None`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::plugin::tes3::*;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Plugin::load_file("Morrowind.esm")?;
    /// if let Some(record) = plugin.get_record("HortatorVotes") {
    ///     // do something with record
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_record(&self, id: &str) -> Option<Ref<Record>> {
        self.id_map.get(id).map(|r| r.borrow())
    }

    /// Finds a record by ID and returns a mutable reference
    ///
    /// If no record exists with the given ID, the return value will be `None`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::plugin::tes3::*;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Plugin::load_file("Morrowind.esm")?;
    /// if let Some(mut record) = plugin.get_record_mut("HortatorVotes") {
    ///     // change something with record
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_record_mut(&self, id: &str) -> Option<RefMut<Record>> {
        self.id_map.get(id).map(|r| r.borrow_mut())
    }

    /// Writes a plugin to the provided writer
    ///
    /// Writes a plugin to any type that implements [`Write`] or a mutable reference to such a type.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if an I/O error occurs or if the plugin data is invalid. Plugin
    /// data being invalid includes situations like record data being too large or string values
    /// containing internal null bytes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut buf: Vec<u8> = vec![];
    /// let plugin = Plugin::load_file("Morrowind.esm")?;
    /// plugin.write(&mut &mut buf)?;
    /// assert_eq!(buf.len(), 79837557);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    pub fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
        let mut header = Record::new(b"TES3");
        let mut buf: Vec<u8> = Vec::with_capacity(HEADER_LENGTH);
        let mut buf_writer = &mut buf;

        // we can get away with this because the game doesn't actually care about this field, it
        // just reads until EOF
        let num_records = if self.records.len() > u32::MAX as usize {
            u32::MAX
        } else {
            self.records.len() as u32
        };

        serialize!(self.version => buf_writer)?;
        serialize!(if self.is_master { FLAG_MASTER } else { 0 } => buf_writer)?;
        serialize_str(&self.author, AUTHOR_LENGTH, &mut buf_writer)?;
        serialize_str(&self.description, DESCRIPTION_LENGTH, &mut buf_writer)?;
        serialize!(num_records => buf_writer)?;

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

    /// Save a plugin to a file
    ///
    /// The file must not exist.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if the file cannot be created or if [`Plugin::write`] fails;
    /// refer to that method for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::plugin::tes3::*;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut plugin = Plugin::new(String::from("test"), String::from("sample plugin"))?;
    /// plugin.is_master = true;
    /// plugin.save_file("sample.esm")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [`Plugin::write`]: #method.write
    pub fn save_file(&self, path: &str) -> io::Result<()> {
        let f = File::create(path)?;
        let mut writer = BufWriter::new(f);
        self.write(writer)
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
        let mut plugin = Plugin::new(String::from("test"), String::from("This is an empty test plugin")).unwrap();
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