use bitflags;
use crate::*;
use crate::tes4::save::{Attributes, ChangeRecord, ChangeType};
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
            return Err(TesError::DecodeFailed { description: String::from("ActorChange expects an NPC or creature change record"), source: None });
        }

        let change_flags = ActorChangeFlags::from_bits(record.flags()).ok_or(TesError::DecodeFailed {
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
            actor_change.attributes = Some(Attributes::read(&mut reader)?);
        }

        if change_flags.contains(ActorChangeFlags::BASE_DATA) {
            actor_change.base = Some(ActorBase {
                flags: ActorFlags::from_bits(extract!(reader as u32)?).ok_or(io_error(TesError::DecodeFailed {
                    description: String::from("Invalid actor flags"),
                    source: None,
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

        Ok(actor_change)
    }

    /// Gets the actor's attributes
    pub fn attributes(&self) -> Option<&Attributes> {
        self.attributes.as_ref()
    }

    /// Gets the actor's attributes mutably
    pub fn attributes_mut(&mut self) -> Option<&mut Attributes> {
        self.attributes.as_mut()
    }

    /// Gets the actor's skills
    pub fn skills(&self) -> Option<&Skills> {
        self.skills.as_ref()
    }

    /// Gets the actor's skills mutably
    pub fn skills_mut(&mut self) -> Option<&mut Skills> {
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

    /// Writes this actor change to the provided change record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write(&self, record: &mut ChangeRecord) -> Result<(), TesError> {
        let mut buf: Vec<u8> = vec![];
        let mut writer = &mut &mut buf;
        let mut flags = ActorChangeFlags::empty();

        if let Some(form_flags) = self.flags {
            flags |= ActorChangeFlags::FORM_FLAGS;
            serialize!(form_flags => writer)?;
        }

        if let Some(ref attributes) = self.attributes {
            flags |= ActorChangeFlags::BASE_ATTRIBUTES;
            attributes.write(&mut writer)?;
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

        if self.factions.len() > 0 {
            flags |= ActorChangeFlags::FACTIONS;
            let len = self.factions.len() as u16;
            serialize!(len => writer)?;
            for faction in self.factions.iter() {
                serialize!(faction.0 => writer)?;
                serialize!(faction.1 => writer)?;
            }
        }

        if self.spells.len() > 0 {
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

        if self.modifiers.len() > 0 {
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
            serialize!(skills.armorer => writer)?;
            serialize!(skills.athletics => writer)?;
            serialize!(skills.blade => writer)?;
            serialize!(skills.block => writer)?;
            serialize!(skills.blunt => writer)?;
            serialize!(skills.hand_to_hand => writer)?;
            serialize!(skills.heavy_armor => writer)?;
            serialize!(skills.alchemy => writer)?;
            serialize!(skills.alteration => writer)?;
            serialize!(skills.conjuration => writer)?;
            serialize!(skills.destruction => writer)?;
            serialize!(skills.illusion => writer)?;
            serialize!(skills.mysticism => writer)?;
            serialize!(skills.restoration => writer)?;
            serialize!(skills.acrobatics => writer)?;
            serialize!(skills.light_armor => writer)?;
            serialize!(skills.marksman => writer)?;
            serialize!(skills.mercantile => writer)?;
            serialize!(skills.security => writer)?;
            serialize!(skills.sneak => writer)?;
            serialize!(skills.speechcraft => writer)?;
        }

        if let Some(combat_style) = self.combat_style {
            flags |= ActorChangeFlags::COMBAT_STYLE;
            serialize!(combat_style => writer)?;
        }

        record.set_data(flags.bits, buf)?;
        Ok(())
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
        assert_eq!(actor_change.attributes.unwrap().intelligence, 40);
    }
}