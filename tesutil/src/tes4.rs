use std::convert::TryFrom;

use crate::{Attribute, Specialization, TesError};
use enum_map::{Enum, EnumMap};
use num_enum::TryFromPrimitive;

pub mod plugin;
pub mod save;

mod world;
pub use world::*;

/// All possible actor values
#[derive(Copy, Clone, Debug, Enum, PartialEq, Eq, TryFromPrimitive)]
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
