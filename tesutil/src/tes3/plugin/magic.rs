use crate::tes3::Skill;
use crate::{Attribute, EffectRange};
use binrw::binrw;
use num_enum::{IntoPrimitive, TryFromPrimitive};

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

/// A form which contains magic effects
pub trait Magic {
    /// Iterate through this magical entity's effects
    fn iter_effects(&self) -> Box<dyn Iterator<Item = &SpellEffect> + '_>;

    /// Iterate through this magical entity's effects mutably
    fn iter_effects_mut(&mut self) -> Box<dyn Iterator<Item = &mut SpellEffect> + '_>;

    /// Add an effect to this magical entity
    fn add_effect(&mut self, effect: SpellEffect);
}
