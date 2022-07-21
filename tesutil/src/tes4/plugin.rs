use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{FindForm, FormId};
use crate::*;

use binrw::BinReaderExt;

mod field;
pub use field::*;

mod group;
pub use group::*;

mod record;
pub use record::*;

mod class;
pub use class::*;

mod spell;
pub use spell::*;

mod magic_effect;
pub use magic_effect::*;

mod birthsign;
pub use birthsign::*;

mod race;
pub use race::*;

mod npc;
pub use npc::*;

mod item;
pub use item::*;

mod potion;
pub use potion::*;

mod soul_gem;
pub use soul_gem::*;

mod magic;
pub use magic::*;

mod enchantable;
pub use enchantable::*;

mod enchantment;
pub use enchantment::*;

mod weapon;
pub use weapon::*;

mod ammo;
pub use ammo::*;

mod book;
pub use book::*;

/// Maximum number of masters that a plugin can have
// - 2 because index FF is reserved for saves, and we also need at least one index for ourselves
pub const MAX_MASTERS: usize = u8::MAX as usize - 2;

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
pub struct Tes4Plugin {
    version: f32,
    is_master: bool,
    next_form_id: u32,
    author: Option<String>,
    description: Option<String>,
    // these two fields are optional and only seen in Oblivion.esm; we store them so we can write
    // them back out, but we don't do anything with them
    offsets: Option<Tes4Field>,
    deleted: Option<Tes4Field>,
    masters: Vec<(String, String)>,
    groups: HashMap<[u8; 4], Group>,
    id_map: HashMap<FormId, Arc<RwLock<Tes4Record>>>,
    settings: HashMap<String, Arc<RwLock<Tes4Record>>>,
    magic_effects: HashMap<MagicEffectType, Arc<RwLock<Tes4Record>>>,
}

/// Version value for Oblivion plugins
pub const VERSION: f32 = 1.;

/// Order that top-level groups should be written in
const GROUP_ORDER: [&'static [u8; 4]; 56] = [
    b"GMST", b"GLOB", b"CLAS", b"FACT", b"HAIR", b"EYES", b"RACE", b"SOUN", b"SKIL", b"MGEF",
    b"SCPT", b"LTEX", b"ENCH", b"SPEL", b"BSGN", b"ACTI", b"APPA", b"ARMO", b"BOOK", b"CLOT",
    b"CONT", b"DOOR", b"INGR", b"LIGH", b"MISC", b"STAT", b"GRAS", b"TREE", b"FLOR", b"FURN",
    b"WEAP", b"AMMO", b"NPC_", b"CREA", b"LVLC", b"SLGM", b"KEYM", b"ALCH", b"SBSP", b"SGST",
    b"LVLI", b"WTHR", b"CLMT", b"REGN", b"CELL", b"WRLD", b"DIAL", b"QUST", b"IDLE", b"PACK",
    b"CSTY", b"LSCR", b"LVSP", b"ANIO", b"WATR", b"EFSH",
];

impl Tes4Plugin {
    pub fn new(author: Option<String>, description: Option<String>) -> Tes4Plugin {
        Tes4Plugin {
            version: VERSION,
            is_master: false,
            next_form_id: 0x800, // not sure what the significance of this number is, but this is what TES4Edit uses
            author,
            description,
            offsets: None,
            deleted: None,
            masters: vec![],
            groups: HashMap::new(),
            id_map: HashMap::new(),
            settings: HashMap::new(),
            magic_effects: HashMap::new(),
        }
    }

    fn add_group_record(&mut self, record: Arc<RwLock<Tes4Record>>) -> Result<(), TesError> {
        let num_masters = self.masters.len();
        // + 1 for our own index
        let max_index = num_masters as u8 + 1;

        let rb = record.read().unwrap();
        let id = rb.id();
        let record_type = *rb.name();
        let index = id.index();
        if index > max_index {
            return Err(decode_failed(format!(
                "Expected maximum index {:02X} but found index {:02X}",
                max_index, index
            )));
        }

        drop(rb);

        // GMSTs and MGEFs don't have fixed form IDs, so we have to track them separately
        if record_type == *b"GMST" {
            let mut rbm = record.write().unwrap();
            rbm.finalize()?;
            for field in rbm.iter() {
                if field.name() == b"EDID" {
                    let key = String::from(field.get_zstring()?);
                    self.settings.insert(key, Arc::clone(&record));
                    break;
                }
            }
        } else if record_type == *b"MGEF" {
            let mut rbm = record.write().unwrap();
            rbm.finalize()?;
            for field in rbm.iter() {
                if field.name() == b"EDID" {
                    // strip the trailing \0
                    let mgef_id = field.get_zstring()?.as_bytes();
                    let key = MagicEffectType::from_id(mgef_id).ok_or_else(|| {
                        TesError::RequirementFailed(format!(
                            "Invalid magic effect type with form ID {:08X}",
                            id.0
                        ))
                    })?;
                    self.magic_effects.insert(key, Arc::clone(&record));
                    break;
                }
            }
        }

        self.id_map.insert(id, record);

        Ok(())
    }

    pub fn add_record(&mut self, record: Tes4Record) -> Result<(), TesError> {
        let record_type = *record.name();
        let group = self
            .groups
            .entry(record_type)
            .or_insert_with(|| Group::new(GroupKind::Top(record_type)));
        let group_record = group.add_record(record)?;
        self.add_group_record(group_record)
    }

    /// Adds a record to this plugin, assigning it the next available form ID
    pub fn add_new_record(&mut self, mut record: Tes4Record) -> Result<FormId, TesError> {
        let form_id = self.get_next_form_id();
        record.set_id(form_id);
        self.add_record(record).map(|_| form_id)
    }

    /// Returns and increments the next available form ID
    pub fn get_next_form_id(&mut self) -> FormId {
        let mut id = FormId(self.next_form_id);
        id.set_index(self.masters.len() as u8);
        self.next_form_id += 1;
        id
    }

    /// Gets a record by form ID
    pub fn get_record(&self, search: &FindForm) -> Option<RwLockReadGuard<Tes4Record>> {
        self.id_map
            .get(&search.form_id(self.masters.iter().map(|(s, _)| s.as_str()))?)
            .and_then(|r| {
                if r.read().unwrap().status() == RecordStatus::Initialized {
                    // FIXME: for now we're just suppressing records that fail to finalize. we
                    //  really ought to return this result to the caller, but I don't feel like
                    //  updating the plugin interface right now.
                    let _ = r.write().unwrap().finalize();
                }

                let rb = r.read().unwrap();
                match rb.status() {
                    RecordStatus::Failed => None,
                    _ => Some(rb),
                }
            })
    }

    /// Gets a record by form ID
    pub fn get_record_mut(&self, search: &FindForm) -> Option<RwLockWriteGuard<Tes4Record>> {
        self.id_map
            .get(&search.form_id(self.masters.iter().map(|(s, _)| s.as_str()))?)
            .and_then(|r| {
                let mut rbm = r.write().unwrap();
                if rbm.status() == RecordStatus::Initialized {
                    // FIXME: for now we're just suppressing records that fail to finalize. we
                    //  really ought to return this result to the caller, but I don't feel like
                    //  updating the plugin interface right now.
                    let _ = rbm.finalize();
                }

                match rbm.status() {
                    RecordStatus::Failed => None,
                    _ => Some(rbm),
                }
            })
    }

    /// Gets a magic effect by magic effect type
    pub fn get_magic_effect(
        &self,
        effect_type: MagicEffectType,
    ) -> Result<Option<MagicEffect>, TesError> {
        if let Some(record) = self.magic_effects.get(&effect_type) {
            let rb = record.read().unwrap();
            Ok(Some(MagicEffect::read(&*rb)?))
        } else {
            Ok(None)
        }
    }

    /// Gets a form by form ID
    pub fn get<T>(&self, search: &FindForm) -> Result<Option<T>, TesError>
    where
        T: Form<Field = Tes4Field, Record = Tes4Record>,
    {
        Ok(match self.get_record(search) {
            Some(record) => Some(T::read(&*record)?),
            None => None,
        })
    }

    /// Gets a game setting as a float by name
    pub fn get_float_setting(&self, name: &str) -> Result<Option<f32>, TesError> {
        // GMST records are eagerly finalized, so we don't need to worry about their status here
        if let Some(record) = self.settings.get(name) {
            for field in record.read().unwrap().iter() {
                if field.name() == b"DATA" {
                    return Ok(Some(field.get_f32()?));
                }
            }

            Err(decode_failed(format!("Game setting {} has no data", name)))
        } else {
            Ok(None)
        }
    }

    /// Updates the plugin from a form with a given form ID
    pub fn update<T>(&mut self, form: &T, search: &FindForm) -> Result<(), TesError>
    where
        T: Form<Field = Tes4Field, Record = Tes4Record>,
    {
        form.write(&mut *self.get_record_mut(search).ok_or_else(|| search.err())?)
    }

    pub fn set_masters(&mut self, masters: Vec<String>) -> Result<(), TesError> {
        if masters.len() > MAX_MASTERS {
            Err(TesError::LimitExceeded {
                description: String::from("Too many masters"),
                max_size: MAX_MASTERS,
                actual_size: masters.len(),
            })
        } else {
            self.masters = masters
                .into_iter()
                .map(|master| {
                    let master_lc = master.to_lowercase();
                    (master, master_lc)
                })
                .collect();
            Ok(())
        }
    }

    pub fn add_master(&mut self, master: String) -> Result<(), TesError> {
        if self.masters.len() >= MAX_MASTERS {
            Err(TesError::LimitExceeded {
                description: String::from("Too many masters"),
                max_size: MAX_MASTERS,
                actual_size: self.masters.len() + 1,
            })
        } else {
            let master_lc = master.to_lowercase();
            self.masters.push((master, master_lc));
            Ok(())
        }
    }
}

impl Plugin for Tes4Plugin {
    /// Reads a plugin from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs or if the plugin data is invalid.
    fn read<T: Read + Seek>(mut f: T) -> Result<Tes4Plugin, TesError> {
        let record = Tes4Record::read(&mut f)?;
        if record.name() != b"TES4" {
            return Err(decode_failed("Not a valid TES4 plugin file"));
        }

        let mut plugin = Tes4Plugin::new(None, None);
        plugin.is_master = record.is_master();

        for field in record.into_iter() {
            match field.name() {
                b"HEDR" => {
                    let data = field.consume();
                    let mut reader = Cursor::new(&data);
                    plugin.version = reader.read_le()?;
                    reader.seek(SeekFrom::Current(4))?; // number of records and groups; not needed
                    plugin.next_form_id = reader.read_le()?;
                }
                b"OFST" => plugin.offsets = Some(field),
                b"DELE" => plugin.deleted = Some(field),
                b"CNAM" => plugin.author = Some(String::from(field.get_zstring()?)),
                b"SNAM" => plugin.description = Some(String::from(field.get_zstring()?)),
                b"MAST" => {
                    // we keep the original case so we can write the plugin back unmodified, but we
                    // also keep a lowercase copy for matching
                    let master = String::from(field.get_zstring()?);
                    let lc_master = master.to_lowercase();
                    plugin.masters.push((master, lc_master));
                }
                b"DATA" => (),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.name_as_str()
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

        let mut here = f.seek(SeekFrom::Current(0))?;
        let eof = f.seek(SeekFrom::End(0))?;
        f.seek(SeekFrom::Start(here))?;

        while here != eof {
            let group = Group::read(&mut f)?;
            for record in group.iter_rc() {
                plugin.add_group_record(record)?;
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

    /// Returns whether this plugin is a master on which other plugins can depend
    fn is_master(&self) -> bool {
        self.is_master
    }

    /// Set whether this plugin is a master
    fn set_is_master(&mut self, is_master: bool) {
        self.is_master = is_master;
    }

    fn iter_masters(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.masters.iter().map(|(m, _)| m.as_str()))
    }

    fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        let mut header = Tes4Record::new(b"TES4");
        let num_records: usize = self.groups.iter().map(|(_, g)| g.len()).sum();

        let mut buf = vec![];
        let mut cursor = Cursor::new(&mut buf);
        cursor.write_le(&self.version)?;
        cursor.write_le(&(num_records as u32))?;
        cursor.write_le(&self.next_form_id)?;
        header.add_field(Tes4Field::new(b"HEDR", buf)?);

        if let Some(ref offsets) = self.offsets {
            header.add_field(offsets.clone());
        }

        if let Some(ref deleted) = self.deleted {
            header.add_field(deleted.clone());
        }

        if let Some(ref author) = self.author {
            header.add_field(Tes4Field::new_zstring(b"CNAM", author.clone())?);
        }

        if let Some(ref description) = self.description {
            header.add_field(Tes4Field::new_zstring(b"SNAM", description.clone())?);
        }

        for (master, _) in &self.masters {
            header.add_field(Tes4Field::new_zstring(b"MAST", master.clone())?);
            header.add_field(Tes4Field::new_u64(b"DATA", 0));
        }

        header.write(&mut f)?;

        for label in GROUP_ORDER.iter().copied() {
            if let Some(group) = self.groups.get(label) {
                group.write(&mut f)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_PLUGIN: &[u8] = include_bytes!("plugin/test/Data/sample.esp");

    #[test]
    fn load_plugin() {
        let cursor = Cursor::new(TEST_PLUGIN);
        let plugin = Tes4Plugin::read(cursor).unwrap();

        let record = plugin
            .get_record(&FindForm::ByIndex(FormId(0x51a8)))
            .unwrap();
        for field in record.iter() {
            match field.name() {
                b"EDID" => assert_eq!(field.get_zstring().unwrap(), "fPotionT1AleDurMult"),
                b"DATA" => assert_eq!(field.get_f32().unwrap(), -0.01),
                _ => panic!("Unexpected field {}", field.name_as_str()),
            }
        }
        drop(record);

        assert_eq!(plugin.author.unwrap(), "tesutil");
        assert_eq!(
            plugin.description.unwrap(),
            "This is the description for the sample plugin"
        );
        assert_eq!(
            plugin
                .masters
                .into_iter()
                .map(|(m, _)| m)
                .collect::<Vec<String>>(),
            ["Oblivion.esm", "Knights.esp"]
        );
    }
}
