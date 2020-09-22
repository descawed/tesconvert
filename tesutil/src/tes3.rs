use std::str;

use enum_map::{Enum, EnumMap};
use num_enum::TryFromPrimitive;

use crate::Specialization;

mod plugin;
pub use plugin::*;

mod world;
pub use world::*;

/// All possible skills
#[derive(Copy, Clone, Debug, Enum, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum Skill {
    Block,
    Armorer,
    MediumArmor,
    HeavyArmor,
    Blunt,
    LongBlade,
    Axe,
    Spear,
    Athletics,
    Enchant,
    Destruction,
    Alteration,
    Illusion,
    Conjuration,
    Mysticism,
    Restoration,
    Alchemy,
    Unarmored,
    Security,
    Sneak,
    Acrobatics,
    LightArmor,
    ShortBlade,
    Marksman,
    Mercantile,
    Speechcraft,
    HandToHand,
}

impl Skill {
    /// Returns an iterator over all skills
    pub fn iter() -> impl Iterator<Item = Skill> {
        use Skill::*;
        static SKILLS: [Skill; 27] = [
            Block,
            Armorer,
            MediumArmor,
            HeavyArmor,
            Blunt,
            LongBlade,
            Axe,
            Spear,
            Athletics,
            Enchant,
            Destruction,
            Alteration,
            Illusion,
            Conjuration,
            Mysticism,
            Restoration,
            Alchemy,
            Unarmored,
            Security,
            Sneak,
            Acrobatics,
            LightArmor,
            ShortBlade,
            Marksman,
            Mercantile,
            Speechcraft,
            HandToHand,
        ];

        SKILLS.iter().copied()
    }

    /// Gets the skill's specialization
    pub fn specialization(&self) -> Specialization {
        match *self {
            Skill::HeavyArmor => Specialization::Combat,
            Skill::MediumArmor => Specialization::Combat,
            Skill::Spear => Specialization::Combat,
            Skill::Armorer => Specialization::Combat,
            Skill::Axe => Specialization::Combat,
            Skill::Blunt => Specialization::Combat,
            Skill::LongBlade => Specialization::Combat,
            Skill::Block => Specialization::Combat,
            Skill::Athletics => Specialization::Combat,
            Skill::Acrobatics => Specialization::Stealth,
            Skill::LightArmor => Specialization::Stealth,
            Skill::Marksman => Specialization::Stealth,
            Skill::Sneak => Specialization::Stealth,
            Skill::HandToHand => Specialization::Stealth,
            Skill::ShortBlade => Specialization::Stealth,
            Skill::Mercantile => Specialization::Stealth,
            Skill::Speechcraft => Specialization::Stealth,
            Skill::Security => Specialization::Stealth,
            _ => Specialization::Magic,
        }
    }
}

/// Character skills
pub type Skills<T> = EnumMap<Skill, T>;

/// Type of skill for a class (major, minor, miscellaneous)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SkillType {
    Major,
    Minor,
    Miscellaneous,
}
