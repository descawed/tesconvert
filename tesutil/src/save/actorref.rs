use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::convert::{Into, TryFrom};
use std::io;

use crate::*;

use bitflags;

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
    pub fn read<T: Read>(f: T) -> io::Result<ScriptVariableValue> {
        let var_type = extract!(f as u16)?;
        match var_type {
            0 => Ok(ScriptVariableValue::Number(extract!(f as f64)?)),
            0xF000 => Ok(ScriptVariableValue::Reference(extract!(f as u32)?)),
            _ => Err(io_error(TesError::DecodeFailed {
                description: format!("Unexpected variable type {}", var_type),
                cause: None,
            })),
        }
    }

    /// Writes a script variable value to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write>(&self, f: T) -> io::Result<()> {
        match self {
            ScriptVariableValue::Number(value) => {
                serialize!(0u16 => f)?;
                serialize!(value => f)?;
            },
            ScriptVariableValue::Reference(value) => {
                serialize!(0xf000u16 => f)?;
                serialize!(value => f)?;
            },
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
    pub fn read<T: Read>(mut f: T) -> io::Result<ScriptVariable> {
        let index = extract!(f as u16)?;
        let value = ScriptVariableValue::read(&mut f)?;
        Ok(ScriptVariable { index, value })
    }

    /// Writes a script variable to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
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
    pub fn read<T: Read>(mut f: T) -> io::Result<Property> {
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
            },
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
            },
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
            },
            0x37 => Ok(Property::Scale(extract!(f as f32)?)),
            0x3d => Ok(Property::CrimeGold(extract!(f as f32)?)),
            0x3e => Ok(Property::OblivionEntry {
                door: extract!(f as u32)?,
                x: extract!(f as f32)?,
                y: extract!(f as f32)?,
                z: extract!(f as f32)?,
            }),
            0x47 => Ok(Property::CantWear),
            0x48 => Ok(Property::Poision(extract!(f as u32)?)),
            0x4f => Ok(Property::Unknown4f(extract!(f as u32)?)),
            0x50 => Ok(Property::BoundItem),
            0x55 => Ok(Property::ShortcutKey(extract!(f as u8)?)),
            _ => Err(io_error(TesError::DecodeFailed {
                description: format!("Unimplemented property type {}", id),
                cause :None,
            })),
        }
    }

    /// Writes a property to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
        match self {
            Property::Script { script, variables, unknown } => {
                check_size(variables, u16::MAX as usize, "Too many script variables").map_err(io_error)?;
                serialize!(0x12u8 => f)?;
                serialize!(script => f)?;
                serialize!(variables.len() as u16 => f)?;
                for variable in variables.iter() {
                    variable.write(&mut f)?;
                }
                serialize!(unknown => f)?;
            },
            Property::EquippedItem => serialize!(0x1bu8 => f)?,
            Property::EquippedAccessory => serialize!(0x1cu8 => f)?,
            Property::Unknown22(value) => {
                serialize!(0x22u8 => f)?;
                serialize!(value => f)?;
            },
            Property::Unknown23(values) => {
                check_size(values, u16::MAX as usize, "Too many Unknown23 values").map_err(io_error)?;
                serialize!(0x23u8 => f)?;
                serialize!(values.len() as u16 => f)?;
                for value in values.iter() {
                    serialize!(value => f)?;
                }
            },
            Property::Owner(value) => {
                serialize!(0x27u8 => f)?;
                serialize!(value => f)?;
            },
            Property::AffectedItemCount(count) => {
                serialize!(0x2au8 => f)?;
                serialize!(count => f)?;
            },
            Property::ItemHealth(health) => {
                serialize!(0x2bu8 => f)?;
                serialize!(health => f)?;
            },
            Property::Time(time) => {
                serialize!(0x2du8 => f)?;
                serialize!(time => f)?;
            },
            Property::EnchantmentPoints(points) => {
                serialize!(0x2eu8 => f)?;
                serialize!(points => f)?;
            },
            Property::Soul(value) => {
                serialize!(0x2fu8 => f)?;
                serialize!(value => f)?;
            },
            Property::LeveledItem(data) => {
                serialize!(0x36u8 => f)?;
                f.write_exact(data)?;
            },
            Property::Scale(value) => {
                serialize!(0x37u8 => f)?;
                serialize!(value => f)?;
            },
            Property::CrimeGold(value) => {
                serialize!(0x3du8 => f)?;
                serialize!(value => f)?;
            },
            Property::OblivionEntry { door, x, y, z } => {
                serialize!(0x3eu8 => f)?;
                serialize!(door => f)?;
                serialize!(x => f)?;
                serialize!(y => f)?;
                serialize!(z => f)?;
            },
            Property::CantWear => serialize!(0x47u8 => f)?,
            Property::Poison(value) => {
                serialize!(0x48u8 => f)?;
                serialize!(value => f)?;
            },
            Property::Unknown4f(value) => {
                serialize!(0x4fu8 => f)?;
                serialize!(value => f)?;
            },
            Property::BoundItem => serialize!(0x50u8 => f)?,
            Property::ShortcutKey(key) => {
                serialize!(0x55u8 => f)?;
                serialize!(key => f)?;
            },
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

/// An active magical effect being applied to the player
#[derive(Debug)]
pub struct ActiveEffect {
    spell: u32,
    effect: u8,
    details: Vec<u8>,
}

/// A class, if the player created a custom class
#[derive(Debug)]
pub struct CustomClass {
    // TODO: make these enums
    favored_attributes: [u32; 2],
    specialization: u32,
    major_skills: [u32; 7],
    flags: u32,
    services: u32,
    skill_trained: u8,
    max_training: u8,
    name: String,
    icon: String,
    unknown: u32,
}

/// Changes to the player
///
/// This is a subset of the functionality for change records detailing changes to a placed instance
/// of an NPC (ACHR) or creature (ACRE). However, these records are quite complex and not fully
/// documented, so for now we're focusing on just the player.
#[derive(Debug)]
pub struct PlayerReferenceChange {
    // location
    cell: u32,
    x: f32,
    y: f32,
    z: f32,
    rx: f32,
    ry: f32,
    rz: f32,
    // temporary attribute changes
    temp_active_effects: [f32; 71],
    tac_unknown: [f32; 71],
    damage: [f32; 71],
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
    skill_xp: [f32; 21],
    advancements: Vec<[u8; 8]>,
    spec_counts: [u8; 3],
    skill_usage: [u32; 21],
    major_skill_advancements: u32,
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
    gender: u8,
    name: String,
    class: u32,
    custom_class: Option<CustomClass>,
}