use crate::tes4::FormId;
use crate::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::io::{Read, Seek, Write};

use binrw::{binrw, BinReaderExt, BinWriterExt};

/// Indicates the type of record being changed
#[binrw]
#[derive(Copy, Clone, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
#[brw(repr = u8)]
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
#[binrw]
#[derive(Debug)]
pub struct ChangeRecord {
    form_id: FormId,
    change_type: ChangeType,
    flags: u32,
    version: u8,
    #[br(temp)]
    #[bw(calc = data.len() as u16)]
    size: u16,
    #[br(count = size)]
    data: Vec<u8>,
}

impl ChangeRecord {
    /// Reads a change record from a binary stream
    pub fn read<T: Read + Seek>(mut f: T) -> Result<ChangeRecord, TesError> {
        Ok(f.read_le()?)
    }

    /// Gets the change type of this record
    pub fn change_type(&self) -> ChangeType {
        self.change_type
    }

    /// Gets the form ID being changed
    pub fn form_id(&self) -> FormId {
        self.form_id
    }

    /// Gets the change record's flags
    ///
    /// The flags indicate which subrecords are present in the change record. The exact meaning of
    /// the flags depends on the change type.
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// Gets the change record's data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Sets the change record's flags and data
    ///
    /// As the flags determine what data is present, the flags must be set at the same time as the
    /// data.
    ///
    /// # Errors
    ///
    /// Fails if the length of the data exceeds `u16::MAX`.
    pub fn set_data(&mut self, flags: u32, data: Vec<u8>) -> Result<(), TesError> {
        check_size(&data, u16::MAX as usize, "Change record data too large")?;
        self.flags = flags;
        self.data = data;
        Ok(())
    }

    /// Writes a change record to a binary stream
    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_le(&self)?;
        Ok(())
    }
}
