use std::convert::TryFrom;
use std::io::{Cursor, Read, Write};

use crate::{
    decode_failed, decode_failed_because, Attribute, EffectRange, Field, MagicSchool,
    Specialization, TesError,
};
use binrw::{binrw, BinReaderExt, BinWriterExt};
use bitflags::bitflags;
use enum_map::{Enum, EnumMap};
use num_enum::{IntoPrimitive, TryFromPrimitive};

mod plugin;
pub use plugin::*;

pub mod save;

mod world;
pub use world::*;

pub mod cosave;

bitflags! {
    #[derive(Default)]
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

    /// Load part of this magic effect from the given field
    pub fn load_from_field(&mut self, field: &Tes4Field) -> Result<(), TesError> {
        match field.name() {
            b"EFID" => {
                self.effect = MagicEffectType::from_id_int(field.get_u32()?)
                    .ok_or_else(|| decode_failed("Unexpected effect ID"))?
            }
            b"EFIT" => {
                let mut reader = field.reader();
                self.effect = MagicEffectType::from_id_int(reader.read_le()?)
                    .ok_or_else(|| decode_failed("Unexpected effect ID"))?;
                self.magnitude = reader.read_le()?;
                self.area = reader.read_le()?;
                self.duration = reader.read_le()?;
                self.range = EffectRange::try_from(reader.read_le::<u32>()? as u8)
                    .map_err(|e| decode_failed_because("Invalid effect range", e))?;
                self.actor_value = ActorValue::try_from(reader.read_le::<u32>()? as u8)
                    .map_err(|e| decode_failed_because("Invalid effect actor value", e))?;
            }
            b"SCIT" => {
                let mut reader = field.reader();
                self.script_effect = Some(ScriptEffect {
                    script: FormId(reader.read_le()?),
                    school: MagicSchool::try_from(reader.read_le::<u32>()? as u8).map_err(|e| {
                        decode_failed_because("Invalid script effect magic school", e)
                    })?,
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
            }
            b"FULL" => {
                if let Some(ref mut script_effect) = self.script_effect {
                    script_effect.name = String::from(field.get_zstring()?);
                } else {
                    return Err(decode_failed(
                        "Unexpected FULL field while parsing magic effect",
                    ));
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

    /// Create a series of fields representing this effect
    pub fn to_fields(&self) -> Result<Vec<Tes4Field>, TesError> {
        let mut fields = vec![];

        let effect_id = self.effect.id();
        fields.push(Tes4Field::new(b"EFID", effect_id.to_vec())?);

        let mut buf = Vec::with_capacity(24);
        let mut writer = Cursor::new(&mut buf);
        let range: u8 = self.range.into();
        let av: u8 = self.actor_value.into();
        writer.write_all(&effect_id)?;
        writer.write_le(&self.magnitude)?;
        writer.write_le(&self.area)?;
        writer.write_le(&self.duration)?;
        writer.write_le(&(range as u32))?;
        writer.write_le(&(av as u32))?;

        fields.push(Tes4Field::new(b"EFIT", buf)?);

        if let Some(ref script_effect) = self.script_effect {
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

            fields.push(Tes4Field::new(b"SCIT", buf)?);
            fields.push(Tes4Field::new_zstring(b"FULL", script_effect.name.clone())?);
        }

        Ok(fields)
    }
}

/// A unique identifier for a record
#[binrw]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct FormId(pub u32);

impl FormId {
    /// Gets a form ID's index (i.e., which plugin in the load order it belongs to)
    pub fn index(&self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// Sets a form ID's index
    pub fn set_index(&mut self, index: u8) {
        self.0 = ((index as u32) << 24) | (self.0 & 0xffffff);
    }
}

/// Container for a search for a form by either plugin or index
#[derive(Debug)]
pub enum FindForm<'a> {
    ByMaster(Option<&'a str>, u32),
    ByIndex(FormId),
}

impl<'a> FindForm<'a> {
    /// Get a concrete form ID from a FindForm
    pub fn form_id<'b, T: Iterator<Item = &'b str>>(&self, mut plugins: T) -> Option<FormId> {
        Some(match self {
            FindForm::ByMaster(plugin, id) => {
                let mut id = FormId(*id);
                let index = match *plugin {
                    Some(name) => plugins.position(|p| p == name)?,
                    None => plugins.count(),
                } as u8;

                if index == 0xff {
                    // index FF is reserved for saves and will never be a plugin index
                    return None;
                }

                id.set_index(index);
                id
            }
            FindForm::ByIndex(id) => *id,
        })
    }

    /// Creates an error referring to this form search
    pub fn err(&self) -> TesError {
        match self {
            FindForm::ByMaster(plugin, id) => TesError::InvalidPluginForm {
                plugin: String::from(plugin.unwrap_or("<none>")),
                form_id: FormId(*id),
            },
            FindForm::ByIndex(id) => TesError::InvalidFormId { form_id: *id },
        }
    }
}

/// All possible actor values
#[derive(Copy, Clone, Debug, Enum, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum ActorValue {
    Strength,
    Intelligence,
    Willpower,
    Agility,
    Speed,
    Endurance,
    Personality,
    Luck,
    Health,
    Magicka,
    Fatigue,
    Encumbrance,
    Armorer,
    Athletics,
    Blade,
    Block,
    Blunt,
    HandToHand,
    HeavyArmor,
    Alchemy,
    Alteration,
    Conjuration,
    Destruction,
    Illusion,
    Mysticism,
    Restoration,
    Acrobatics,
    LightArmor,
    Marksman,
    Mercantile,
    Security,
    Sneak,
    Speechcraft,
    Aggression,
    Confidence,
    Energy,
    Responsibility,
    Bounty,
    Fame,
    Infamy,
    MagickaMultiplier,
    NightEyeBonus,
    AttackBonus,
    DefendBonus,
    CastingPenalty,
    Blindness,
    Chameleon,
    Invisibility,
    Paralysis,
    Silence,
    Confusion,
    DetectItemRange,
    SpellAbsorbChance,
    SpellReflectChance,
    SwimSpeedMultiplier,
    WaterBreathing,
    WaterWalking,
    StuntedMagicka,
    DetectLifeRange,
    ReflectDamage,
    Telekinesis,
    ResistFire,
    ResistFrost,
    ResistDisease,
    ResistMagic,
    ResistNormalWeapons,
    ResistParalysis,
    ResistPoison,
    ResistShock,
    Vampirism,
    Darkness,
    ResistWaterDamage,
}

impl From<Skill> for ActorValue {
    fn from(value: Skill) -> Self {
        match value {
            Skill::Armorer => ActorValue::Armorer,
            Skill::Athletics => ActorValue::Athletics,
            Skill::Blade => ActorValue::Blade,
            Skill::Block => ActorValue::Block,
            Skill::Blunt => ActorValue::Blunt,
            Skill::HandToHand => ActorValue::HandToHand,
            Skill::HeavyArmor => ActorValue::HeavyArmor,
            Skill::Alchemy => ActorValue::Alchemy,
            Skill::Alteration => ActorValue::Alteration,
            Skill::Conjuration => ActorValue::Conjuration,
            Skill::Destruction => ActorValue::Destruction,
            Skill::Illusion => ActorValue::Illusion,
            Skill::Mysticism => ActorValue::Mysticism,
            Skill::Restoration => ActorValue::Restoration,
            Skill::Acrobatics => ActorValue::Acrobatics,
            Skill::LightArmor => ActorValue::LightArmor,
            Skill::Marksman => ActorValue::Marksman,
            Skill::Mercantile => ActorValue::Mercantile,
            Skill::Security => ActorValue::Security,
            Skill::Sneak => ActorValue::Sneak,
            Skill::Speechcraft => ActorValue::Speechcraft,
        }
    }
}

impl From<Attribute> for ActorValue {
    fn from(value: Attribute) -> Self {
        match value {
            Attribute::Strength => ActorValue::Strength,
            Attribute::Intelligence => ActorValue::Intelligence,
            Attribute::Willpower => ActorValue::Willpower,
            Attribute::Agility => ActorValue::Agility,
            Attribute::Speed => ActorValue::Speed,
            Attribute::Endurance => ActorValue::Endurance,
            Attribute::Personality => ActorValue::Personality,
            Attribute::Luck => ActorValue::Luck,
        }
    }
}

/// Character actor values
pub type ActorValues<T> = EnumMap<ActorValue, T>;

/// All possible skills
#[derive(Copy, Clone, Debug, Enum, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum Skill {
    Armorer,
    Athletics,
    Blade,
    Block,
    Blunt,
    HandToHand,
    HeavyArmor,
    Alchemy,
    Alteration,
    Conjuration,
    Destruction,
    Illusion,
    Mysticism,
    Restoration,
    Acrobatics,
    LightArmor,
    Marksman,
    Mercantile,
    Security,
    Sneak,
    Speechcraft,
}

impl Skill {
    /// Gets the skill's specialization
    pub fn specialization(&self) -> Specialization {
        match *self {
            Skill::HeavyArmor => Specialization::Combat,
            Skill::Armorer => Specialization::Combat,
            Skill::Blade => Specialization::Combat,
            Skill::Blunt => Specialization::Combat,
            Skill::Block => Specialization::Combat,
            Skill::Athletics => Specialization::Combat,
            Skill::HandToHand => Specialization::Combat,
            Skill::Acrobatics => Specialization::Stealth,
            Skill::LightArmor => Specialization::Stealth,
            Skill::Marksman => Specialization::Stealth,
            Skill::Sneak => Specialization::Stealth,
            Skill::Mercantile => Specialization::Stealth,
            Skill::Speechcraft => Specialization::Stealth,
            Skill::Security => Specialization::Stealth,
            _ => Specialization::Magic,
        }
    }
}

impl TryFrom<ActorValue> for Skill {
    type Error = TesError;

    fn try_from(value: ActorValue) -> Result<Self, Self::Error> {
        match value {
            ActorValue::Armorer => Ok(Skill::Armorer),
            ActorValue::Athletics => Ok(Skill::Athletics),
            ActorValue::Blade => Ok(Skill::Blade),
            ActorValue::Block => Ok(Skill::Block),
            ActorValue::Blunt => Ok(Skill::Blunt),
            ActorValue::HandToHand => Ok(Skill::HandToHand),
            ActorValue::HeavyArmor => Ok(Skill::HeavyArmor),
            ActorValue::Alchemy => Ok(Skill::Alchemy),
            ActorValue::Alteration => Ok(Skill::Alteration),
            ActorValue::Conjuration => Ok(Skill::Conjuration),
            ActorValue::Destruction => Ok(Skill::Destruction),
            ActorValue::Illusion => Ok(Skill::Illusion),
            ActorValue::Mysticism => Ok(Skill::Mysticism),
            ActorValue::Restoration => Ok(Skill::Restoration),
            ActorValue::Acrobatics => Ok(Skill::Acrobatics),
            ActorValue::LightArmor => Ok(Skill::LightArmor),
            ActorValue::Marksman => Ok(Skill::Marksman),
            ActorValue::Mercantile => Ok(Skill::Mercantile),
            ActorValue::Security => Ok(Skill::Security),
            ActorValue::Sneak => Ok(Skill::Sneak),
            ActorValue::Speechcraft => Ok(Skill::Speechcraft),
            invalid => Err(TesError::InvalidMapping(
                format!("{:?}", invalid),
                String::from("Skill"),
            )),
        }
    }
}

/// Character skills
pub type Skills<T> = EnumMap<Skill, T>;
