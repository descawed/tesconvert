use std::convert::TryFrom;
use std::io::Cursor;

use crate::tes4::{Magic, SpellEffect, Tes4Field, Tes4Record};
use crate::{decode_failed, decode_failed_because, Field, Form, Record, TesError};

use binrw::{BinReaderExt, BinWriterExt};
use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum SpellType {
    Spell,
    Disease,
    Power,
    LesserPower,
    Ability,
    Poison,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum SpellLevel {
    Novice,
    Apprentice,
    Journeyman,
    Expert,
    Master,
}

bitflags! {
    struct SpellFlags: u8 {
        const MANUAL_SPELL_COST = 0x01;
        const IMMUNE_TO_SILENCE_1 = 0x02;
        const PLAYER_START_SPELL = 0x04;
        const IMMUNE_TO_SILENCE_2 = 0x08;
        const AREA_IGNORES_LOS = 0x10;
        const SCRIPT_EFFECT_ALWAYS_APPLIES = 0x20;
        const DISALLOW_ABSORB_REFLECT = 0x40;
    }
}

/// A magic spell
#[derive(Debug)]
pub struct Spell {
    editor_id: Option<String>,
    name: Option<String>,
    pub spell_type: SpellType,
    pub cost: u32,
    pub level: SpellLevel,
    flags: SpellFlags,
    effects: Vec<SpellEffect>,
}

impl Spell {
    /// Creates a new spell
    pub fn new(editor_id: Option<String>, name: Option<String>) -> Spell {
        Spell {
            editor_id,
            name,
            spell_type: SpellType::Spell,
            cost: 0,
            level: SpellLevel::Novice,
            flags: SpellFlags::empty(),
            effects: vec![],
        }
    }

    /// Is this spell's cost auto-calculated?
    pub fn is_auto_calc(&self) -> bool {
        !self.flags.contains(SpellFlags::MANUAL_SPELL_COST)
    }

    /// Sets whether this spell's cost is auto-calculated
    pub fn set_auto_calc(&mut self, value: bool) {
        self.flags.set(SpellFlags::MANUAL_SPELL_COST, !value);
    }

    /// Is this spell immune to silence?
    pub fn is_immune_to_silence(&self) -> bool {
        self.flags
            .contains(SpellFlags::IMMUNE_TO_SILENCE_1 | SpellFlags::IMMUNE_TO_SILENCE_2)
    }

    /// Sets whether this spell is immune to silence
    pub fn set_immune_to_silence(&mut self, value: bool) {
        self.flags.set(
            SpellFlags::IMMUNE_TO_SILENCE_1 | SpellFlags::IMMUNE_TO_SILENCE_2,
            value,
        );
    }

    /// Does the player start with this spell?
    pub fn is_player_start_spell(&self) -> bool {
        self.flags.contains(SpellFlags::PLAYER_START_SPELL)
    }

    /// Sets whether the player starts with this spell
    pub fn set_player_start_spell(&mut self, value: bool) {
        self.flags.set(SpellFlags::PLAYER_START_SPELL, value);
    }

    /// Does this spell's area effect ignore LOS?
    pub fn area_ignores_los(&self) -> bool {
        self.flags.contains(SpellFlags::AREA_IGNORES_LOS)
    }

    /// Sets whether this spell's area effect ignores LOS
    pub fn set_area_ignores_los(&mut self, value: bool) {
        self.flags.set(SpellFlags::AREA_IGNORES_LOS, value);
    }

    /// Are script effects on this spell affected by resistances?
    pub fn script_effect_always_applies(&self) -> bool {
        self.flags
            .contains(SpellFlags::SCRIPT_EFFECT_ALWAYS_APPLIES)
    }

    /// Sets whether script effects on this spell are affected by resistances
    pub fn set_script_effect_always_applies(&mut self, value: bool) {
        self.flags
            .set(SpellFlags::SCRIPT_EFFECT_ALWAYS_APPLIES, value);
    }

    /// Can this spell be absorbed/reflected?
    pub fn can_be_absorbed_or_reflected(&self) -> bool {
        !self.flags.contains(SpellFlags::DISALLOW_ABSORB_REFLECT)
    }

    /// Sets whether this spell can be absorbed/reflected
    pub fn set_can_be_absorbed_or_reflected(&mut self, value: bool) {
        self.flags.set(SpellFlags::DISALLOW_ABSORB_REFLECT, !value);
    }
}

impl Magic for Spell {
    fn iter_effects(&self) -> Box<dyn Iterator<Item = &SpellEffect> + '_> {
        Box::new(self.effects.iter())
    }

    fn iter_effects_mut(&mut self) -> Box<dyn Iterator<Item = &mut SpellEffect> + '_> {
        Box::new(self.effects.iter_mut())
    }

    fn add_effect(&mut self, effect: SpellEffect) {
        self.effects.push(effect);
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }
}

impl Form for Spell {
    type Field = Tes4Field;
    type Record = Tes4Record;

    const RECORD_TYPE: &'static [u8; 4] = b"SPEL";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Spell::assert(record)?;

        let mut spell = Spell::new(None, None);

        for field in record.iter() {
            match field.name() {
                b"EDID" => spell.editor_id = Some(String::from(field.get_zstring()?)),
                b"SPIT" => {
                    let mut reader = field.reader();
                    spell.spell_type = SpellType::try_from(reader.read_le::<u32>()? as u8)
                        .map_err(|e| decode_failed_because("Invalid spell type", e))?;
                    spell.cost = reader.read_le()?;
                    spell.level = SpellLevel::try_from(reader.read_le::<u32>()? as u8)
                        .map_err(|e| decode_failed_because("Invalid spell level", e))?;
                    spell.flags = SpellFlags::from_bits(reader.read_le::<u32>()? as u8)
                        .ok_or_else(|| decode_failed("Invalid spell flags"))?;
                }
                _ => spell.read_magic_field(&field)?,
            }
        }

        Ok(spell)
    }

    fn write(&self, record: &mut Self::Record) -> Result<(), TesError> {
        Spell::assert(record)?;

        record.clear();

        if let Some(ref editor_id) = self.editor_id {
            record.add_field(Tes4Field::new_zstring(b"EDID", editor_id.clone())?);
        }

        if let Some(ref name) = self.name {
            record.add_field(Tes4Field::new_zstring(b"FULL", name.clone())?);
        }

        let mut buf = Vec::with_capacity(16);
        let mut writer = Cursor::new(&mut buf);
        let spell_type: u8 = self.spell_type.into();
        let spell_level: u8 = self.level.into();

        writer.write_le(&(spell_type as u32))?;
        writer.write_le(&self.cost)?;
        writer.write_le(&(spell_level as u32))?;
        writer.write_le(&self.flags.bits)?;

        record.add_field(Tes4Field::new(b"SPIT", buf)?);

        self.write_magic_effects(record)?;

        Ok(())
    }
}
