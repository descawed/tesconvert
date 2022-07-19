use std::convert::TryFrom;

use crate::tes3::{MagicEffectType, Skill, SpellEffect, Tes3Field, Tes3Record};
use crate::{
    decode_failed, decode_failed_because, Attribute, EffectRange, Field, Form, Record, TesError,
};

use binrw::BinReaderExt;
use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum SpellType {
    #[default]
    Spell,
    Ability,
    Blight,
    Disease,
    Curse,
    Power,
}

bitflags! {
    #[derive(Default)]
    struct SpellFlags: u32 {
        const AUTO_CALC = 0x01;
        const PC_START_SPELL = 0x02;
        const ALWAYS_SUCCEEDS = 0x04;
    }
}

/// A spell, ability, or disease
#[derive(Debug, Default)]
pub struct Spell {
    id: String,
    name: String,
    spell_type: SpellType,
    cost: u32,
    flags: SpellFlags,
    effects: Vec<SpellEffect>,
}

impl Spell {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn spell_type(&self) -> SpellType {
        self.spell_type
    }

    pub fn cost(&self) -> u32 {
        self.cost
    }

    /// Is this spell's cost auto-calculated?
    pub fn is_auto_calc(&self) -> bool {
        self.flags.contains(SpellFlags::AUTO_CALC)
    }

    /// Is this a player start spell?
    pub fn is_player_start_spell(&self) -> bool {
        self.flags.contains(SpellFlags::PC_START_SPELL)
    }

    /// Does casting this spell always succeed?
    pub fn always_succeeds(&self) -> bool {
        self.flags.contains(SpellFlags::ALWAYS_SUCCEEDS)
    }

    /// Iterates over this spell's effects
    pub fn effects(&self) -> impl Iterator<Item = &SpellEffect> {
        self.effects.iter()
    }
}

impl Form for Spell {
    type Field = Tes3Field;
    type Record = Tes3Record;

    const RECORD_TYPE: &'static [u8; 4] = b"SPEL";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Spell::assert(record)?;

        let mut spell = Spell::default();

        for field in record.iter() {
            match field.name() {
                b"NAME" => spell.id = String::from(field.get_zstring()?),
                b"FNAM" => spell.name = String::from(field.get_zstring()?),
                b"SPDT" => {
                    let mut reader = field.reader();
                    spell.spell_type = SpellType::try_from(reader.read_le::<u32>()? as u8)
                        .map_err(|e| decode_failed_because("Invalid spell type", e))?;
                    spell.cost = reader.read_le()?;
                    spell.flags = SpellFlags::from_bits(reader.read_le()?)
                        .ok_or_else(|| decode_failed("Invalid spell flags"))?;
                }
                b"ENAM" => spell.effects.push(field.reader().read_le()?),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(spell)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        unimplemented!()
    }
}
