use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::convert::TryFrom;
use std::io::{Cursor, Seek, SeekFrom};

use crate::tes4::plugin::Class;
use crate::tes4::save::{ChangeRecord, ChangeType, FormChange, FORM_PLAYER_REF};
use crate::tes4::{ActorValues, Skills};
use crate::*;

use bitflags::bitflags;

bitflags! {
    struct ActorReferenceChangeFlags: u32 {
        const FORM_FLAGS = 0x00000001;
        const CREATED = 0x00000002;
        const MOVED = 0x00000004;
        const HAVOK_MOVED = 0x00000008;
        const SCALE = 0x00000010;
        const LIFE_STATE = 0x00000040;
        const OWNER_CRIME_GOLD = 0x00000080;
        const INVESTMENT_GOLD = 0x00002000;
        const OBLIVION_ENTRY = 0x00004000;
        const DISP_MODIFIERS = 0x00008000;
        const NON_SAVED_PACKAGE = 0x00010000;
        const INTERRUPT_FOLLOW_DIALOGUE = 0x00020000;
        const TRESPASS = 0x00040000;
        const RUN_ONCE = 0x00080000;
        const MAGIC_MODIFIERS = 0x00100000;
        const SCRIPT_MODIFIERS = 0x00200000;
        const GAME_MODIFIERS = 0x00400000;
        const OBLIVION_FLAG = 0x00800000;
        const MOVEMENT_EXTRA = 0x01000000;
        const ANIMATION = 0x02000000;
        const SCRIPT = 0x04000000;
        const INVENTORY = 0x08000000;
        const LEVELED_CREATURE = 0x10000000;
        const EQUIPMENT = 0x20000000;
        const ENABLED_DISABLED = 0x40000000;
        const CELL_CHANGED = 0x80000000;
    }
}

/// Determines an actor's processing priority
#[derive(Copy, Clone, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum ActorFlag {
    High = 0,
    MidHigh = 1,
    MidLow = 2,
    Low = 3,
    None = 0xff,
}

/// Value of a script variable
#[derive(Debug)]
pub enum ScriptVariableValue {
    Reference(u32),
    Number(f64),
}

impl ScriptVariableValue {
    /// Reads a script variable value from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O or decoding error occurs
    pub fn read<T: Read>(mut f: T) -> Result<ScriptVariableValue, TesError> {
        let var_type = extract!(f as u16)?;
        match var_type {
            0 => Ok(ScriptVariableValue::Number(extract!(f as f64)?)),
            0xF000 => Ok(ScriptVariableValue::Reference(extract!(f as u32)?)),
            _ => Err(decode_failed(format!(
                "Unexpected variable type {}",
                var_type
            ))),
        }
    }

    /// Writes a script variable value to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write>(&self, mut f: T) -> Result<(), TesError> {
        match self {
            ScriptVariableValue::Number(value) => {
                serialize!(0u16 => f)?;
                serialize!(value => f)?;
            }
            ScriptVariableValue::Reference(value) => {
                serialize!(0xf000u16 => f)?;
                serialize!(value => f)?;
            }
        }

        Ok(())
    }
}

/// Variables of a script referenced by a script property
#[derive(Debug)]
pub struct ScriptVariable {
    index: u16,
    value: ScriptVariableValue,
}

impl ScriptVariable {
    /// Reads a script variable from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O or decoding error occurs
    pub fn read<T: Read>(mut f: T) -> Result<ScriptVariable, TesError> {
        let index = extract!(f as u16)?;
        let value = ScriptVariableValue::read(&mut f)?;
        Ok(ScriptVariable { index, value })
    }

    /// Writes a script variable to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write>(&self, mut f: T) -> Result<(), TesError> {
        serialize!(self.index => f)?;
        self.value.write(&mut f)?;
        Ok(())
    }
}

/// Miscellaneous properties that appear in a change record's properties section
///
/// Currently, only properties that appear in inventory items are implemented.
#[derive(Debug)]
pub enum Property {
    Script {
        script: u32,
        variables: Vec<ScriptVariable>,
        unknown: u8,
    },
    EquippedItem,
    EquippedAccessory,
    Unknown22(u32),
    Unknown23(Vec<u32>),
    Owner(u32),
    AffectedItemCount(u16),
    ItemHealth(f32),
    Time(f32),
    EnchantmentPoints(f32),
    Soul(u8),
    LeveledItem([u8; 5]),
    Scale(f32),
    CrimeGold(f32),
    OblivionEntry {
        door: u32,
        x: f32,
        y: f32,
        z: f32,
    },
    CantWear,
    Poison(u32),
    Unknown4f(u32),
    BoundItem,
    ShortcutKey(u8),
}

impl Property {
    /// Reads a property from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O or decoding error occurs
    pub fn read<T: Read>(mut f: T) -> Result<Property, TesError> {
        let id = extract!(f as u8)?;
        match id {
            0x12 => {
                let script = extract!(f as u32)?;
                let num_vars = extract!(f as u16)? as usize;
                let mut variables = Vec::with_capacity(num_vars);
                for _ in 0..num_vars {
                    variables.push(ScriptVariable::read(&mut f)?);
                }
                let unknown = extract!(f as u8)?;

                Ok(Property::Script {
                    script,
                    variables,
                    unknown,
                })
            }
            0x1b => Ok(Property::EquippedItem),
            0x1c => Ok(Property::EquippedAccessory),
            0x22 => Ok(Property::Unknown22(extract!(f as u32)?)),
            0x23 => {
                let num_items = extract!(f as u16)? as usize;
                let mut items = Vec::with_capacity(num_items);
                for _ in 0..num_items {
                    items.push(extract!(f as u32)?);
                }

                Ok(Property::Unknown23(items))
            }
            0x27 => Ok(Property::Owner(extract!(f as u32)?)),
            0x2a => Ok(Property::AffectedItemCount(extract!(f as u16)?)),
            0x2b => Ok(Property::ItemHealth(extract!(f as f32)?)),
            0x2d => Ok(Property::Time(extract!(f as f32)?)),
            0x2e => Ok(Property::EnchantmentPoints(extract!(f as f32)?)),
            0x2f => Ok(Property::Soul(extract!(f as u8)?)),
            0x36 => {
                let mut buf = [0u8; 5];
                f.read_exact(&mut buf)?;
                Ok(Property::LeveledItem(buf))
            }
            0x37 => Ok(Property::Scale(extract!(f as f32)?)),
            0x3d => Ok(Property::CrimeGold(extract!(f as f32)?)),
            0x3e => Ok(Property::OblivionEntry {
                door: extract!(f as u32)?,
                x: extract!(f as f32)?,
                y: extract!(f as f32)?,
                z: extract!(f as f32)?,
            }),
            0x47 => Ok(Property::CantWear),
            0x48 => Ok(Property::Poison(extract!(f as u32)?)),
            0x4f => Ok(Property::Unknown4f(extract!(f as u32)?)),
            0x50 => Ok(Property::BoundItem),
            0x55 => Ok(Property::ShortcutKey(extract!(f as u8)?)),
            _ => Err(decode_failed(format!("Unimplemented property type {}", id))),
        }
    }

    /// Writes a property to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write>(&self, mut f: T) -> Result<(), TesError> {
        match self {
            Property::Script {
                script,
                variables,
                unknown,
            } => {
                check_size(variables, u16::MAX as usize, "Too many script variables")?;
                serialize!(0x12u8 => f)?;
                serialize!(script => f)?;
                serialize!(variables.len() as u16 => f)?;
                for variable in variables.iter() {
                    variable.write(&mut f)?;
                }
                serialize!(unknown => f)?;
            }
            Property::EquippedItem => {
                serialize!(0x1bu8 => f)?;
            }
            Property::EquippedAccessory => {
                serialize!(0x1cu8 => f)?;
            }
            Property::Unknown22(value) => {
                serialize!(0x22u8 => f)?;
                serialize!(value => f)?;
            }
            Property::Unknown23(values) => {
                check_size(values, u16::MAX as usize, "Too many Unknown23 values")?;
                serialize!(0x23u8 => f)?;
                serialize!(values.len() as u16 => f)?;
                for value in values.iter() {
                    serialize!(value => f)?;
                }
            }
            Property::Owner(value) => {
                serialize!(0x27u8 => f)?;
                serialize!(value => f)?;
            }
            Property::AffectedItemCount(count) => {
                serialize!(0x2au8 => f)?;
                serialize!(count => f)?;
            }
            Property::ItemHealth(health) => {
                serialize!(0x2bu8 => f)?;
                serialize!(health => f)?;
            }
            Property::Time(time) => {
                serialize!(0x2du8 => f)?;
                serialize!(time => f)?;
            }
            Property::EnchantmentPoints(points) => {
                serialize!(0x2eu8 => f)?;
                serialize!(points => f)?;
            }
            Property::Soul(value) => {
                serialize!(0x2fu8 => f)?;
                serialize!(value => f)?;
            }
            Property::LeveledItem(data) => {
                serialize!(0x36u8 => f)?;
                f.write_exact(data)?;
            }
            Property::Scale(value) => {
                serialize!(0x37u8 => f)?;
                serialize!(value => f)?;
            }
            Property::CrimeGold(value) => {
                serialize!(0x3du8 => f)?;
                serialize!(value => f)?;
            }
            Property::OblivionEntry { door, x, y, z } => {
                serialize!(0x3eu8 => f)?;
                serialize!(door => f)?;
                serialize!(x => f)?;
                serialize!(y => f)?;
                serialize!(z => f)?;
            }
            Property::CantWear => {
                serialize!(0x47u8 => f)?;
            }
            Property::Poison(value) => {
                serialize!(0x48u8 => f)?;
                serialize!(value => f)?;
            }
            Property::Unknown4f(value) => {
                serialize!(0x4fu8 => f)?;
                serialize!(value => f)?;
            }
            Property::BoundItem => {
                serialize!(0x50u8 => f)?;
            }
            Property::ShortcutKey(key) => {
                serialize!(0x55u8 => f)?;
                serialize!(key => f)?;
            }
        }

        Ok(())
    }
}

/// An item in the player's inventory
#[derive(Debug)]
pub struct Item {
    iref: u32,
    stack_count: i32,
    changes: Vec<Vec<Property>>,
}

impl Item {
    /// Reads an item from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn read<T: Read>(mut f: T) -> Result<Item, TesError> {
        let iref = extract!(f as u32)?;
        let stack_count = extract!(f as i32)?;
        let num_changes = extract!(f as u32)? as usize;
        let mut changes = Vec::with_capacity(num_changes);
        for _ in 0..num_changes {
            let num_properties = extract!(f as u16)? as usize;
            let mut properties = Vec::with_capacity(num_properties);
            for _ in 0..num_properties {
                properties.push(Property::read(&mut f)?);
            }
            changes.push(properties);
        }

        Ok(Item {
            iref,
            stack_count,
            changes,
        })
    }

    /// Writes an item to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write>(&self, mut f: T) -> Result<(), TesError> {
        serialize!(self.iref => f)?;
        serialize!(self.stack_count => f)?;
        serialize!(self.changes.len() as u32 => f)?;
        for change in self.changes.iter() {
            serialize!(change.len() as u16 => f)?;
            for property in change.iter() {
                property.write(&mut f)?;
            }
        }

        Ok(())
    }
}

/// An active magical effect being applied to the player
#[derive(Debug)]
pub struct ActiveEffect {
    spell: u32,
    effect: u8,
    details: Vec<u8>,
}

impl ActiveEffect {
    /// Reads an active effect from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn read<T: Read>(mut f: T) -> Result<ActiveEffect, TesError> {
        let size = extract!(f as u16)? as usize;
        let spell = extract!(f as u32)?;
        let effect = extract!(f as u8)?;
        let mut details = vec![0u8; size];
        f.read_exact(&mut details)?;
        Ok(ActiveEffect {
            spell,
            effect,
            details,
        })
    }

    /// Writes an active effect to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write>(&self, mut f: T) -> Result<(), TesError> {
        serialize!(self.details.len() as u16 => f)?;
        serialize!(self.spell => f)?;
        serialize!(self.effect => f)?;
        f.write_exact(&self.details[..])?;

        Ok(())
    }
}

/// Changes to the player
///
/// This is a subset of the functionality for change records detailing changes to a placed instance
/// of an NPC (ACHR) or creature (ACRE). However, these records are quite complex and not fully
/// documented, so for now we're focusing on just the player.
pub struct PlayerReferenceChange {
    flags: u32,
    // location
    cell: u32,
    x: f32,
    y: f32,
    z: f32,
    rx: f32,
    ry: f32,
    rz: f32,
    // temporary attribute changes
    temp_active_effects: ActorValues<f32>,
    tac_unknown: ActorValues<f32>,
    damage: ActorValues<f32>,
    health_delta: f32,
    magicka_delta: f32,
    fatigue_delta: f32,
    // flag
    actor_flag: ActorFlag,
    // inventory
    inventory: Vec<Item>,
    // properties
    properties: Vec<Property>,
    // TODO: do we need to grab any of the modifier sections?
    raw: Vec<u8>,
    // player stats
    statistics: [u32; 34],
    stat_unknown1: [u8; 118],
    birthsign: u32,
    stat_unknown2: [u32; 13],
    stat_unknown3: [u8; 2],
    stat_unknown4: Vec<u32>,
    stat_unknown5: [u8; 2],
    oblivion_doors: Vec<(u32, u8)>,
    stat_unknown6: [u8; 2],
    stat_active_effects: Vec<ActiveEffect>,
    pub skill_xp: Skills<f32>,
    pub advancements: Vec<Attributes<u8>>,
    pub spec_increases: Specializations<u8>,
    pub skill_usage: Skills<u32>,
    pub major_skill_advancements: u32,
    stat_unknown7: u8,
    active_quest: u32,
    known_topics: Vec<u32>,
    open_quests: Vec<(u32, u8, u8)>,
    known_magic_effects: Vec<[u8; 4]>,
    facegen_symmetric: [u8; 200],
    facegen_asymmetric: [u8; 120],
    facegen_texture: [u8; 200],
    race: u32,
    hair: u32,
    eyes: u32,
    hair_length: f32,
    hair_color: [u8; 3],
    stat_unknown8: u8,
    pub is_female: bool,
    name: String,
    class: u32,
    custom_class: Option<Class>,
    stat_unknown9: u32,
}

impl FormChange for PlayerReferenceChange {
    /// Reads a player reference change from a raw change record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs or if the data is invalid
    fn read(record: &ChangeRecord) -> Result<PlayerReferenceChange, TesError> {
        if record.form_id() != FORM_PLAYER_REF {
            return Err(decode_failed(
                "Only the player's change record may currently be decoded",
            ));
        }

        if record.change_type() != ChangeType::CharacterReference {
            return Err(decode_failed(
                "Only character reference change record may currently be decoded",
            ));
        }

        let mut data = record.data();
        let data_size = data.len();
        let mut reader = Cursor::new(&mut data);

        let flags = record.flags();

        // location
        let cell = extract!(reader as u32)?;
        let x = extract!(reader as f32)?;
        let y = extract!(reader as f32)?;
        let z = extract!(reader as f32)?;
        let rx = extract!(reader as f32)?;
        let ry = extract!(reader as f32)?;
        let rz = extract!(reader as f32)?;

        let mut temp_active_effects = ActorValues::new();
        let mut tac_unknown = ActorValues::new();
        let mut damage = ActorValues::new();

        for effect in temp_active_effects.values_mut() {
            *effect = extract!(reader as f32)?;
        }

        for tac in tac_unknown.values_mut() {
            *tac = extract!(reader as f32)?;
        }

        for dmg in damage.values_mut() {
            *dmg = extract!(reader as f32)?;
        }

        let health_delta = extract!(reader as f32)?;
        let magicka_delta = extract!(reader as f32)?;
        let fatigue_delta = extract!(reader as f32)?;

        let actor_flag = ActorFlag::try_from(extract!(reader as u8)?)
            .map_err(|e| decode_failed_because("Invalid actor flags", e))?;

        // inventory might not be present if the save is from the very beginning of the game
        let inventory = if flags & 0x08000000 != 0 {
            let num_items = extract!(reader as u16)? as usize;
            let mut inventory = Vec::with_capacity(num_items);
            for _ in 0..num_items {
                inventory.push(Item::read(&mut reader)?);
            }
            inventory
        } else {
            vec![]
        };

        let num_properties = extract!(reader as u16)? as usize;
        let mut properties = Vec::with_capacity(num_properties);
        for _ in 0..num_properties {
            properties.push(Property::read(&mut reader)?);
        }

        // the following data is not fully decoded and/or not relevant to us here, so we just grab
        // it all raw so we can spit it back out later
        let start = reader.seek(SeekFrom::Current(0))?;
        let mut end = data_size as u64;
        for _ in start..end - 1 {
            // scan for certain marker bytes to tell when we've reached the player statistics section
            let landmark = (extract!(reader as u8)?, extract!(reader as u8)?);
            if landmark == (0xec, 0x42) {
                // we have a potential match; check for the next pair of landmark bytes
                reader.seek(SeekFrom::Current(19))?;
                let landmark = (extract!(reader as u8)?, extract!(reader as u8)?);
                if landmark == (0x96, 0x42) {
                    // we found it!
                    end = reader.seek(SeekFrom::Current(0))? + 28;
                    break;
                }
                reader.seek(SeekFrom::Current(-22))?;
            } else {
                reader.seek(SeekFrom::Current(-1))?;
            }
        }

        let size = (end - start) as usize;
        reader.seek(SeekFrom::Start(start))?;
        let mut raw = vec![0u8; size];
        reader.read_exact(&mut raw[..])?;

        // player statistics
        let mut statistics = [0u32; 34];
        for statistic in &mut statistics {
            *statistic = extract!(reader as u32)?;
        }

        let mut stat_unknown1 = [0u8; 118];
        reader.read_exact(&mut stat_unknown1)?;

        let birthsign = extract!(reader as u32)?;

        let mut stat_unknown2 = [0u32; 13];
        for unk in &mut stat_unknown2 {
            *unk = extract!(reader as u32)?;
        }

        let num2 = extract!(reader as u16)? as usize;

        let mut stat_unknown3 = [0u8; 2];
        reader.read_exact(&mut stat_unknown3)?;

        let mut stat_unknown4 = Vec::with_capacity(num2);
        for _ in 0..num2 {
            stat_unknown4.push(extract!(reader as u32)?);
        }

        let mut stat_unknown5 = [0u8; 2];
        reader.read_exact(&mut stat_unknown5)?;

        let num_doors = extract!(reader as u16)? as usize;
        let mut oblivion_doors = Vec::with_capacity(num_doors);
        for _ in 0..num_doors {
            oblivion_doors.push((extract!(reader as u32)?, extract!(reader as u8)?));
        }

        let mut stat_unknown6 = [0u8; 2];
        reader.read_exact(&mut stat_unknown6)?;

        let num_active_effects = extract!(reader as u16)? as usize;
        let mut stat_active_effects = Vec::with_capacity(num_active_effects);
        for _ in 0..num_active_effects {
            stat_active_effects.push(ActiveEffect::read(&mut reader)?);
        }

        let mut skill_xp = Skills::new();
        for skill in skill_xp.values_mut() {
            *skill = extract!(reader as f32)?;
        }

        let num_advancements = extract!(reader as u32)? as usize;
        let mut advancements = Vec::with_capacity(num_advancements);
        for _ in 0..num_advancements {
            let mut attributes = Attributes::new();
            for attribute in attributes.values_mut() {
                *attribute = extract!(reader as u8)?;
            }

            advancements.push(attributes);
        }

        let mut spec_increases = Specializations::new();
        for specialization in spec_increases.values_mut() {
            *specialization = extract!(reader as u8)?;
        }

        let mut skill_usage = Skills::new();
        for skill in skill_usage.values_mut() {
            *skill = extract!(reader as u32)?;
        }

        let major_skill_advancements = extract!(reader as u32)?;
        let stat_unknown7 = extract!(reader as u8)?;
        let active_quest = extract!(reader as u32)?;

        let num_known_topics = extract!(reader as u16)? as usize;
        let mut known_topics = Vec::with_capacity(num_known_topics);
        for _ in 0..num_known_topics {
            known_topics.push(extract!(reader as u32)?);
        }

        let num_open_quests = extract!(reader as u16)? as usize;
        let mut open_quests = Vec::with_capacity(num_open_quests);
        for _ in 0..num_open_quests {
            open_quests.push((
                extract!(reader as u32)?,
                extract!(reader as u8)?,
                extract!(reader as u8)?,
            ));
        }

        let num_magic_effects = extract!(reader as u32)? as usize;
        let mut known_magic_effects = Vec::with_capacity(num_magic_effects);
        for _ in 0..num_magic_effects {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            known_magic_effects.push(buf);
        }

        let mut facegen_symmetric = [0u8; 200];
        reader.read_exact(&mut facegen_symmetric)?;

        let mut facegen_asymmetric = [0u8; 120];
        reader.read_exact(&mut facegen_asymmetric)?;

        let mut facegen_texture = [0u8; 200];
        reader.read_exact(&mut facegen_texture)?;

        let race = extract!(reader as u32)?;
        let hair = extract!(reader as u32)?;
        let eyes = extract!(reader as u32)?;
        let hair_length = extract!(reader as f32)?;

        let mut hair_color = [0u8; 3];
        reader.read_exact(&mut hair_color)?;

        let stat_unknown8 = extract!(reader as u8)?;

        let is_female = extract!(reader as u8)? != 0;
        let name = extract_bzstring(&mut reader)?;
        let class = extract!(reader as u32)?;

        // we could check if the form ID of the class was FORM_PLAYER_CUSTOM_CLASS, but that would
        // require the save to be passed into this function so we could look up the iref. to avoid
        // that, we just check if there are more bytes, and if so, assume there's a class
        let here = reader.seek(SeekFrom::Current(0))?;
        let there = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(here))?;
        let custom_class = if there - here > 4 {
            Some(Class::read_custom(&mut reader)?)
        } else {
            None
        };

        let stat_unknown9 = extract!(reader as u32)?;

        Ok(PlayerReferenceChange {
            flags,
            cell,
            x,
            y,
            z,
            rx,
            ry,
            rz,
            temp_active_effects,
            tac_unknown,
            damage,
            health_delta,
            magicka_delta,
            fatigue_delta,
            actor_flag,
            inventory,
            properties,
            raw,
            statistics,
            stat_unknown1,
            birthsign,
            stat_unknown2,
            stat_unknown3,
            stat_unknown4,
            stat_unknown5,
            oblivion_doors,
            stat_unknown6,
            stat_active_effects,
            skill_xp,
            advancements,
            spec_increases,
            skill_usage,
            major_skill_advancements,
            stat_unknown7,
            active_quest,
            known_topics,
            open_quests,
            known_magic_effects,
            facegen_symmetric,
            facegen_asymmetric,
            facegen_texture,
            race,
            hair,
            eyes,
            hair_length,
            hair_color,
            stat_unknown8,
            is_female,
            name,
            class,
            custom_class,
            stat_unknown9,
        })
    }

    /// Writes a player reference change to a raw change record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    fn write(&self, record: &mut ChangeRecord) -> Result<(), TesError> {
        let mut buf: Vec<u8> = vec![];
        let mut writer = &mut &mut buf;

        serialize!(self.cell => writer)?;
        serialize!(self.x => writer)?;
        serialize!(self.y => writer)?;
        serialize!(self.z => writer)?;
        serialize!(self.rx => writer)?;
        serialize!(self.ry => writer)?;
        serialize!(self.rz => writer)?;

        for effect in self.temp_active_effects.values() {
            serialize!(effect => writer)?;
        }

        for unknown in self.tac_unknown.values() {
            serialize!(unknown => writer)?;
        }

        for damage in self.damage.values() {
            serialize!(damage => writer)?;
        }

        serialize!(self.health_delta => writer)?;
        serialize!(self.magicka_delta => writer)?;
        serialize!(self.fatigue_delta => writer)?;

        serialize!(Into::<u8>::into(self.actor_flag) => writer)?;

        if self.flags & 0x08000000 != 0 {
            serialize!(self.inventory.len() as u16 => writer)?;
            for item in self.inventory.iter() {
                item.write(&mut writer)?;
            }
        }

        serialize!(self.properties.len() as u16 => writer)?;
        for property in self.properties.iter() {
            property.write(&mut writer)?;
        }

        writer.write_exact(&self.raw)?;

        for stat in self.statistics.iter() {
            serialize!(stat => writer)?;
        }

        writer.write_exact(&self.stat_unknown1)?;
        serialize!(self.birthsign => writer)?;
        for unknown in self.stat_unknown2.iter() {
            serialize!(unknown => writer)?;
        }

        serialize!(self.stat_unknown4.len() as u16 => writer)?;
        writer.write_exact(&self.stat_unknown3)?;
        for unknown in self.stat_unknown4.iter() {
            serialize!(unknown => writer)?;
        }
        writer.write_exact(&self.stat_unknown5)?;

        serialize!(self.oblivion_doors.len() as u16 => writer)?;
        for (door, flag) in self.oblivion_doors.iter() {
            serialize!(door => writer)?;
            serialize!(flag => writer)?;
        }

        writer.write_exact(&self.stat_unknown6)?;

        serialize!(self.stat_active_effects.len() as u16 => writer)?;
        for effect in self.stat_active_effects.iter() {
            effect.write(&mut writer)?;
        }

        for skill in self.skill_xp.values() {
            serialize!(skill => writer)?;
        }

        serialize!(self.advancements.len() as u32 => writer)?;
        for adv in self.advancements.iter().flat_map(|v| v.values()) {
            serialize!(adv => writer)?;
        }

        for spec in self.spec_increases.values() {
            serialize!(spec => writer)?;
        }

        for skill in self.skill_usage.values() {
            serialize!(skill => writer)?;
        }

        serialize!(self.major_skill_advancements => writer)?;
        serialize!(self.stat_unknown7 => writer)?;
        serialize!(self.active_quest => writer)?;

        serialize!(self.known_topics.len() as u16 => writer)?;
        for topic in self.known_topics.iter() {
            serialize!(topic => writer)?;
        }

        serialize!(self.open_quests.len() as u16 => writer)?;
        for (quest, stage, log_entry) in self.open_quests.iter() {
            serialize!(quest => writer)?;
            serialize!(stage => writer)?;
            serialize!(log_entry => writer)?;
        }

        serialize!(self.known_magic_effects.len() as u32 => writer)?;
        for effect in self.known_magic_effects.iter() {
            writer.write_exact(effect)?;
        }

        writer.write_exact(&self.facegen_symmetric)?;
        writer.write_exact(&self.facegen_asymmetric)?;
        writer.write_exact(&self.facegen_texture)?;

        serialize!(self.race => writer)?;
        serialize!(self.hair => writer)?;
        serialize!(self.eyes => writer)?;
        serialize!(self.hair_length => writer)?;
        writer.write_exact(&self.hair_color)?;
        serialize!(self.stat_unknown8 => writer)?;
        serialize!(if self.is_female { 1u8 } else { 0u8 } => writer)?;
        serialize_bzstring(&mut writer, &self.name)?;
        serialize!(self.class => writer)?;

        if let Some(ref custom_class) = self.custom_class {
            custom_class.write_custom(&mut writer)?;
        }

        serialize!(self.stat_unknown9 => writer)?;

        record.set_data(self.flags, buf)?;

        Ok(())
    }
}

impl PlayerReferenceChange {
    /// Gets the player's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the player's name
    ///
    /// # Errors
    ///
    /// Fails if the player's name is longer than [`MAX_BSTRING`]
    ///
    /// [`MAX_BSTRING`]: constant.MAX_BSTRING.html
    pub fn set_name(&mut self, name: String) -> Result<(), TesError> {
        check_size(&name, MAX_BSTRING, "Player name too long")?;
        self.name = name;
        Ok(())
    }

    /// Gets the player's custom class, if any
    pub fn class(&self) -> Option<&Class> {
        self.custom_class.as_ref()
    }

    /// Sets the player's custom class
    pub fn set_class(&mut self, class: Option<Class>, iref: u32) {
        self.class = iref;
        self.custom_class = class;
    }

    /// Gets the player's race as an iref
    pub fn race(&self) -> u32 {
        self.race
    }

    /// Sets the player's race as an iref
    pub fn set_race(&mut self, race: u32) {
        self.race = race;
    }

    /// Gets the player's birthsign as an iref
    pub fn birthsign(&self) -> u32 {
        self.birthsign
    }

    /// Sets the player's birthsign as an iref
    pub fn set_birthsign(&mut self, birthsign: u32) {
        self.birthsign = birthsign;
    }

    /// Iterates through the player's known magic effects
    pub fn known_magic_effects(&self) -> impl Iterator<Item = [u8; 4]> + '_ {
        self.known_magic_effects.iter().copied()
    }

    /// Sets the player's known magic effects
    pub fn set_known_magic_effects(&mut self, effects: Vec<[u8; 4]>) {
        self.known_magic_effects = effects;
    }

    /// Iterates through the player's active magic effects
    pub fn active_magic_effects(&self) -> impl Iterator<Item = &ActiveEffect> + '_ {
        self.stat_active_effects.iter()
    }

    /// Clears the player's active magic effects
    pub fn clear_active_magic_effects(&mut self) {
        self.stat_active_effects.clear();
    }

    /// Gets changes to player actor values from active effects
    pub fn active_effect_modifiers(&self) -> &ActorValues<f32> {
        &self.temp_active_effects
    }

    /// Gets changes to player actor values from active effects, mutably
    pub fn active_effect_modifiers_mut(&mut self) -> &mut ActorValues<f32> {
        &mut self.temp_active_effects
    }

    /// Gets damage to player actor values
    pub fn damage_modifiers(&self) -> &ActorValues<f32> {
        &self.damage
    }

    /// Gets damage to player actor values
    pub fn damage_modifiers_mut(&mut self) -> &mut ActorValues<f32> {
        &mut self.damage
    }

    /// Gets the change in the player's health
    pub fn health_delta(&self) -> f32 {
        self.health_delta
    }

    /// Sets the change in the player's health
    pub fn set_health_delta(&mut self, value: f32) {
        self.health_delta = value;
    }

    /// Gets the change in the player's magicka
    pub fn magicka_delta(&self) -> f32 {
        self.magicka_delta
    }

    /// Sets the change in the player's magicka
    pub fn set_magicka_delta(&mut self, value: f32) {
        self.magicka_delta = value;
    }

    /// Gets the change in the player's fatigue
    pub fn fatigue_delta(&self) -> f32 {
        self.fatigue_delta
    }

    /// Sets the change in the player's fatigue
    pub fn set_fatigue_delta(&mut self, value: f32) {
        self.fatigue_delta = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tes4::save::{Save, TEST_SAVE};

    #[test]
    fn read_player_ref_change() {
        let save = Save::read(&mut TEST_SAVE.as_ref()).unwrap();
        let player = save.get_change_record(FORM_PLAYER_REF).unwrap();
        let player_change = PlayerReferenceChange::read(player).unwrap();
        assert_eq!(player_change.name, "test");
        assert!(!player_change.is_female);
    }

    #[test]
    fn write_player_ref_change() {
        let mut save = Save::read(&mut TEST_SAVE.as_ref()).unwrap();
        let mut player = save.get_change_record_mut(FORM_PLAYER_REF).unwrap();
        let original = player.data().to_vec();
        let player_change = PlayerReferenceChange::read(player).unwrap();
        player_change.write(&mut player).unwrap();
        assert_eq!(original, player.data());
    }
}
