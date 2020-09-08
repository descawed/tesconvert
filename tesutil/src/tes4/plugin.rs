use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::rc::{Rc, Weak};

use crate::plugin::FieldInterface;
use crate::*;

mod field;
pub use field::*;

mod group;
pub use group::*;

mod record;
pub use record::*;

/// Maximum number of masters that a plugin can have
// - 2 because index FF is reserved for saves, and we also need at least one index for ourselves
pub const MAX_MASTERS: usize = u8::MAX as usize - 2;

/// A unique identifier for a record
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct FormId(pub u32);

impl FormId {
    /// Gets a form ID's index (i.e., which plugin in the load order it belongs to)
    pub fn index(&self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// Sets a form ID's index
    pub fn set_index(&mut self, index: u8) {
        self.0 = ((index as u32) << 24) | (self.0 & 0xffffff);
    }
}

/// Represents a plugin file
///
/// This type can be used to read and write plugin (mod) files. These are .esm files (masters, on
/// which other plugins may depend) and .esp files (regular plugin files). Note that the
/// `is_master` flag determines whether a plugin is a master, not the file extension; using .esm for
/// masters and .esp for non-masters is merely convention.
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
    next_form_id: u32,
    author: Option<String>,
    description: Option<String>,
    // these two fields are optional and only seen in Oblivion.esm; we store them so we can write
    // them back out, but we don't do anything with them
    offsets: Option<Field>,
    deleted: Option<Field>,
    masters: Vec<String>,
    groups: HashMap<[u8; 4], Group>,
    id_map: HashMap<FormId, Rc<RefCell<Record>>>,
}

/// Version value for Oblivion plugins
pub const VERSION: f32 = 1.;

impl Plugin {
    pub fn new(author: Option<String>, description: Option<String>) -> Plugin {
        Plugin {
            version: VERSION,
            is_master: false,
            next_form_id: 1,
            author,
            description,
            offsets: None,
            deleted: None,
            masters: vec![],
            groups: HashMap::new(),
            id_map: HashMap::new(),
        }
    }

    /// Reads a plugin from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs or if the plugin data is invalid.
    pub fn read<T: Read + Seek>(mut f: T) -> Result<Plugin, TesError> {
        let (record, _) = Record::read(&mut f)?;
        if record.name() != b"TES4" {
            return Err(decode_failed("Not a valid TES4 plugin file"));
        }

        let mut plugin = Plugin::new(None, None);
        plugin.is_master = record.is_master();

        for field in record.into_iter() {
            match field.name() {
                b"HEDR" => {
                    let data = field.consume();
                    let reader: &mut &[u8] = &mut data.as_ref();
                    plugin.version = extract!(reader as f32)?;
                    extract!(reader as u32)?; // number of records and groups; not needed
                    plugin.next_form_id = extract!(reader as u32)?;
                }
                b"OFST" => plugin.offsets = Some(field),
                b"DELE" => plugin.deleted = Some(field),
                b"CNAM" => plugin.author = Some(String::from(field.get_zstring()?)),
                b"SNAM" => plugin.description = Some(String::from(field.get_zstring()?)),
                b"MAST" => plugin.masters.push(String::from(field.get_zstring()?)),
                b"DATA" => (),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.display_name()
                    )))
                }
            }
        }

        let num_masters = plugin.masters.len();
        if num_masters > MAX_MASTERS {
            return Err(decode_failed(format!(
                "Too many masters: expected at most {}, found {}",
                MAX_MASTERS, num_masters
            )));
        }
        // + 1 for our own index
        let max_index = num_masters as u8 + 1;

        let mut here = f.seek(SeekFrom::Current(0))?;
        let eof = f.seek(SeekFrom::End(0))?;
        f.seek(SeekFrom::Start(here))?;

        while here != eof {
            let (group, _) = Group::read(&mut f)?;
            for record in group.iter_rc() {
                let id = record.borrow().id();
                let index = id.index();
                if index > max_index {
                    return Err(decode_failed(format!(
                        "Expected maximum index {:02X} but found index {:02X}",
                        max_index, index
                    )));
                }
                plugin.id_map.insert(id, record);
            }

            if let GroupKind::Top(label) = group.kind() {
                plugin.groups.insert(label, group);
            } else {
                return Err(decode_failed("Found non-top-level group at top level"));
            }

            here = f.seek(SeekFrom::Current(0))?;
        }

        Ok(plugin)
    }

    /// Gets a record by form ID
    pub fn get_record(&self, form_id: FormId) -> Option<Ref<Record>> {
        self.id_map.get(&form_id).map(|r| r.borrow())
    }

    /// Gets a record by master and form ID
    ///
    /// This allows you to get a record without knowing the exact index that the master is loaded at.
    pub fn get_record_by_master(&self, master: &str, mut form_id: FormId) -> Option<Ref<Record>> {
        let index = self.masters.iter().position(|m| m == master)?;
        form_id.set_index(index as u8);
        self.get_record(form_id)
    }

    /// Iterate over this plugin's records with the corresponding master
    pub fn iter_records_with_masters(&self) -> impl Iterator<Item = (&str, Weak<RefCell<Record>>)> {
        // the move is necessary to take ownership of the reference to self, which would otherwise
        // be dropped at the end of the function
        self.id_map.iter().map(move |(form_id, record)| {
            let index = form_id.index() as usize;
            (&self.masters[index][..], Rc::downgrade(record))
        })
    }
}
