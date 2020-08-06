mod change;
pub use change::*;

use crate::*;
use crate::plugin::tes4::Record;
use std::io;
use std::io::{Read, Write, BufReader};
use std::fs::File;

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
    regions: Vec<(u32, u32)>,
    change_records: Vec<ChangeRecord>,
    temporary_effects: Vec<u8>,
    form_ids: Vec<u32>,
    world_spaces: Vec<u32>,
}

/// Current save version
pub const VERSION: (u8, u8) = (0, 125);

impl Save {
    /// Read a save file from a binary stream
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
        
        let num_globals = extract!(f as u8)? as usize;
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
            let has_quick_key = quick_keys_data[i] == 0;
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

        extract!(f as u16)?; // region size
        let num_regions = extract!(f as u16)? as usize;
        let mut regions = Vec::with_capacity(num_regions);
        for _ in 0..num_regions {
            let iref = extract!(f as u32)?;
            let unknown6 = extract!(f as u32)?;
            regions.push((iref, unknown6));
        }

        let mut change_records = Vec::with_capacity(num_change_records);
        for _ in 0..num_change_records {
            change_records.push(ChangeRecord::read(&mut f)?.0);
        }
        
        let temp_effects_size = extract!(f as u32)? as usize;
        let mut temporary_effects = Vec::with_capacity(temp_effects_size);
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
            regions,
            change_records,
            temporary_effects,
            form_ids,
            world_spaces,
        })
    }

    /// Load a save file
    pub fn load_file(path: &str) -> io::Result<Save> {
        let f = File::open(path)?;
        let mut reader = BufReader::new(f);
        Save::read(reader)
    }
}