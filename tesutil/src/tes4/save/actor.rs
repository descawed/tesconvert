use std::io::{Cursor, Read};

use crate::tes4::save::{Attributes, ChangeRecord, ChangeType, FormChange};
use crate::tes4::{ActorFlags, Skills};
use crate::*;

use binrw::{binrw, BinReaderExt, BinWriterExt};
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

/// Actor base data
#[binrw]
#[derive(Debug)]
pub struct ActorBase {
    #[br(try_map = |f| ActorFlags::from_bits(f).ok_or("Invalid actor flags"))]
    #[bw(map = |f| f.bits)]
    flags: ActorFlags,
    pub magicka: u16,
    pub fatigue: u16,
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
            return Err(decode_failed(
                "ActorChange expects an NPC or creature change record",
            ));
        }

        let change_flags = ActorChangeFlags::from_bits(record.flags())
            .ok_or_else(|| decode_failed("Invalid actor change flags"))?;

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

        let data = record.data();
        let mut reader = Cursor::new(&data);

        if change_flags.contains(ActorChangeFlags::FORM_FLAGS) {
            actor_change.flags = Some(reader.read_le()?);
        }

        if change_flags.contains(ActorChangeFlags::BASE_ATTRIBUTES) {
            let mut attributes = Attributes::default();
            for attribute in attributes.values_mut() {
                *attribute = reader.read_le()?;
            }

            actor_change.attributes = Some(attributes);
        }

        if change_flags.contains(ActorChangeFlags::BASE_DATA) {
            actor_change.base = Some(reader.read_le()?);
        }

        if change_flags.contains(ActorChangeFlags::FACTIONS) {
            let num_factions: u16 = reader.read_le()?;
            for _ in 0..num_factions {
                actor_change
                    .factions
                    .push((reader.read_le()?, reader.read_le()?));
            }
        }

        if change_flags.contains(ActorChangeFlags::SPELL_LIST) {
            let num_spells: u16 = reader.read_le()?;
            for _ in 0..num_spells {
                actor_change.spells.push(reader.read_le()?);
            }
        }

        if change_flags.contains(ActorChangeFlags::AI_DATA) {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            actor_change.ai_data = Some(buf);
        }

        if change_flags.contains(ActorChangeFlags::BASE_HEALTH) {
            actor_change.base_health = Some(reader.read_le()?);
        }

        if change_flags.contains(ActorChangeFlags::BASE_MODIFIERS) {
            let num_modifiers: u16 = reader.read_le()?;
            for _ in 0..num_modifiers {
                actor_change
                    .modifiers
                    .push((reader.read_le()?, reader.read_le()?));
            }
        }

        if change_flags.contains(ActorChangeFlags::FULL_NAME) {
            actor_change.full_name = Some(read_bstring(&mut reader)?);
        }

        if change_flags.contains(ActorChangeFlags::SKILLS) {
            let mut skills = Skills::default();
            for skill in skills.values_mut() {
                *skill = reader.read_le()?;
            }

            actor_change.skills = Some(skills);
        }

        if change_flags.contains(ActorChangeFlags::COMBAT_STYLE) {
            actor_change.combat_style = Some(reader.read_le()?);
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
        let mut writer = Cursor::new(&mut buf);
        let mut flags = ActorChangeFlags::empty();

        if let Some(form_flags) = self.flags {
            flags |= ActorChangeFlags::FORM_FLAGS;
            writer.write_le(&form_flags)?;
        }

        if let Some(ref attributes) = self.attributes {
            flags |= ActorChangeFlags::BASE_ATTRIBUTES;
            for attribute in attributes.values() {
                writer.write_le(&attribute)?;
            }
        }

        if let Some(ref base_data) = self.base {
            flags |= ActorChangeFlags::BASE_DATA;
            writer.write_le(base_data)?;
        }

        if !self.factions.is_empty() {
            flags |= ActorChangeFlags::FACTIONS;
            let len = self.factions.len() as u16;
            writer.write_le(&len)?;
            for faction in self.factions.iter() {
                writer.write_le(&faction.0)?;
                writer.write_le(&faction.1)?;
            }
        }

        if !self.spells.is_empty() {
            flags |= ActorChangeFlags::SPELL_LIST;
            let len = self.spells.len() as u16;
            writer.write_le(&len)?;
            for spell in self.spells.iter() {
                writer.write_le(&spell)?;
            }
        }

        if let Some(ref ai_data) = self.ai_data {
            flags |= ActorChangeFlags::AI_DATA;
            writer.write_all(ai_data)?;
        }

        if let Some(base_health) = self.base_health {
            flags |= ActorChangeFlags::BASE_HEALTH;
            writer.write_le(&base_health)?;
        }

        if !self.modifiers.is_empty() {
            flags |= ActorChangeFlags::BASE_MODIFIERS;
            let len = self.modifiers.len() as u16;
            writer.write_le(&len)?;
            for modifier in self.modifiers.iter() {
                writer.write_le(&modifier.0)?;
                writer.write_le(&modifier.1)?;
            }
        }

        if let Some(ref name) = self.full_name {
            flags |= ActorChangeFlags::FULL_NAME;
            write_bstring(&mut writer, &name[..])?;
        }

        if let Some(ref skills) = self.skills {
            flags |= ActorChangeFlags::SKILLS;
            for skill in skills.values() {
                writer.write_le(&skill)?;
            }
        }

        if let Some(combat_style) = self.combat_style {
            flags |= ActorChangeFlags::COMBAT_STYLE;
            writer.write_le(&combat_style)?;
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

    /// Gets the actor's base health
    pub fn base_health(&self) -> Option<u32> {
        self.base_health
    }

    /// Sets the actor's base health
    pub fn set_base_health(&mut self, value: Option<u32>) {
        self.base_health = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tes4::save::*;
    use std::io::Cursor;

    #[test]
    fn read_actor_change() {
        let mut record_ref = TEST_SAVE.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let save = Save::read(cursor).unwrap();
        let player = save.get_change_record(FORM_PLAYER).unwrap();
        let actor_change = ActorChange::read(player).unwrap();
        assert_eq!(
            actor_change.attributes.unwrap()[Attribute::Intelligence],
            40
        );
    }
}
