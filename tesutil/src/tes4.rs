use std::convert::TryFrom;

use crate::{Attribute, Specialization, TesError};
use binrw::binrw;
use enum_map::{Enum, EnumMap};
use num_enum::{IntoPrimitive, TryFromPrimitive};

mod plugin;
pub use plugin::*;

pub mod save;

mod world;
pub use world::*;

pub mod cosave;

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
