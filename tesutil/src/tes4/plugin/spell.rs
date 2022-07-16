use std::convert::TryFrom;
use std::io::{Cursor, Read, Write};

use crate::tes4::{ActorValue, FormId, MagicEffectType, Tes4Field, Tes4Record, MAGIC_EFFECTS};
use crate::{
    decode_failed, decode_failed_because, EffectRange, Field, Form, MagicSchool, Record, TesError,
};

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

/// Script details for a scripted magic effect
#[derive(Debug)]
pub struct ScriptEffect {
    script: FormId,
    school: MagicSchool,
    visual_effect: Option<MagicEffectType>,
    pub is_hostile: bool,
    name: String,
}

/// An individual effect of a spell
#[derive(Debug)]
pub struct SpellEffect {
    effect: MagicEffectType,
    magnitude: u32,
    area: u32,
    duration: u32,
    range: EffectRange,
    actor_value: ActorValue,
    script_effect: Option<ScriptEffect>,
}

impl SpellEffect {
    /// Creates a new spell effect
    pub fn new(effect: MagicEffectType) -> SpellEffect {
        let base_effect = &MAGIC_EFFECTS[effect];

        SpellEffect {
            effect,
            magnitude: 0,
            area: 0,
            duration: 0,
            range: if base_effect.allows_range(EffectRange::Self_) {
                EffectRange::Self_
            } else if base_effect.allows_range(EffectRange::Touch) {
                EffectRange::Touch
            } else {
                EffectRange::Target
            },
            actor_value: effect.default_actor_value(),
            script_effect: None,
        }
    }

    /// Gets this effect's effect type
    pub fn effect_type(&self) -> MagicEffectType {
        self.effect
    }

    /// Gets the effect's magnitude
    pub fn magnitude(&self) -> u32 {
        self.magnitude
    }

    /// Sets the effect's magnitude
    pub fn set_magnitude(&mut self, value: u32) -> Result<(), TesError> {
        // we can "cheat" by using the hard-coded default effects instead of looking up the actual
        // effect because the properties that we're checking are hard-coded and can't be changed by
        // mods
        if !MAGIC_EFFECTS[self.effect].has_magnitude() {
            // Morrowind uses a magnitude of 1 for spells with no magnitude, so we'll accept a value
            // of 1 and treat it as 0
            if value > 1 {
                Err(TesError::LimitExceeded {
                    description: format!("{:?} cannot have a magnitude", self.effect),
                    max_size: 0,
                    actual_size: value as usize,
                })
            } else {
                self.magnitude = 0;
                Ok(())
            }
        } else {
            self.magnitude = value;
            Ok(())
        }
    }

    /// Gets the effect's area
    pub fn area(&self) -> u32 {
        self.area
    }

    /// Sets the effect's area
    pub fn set_area(&mut self, value: u32) -> Result<(), TesError> {
        if self.range == EffectRange::Self_ && value > 1 {
            Err(TesError::LimitExceeded {
                description: String::from("Cast-on-self spells cannot have an area"),
                max_size: 0,
                actual_size: value as usize,
            })
        } else if !MAGIC_EFFECTS[self.effect].has_area() && value > 1 {
            Err(TesError::LimitExceeded {
                description: format!("{:?} cannot have an area", self.effect),
                max_size: 0,
                actual_size: value as usize,
            })
        } else {
            self.area = value;
            Ok(())
        }
    }

    /// Gets the effect's duration
    pub fn duration(&self) -> u32 {
        self.duration
    }

    /// Sets the effect's duration
    pub fn set_duration(&mut self, value: u32) -> Result<(), TesError> {
        if !MAGIC_EFFECTS[self.effect].has_duration() {
            if value > 1 {
                Err(TesError::LimitExceeded {
                    description: format!("{:?} cannot have a duration", self.effect),
                    max_size: 0,
                    actual_size: value as usize,
                })
            } else {
                self.duration = 0;
                Ok(())
            }
        } else {
            self.duration = value;
            Ok(())
        }
    }

    /// Gets the effect's range
    pub fn range(&self) -> EffectRange {
        self.range
    }

    /// Sets the effect's range
    pub fn set_range(&mut self, range: EffectRange) -> Result<(), TesError> {
        if !MAGIC_EFFECTS[self.effect].allows_range(range) {
            return Err(TesError::RequirementFailed(format!(
                "{:?} does not allow range {:?}",
                self.effect, range
            )));
        }

        if range == EffectRange::Self_ {
            self.area = 0;
        }

        self.range = range;
        Ok(())
    }

    /// Gets the effect's actor value
    pub fn actor_value(&self) -> ActorValue {
        self.actor_value
    }

    /// Sets the effect's actor value
    pub fn set_actor_value(&mut self, value: ActorValue) -> Result<(), TesError> {
        self.actor_value = value;
        Ok(())
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

    /// Adds an effect to this spell
    pub fn add_effect(&mut self, effect: SpellEffect) {
        self.effects.push(effect);
    }

    /// Iterates over this spell's effects
    pub fn effects(&self) -> impl Iterator<Item = &SpellEffect> {
        self.effects.iter()
    }
}

impl Form for Spell {
    type Field = Tes4Field;
    type Record = Tes4Record;

    fn record_type() -> &'static [u8; 4] {
        b"SPEL"
    }

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Spell::assert(record)?;

        let mut spell = Spell::new(None, None);

        for field in record.iter() {
            match field.name() {
                b"EDID" => spell.editor_id = Some(String::from(field.get_zstring()?)),
                b"FULL" => {
                    let name = String::from(field.get_zstring()?);
                    if let Some(last_effect) = spell.effects.iter_mut().last() {
                        if let Some(ref mut script_effect) = last_effect.script_effect {
                            script_effect.name = name;
                        } else {
                            return Err(decode_failed("Unexpected FULL field in SPEL record"));
                        }
                    } else {
                        spell.name = Some(name);
                    }
                }
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
                b"EFID" => (), // the effect ID is also contained in the EFIT field, so we'll just get it from there
                b"EFIT" => {
                    let mut reader = field.reader();
                    spell.effects.push(SpellEffect {
                        effect: {
                            let mut id = [0u8; 4];
                            reader.read_exact(&mut id)?;
                            MagicEffectType::from_id(&id).ok_or_else(|| {
                                decode_failed(format!("Unexpected effect ID {:?}", id))
                            })?
                        },
                        magnitude: reader.read_le()?,
                        area: reader.read_le()?,
                        duration: reader.read_le()?,
                        range: EffectRange::try_from(reader.read_le::<u32>()? as u8)
                            .map_err(|e| decode_failed_because("Invalid effect range", e))?,
                        actor_value: ActorValue::try_from(reader.read_le::<u32>()? as u8)
                            .map_err(|e| decode_failed_because("Invalid effect actor value", e))?,
                        script_effect: None,
                    })
                }
                b"SCIT" => {
                    if let Some(last_effect) = spell.effects.iter_mut().last() {
                        let mut reader = field.reader();
                        last_effect.script_effect = Some(ScriptEffect {
                            script: FormId(reader.read_le()?),
                            school: MagicSchool::try_from(reader.read_le::<u32>()? as u8).map_err(
                                |e| decode_failed_because("Invalid script effect magic school", e),
                            )?,
                            visual_effect: {
                                let id: u32 = reader.read_le()?;
                                if id == 0 {
                                    None
                                } else {
                                    let id_bytes = id.to_le_bytes();
                                    Some(MagicEffectType::from_id(&id_bytes).ok_or_else(|| {
                                        decode_failed(format!("Unexpected effect ID {:?}", id))
                                    })?)
                                }
                            },
                            is_hostile: reader.read_le::<u32>()? & 1 != 0, // other "flag" bits are garbage?
                            name: String::new(),
                        });
                    } else {
                        return Err(decode_failed("Unexpected SCIT field in SPEL record"));
                    }
                }
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected {} field in SPEL record",
                        field.name_as_str()
                    )))
                }
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

        for effect in &self.effects {
            let effect_id = effect.effect.id();
            record.add_field(Tes4Field::new(b"EFID", effect_id.to_vec())?);

            let mut buf = Vec::with_capacity(24);
            let mut writer = Cursor::new(&mut buf);
            let range: u8 = effect.range.into();
            let av: u8 = effect.actor_value.into();
            writer.write_all(&effect_id)?;
            writer.write_le(&effect.magnitude)?;
            writer.write_le(&effect.area)?;
            writer.write_le(&effect.duration)?;
            writer.write_le(&(range as u32))?;
            writer.write_le(&(av as u32))?;

            record.add_field(Tes4Field::new(b"EFIT", buf)?);

            if let Some(ref script_effect) = effect.script_effect {
                let mut buf = Vec::with_capacity(16);
                let mut writer = Cursor::new(&mut buf);
                let school: u8 = script_effect.school.into();
                let flags = if script_effect.is_hostile { 1u32 } else { 0 };
                writer.write_le(&script_effect.script.0)?;
                writer.write_le(&(school as u32))?;
                match script_effect.visual_effect {
                    Some(ref effect) => {
                        writer.write_all(&effect.id())?;
                    }
                    None => {
                        writer.write_le(&0u32)?;
                    }
                }
                writer.write_le(&flags)?;

                record.add_field(Tes4Field::new(b"SCIT", buf)?);
                record.add_field(Tes4Field::new_zstring(b"FULL", script_effect.name.clone())?);
            }
        }

        Ok(())
    }
}
