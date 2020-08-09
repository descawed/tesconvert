use bitflags;
use crate::*;
use crate::save::{ChangeRecord, ChangeType};
use std::io::Read;

bitflags! {
    struct ActorChangeFlags: u32 {
        const FORM_FLAGS = 0x00000001;
        const BASE_HEALTH = 0x00000004;
        const BASE_ATTRIBUTES = 0x00000008;
        const BASE_DATA = 0x00000010;
        const SPELL_LIST = 0x00000020;
        const FACTIONS = 0x00000040;
        const FULL_NAME = 0x00000080;
        const AI_DATA = 0x00000100;
        const SKILLS = 0x00000200;
        const COMBAT_STYLE = 0x00000400;
        const BASE_MODIFIERS = 0x10000000;
    }
}

/// An actor's attributes
#[derive(Debug)]
pub struct Attributes {
    pub strength: u8,
    pub intelligence: u8,
    pub willpower: u8,
    pub agility: u8,
    pub speed: u8,
    pub endurance: u8,
    pub personality: u8,
    pub luck: u8,
}

bitflags! {
    struct ActorFlags: u32 {
        const FEMALE = 0x00000001;
        const ESSENTIAL = 0x00000002;
        const RESPAWN = 0x00000008;
        const AUTO_CALC = 0x00000010;
        const PC_LEVEL_OFFSET = 0x00000080;
        const NO_LOW_LEVEL_PROCESSING = 0x00000200;
        const NO_RUMORS = 0x00002000;
        const SUMMONABLE = 0x00004000;
        const NO_PERSUASION = 0x00008000;
        const CAN_CORPSE_CHECK = 0x00100000;
        const UNKNOWN = 0x40000000;
    }
}

/// Actor base data
#[derive(Debug)]
pub struct ActorBase {
    flags: ActorFlags,
    magicka: u16,
    fatigue: u16,
    gold: u16,
    level: i16,
    calc_min: u16,
    calc_max: u16,
}

/// An actor's skills
#[derive(Debug)]
pub struct Skills {
    pub armorer: u8,
    pub athletics: u8,
    pub blade: u8,
    pub block: u8,
    pub blunt: u8,
    pub hand_to_hand: u8,
    pub heavy_armor: u8,
    pub alchemy: u8,
    pub alteration: u8,
    pub conjuration: u8,
    pub destruction: u8,
    pub illusion: u8,
    pub mysticism: u8,
    pub restoration: u8,
    pub acrobatics: u8,
    pub light_armor: u8,
    pub marksman: u8,
    pub mercantile: u8,
    pub security: u8,
    pub sneak: u8,
    pub speechcraft: u8,
}

/// A change record for an NPC or creature
#[derive(Debug)]
pub struct ActorChange {
    change_type: ChangeType,
    flags: Option<u32>,
    attributes: Option<Attributes>,
    base: Option<ActorBase>,
    factions: Vec<(u32, i8)>,
    spells: Vec<u32>,
    ai_data: Option<[u8; 4]>,
    base_health: Option<u32>,
    modifiers: Vec<(u8, f32)>,
    full_name: Option<String>,
    skills: Option<Skills>,
    combat_style: Option<u32>,
}

impl ActorChange {
    /// Read an `ActorChange` from a raw change record
    ///
    /// # Errors
    ///
    /// Fails if the format is not of the right type or if the data is invalid.
    pub fn read(record: &ChangeRecord) -> Result<ActorChange, TesError> {
        let change_type = record.change_type();
        if change_type != ChangeType::Npc && change_type != ChangeType::Creature {
            return Err(TesError::DecodeFailed { description: String::from("ActorChange expects an NPC or creature change record"), cause: None });
        }

        let change_flags = ActorChangeFlags::from_bits(record.flags()).ok_or(TesError::DecodeFailed {
            description: String::from("Invalid actor change flags"),
            cause: None,
        })?;

        let mut actor_change = ActorChange {
            change_type,
            flags: None,
            attributes: None,
            base: None,
            factions: vec![],
            spells: vec![],
            ai_data: None,
            base_health: None,
            modifiers: vec![],
            full_name: None,
            skills: None,
            combat_style: None,
        };

        let mut data = record.data();
        let mut reader = &mut data;

        wrap_decode("Failed to decode actor change record", || {
            if change_flags.contains(ActorChangeFlags::FORM_FLAGS) {
                actor_change.flags = Some(extract!(reader as u32)?);
            }

            if change_flags.contains(ActorChangeFlags::BASE_ATTRIBUTES) {
                actor_change.attributes = Some(Attributes {
                    strength: extract!(reader as u8)?,
                    intelligence: extract!(reader as u8)?,
                    willpower: extract!(reader as u8)?,
                    agility: extract!(reader as u8)?,
                    speed: extract!(reader as u8)?,
                    endurance: extract!(reader as u8)?,
                    personality: extract!(reader as u8)?,
                    luck: extract!(reader as u8)?,
                });
            }

            if change_flags.contains(ActorChangeFlags::BASE_DATA) {
                actor_change.base = Some(ActorBase {
                    flags: ActorFlags::from_bits(extract!(reader as u32)?).ok_or(io_error(TesError::DecodeFailed {
                        description: String::from("Invalid actor flags"),
                        cause: None,
                    }))?,
                    magicka: extract!(reader as u16)?,
                    fatigue: extract!(reader as u16)?,
                    gold: extract!(reader as u16)?,
                    level: extract!(reader as i16)?,
                    calc_min: extract!(reader as u16)?,
                    calc_max: extract!(reader as u16)?,
                });
            }

            if change_flags.contains(ActorChangeFlags::FACTIONS) {
                let num_factions = extract!(reader as u16)?;
                for _ in 0..num_factions {
                    actor_change.factions.push((extract!(reader as u32)?, extract!(reader as i8)?));
                }
            }

            if change_flags.contains(ActorChangeFlags::SPELL_LIST) {
                let num_spells = extract!(reader as u16)?;
                for _ in 0..num_spells {
                    actor_change.spells.push(extract!(reader as u32)?);
                }
            }

            if change_flags.contains(ActorChangeFlags::AI_DATA) {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                actor_change.ai_data = Some(buf);
            }

            if change_flags.contains(ActorChangeFlags::BASE_HEALTH) {
                actor_change.base_health = Some(extract!(reader as u32)?);
            }

            if change_flags.contains(ActorChangeFlags::BASE_MODIFIERS) {
                let num_modifiers = extract!(reader as u16)?;
                for _ in 0..num_modifiers {
                    actor_change.modifiers.push((extract!(reader as u8)?, extract!(reader as f32)?));
                }
            }

            if change_flags.contains(ActorChangeFlags::FULL_NAME) {
                actor_change.full_name = Some(extract_bstring(&mut reader)?);
            }

            if change_flags.contains(ActorChangeFlags::SKILLS) {
                actor_change.skills = Some(Skills {
                    armorer: extract!(reader as u8)?,
                    athletics: extract!(reader as u8)?,
                    blade: extract!(reader as u8)?,
                    block: extract!(reader as u8)?,
                    blunt: extract!(reader as u8)?,
                    hand_to_hand: extract!(reader as u8)?,
                    heavy_armor: extract!(reader as u8)?,
                    alchemy: extract!(reader as u8)?,
                    alteration: extract!(reader as u8)?,
                    conjuration: extract!(reader as u8)?,
                    destruction: extract!(reader as u8)?,
                    illusion: extract!(reader as u8)?,
                    mysticism: extract!(reader as u8)?,
                    restoration: extract!(reader as u8)?,
                    acrobatics: extract!(reader as u8)?,
                    light_armor: extract!(reader as u8)?,
                    marksman: extract!(reader as u8)?,
                    mercantile: extract!(reader as u8)?,
                    security: extract!(reader as u8)?,
                    sneak: extract!(reader as u8)?,
                    speechcraft: extract!(reader as u8)?,
                });
            }

            if change_flags.contains(ActorChangeFlags::COMBAT_STYLE) {
                actor_change.combat_style = Some(extract!(reader as u32)?);
            }

            Ok(())
        })?;

        Ok(actor_change)
    }

    /// Get the actor's attributes
    pub fn attributes(&self) -> Option<&Attributes> {
        self.attributes.as_ref()
    }

    /// Get the actor's attributes mutably
    pub fn attributes_mut(&mut self) -> Option<&mut Attributes> {
        self.attributes.as_mut()
    }

    /// Get the actor's skills
    pub fn skills(&self) -> Option<&Skills> {
        self.skills.as_ref()
    }

    /// Get the actor's skills mutably
    pub fn skills_mut(&mut self) -> Option<&mut Skills> {
        self.skills.as_mut()
    }

    /// Get the actor's full name
    pub fn full_name(&self) -> Option<&str> {
        self.full_name.as_ref().map(|v| &v[..])
    }

    /// Set the actor's full name
    ///
    /// # Errors
    ///
    /// Fails if the length of the name exceeds [`MAX_BSTRING`].
    ///
    /// [`MAX_BSTRING`]: constant.MAX_BSTRING.html
    pub fn set_full_name(&mut self, name: Option<String>) -> Result<(), TesError> {
        if let Some(ref s) = name {
            check_size(s, MAX_BSTRING, "NPC full name too long")?;
        }

        self.full_name = name;
        Ok(())
    }
}