use std::str;

use crate::{Attribute, EffectRange, Specialization};

use binrw::binrw;
use enum_map::{Enum, EnumMap};
use num_enum::{IntoPrimitive, TryFromPrimitive};

mod plugin;
pub use plugin::*;

mod world;
pub use world::*;

/// The different types of magical effects available in Morrowind
#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum MagicEffectType {
    WaterBreathing,
    SwiftSwim,
    WaterWalking,
    Shield,
    FireShield,
    LightningShield,
    FrostShield,
    Burden,
    Feather,
    Jump,
    Levitate,
    SlowFall,
    Lock,
    Open,
    FireDamage,
    ShockDamage,
    FrostDamage,
    DrainAttribute,
    DrainHealth,
    DrainMagicka,
    DrainFatigue,
    DrainSkill,
    DamageAttribute,
    DamageHealth,
    DamageMagicka,
    DamageFatigue,
    DamageSkill,
    Poison,
    WeaknessToFire,
    WeaknessToFrost,
    WeaknessToShock,
    WeaknessToMagicka,
    WeaknessToCommonDisease,
    WeaknessToBlightDisease,
    WeaknessToCorprusDisease,
    WeaknessToPoison,
    WeaknessToNormalWeapons,
    DisintegrateWeapon,
    DisintegrateArmor,
    Invisibility,
    Chameleon,
    Light,
    Sanctuary,
    NightEye,
    Charm,
    Paralyze,
    Silence,
    Blind,
    Sound,
    CalmHumanoid,
    CalmCreature,
    FrenzyHumanoid,
    FrenzyCreature,
    DemoralizeHumanoid,
    DemoralizeCreature,
    RallyHumanoid,
    RallyCreature,
    Dispel,
    Soultrap,
    Telekinesis,
    Mark,
    Recall,
    DivineIntervention,
    AlmsiviIntervention,
    DetectAnimal,
    DetectEnchantment,
    DetectKey,
    SpellAbsorption,
    Reflect,
    CureCommonDisease,
    CureBlightDisease,
    CureCorprusDisease,
    CurePoison,
    CureParalyzation,
    RestoreAttribute,
    RestoreHealth,
    RestoreMagicka,
    RestoreFatigue,
    RestoreSkill,
    FortifyAttribute,
    FortifyHealth,
    FortifyMagicka,
    FortifyFatigue,
    FortifySkill,
    FortifyMaximumMagicka,
    AbsorbAttribute,
    AbsorbHealth,
    AbsorbMagicka,
    AbsorbFatigue,
    AbsorbSkill,
    ResistFire,
    ResistFrost,
    ResistShock,
    ResistMagicka,
    ResistCommonDisease,
    ResistBlightDisease,
    ResistCorprusDisease,
    ResistPoison,
    ResistNormalWeapons,
    ResistParalysis,
    RemoveCurse,
    TurnUndead,
    SummonScamp,
    SummonClannfear,
    SummonDaedroth,
    SummonDremora,
    SummonAncestralGhost,
    SummonSkeletalMinion,
    SummonBonewalker,
    SummonGreaterBonewalker,
    SummonBonelord,
    SummonWingedTwilight,
    SummonHunger,
    SummonGoldenSaint,
    SummonFlameAtronach,
    SummonFrostAtronach,
    SummonStormAtronach,
    FortifyAttack,
    CommandCreature,
    CommandHumanoid,
    BoundDagger,
    BoundLongsword,
    BoundMace,
    BoundBattleAxe,
    BoundSpear,
    BoundLongbow,
    ExtraSpell,
    BoundCuirass,
    BoundHelm,
    BoundBoots,
    BoundShield,
    BoundGloves,
    Corprus,
    Vampirism,
    SummonCenturionSphere,
    SunDamage,
    StuntedMagicka,
    SummonFabricant,
    CallWolf,
    CallBear,
    SummonBonewolf,
    SummonCreature04,
    SummonCreature05,
}

/// An individual effect of a spell or potion
#[binrw]
#[derive(Debug)]
pub struct SpellEffect {
    #[br(try_map = |e: u16| MagicEffectType::try_from(e as u8))]
    #[bw(map = |e| *e as u16)]
    effect: MagicEffectType,
    #[br(try_map = |s: u8| if s == 0xff { Ok(None) } else { Skill::try_from(s).map(|v| Some(v)) })]
    #[bw(map = |s| s.map_or(0xff, |v| v as u8))]
    skill: Option<Skill>,
    #[br(try_map = |a: u8| if a == 0xff { Ok(None) } else { Attribute::try_from(a).map(|v| Some(v)) })]
    #[bw(map = |a| a.map_or(0xff, |v| v as u8))]
    attribute: Option<Attribute>,
    #[br(try_map = |e: u32| EffectRange::try_from(e as u8))]
    #[bw(map = |e| *e as u32)]
    range: EffectRange,
    area: u32,
    duration: u32,
    min_magnitude: u32,
    max_magnitude: u32,
}

impl SpellEffect {
    pub fn effect(&self) -> MagicEffectType {
        self.effect
    }

    pub fn skill(&self) -> Option<Skill> {
        self.skill
    }

    pub fn attribute(&self) -> Option<Attribute> {
        self.attribute
    }

    /// Gets the spell's range
    pub fn range(&self) -> EffectRange {
        self.range
    }

    /// Gets the spell's area
    pub fn area(&self) -> u32 {
        self.area
    }

    /// Gets the spell's duration
    pub fn duration(&self) -> u32 {
        self.duration
    }

    /// Get the spell's range of magnitude
    pub fn magnitude(&self) -> (u32, u32) {
        (self.min_magnitude, self.max_magnitude)
    }
}

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
