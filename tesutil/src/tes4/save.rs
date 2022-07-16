use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::tes4::{FormId, Tes4Field, Tes4Record};
use crate::*;

mod change;
pub use change::*;

mod form;
pub use form::*;

mod actor;
pub use actor::*;

mod actorref;
pub use actorref::*;

/// Form ID of the player's base record
pub const FORM_PLAYER: FormId = FormId(7);
/// Form ID of the player's reference
pub const FORM_PLAYER_REF: FormId = FormId(0x14);
/// Form ID of the player's custom class
pub const FORM_PLAYER_CUSTOM_CLASS: FormId = FormId(0x00022843);

/// An Oblivion save game
///
/// Unlike Morrowind, Oblivion saves use a completely different format than plugins.
#[derive(Debug)]
pub struct Save {
    version: (u8, u8),
    exe_time: [u8; 16],
    header_version: u32,
    save_number: u32,
    player_name: String,
    player_level: u16,
    player_location: String,
    game_days: f32,
    game_ticks: u32,
    game_time: [u8; 16],
    screen_width: u32,
    screen_height: u32,
    screen_data: Vec<u8>,
    plugins: Vec<String>,
    next_form_id: u32,
    world_id: u32,
    world_x: u32,
    world_y: u32,
    player_cell: u32,
    player_x: f32,
    player_y: f32,
    player_z: f32,
    globals: Vec<(u32, f32)>,
    deaths: Vec<(u32, u16)>,
    game_seconds: f32,
    processes_data: Vec<u8>,
    spec_event_data: Vec<u8>,
    weather_data: Vec<u8>,
    player_combat_count: u32,
    created_ids: Vec<FormId>,
    created_records: HashMap<FormId, RwLock<Tes4Record>>,
    quick_keys: Vec<Option<u32>>,
    reticle_data: Vec<u8>,
    interface_data: Vec<u8>,
    region_data: Vec<u8>,
    change_ids: Vec<FormId>,
    change_records: HashMap<FormId, ChangeRecord>,
    temporary_effects: Vec<u8>,
    form_ids: Vec<FormId>,
    world_spaces: Vec<u32>,
}

// creating a new save from scratch isn't currently supported, so no need for this
// pub const VERSION: (u8, u8) = (0, 125);

impl Save {
    /// Read a save file from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs or if the save format is invalid.
    pub fn read<T: Read + Seek>(mut f: T) -> Result<Save, TesError> {
        let mut magic = [0u8; 12];
        f.read_exact(&mut magic)?;
        if magic != *b"TES4SAVEGAME" {
            return Err(decode_failed("Not a valid Oblivion save"));
        }

        let version = (f.read_le()?, f.read_le()?);
        let mut exe_time = [0u8; 16];
        f.read_exact(&mut exe_time)?;

        let header_version = f.read_le()?;
        f.seek(SeekFrom::Current(4))?; // save header size, but I don't think we need this?
        let save_number = f.read_le()?;
        let player_name = read_bzstring(&mut f)?;
        let player_level = f.read_le()?;
        let player_location = read_bzstring(&mut f)?;
        let game_days = f.read_le()?;
        let game_ticks = f.read_le()?;

        let mut game_time = [0u8; 16];
        f.read_exact(&mut game_time)?;

        let screen_size = f.read_le::<u32>()? as usize;
        let screen_width = f.read_le()?;
        let screen_height = f.read_le()?;
        // - 8 because we already read the width and height
        let mut screen_data = vec![0u8; screen_size - 8];
        f.read_exact(&mut screen_data)?;

        let num_plugins = f.read_le::<u8>()? as usize;
        let mut plugins = Vec::with_capacity(num_plugins);
        for _ in 0..num_plugins {
            plugins.push(read_bstring(&mut f)?);
        }

        f.seek(SeekFrom::Current(4))?; // form IDs offset; don't need it
        let num_change_records = f.read_le::<u32>()? as usize;
        let next_form_id = f.read_le()?;
        let world_id = f.read_le()?;
        let world_x = f.read_le()?;
        let world_y = f.read_le()?;
        let player_cell = f.read_le()?;
        let player_x = f.read_le()?;
        let player_y = f.read_le()?;
        let player_z = f.read_le()?;

        let num_globals = f.read_le::<u16>()? as usize;
        let mut globals = Vec::with_capacity(num_globals);
        for _ in 0..num_globals {
            let iref = f.read_le()?;
            let value = f.read_le()?;
            globals.push((iref, value));
        }

        f.seek(SeekFrom::Current(2))?; // another size we don't need
        let num_deaths = f.read_le::<u32>()? as usize;
        let mut deaths = Vec::with_capacity(num_deaths);
        for _ in 0..num_deaths {
            let actor = f.read_le()?;
            let count = f.read_le()?;
            deaths.push((actor, count));
        }

        let game_seconds = f.read_le()?;

        let processes_size = f.read_le::<u16>()? as usize;
        let mut processes_data = vec![0u8; processes_size];
        f.read_exact(&mut processes_data)?;

        let spec_event_size = f.read_le::<u16>()? as usize;
        let mut spec_event_data = vec![0u8; spec_event_size];
        f.read_exact(&mut spec_event_data)?;

        let weather_size = f.read_le::<u16>()? as usize;
        let mut weather_data = vec![0u8; weather_size];
        f.read_exact(&mut weather_data)?;

        let player_combat_count = f.read_le()?;

        let num_created = f.read_le::<u32>()? as usize;
        let mut created_ids = Vec::with_capacity(num_created);
        let mut created_records = HashMap::with_capacity(num_created);
        for _ in 0..num_created {
            let record = Tes4Record::read(&mut f)?;
            let form_id = record.id();
            created_ids.push(form_id);
            created_records.insert(form_id, RwLock::new(record));
        }

        let quick_keys_size = f.read_le::<u16>()? as usize;
        let mut quick_keys_data = vec![0u8; quick_keys_size];
        f.read_exact(&mut quick_keys_data)?;
        let mut i = 0;
        let mut quick_keys = vec![];
        while i < quick_keys_size {
            let has_quick_key = quick_keys_data[i] == 1;
            i += 1;
            if has_quick_key {
                if i + 4 <= quick_keys_size {
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(&quick_keys_data[i..i + 4]);
                    quick_keys.push(Some(u32::from_le_bytes(buf)));
                    i += 4;
                } else {
                    return Err(TesError::DecodeFailed {
                        description: format!("Invalid quick key data at index {}", i),
                        source: None,
                    });
                }
            } else {
                quick_keys.push(None);
            }
        }

        let reticle_size = f.read_le::<u16>()? as usize;
        let mut reticle_data = vec![0u8; reticle_size];
        f.read_exact(&mut reticle_data)?;

        let interface_size = f.read_le::<u16>()? as usize;
        let mut interface_data = vec![0u8; interface_size];
        f.read_exact(&mut interface_data)?;

        let region_size = f.read_le::<u16>()? as usize;
        let mut region_data = vec![0u8; region_size];
        f.read_exact(&mut region_data)?;

        let mut change_ids = Vec::with_capacity(num_change_records);
        let mut change_records = HashMap::with_capacity(num_change_records);
        for _ in 0..num_change_records {
            let record = ChangeRecord::read(&mut f)?;
            let form_id = record.form_id();
            change_ids.push(form_id);
            change_records.insert(form_id, record);
        }

        let temp_effects_size = f.read_le::<u32>()? as usize;
        let mut temporary_effects = vec![0u8; temp_effects_size];
        f.read_exact(&mut temporary_effects)?;

        let num_form_ids = f.read_le::<u32>()? as usize;
        let mut form_ids = Vec::with_capacity(num_form_ids);
        for _ in 0..num_form_ids {
            form_ids.push(FormId(f.read_le()?));
        }

        let num_world_spaces = f.read_le::<u32>()? as usize;
        let mut world_spaces = Vec::with_capacity(num_world_spaces);
        for _ in 0..num_world_spaces {
            world_spaces.push(f.read_le()?);
        }

        Ok(Save {
            version,
            exe_time,
            header_version,
            save_number,
            player_name,
            player_level,
            player_location,
            game_days,
            game_ticks,
            game_time,
            screen_width,
            screen_height,
            screen_data,
            plugins,
            next_form_id,
            world_id,
            world_x,
            world_y,
            player_cell,
            player_x,
            player_y,
            player_z,
            globals,
            deaths,
            game_seconds,
            processes_data,
            spec_event_data,
            weather_data,
            player_combat_count,
            created_ids,
            created_records,
            quick_keys,
            reticle_data,
            interface_data,
            region_data,
            change_ids,
            change_records,
            temporary_effects,
            form_ids,
            world_spaces,
        })
    }

    /// Load a save file
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be found or if [`Save::read`] fails.
    ///
    /// [`Save::read`]: #method.read
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Save, TesError> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        Save::read(reader)
    }

    /// Gets the player's name
    pub fn player_name(&self) -> &str {
        &self.player_name
    }

    /// Sets the player's name
    ///
    /// # Errors
    ///
    /// Fails if the player's name is longer than [`MAX_BSTRING`].
    ///
    /// [`MAX_BSTRING`]: constant.MAX_BSTRING.html
    pub fn set_player_name(&mut self, name: String) -> Result<(), TesError> {
        check_size(&name, MAX_BSTRING, "Player name too long")?;
        self.player_name = name;
        Ok(())
    }

    /// Gets a change record by form ID
    ///
    /// Returns `None` if no change record exists for the given form ID.
    pub fn get_change_record(&self, form_id: FormId) -> Option<&ChangeRecord> {
        self.change_records.get(&form_id)
    }

    /// Gets a change record by form ID, mutably
    ///
    /// Returns `None` if no change record exists for the given form ID.
    pub fn get_change_record_mut(&mut self, form_id: FormId) -> Option<&mut ChangeRecord> {
        self.change_records.get_mut(&form_id)
    }

    /// Gets a form change by form ID
    pub fn get_form_change<T: FormChange>(&self, form_id: FormId) -> Result<Option<T>, TesError> {
        Ok(match self.get_change_record(form_id) {
            Some(record) => Some(T::read(record)?),
            None => None,
        })
    }

    /// Updates the save from a form with a given form ID
    pub fn update_form_change<T: FormChange>(
        &mut self,
        form: &T,
        form_id: FormId,
    ) -> Result<(), TesError> {
        form.write(
            &mut *self
                .get_change_record_mut(form_id)
                .ok_or(TesError::InvalidFormId { form_id })?,
        )
    }

    /// Gets a created record by form ID
    ///
    /// Returns `None` if no created record exists for the given form ID.
    pub fn get_record(&self, form_id: FormId) -> Option<RwLockReadGuard<Tes4Record>> {
        self.created_records
            .get(&form_id)
            .map(|r| r.read().unwrap())
    }

    /// Gets a created record by form ID, mutably
    ///
    /// Returns `None` if no created record exists for the given form ID.
    pub fn get_record_mut(&self, form_id: FormId) -> Option<RwLockWriteGuard<Tes4Record>> {
        self.created_records
            .get(&form_id)
            .map(|r| r.write().unwrap())
    }

    /// Adds a created record
    ///
    /// The form ID present on the record will be ignored and a new one will be generated. Returns
    /// the iref of the new form ID.
    pub fn add_record(&mut self, mut record: Tes4Record) -> u32 {
        // FIXME: handle next form ID wrapping around or being in use
        let form_id = FormId(self.next_form_id);
        self.next_form_id += 1;
        record.set_id(form_id);

        self.created_records.insert(form_id, RwLock::new(record));
        self.created_ids.push(form_id);

        let new_iref = self.form_ids.len() as u32;
        self.form_ids.push(form_id);
        new_iref
    }

    /// Gets a created form by form ID, mutably
    pub fn get_form<T>(&self, form_id: FormId) -> Result<Option<T>, TesError>
    where
        T: Form<Field = Tes4Field, Record = Tes4Record>,
    {
        Ok(match self.get_record(form_id) {
            Some(record) => Some(T::read(&*record)?),
            None => None,
        })
    }

    /// Updates the save from a form with a given form ID
    pub fn update_form<T>(&mut self, form: &T, form_id: FormId) -> Result<(), TesError>
    where
        T: Form<Field = Tes4Field, Record = Tes4Record>,
    {
        form.write(
            &mut *self
                .get_record_mut(form_id)
                .ok_or(TesError::InvalidFormId { form_id })?,
        )
    }

    /// Adds a form to the save
    ///
    /// Returns the iref of the new record.
    pub fn add_form<T>(&mut self, form: &T) -> Result<u32, TesError>
    where
        T: Form<Field = Tes4Field, Record = Tes4Record>,
    {
        let mut record = Tes4Record::new(T::record_type());
        form.write(&mut record)?;
        Ok(self.add_record(record))
    }

    /// Gets the form ID for an iref, if one exists
    ///
    /// Returns `None` if there is no form ID for the given iref
    pub fn iref_to_form_id(&self, iref: u32) -> Option<FormId> {
        if iref > 0xff000000 {
            Some(FormId(iref))
        } else {
            self.form_ids.get(iref as usize).copied()
        }
    }

    /// Gets the iref of a form ID, if one exists
    pub fn form_id_to_iref(&self, form_id: FormId) -> Option<u32> {
        // if this becomes a bottleneck, make a reverse mapping of form IDs to irefs
        self.form_ids
            .iter()
            .position(|f| *f == form_id)
            .map(|i| i as u32)
    }

    /// Inserts a form ID if it does not already exist, returning its iref
    pub fn insert_form_id(&mut self, form_id: FormId) -> u32 {
        match self.form_id_to_iref(form_id) {
            Some(iref) => iref,
            None => {
                let iref = self.form_ids.len() as u32;
                self.form_ids.push(form_id);
                iref
            }
        }
    }

    /// Iterates over this save's plugins
    pub fn iter_plugins(&self) -> impl Iterator<Item = &str> {
        self.plugins.iter().map(|s| s.as_str())
    }

    /// Write a save to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs.
    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_all(b"TES4SAVEGAME")?;
        f.write_all(&[self.version.0, self.version.1])?;
        f.write_all(&self.exe_time)?;

        f.write_le(&self.header_version)?;
        // header size = screenshot size + hard-coded fields + name and location bzstrings
        let header_size =
            self.screen_data.len() + 46 + self.player_name.len() + self.player_location.len();
        f.write_le(&(header_size as u32))?;
        f.write_le(&self.save_number)?;
        write_bzstring(&mut f, &self.player_name)?;
        f.write_le(&self.player_level)?;
        write_bzstring(&mut f, &self.player_location)?;
        f.write_le(&self.game_days)?;
        f.write_le(&self.game_ticks)?;
        f.write_all(&self.game_time)?;
        let screen_size = self.screen_data.len() + 8;
        f.write_le(&(screen_size as u32))?;
        f.write_le(&self.screen_width)?;
        f.write_le(&self.screen_height)?;
        f.write_all(&self.screen_data)?;

        f.write_le(&(self.plugins.len() as u8))?;
        for plugin in self.plugins.iter() {
            write_bstring(&mut f, plugin)?;
        }

        // we don't have this value yet, so record the current offset so we can come back later
        let form_id_offset = f.seek(SeekFrom::Current(0))?;
        // write dummy value
        f.write_all(b"\0\0\0\0")?;
        f.write_le(&(self.change_records.len() as u32))?;
        f.write_le(&self.next_form_id)?;
        f.write_le(&self.world_id)?;
        f.write_le(&self.world_x)?;
        f.write_le(&self.world_y)?;
        f.write_le(&self.player_cell)?;
        f.write_le(&self.player_x)?;
        f.write_le(&self.player_y)?;
        f.write_le(&self.player_z)?;

        f.write_le(&(self.globals.len() as u16))?;
        for (iref, value) in self.globals.iter() {
            f.write_le(&iref)?;
            f.write_le(&value)?;
        }

        let tes_class_size = self.deaths.len() * 6 + 8;
        f.write_le(&(tes_class_size as u16))?;
        f.write_le(&(self.deaths.len() as u32))?;
        for (actor, count) in self.deaths.iter() {
            f.write_le(&actor)?;
            f.write_le(&count)?;
        }

        f.write_le(&self.game_seconds)?;

        f.write_le(&(self.processes_data.len() as u16))?;
        f.write_all(&self.processes_data)?;

        f.write_le(&(self.spec_event_data.len() as u16))?;
        f.write_all(&self.spec_event_data)?;

        f.write_le(&(self.weather_data.len() as u16))?;
        f.write_all(&self.weather_data)?;

        f.write_le(&self.player_combat_count)?;

        f.write_le(&(self.created_records.len() as u32))?;
        for form_id in &self.created_ids {
            if let Some(created_record) = self.created_records.get(form_id) {
                created_record.read().unwrap().write(&mut f)?;
            }
        }

        f.write_le(&0u16)?;
        let start = f.seek(SeekFrom::Current(0))?;
        for quick_key in self.quick_keys.iter() {
            if let Some(setting) = quick_key {
                f.write_le(&1u8)?;
                f.write_le(&setting)?;
            } else {
                f.write_le(&0u8)?;
            }
        }
        // calculate the number of bytes we just wrote and update the size at the beginning
        let end = f.seek(SeekFrom::Current(0))?;
        f.seek(SeekFrom::Start(start - 2))?;
        f.write_le(&((end - start) as u16))?;
        f.seek(SeekFrom::Start(end))?;

        f.write_le(&(self.reticle_data.len() as u16))?;
        f.write_all(&self.reticle_data)?;

        f.write_le(&(self.interface_data.len() as u16))?;
        f.write_all(&self.interface_data)?;

        f.write_le(&(self.region_data.len() as u16))?;
        f.write_all(&self.region_data)?;

        for id in &self.change_ids {
            if let Some(change_record) = self.change_records.get(id) {
                change_record.write(&mut f)?;
            }
        }

        f.write_le(&(self.temporary_effects.len() as u32))?;
        f.write_all(&self.temporary_effects)?;

        // now go back and fill in the form ID offset
        let current_pos = f.seek(SeekFrom::Current(0))?;
        f.seek(SeekFrom::Start(form_id_offset))?;
        f.write_le(&(current_pos as u32))?;
        f.seek(SeekFrom::Start(current_pos))?;

        f.write_le(&(self.form_ids.len() as u32))?;
        for form_id in self.form_ids.iter() {
            f.write_le(&form_id.0)?;
        }

        f.write_le(&(self.world_spaces.len() as u32))?;
        for world_space in self.world_spaces.iter() {
            f.write_le(&world_space)?;
        }

        Ok(())
    }

    /// Write a save to a file
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be created or if an I/O error occurs.
    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<(), TesError> {
        let f = File::create(path)?;
        let writer = BufWriter::new(f);
        self.write(writer)
    }
}

#[cfg(test)]
static TEST_SAVE: &[u8] = include_bytes!("save/test/quicksave.ess");

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_save() {
        let mut record_ref = TEST_SAVE.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let save = Save::read(cursor).unwrap();
        assert_eq!(save.player_name, "test");
        assert_eq!(save.player_location, "Vilverin Canosel");
        assert_eq!(save.plugins.len(), 11);
    }

    #[test]
    fn set_name() {
        let mut record_ref = TEST_SAVE.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let mut save = Save::read(cursor).unwrap();
        save.set_player_name(String::from("short name")).unwrap();
        save.set_player_name(
            String::from("name that is too long oh nooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo")
        ).unwrap_err();
    }

    #[test]
    fn write_save() {
        let mut record_ref = TEST_SAVE.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let save = Save::read(cursor).unwrap();
        let mut buf = vec![0u8; TEST_SAVE.len()];
        let cursor = Cursor::new(&mut buf);
        save.write(cursor).unwrap();
        assert_eq!(TEST_SAVE, buf.as_slice());
    }
}
