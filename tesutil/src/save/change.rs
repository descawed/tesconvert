use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::convert::{Into, TryFrom};
use std::io;
use std::io::{Read, Write};
use crate::*;

/// Indicates the type of record being changed
#[derive(Copy, Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum ChangeType {
    Faction = 6,
    Apparatus = 19,
    Armor = 20,
    Book = 21,
    Clothing = 22,
    Ingredient = 23,
    Light = 26,
    Miscellaneous = 27,
    Weapon = 33,
    Ammo = 34,
    Npc = 35,
    Creature = 36,
    SoulGem = 38,
    Key = 39,
    Alchemy = 40,
    Cell = 48,
    ItemReference = 49,
    CharacterReference = 50,
    CreatureReference = 51,
    Info = 58,
    Quest = 59,
    Package = 61,
}

/// A record in a save that records changes to objects
pub struct ChangeRecord {
    form_id: u32,
    change_type: ChangeType,
    flags: u32,
    version: u8,
    data: Vec<u8>,
}

impl ChangeRecord {
    /// Read a change record from a binary stream
    pub fn read<T: Read>(mut f: T) -> io::Result<(ChangeRecord, usize)> {
        let form_id = extract!(f as u32)?;
        let change_type = ChangeType::try_from(extract!(f as u8)?).map_err(|e| io_error(e))?;
        let flags = extract!(f as u32)?;
        let version = extract!(f as u8)?;
        let data_size = extract!(f as u16)? as usize;
        let mut data = vec![0u8; data_size];
        f.read_exact(&mut &mut data)?;

        Ok((ChangeRecord {
            form_id,
            change_type,
            flags,
            version,
            data,
        }, data_size + 12)) // 12 byte header
    }

    /// Write a change record to a binary stream
    pub fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
        serialize!(self.form_id => f)?;
        serialize!(Into::<u8>::into(self.change_type) => f)?;
        serialize!(self.flags => f)?;
        serialize!(self.version => f)?;
        // TODO: when this type is fully implemented, don't allow data.len() to exceed u16::MAX
        serialize!(self.data.len() as u16 => f)?;
        f.write_exact(&mut self.data.as_ref())?;
        Ok(())
    }
}