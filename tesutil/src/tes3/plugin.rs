use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;
use std::str;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::plugin::*;
use crate::*;

mod field;
pub use field::*;

mod npc;
pub use npc::*;

mod player;
pub use player::*;

mod record;
pub use record::*;

mod reference;
pub use reference::*;

mod class;
pub use class::*;

mod gmst;
pub use gmst::*;

mod spell;
pub use spell::*;

mod active_effects;
pub use active_effects::*;

mod race;
pub use race::*;

mod birthsign;
pub use birthsign::*;

/// Save game information
///
/// For saves (.ess files), this information is included in the TES3 record.
#[derive(Debug)]
pub struct SaveInfo {
    current_health: f32,
    max_health: f32,
    hour: f32,
    unknown1: [u8; 12],
    current_cell: String,
    unknown2: [u8; 4],
    player_name: String,
}

impl SaveInfo {
    /// Get the player's current health
    pub fn current_health(&self) -> f32 {
        self.current_health
    }

    /// Set the player's current health
    ///
    /// # Errors
    ///
    /// Fails if you attempt to set the player's current health greater than their maximum health or
    /// to a negative value.
    pub fn set_current_health(&mut self, value: f32) -> Result<(), TesError> {
        check_range(
            value,
            0.,
            self.max_health,
            "current health must be between 0 and max health",
        )?;
        self.current_health = value;
        Ok(())
    }

    /// Get the player's maximum health
    pub fn max_health(&self) -> f32 {
        self.max_health
    }

    /// Set the player's maximum health
    ///
    /// # Errors
    ///
    /// Fails if you attempt to set the player's maximum health lower than their current health.
    pub fn set_max_health(&mut self, value: f32) -> Result<(), TesError> {
        check_range(
            value,
            self.current_health,
            f32::MAX,
            "max health must be >= current health",
        )?;
        self.max_health = value;
        Ok(())
    }

    /// Get the current hour of the day in the game
    pub fn hour(&self) -> f32 {
        self.hour
    }

    /// Set the current hour of the day in the game
    ///
    /// # Errors
    ///
    /// Fails if you attempt to set the hour to a value outside the range 0-24.
    pub fn set_hour(&mut self, value: f32) -> Result<(), TesError> {
        check_range(value, 0., 24., "hour must be between 0-24")?;
        self.hour = value;
        Ok(())
    }

    /// Gets the name of the player's current cell
    pub fn current_cell(&self) -> &str {
        &self.current_cell
    }

    /// Sets the name of the player's current cell
    ///
    /// # Errors
    ///
    /// Fails if the length of the name of the cell exceeds [`CELL_LENGTH`]
    ///
    /// [`CELL_LENGTH`]: constant.CELL_LENGTH.html
    pub fn set_current_cell(&mut self, cell: String) -> Result<(), TesError> {
        check_size(&cell, CELL_LENGTH, "cell name too long")?;
        self.current_cell = cell;
        Ok(())
    }

    /// Gets the player's name
    pub fn player_name(&self) -> &str {
        &self.player_name
    }

    /// Sets the player's name
    ///
    /// Note: this is only the name that shows up in the save menu. The name that will appear in
    /// game is the name on the player's NPC record.
    ///
    /// # Errors
    ///
    /// Fails if the length of the name exceeds [`NAME_LENGTH`].
    ///
    /// [`NAME_LENGTH`]: constant.NAME_LENGTH.html
    pub fn set_player_name(&mut self, name: String) -> Result<(), TesError> {
        check_size(&name, NAME_LENGTH, "player name too long")?;
        self.player_name = name;
        Ok(())
    }
}

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
pub struct Tes3Plugin {
    version: f32,
    is_master: bool,
    author: String,
    description: String,
    save: Option<SaveInfo>,
    screen_data: Vec<u8>,
    masters: Vec<(String, u64)>,
    records: Vec<Arc<RwLock<Tes3Record>>>,
    id_map: HashMap<String, HashMap<[u8; 4], Arc<RwLock<Tes3Record>>>>,
    type_map: HashMap<[u8; 4], Vec<Arc<RwLock<Tes3Record>>>>,
}

const HEADER_LENGTH: usize = 300;
const FLAG_MASTER: u32 = 0x1;

/// Maximum length of the plugin author string
pub const AUTHOR_LENGTH: usize = 32;
/// Maximum length of the plugin description string
pub const DESCRIPTION_LENGTH: usize = 256;
/// Maximum length of player name
pub const NAME_LENGTH: usize = 32;
/// Maximum length of player cell name
pub const CELL_LENGTH: usize = 64;

/// Morrowind version 1.2
pub const VERSION_1_2: f32 = 1.2;
/// Morrowind version 1.3
pub const VERSION_1_3: f32 = 1.3;

impl Tes3Plugin {
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
    /// use tesutil::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Tes3Plugin::new(String::from("test"), String::from("sample plugin"))?;
    /// plugin.set_is_master(true);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`add_master`]: #method.add_master
    /// [`add_record`]: #method.add_record
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`AUTHOR_LENGTH`]: constant.AUTHOR_LENGTH.html
    /// [`DESCRIPTION_LENGTH`]: constant.DESCRIPTION_LENGTH.html
    pub fn new(author: String, description: String) -> Result<Tes3Plugin, TesError> {
        check_size(&author, AUTHOR_LENGTH, "author value too long")?;
        check_size(
            &description,
            DESCRIPTION_LENGTH,
            "description value too long",
        )?;
        Ok(Tes3Plugin {
            version: VERSION_1_3,
            is_master: false,
            author,
            description,
            save: None,
            screen_data: vec![],
            masters: vec![],
            records: vec![],
            id_map: HashMap::new(),
            type_map: HashMap::new(),
        })
    }

    /// Read a plugin file from the provided reader
    ///
    /// Reads a plugin from any type that implements [`Read`] or a mutable reference to such a type.
    ///
    /// # Errors
    ///
    /// Returns an error if the format of the plugin data is invalid or if an I/O error
    /// occurs while reading the plugin data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::tes3::*;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let buf: Vec<u8> = vec![/* raw plugin data */];
    /// let plugin = Tes3Plugin::read(&mut &buf[..])?;
    /// println!("Plugin info: author {}, description {}", plugin.author(), plugin.description());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
    pub fn read<T: Read + Seek>(mut f: T) -> Result<Tes3Plugin, TesError> {
        let header = Tes3Record::read(&mut f)?;
        if header.name() != b"TES3" {
            return Err(decode_failed(format!(
                "Expected TES3 record, got {}",
                header.display_name()
            )));
        }

        let mut fields = header.into_iter();
        let header = match fields.next() {
            Some(field) if field.name() == b"HEDR" => field,
            _ => return Err(decode_failed("Missing HEDR field")),
        };

        let header_data = header.consume();
        if header_data.len() != HEADER_LENGTH {
            return Err(decode_failed("Invalid HEDR field"));
        }

        // decode header structure
        let mut head_reader: &[u8] = header_data.as_ref();
        let version = extract!(head_reader as f32)?;
        let flags = extract!(head_reader as u32)?;
        let author = extract_string(AUTHOR_LENGTH, &mut head_reader)?;
        let description = extract_string(DESCRIPTION_LENGTH, &mut head_reader)?;
        let num_records = extract!(head_reader as u32)? as usize;

        let mut plugin = Tes3Plugin {
            version,
            is_master: flags & FLAG_MASTER != 0,
            author,
            description,
            save: None,
            screen_data: vec![],
            masters: vec![],
            records: Vec::with_capacity(num_records),
            id_map: HashMap::with_capacity(num_records),
            type_map: HashMap::new(),
        };

        let mut master_name = None;
        for field in fields {
            match field.name() {
                b"MAST" => {
                    if let Some(name) = master_name {
                        return Err(decode_failed(format!("Missing size for master {}", name)));
                    }

                    let string_name = field.get_zstring()?;
                    master_name = Some(String::from(string_name));
                }
                b"DATA" => {
                    if let Some(name) = master_name {
                        let size = field.get_u64()?;
                        plugin.add_master(name, size)?;
                        master_name = None;
                    } else {
                        return Err(decode_failed("Data field without master"));
                    }
                }
                b"GMDT" => {
                    // TODO: write a test for this part
                    let data = field.consume();
                    let mut reader: &mut &[u8] = &mut data.as_ref();
                    let current_health = extract!(reader as f32)?;
                    let max_health = extract!(reader as f32)?;
                    let hour = extract!(reader as f32)?;
                    let mut unknown1 = [0u8; 12];
                    reader.read_exact(&mut unknown1)?;
                    let current_cell = extract_string(CELL_LENGTH, &mut reader)?;
                    let mut unknown2 = [0u8; 4];
                    reader.read_exact(&mut unknown2)?;
                    let player_name = extract_string(NAME_LENGTH, &mut reader)?;

                    plugin.save = Some(SaveInfo {
                        current_health,
                        max_health,
                        hour,
                        unknown1,
                        current_cell,
                        unknown2,
                        player_name,
                    });
                }
                b"SCRD" => (),
                b"SCRS" => plugin.screen_data = field.consume(),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field in header: {}",
                        field.name_as_str()
                    )))
                }
            }
        }

        if let Some(name) = master_name {
            return Err(decode_failed(format!("Missing size for master {}", name)));
        }

        // num_records is actually not guaranteed to be correct, so we ignore it and just read until we hit EOF
        let mut here = f.seek(SeekFrom::Current(0))?;
        let eof = f.seek(SeekFrom::End(0))?;
        f.seek(SeekFrom::Start(here))?;

        while here != eof {
            plugin.add_record(Tes3Record::read_lazy(&mut f)?)?;
            here = f.seek(SeekFrom::Current(0))?;
        }

        Ok(plugin)
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
    /// use tesutil::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let plugin = Tes3Plugin::new(String::from("test"), String::from("sample plugin"))?;
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
    /// use tesutil::*;
    /// use tesutil::tes3::*;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Tes3Plugin::load_file("Morrowind.esm")?;
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
    /// use tesutil::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Tes3Plugin::new(String::from("wrong author"), String::from("some description"))?;
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
    /// use tesutil::tes3::*;
    /// use tesutil::Plugin;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Tes3Plugin::load_file("Bloodmoon.esm")?;
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
    /// use tesutil::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Tes3Plugin::new(String::from("author"), String::from("some description"))?;
    /// plugin.set_description(String::from("updated description"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`DESCRIPTION_LENGTH`]: constant.AUTHOR_LENGTH.html
    pub fn set_description(&mut self, description: String) -> Result<(), TesError> {
        check_size(
            &description,
            DESCRIPTION_LENGTH,
            "description value too long",
        )?;
        self.description = description;
        Ok(())
    }

    /// Get save game info if any is present
    pub fn get_save_info(&self) -> Option<&SaveInfo> {
        self.save.as_ref()
    }

    /// Get save game info if any is present, mutably
    pub fn get_save_info_mut(&mut self) -> Option<&mut SaveInfo> {
        self.save.as_mut()
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
    /// use tesutil::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Tes3Plugin::new(String::from("author"), String::from("some description"))?;
    /// plugin.add_master(String::from("Morrowind.esm"), 79837557)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::DuplicateMaster`]: enum.PluginError.html#variant.DuplicateMaster
    pub fn add_master(&mut self, name: String, size: u64) -> Result<(), TesError> {
        // don't add it if it's already in the list
        if !self
            .masters
            .iter()
            .any(|m| m.0.to_lowercase() == name.to_lowercase())
        {
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
    /// use tesutil::tes3::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Tes3Plugin::new(String::from("test"), String::from("sample plugin"))?;
    /// let mut record = Tes3Record::new(b"GMST");
    /// record.add_field(Tes3Field::new_string(b"NAME", String::from("iDispKilling"))?);
    /// record.add_field(Tes3Field::new_i32(b"INTV", -50));
    /// plugin.add_record(record)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`PluginError::DuplicateId`]: enum.PluginError.html#variant.DuplicateId
    pub fn add_record(&mut self, mut record: Tes3Record) -> Result<(), TesError> {
        record.finalize()?;
        let r = Arc::new(RwLock::new(record));
        let rb = r.read().unwrap();
        let key = *rb.name();
        let has_id = rb.has_id();
        drop(rb);
        // if this is a record type that doesn't have an ID, we can avoid finalizing it now
        if has_id {
            r.write().unwrap().finalize()?;
            let rb = r.read().unwrap();
            if let Some(id) = rb.id() {
                let key = String::from(id);
                let name = rb.name();
                let type_map = self.id_map.entry(key).or_insert_with(HashMap::new);
                // FIXME: it appears there are duplicates in the save file even among the same type (specifically CREC records)
                /*if type_map.contains_key(name) {
                    return Err(TesError::DuplicateId(key));
                }*/

                type_map.insert(*name, Arc::clone(&r));
            }
        }
        let records = self.type_map.entry(key).or_insert_with(Vec::new);
        records.push(Arc::clone(&r));
        self.records.push(r);
        Ok(())
    }

    /// Finds a record by ID
    ///
    /// If no record exists with the given ID, the return value will be `None`.
    ///
    /// # Errors
    ///
    /// Fails if there is more than one record with the given ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::tes3::*;
    /// use tesutil::Plugin;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Tes3Plugin::load_file("Morrowind.esm")?;
    /// if let Some(record) = plugin.get_record("HortatorVotes") {
    ///     // do something with record
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_record(&self, id: &str) -> Result<Option<RwLockReadGuard<Tes3Record>>, TesError> {
        if let Some(ref type_map) = self.id_map.get(id) {
            if type_map.is_empty() {
                Ok(None)
            } else if type_map.len() > 1 {
                Err(TesError::LimitExceeded {
                    description: format!("More than one record with ID {}", id),
                    max_size: 1,
                    actual_size: type_map.len(),
                })
            } else {
                Ok(Some(type_map.values().next().unwrap().read().unwrap()))
            }
        } else {
            Ok(None)
        }
    }

    /// Finds a record by ID and type
    pub fn get_record_with_type(
        &self,
        id: &str,
        name: &[u8; 4],
    ) -> Option<RwLockReadGuard<Tes3Record>> {
        self.id_map.get(id)?.get(name).map(|v| v.read().unwrap())
    }

    /// Gets an iterator over fields with a particular type
    pub fn get_records_by_type(
        &self,
        name: &[u8; 4],
    ) -> Option<impl Iterator<Item = RwLockReadGuard<Tes3Record>>> {
        self.type_map.get(name).map(|v| {
            v.iter().filter_map(|r| {
                if r.read().unwrap().status() == RecordStatus::Initialized {
                    let _ = r.write().unwrap().finalize();
                }

                let rb = r.read().unwrap();
                match rb.status() {
                    RecordStatus::Finalized => Some(rb),
                    _ => None,
                }
            })
        })
    }

    /// Finds a record by ID and returns a mutable reference
    ///
    /// If no record exists with the given ID, the return value will be `None`.
    ///
    /// # Errors
    ///
    /// Fails if there is more than one record with the given ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::tes3::*;
    /// use tesutil::Plugin;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut plugin = Tes3Plugin::load_file("Morrowind.esm")?;
    /// if let Some(mut record) = plugin.get_record_mut("HortatorVotes") {
    ///     // change something with record
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_record_mut(
        &mut self,
        id: &str,
    ) -> Result<Option<RwLockWriteGuard<Tes3Record>>, TesError> {
        if let Some(type_map) = self.id_map.get_mut(id) {
            if type_map.is_empty() {
                Ok(None)
            } else if type_map.len() > 1 {
                Err(TesError::LimitExceeded {
                    description: format!("More than one record with ID {}", id),
                    max_size: 1,
                    actual_size: type_map.len(),
                })
            } else {
                Ok(Some(type_map.values_mut().next().unwrap().write().unwrap()))
            }
        } else {
            Ok(None)
        }
    }

    /// Loads a form by ID and type
    ///
    /// # Errors
    ///
    /// Fails if the matching record contains invalid data.
    pub fn get<T: Form<Field = Tes3Field, Record = Tes3Record>>(
        &self,
        id: &str,
    ) -> Result<Option<T>, TesError> {
        Ok(match self.get_record_with_type(id, T::record_type()) {
            Some(record) => Some(T::read(&*record)?),
            None => None,
        })
    }

    /// Finds a record by ID and type and returns a mutable reference
    pub fn get_record_with_type_mut(
        &mut self,
        id: &str,
        name: &[u8; 4],
    ) -> Option<RwLockWriteGuard<Tes3Record>> {
        self.id_map
            .get_mut(id)?
            .get_mut(name)
            .map(|v| v.write().unwrap())
    }

    /// Gets an iterator over the names of this plugin's master files
    pub fn iter_masters(&self) -> impl Iterator<Item = &str> {
        self.masters.iter().map(|(n, _)| &n[..])
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
    /// use tesutil::tes3::*;
    /// use tesutil::Plugin;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut buf: Vec<u8> = vec![];
    /// let plugin = Tes3Plugin::load_file("Morrowind.esm")?;
    /// plugin.write(&mut &mut buf)?;
    /// assert_eq!(buf.len(), 79837557);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        let mut header = Tes3Record::new(b"TES3");
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

        header.add_field(Tes3Field::new(b"HEDR", buf).unwrap());

        for (name, size) in &self.masters {
            let mast = Tes3Field::new_zstring(b"MAST", name.clone())?;
            header.add_field(mast);
            header.add_field(Tes3Field::new_u64(b"DATA", *size));
        }

        if let Some(ref save) = self.save {
            let mut game_data = vec![0u8; 0x7c];
            let mut writer = &mut &mut game_data;
            serialize!(save.current_health => writer)?;
            serialize!(save.max_health => writer)?;
            serialize!(save.hour => writer)?;
            writer.write_exact(&save.unknown1)?;
            serialize_str(&save.current_cell, CELL_LENGTH, &mut writer)?;
            writer.write_exact(&save.unknown2)?;
            serialize_str(&save.player_name, NAME_LENGTH, &mut writer)?;

            header.add_field(Tes3Field::new(b"GMDT", game_data).unwrap());
        }

        if !self.screen_data.is_empty() {
            // as far as I can tell, the contents of this field are always the same
            header.add_field(
                Tes3Field::new(
                    b"SCRD",
                    vec![
                        0, 0, 0xff, 0, 0, 0xff, 0, 0, 0xff, 0, 0, 0, 0, 0, 0, 0, 0x20, 0, 0, 0,
                    ],
                )
                .unwrap(),
            );
            header.add_field(Tes3Field::new(b"SCRS", self.screen_data.clone()).unwrap());
        }

        header.write(&mut f)?;

        for record in self.records.iter() {
            record.read().unwrap().write(&mut f)?;
        }

        Ok(())
    }
}

impl Plugin for Tes3Plugin {
    /// Reads a plugin from a file
    ///
    /// Reads a plugin from the file at `path`. The entire plugin is read into memory and retains
    /// no reference to the file once the read is complete.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or if [`Plugin::read`] fails;
    /// refer to that method for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::tes3::*;
    /// use tesutil::Plugin;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let plugin = Tes3Plugin::load_file("Morrowind.esm")?;
    /// assert!(plugin.is_master());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [`Plugin::read`]: #method.read
    fn load_file<P: AsRef<Path>>(path: P) -> Result<Tes3Plugin, TesError> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        Tes3Plugin::read(reader)
    }

    /// Returns whether this plugin is a master on which other plugins can depend
    fn is_master(&self) -> bool {
        self.is_master
    }

    /// Set whether this plugin is a master
    fn set_is_master(&mut self, is_master: bool) {
        self.is_master = is_master;
    }

    /// Save a plugin to a file
    ///
    /// The file must not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or if [`Plugin::write`] fails;
    /// refer to that method for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tesutil::tes3::*;
    /// use tesutil::Plugin;
    /// use tesutil::TesError;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut plugin = Tes3Plugin::new(String::from("test"), String::from("sample plugin"))?;
    /// plugin.set_is_master(true);
    /// plugin.save_file("sample.esm")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Plugin::write`]: #method.write
    fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<(), TesError> {
        let f = File::create(path)?;
        let writer = BufWriter::new(f);
        self.write(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_PLUGIN: &[u8] = include_bytes!("plugin/test/multipatch.esp");
    static EXPECTED_PLUGIN: &[u8] = include_bytes!("plugin/test/expected.esp");

    #[test]
    fn read_plugin() {
        let cursor = io::Cursor::new(TEST_PLUGIN);
        let plugin = Tes3Plugin::read(cursor).unwrap();
        assert_eq!(plugin.version, VERSION_1_3);
        assert!(!plugin.is_master);
        assert_eq!(plugin.author, "tes3cmd multipatch");
        assert_eq!(
            plugin.description,
            "options: cellnames,fogbug,merge_lists,summons_persist"
        );
        assert_eq!(plugin.masters.len(), 0);
        assert_eq!(plugin.records.len(), 8);
    }

    #[test]
    fn write_plugin() {
        let buf: Vec<u8> = Vec::with_capacity(EXPECTED_PLUGIN.len());
        let mut plugin = Tes3Plugin::new(
            String::from("test"),
            String::from("This is an empty test plugin"),
        )
        .unwrap();
        plugin.is_master = true;
        plugin
            .add_master(String::from("Morrowind.esm"), 79837557)
            .unwrap();

        let mut test_record = Tes3Record::new(b"GMST");
        test_record
            .add_field(Tes3Field::new_string(b"NAME", String::from("iDispKilling")).unwrap());
        test_record.add_field(Tes3Field::new_i32(b"INTV", -50));
        plugin.add_record(test_record).unwrap();

        let mut cursor = io::Cursor::new(buf);
        plugin.write(&mut cursor).unwrap();

        assert_eq!(cursor.into_inner(), EXPECTED_PLUGIN);
    }

    #[test]
    fn fetch_record() {
        let cursor = io::Cursor::new(TEST_PLUGIN);
        let plugin = Tes3Plugin::read(cursor).unwrap();
        let record = plugin.get_record("BM_wolf_grey_summon").unwrap().unwrap();
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
