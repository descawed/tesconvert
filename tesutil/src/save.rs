mod change;
pub use change::*;

mod actor;
pub use actor::*;

mod actorref;
pub use actorref::*;

use crate::*;
use crate::plugin::tes4::Record;
use std::io;
use std::io::{Read, Write, BufReader, BufWriter, Seek, SeekFrom};
use std::fs::File;
use std::collections::HashMap;

/// Form ID of the player's base record
pub const FORM_PLAYER: u32 = 7;
/// Form ID of the player's reference
pub const FORM_PLAYER_REF: u32 = 0x14;
/// Form ID of the player's custom class
pub const FORM_PLAYER_CUSTOM_CLASS: u32 = 0x00022843;

/// An Oblivion save game
///
/// Unlike Morrowind, Oblivion saves use a completely different format than plugins.
pub struct Save {
    version: (u8, u8),
    exe_time: [u8; 16], // TODO: implement for real
    header_version: u32,
    save_number: u32,
    player_name: String,
    player_level: u16,
    player_location: String,
    game_days: f32,
    game_ticks: u32,
    game_time: [u8; 16], // TODO: implement for real
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
    created_records: Vec<Record>,
    quick_keys: Vec<Option<u32>>,
    reticle_data: Vec<u8>,
    interface_data: Vec<u8>,
    region_data: Vec<u8>,
    change_ids: Vec<u32>,
    change_records: HashMap<u32, ChangeRecord>,
    temporary_effects: Vec<u8>,
    form_ids: Vec<u32>,
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
    pub fn read<T: Read>(mut f: T) -> io::Result<Save> {
        let mut magic = [0u8; 12];
        f.read_exact(&mut magic)?;
        if magic != *b"TES4SAVEGAME" {
            return Err(io_error("Not a valid Oblivion save"));
        }

        let version = (extract!(f as u8)?, extract!(f as u8)?);
        let mut exe_time = [0u8; 16];
        f.read_exact(&mut exe_time)?;

        let header_version = extract!(f as u32)?;
        extract!(f as u32)?; // save header size, but I don't think we need this?
        let save_number = extract!(f as u32)?;
        let player_name = extract_bzstring(&mut f)?;
        let player_level = extract!(f as u16)?;
        let player_location = extract_bzstring(&mut f)?;
        let game_days = extract!(f as f32)?;
        let game_ticks = extract!(f as u32)?;
        
        let mut game_time = [0u8; 16];
        f.read_exact(&mut game_time)?;
        
        let screen_size = extract!(f as u32)? as usize;
        let screen_width = extract!(f as u32)?;
        let screen_height = extract!(f as u32)?;
        // - 8 because we already read the width and height
        let mut screen_data = vec![0u8; screen_size - 8];
        f.read_exact(&mut screen_data)?;
        
        let num_plugins = extract!(f as u8)? as usize;
        let mut plugins = Vec::with_capacity(num_plugins);
        for _ in 0..num_plugins {
            plugins.push(extract_bstring(&mut f)?);
        }
        
        extract!(f as u32)?; // form IDs offset; don't need it
        let num_change_records = extract!(f as u32)? as usize;
        let next_form_id = extract!(f as u32)?;
        let world_id = extract!(f as u32)?;
        let world_x = extract!(f as u32)?;
        let world_y = extract!(f as u32)?;
        let player_cell = extract!(f as u32)?;
        let player_x = extract!(f as f32)?;
        let player_y = extract!(f as f32)?;
        let player_z = extract!(f as f32)?;
        
        let num_globals = extract!(f as u16)? as usize;
        let mut globals = Vec::with_capacity(num_globals);
        for _ in 0..num_globals {
            let iref = extract!(f as u32)?;
            let value = extract!(f as f32)?;
            globals.push((iref, value));
        }
        
        extract!(f as u16)?; // another size we don't need
        let num_deaths = extract!(f as u32)? as usize;
        let mut deaths = Vec::with_capacity(num_deaths);
        for _ in 0..num_deaths {
            let actor = extract!(f as u32)?;
            let count = extract!(f as u16)?;
            deaths.push((actor, count));
        }
        
        let game_seconds = extract!(f as f32)?;
        
        let processes_size = extract!(f as u16)? as usize;
        let mut processes_data = vec![0u8; processes_size];
        f.read_exact(&mut processes_data)?;

        let spec_event_size = extract!(f as u16)? as usize;
        let mut spec_event_data = vec![0u8; spec_event_size];
        f.read_exact(&mut spec_event_data)?;

        let weather_size = extract!(f as u16)? as usize;
        let mut weather_data = vec![0u8; weather_size];
        f.read_exact(&mut weather_data)?;

        let player_combat_count = extract!(f as u32)?;

        let num_created = extract!(f as u32)? as usize;
        let mut created_records = Vec::with_capacity(num_created);
        for _ in 0..num_created {
            created_records.push(Record::read(&mut f)?.ok_or(io_error("Expected created record"))?.0);
        }

        let quick_keys_size = extract!(f as u16)? as usize;
        let mut quick_keys_data = vec![0u8; quick_keys_size];
        f.read_exact(&mut quick_keys_data)?;
        let mut i = 0;
        let mut quick_keys = vec![];
        while i < quick_keys_size {
            let has_quick_key = quick_keys_data[i] == 1;
            i += 1;
            if has_quick_key {
                if i+4 <= quick_keys_size {
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(&quick_keys_data[i..i+4]);
                    quick_keys.push(Some(u32::from_le_bytes(buf)));
                    i += 4;
                } else {
                    return Err(io_error(format!("Invalid quick key data at index {}", i)));
                }
            } else {
                quick_keys.push(None);
            }
        }

        let reticle_size = extract!(f as u16)? as usize;
        let mut reticle_data = vec![0u8; reticle_size];
        f.read_exact(&mut reticle_data)?;

        let interface_size = extract!(f as u16)? as usize;
        let mut interface_data = vec![0u8; interface_size];
        f.read_exact(&mut interface_data)?;

        let region_size = extract!(f as u16)? as usize;
        let mut region_data = vec![0u8; region_size];
        f.read_exact(&mut region_data)?;

        let mut change_ids = Vec::with_capacity(num_change_records);
        let mut change_records = HashMap::with_capacity(num_change_records);
        for _ in 0..num_change_records {
            let record = ChangeRecord::read(&mut f)?.0;
            let form_id = record.form_id();
            change_ids.push(form_id);
            change_records.insert(form_id, record);
        }
        
        let temp_effects_size = extract!(f as u32)? as usize;
        let mut temporary_effects = vec![0u8; temp_effects_size];
        f.read_exact(&mut temporary_effects)?;
        
        let num_form_ids = extract!(f as u32)? as usize;
        let mut form_ids = Vec::with_capacity(num_form_ids);
        for _ in 0..num_form_ids {
            form_ids.push(extract!(f as u32)?);
        }

        let num_world_spaces = extract!(f as u32)? as usize;
        let mut world_spaces = Vec::with_capacity(num_world_spaces);
        for _ in 0..num_world_spaces {
            world_spaces.push(extract!(f as u32)?);
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
    pub fn load_file(path: &str) -> io::Result<Save> {
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
    pub fn get_change_record(&self, form_id: u32) -> Option<&ChangeRecord> {
        self.change_records.get(&form_id)
    }

    /// Gets a change record by form ID, mutably
    ///
    /// Returns `None` if no change record exists for the given form ID.
    pub fn get_change_record_mut(&mut self, form_id: u32) -> Option<&mut ChangeRecord> {
        self.change_records.get_mut(&form_id)
    }

    /// Write a save to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs.
    pub fn write<T: Write + Seek>(&self, mut f: T) -> io::Result<()> {
        f.write_exact(b"TES4SAVEGAME")?;
        f.write_exact(&[self.version.0, self.version.1])?;
        f.write_exact(&self.exe_time)?;

        // TODO: when this type is fully implemented, ensure that all setters do validation so we
        //  don't have to do it here

        serialize!(self.header_version => f)?;
        // header size = screenshot size + hard-coded fields + name and location bzstrings
        let header_size = self.screen_data.len() + 46 + self.player_name.len() + self.player_location.len();
        serialize!(header_size as u32 => f)?;
        serialize!(self.save_number => f)?;
        serialize_bzstring(&mut f, &self.player_name)?;
        serialize!(self.player_level => f)?;
        serialize_bzstring(&mut f, &self.player_location)?;
        serialize!(self.game_days => f)?;
        serialize!(self.game_ticks => f)?;
        f.write_exact(&self.game_time)?;
        let screen_size = self.screen_data.len() + 8;
        serialize!(screen_size as u32 => f)?;
        serialize!(self.screen_width => f)?;
        serialize!(self.screen_height => f)?;
        f.write_exact(&self.screen_data)?;

        serialize!(self.plugins.len() as u8 => f)?;
        for plugin in self.plugins.iter() {
            serialize_bstring(&mut f, plugin)?;
        }

        // we don't have this value yet, so record the current offset so we can come back later
        let form_id_offset = f.seek(SeekFrom::Current(0))?;
        // write dummy value
        f.write_exact(b"\0\0\0\0")?;
        serialize!(self.change_records.len() as u32 => f)?;
        serialize!(self.next_form_id => f)?;
        serialize!(self.world_id => f)?;
        serialize!(self.world_x => f)?;
        serialize!(self.world_y => f)?;
        serialize!(self.player_cell => f)?;
        serialize!(self.player_x => f)?;
        serialize!(self.player_y => f)?;
        serialize!(self.player_z => f)?;

        serialize!(self.globals.len() as u16 => f)?;
        for (iref, value) in self.globals.iter() {
            serialize!(iref => f)?;
            serialize!(value => f)?;
        }

        let tes_class_size = self.deaths.len()*6 + 8;
        serialize!(tes_class_size as u16 => f)?;
        serialize!(self.deaths.len() as u32 => f)?;
        for (actor, count) in self.deaths.iter() {
            serialize!(actor => f)?;
            serialize!(count => f)?;
        }

        serialize!(self.game_seconds => f)?;

        serialize!(self.processes_data.len() as u16 => f)?;
        f.write_exact(&self.processes_data)?;

        serialize!(self.spec_event_data.len() as u16 => f)?;
        f.write_exact(&self.spec_event_data)?;

        serialize!(self.weather_data.len() as u16 => f)?;
        f.write_exact(&self.weather_data)?;

        serialize!(self.player_combat_count => f)?;

        serialize!(self.created_records.len() as u32 => f)?;
        for record in self.created_records.iter() {
            record.write(&mut f)?;
        }

        serialize!(self.quick_keys.len() as u16 => f)?;
        for quick_key in self.quick_keys.iter() {
            if let Some(setting) = quick_key {
                serialize!(1u8 => f)?;
                serialize!(setting => f)?;
            } else {
                serialize!(0u8 => f)?;
            }
        }

        serialize!(self.reticle_data.len() as u16 => f)?;
        f.write_exact(&self.reticle_data)?;

        serialize!(self.interface_data.len() as u16 => f)?;
        f.write_exact(&self.interface_data)?;

        serialize!(self.region_data.len() as u16 => f)?;
        f.write_exact(&self.region_data)?;

        for id in self.change_ids.iter() {
            if let Some(change_record) = self.change_records.get(id) {
                change_record.write(&mut f)?;
            }
        }

        serialize!(self.temporary_effects.len() as u32 => f)?;
        f.write_exact(&self.temporary_effects)?;

        // now go back and fill in the form ID offset
        let current_pos = f.seek(SeekFrom::Current(0))?;
        f.seek(SeekFrom::Start(form_id_offset))?;
        serialize!(current_pos as u32 => f)?;
        f.seek(SeekFrom::Start(current_pos))?;

        serialize!(self.form_ids.len() as u32 => f)?;
        for form_id in self.form_ids.iter() {
            serialize!(form_id => f)?;
        }

        serialize!(self.world_spaces.len() as u32 => f)?;
        for world_space in self.world_spaces.iter() {
            serialize!(world_space => f)?;
        }

        Ok(())
    }

    /// Write a save to a file
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be created or if an I/O error occurs.
    pub fn save_file(&self, path: &str) -> io::Result<()> {
        let f = File::create(path)?;
        let writer = BufWriter::new(f);
        self.write(writer)
    }
}

#[cfg(test)]
static TEST_SAVE: &[u8] = include_bytes!("save/test/autosave.ess");

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_save() {
        let save = Save::read(&mut TEST_SAVE.as_ref()).unwrap();
        assert_eq!(save.player_name, "test");
        assert_eq!(save.player_location, "Imperial Prison");
        assert_eq!(save.plugins.len(), 11);
    }

    #[test]
    fn set_name() {
        let mut save = Save::read(&mut TEST_SAVE.as_ref()).unwrap();
        save.set_player_name(String::from("short name")).unwrap();
        save.set_player_name(
            String::from("name that is too long oh nooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo")
        ).unwrap_err();
    }

    #[test]
    fn write_save() {
        let save = Save::read(&mut TEST_SAVE.as_ref()).unwrap();
        let mut buf = vec![0u8; TEST_SAVE.len()];
        let cursor = Cursor::new(&mut buf);
        // FIXME: if this test fails, CLion gets confused about the results due to the large size of the output
        save.write(cursor).unwrap();
        assert_eq!(TEST_SAVE, buf.as_slice());
    }
}