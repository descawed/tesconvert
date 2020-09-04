use enum_map::{Enum, EnumMap};

pub mod plugin;

mod world;
pub use world::*;

/// All possible skills
#[derive(Copy, Clone, Debug, Enum)]
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

/// Character skills
pub type Skills<T> = EnumMap<Skill, T>;
