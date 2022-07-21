use crate::tes4::{ActorValue, FormId, MagicEffectType, Tes4Field, Tes4Record, MAGIC_EFFECTS};
use crate::{decode_failed, decode_failed_because, EffectRange, Field, MagicSchool, TesError};
use binrw::{BinReaderExt, BinWriterExt};
use std::io::{Cursor, Write};

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

impl Default for SpellEffect {
    fn default() -> Self {
        SpellEffect {
            effect: MagicEffectType::AbsorbAttribute,
            magnitude: 0,
            area: 0,
            duration: 0,
            range: EffectRange::Self_,
            actor_value: ActorValue::Vampirism,
            script_effect: None,
        }
    }
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

    /// Gets the effect's script effect data, if any
    pub fn script_effect(&self) -> Option<&ScriptEffect> {
        self.script_effect.as_ref()
    }

    /// Sets the effect's script effect data
    pub fn set_script_effect(&mut self, script_effect: Option<ScriptEffect>) {
        self.script_effect = script_effect;
    }
}

/// A form which contains magic effects
pub trait Magic {
    /// Iterate through this magical entity's effects
    fn iter_effects(&self) -> Box<dyn Iterator<Item = &SpellEffect> + '_>;

    /// Iterate through this magical entity's effects mutably
    fn iter_effects_mut(&mut self) -> Box<dyn Iterator<Item = &mut SpellEffect> + '_>;

    /// Add an effect to this magical entity
    fn add_effect(&mut self, effect: SpellEffect);

    /// Get this magical entity's name if it has one
    fn name(&self) -> Option<&str>;

    /// Set this magical entity's name
    fn set_name(&mut self, name: Option<String>);

    /// Read magic data from a field
    fn read_magic_field(&mut self, field: &Tes4Field) -> Result<(), TesError> {
        match field.name() {
            b"EFID" => {
                let mut effect = SpellEffect::default();
                effect.effect = MagicEffectType::from_id_int(field.get_u32()?)
                    .ok_or_else(|| decode_failed("Unexpected effect ID"))?;
                self.add_effect(effect);
            }
            b"EFIT" => {
                if let Some(last_effect) = self.iter_effects_mut().last() {
                    let mut reader = field.reader();
                    last_effect.effect = MagicEffectType::from_id_int(reader.read_le()?)
                        .ok_or_else(|| decode_failed("Unexpected effect ID"))?;
                    last_effect.magnitude = reader.read_le()?;
                    last_effect.area = reader.read_le()?;
                    last_effect.duration = reader.read_le()?;
                    last_effect.range = EffectRange::try_from(reader.read_le::<u32>()? as u8)
                        .map_err(|e| decode_failed_because("Invalid effect range", e))?;
                    last_effect.actor_value = ActorValue::try_from(reader.read_le::<u32>()? as u8)
                        .map_err(|e| decode_failed_because("Invalid effect actor value", e))?;
                } else {
                    return Err(decode_failed("Orphaned EFIT field in magic record"));
                }
            }
            b"SCIT" => {
                if let Some(last_effect) = self.iter_effects_mut().last() {
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
                    return Err(decode_failed("Orphaned SCIT field in magic record"));
                }
            }
            b"FULL" => {
                if let Some(last_effect) = self.iter_effects_mut().last() {
                    if let Some(ref mut script_effect) = last_effect.script_effect {
                        script_effect.name = String::from(field.get_zstring()?);
                    } else {
                        return Err(decode_failed(
                            "Unexpected FULL field while parsing magic effect",
                        ));
                    }
                } else {
                    self.set_name(Some(String::from(field.get_zstring()?)));
                }
            }
            _ => {
                return Err(decode_failed(format!(
                    "Unexpected {} field while parsing magic effect",
                    field.name_as_str()
                )))
            }
        }

        Ok(())
    }

    /// Write zero or more magic effect data fields to the provided record
    fn write_magic_effects(&self, record: &mut Tes4Record) -> Result<(), TesError> {
        for effect in self.iter_effects() {
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
