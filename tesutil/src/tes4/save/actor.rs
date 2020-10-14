use std::io::Read;

use crate::tes4::save::{Attributes, ChangeRecord, ChangeType, FormChange};
use crate::tes4::Skills;
use crate::*;

use bitflags::bitflags;

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
    pub level: i16,
    calc_min: u16,
    calc_max: u16,
}

impl Default for ActorBase {
    fn default() -> Self {
        ActorBase {
            flags: ActorFlags::empty(),
            magicka: 0,
            fatigue: 0,
            gold: 0,
            level: 0,
            calc_min: 1,
            calc_max: 0,
        }
    }
}

/// A change record for an NPC or creature
#[derive(Debug)]
pub struct ActorChange {
    change_type: ChangeType,
    flags: Option<u32>,
    attributes: Option<Attributes<u8>>,
    base: Option<ActorBase>,
    factions: Vec<(u32, i8)>,
    spells: Vec<u32>,
    ai_data: Option<[u8; 4]>,
    base_health: Option<u32>,
    modifiers: Vec<(u8, f32)>,
    full_name: Option<String>,
    skills: Option<Skills<u8>>,
    combat_style: Option<u32>,
}

impl FormChange for ActorChange {
    /// Read an `ActorChange` from a raw change record
    ///
    /// # Errors
    ///
    /// Fails if the format is not of the right type or if the data is invalid.
    fn read(record: &ChangeRecord) -> Result<ActorChange, TesError> {
        let change_type = record.change_type();
        if change_type != ChangeType::Npc && change_type != ChangeType::Creature {
            return Err(TesError::DecodeFailed {
                description: String::from("ActorChange expects an NPC or creature change record"),
                source: None,
            });
        }

        let change_flags =
            ActorChangeFlags::from_bits(record.flags()).ok_or(TesError::DecodeFailed {
                description: String::from("Invalid actor change flags"),
                source: None,
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

        if change_flags.contains(ActorChangeFlags::FORM_FLAGS) {
            actor_change.flags = Some(extract!(reader as u32)?);
        }

        if change_flags.contains(ActorChangeFlags::BASE_ATTRIBUTES) {
            let mut attributes = Attributes::new();
            for attribute in attributes.values_mut() {
                *attribute = extract!(reader as u8)?;
            }

            actor_change.attributes = Some(attributes);
        }

        if change_flags.contains(ActorChangeFlags::BASE_DATA) {
            actor_change.base = Some(ActorBase {
                flags: ActorFlags::from_bits(extract!(reader as u32)?).ok_or_else(|| {
                    io_error(TesError::DecodeFailed {
                        description: String::from("Invalid actor flags"),
                        source: None,
                    })
                })?,
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
                actor_change
                    .factions
                    .push((extract!(reader as u32)?, extract!(reader as i8)?));
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
                actor_change
                    .modifiers
                    .push((extract!(reader as u8)?, extract!(reader as f32)?));
            }
        }

        if change_flags.contains(ActorChangeFlags::FULL_NAME) {
            actor_change.full_name = Some(extract_bstring(&mut reader)?);
        }

        if change_flags.contains(ActorChangeFlags::SKILLS) {
            let mut skills = Skills::new();
            for skill in skills.values_mut() {
                *skill = extract!(reader as u8)?;
            }

            actor_change.skills = Some(skills);
        }

        if change_flags.contains(ActorChangeFlags::COMBAT_STYLE) {
            actor_change.combat_style = Some(extract!(reader as u32)?);
        }

        Ok(actor_change)
    }

    /// Writes this actor change to the provided change record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    fn write(&self, record: &mut ChangeRecord) -> Result<(), TesError> {
        let mut buf: Vec<u8> = vec![];
        let mut writer = &mut &mut buf;
        let mut flags = ActorChangeFlags::empty();

        if let Some(form_flags) = self.flags {
            flags |= ActorChangeFlags::FORM_FLAGS;
            serialize!(form_flags => writer)?;
        }

        if let Some(ref attributes) = self.attributes {
            flags |= ActorChangeFlags::BASE_ATTRIBUTES;
            for attribute in attributes.values() {
                serialize!(attribute => writer)?;
            }
        }

        if let Some(ref base_data) = self.base {
            flags |= ActorChangeFlags::BASE_DATA;
            serialize!(base_data.flags.bits => writer)?;
            serialize!(base_data.magicka => writer)?;
            serialize!(base_data.fatigue => writer)?;
            serialize!(base_data.gold => writer)?;
            serialize!(base_data.level => writer)?;
            serialize!(base_data.calc_min => writer)?;
            serialize!(base_data.calc_max => writer)?;
        }

        if !self.factions.is_empty() {
            flags |= ActorChangeFlags::FACTIONS;
            let len = self.factions.len() as u16;
            serialize!(len => writer)?;
            for faction in self.factions.iter() {
                serialize!(faction.0 => writer)?;
                serialize!(faction.1 => writer)?;
            }
        }

        if !self.spells.is_empty() {
            flags |= ActorChangeFlags::SPELL_LIST;
            let len = self.spells.len() as u16;
            serialize!(len => writer)?;
            for spell in self.spells.iter() {
                serialize!(spell => writer)?;
            }
        }

        if let Some(ref ai_data) = self.ai_data {
            flags |= ActorChangeFlags::AI_DATA;
            writer.write_exact(ai_data)?;
        }

        if let Some(base_health) = self.base_health {
            flags |= ActorChangeFlags::BASE_HEALTH;
            serialize!(base_health => writer)?;
        }

        if !self.modifiers.is_empty() {
            flags |= ActorChangeFlags::BASE_MODIFIERS;
            let len = self.modifiers.len() as u16;
            serialize!(len => writer)?;
            for modifier in self.modifiers.iter() {
                serialize!(modifier.0 => writer)?;
                serialize!(modifier.1 => writer)?;
            }
        }

        if let Some(ref name) = self.full_name {
            flags |= ActorChangeFlags::FULL_NAME;
            serialize_bstring(&mut writer, &name[..])?;
        }

        if let Some(ref skills) = self.skills {
            flags |= ActorChangeFlags::SKILLS;
            for skill in skills.values() {
                serialize!(skill => writer)?;
            }
        }

        if let Some(combat_style) = self.combat_style {
            flags |= ActorChangeFlags::COMBAT_STYLE;
            serialize!(combat_style => writer)?;
        }

        record.set_data(flags.bits, buf)?;
        Ok(())
    }
}

impl ActorChange {
    /// Gets the actor's attributes
    pub fn attributes(&self) -> Option<&Attributes<u8>> {
        self.attributes.as_ref()
    }

    /// Gets the actor's attributes mutably
    pub fn attributes_mut(&mut self) -> Option<&mut Attributes<u8>> {
        self.attributes.as_mut()
    }

    /// Gets the actor's skills
    pub fn skills(&self) -> Option<&Skills<u8>> {
        self.skills.as_ref()
    }

    /// Gets the actor's skills mutably
    pub fn skills_mut(&mut self) -> Option<&mut Skills<u8>> {
        self.skills.as_mut()
    }

    /// Gets the actor's full name
    pub fn full_name(&self) -> Option<&str> {
        self.full_name.as_ref().map(|v| &v[..])
    }

    /// Gets the actor's base information
    pub fn actor_base(&self) -> Option<&ActorBase> {
        self.base.as_ref()
    }

    /// Gets the actor's base information mutably
    pub fn actor_base_mut(&mut self) -> Option<&mut ActorBase> {
        self.base.as_mut()
    }

    /// Sets the actor's base data
    pub fn set_actor_base(&mut self, base: Option<ActorBase>) {
        self.base = base;
    }

    /// Sets the actor's full name
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

    /// Gets the actor's spells
    pub fn spells(&self) -> impl Iterator<Item = u32> + '_ {
        self.spells.iter().copied()
    }

    /// Sets the actor's spells
    pub fn set_spells(&mut self, spells: Vec<u32>) {
        self.spells = spells;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tes4::save::*;

    #[test]
    fn read_actor_change() {
        let save = Save::read(&mut TEST_SAVE.as_ref()).unwrap();
        let player = save.get_change_record(FORM_PLAYER).unwrap();
        let actor_change = ActorChange::read(player).unwrap();
        assert_eq!(
            actor_change.attributes.unwrap()[Attribute::Intelligence],
            40
        );
    }
}
