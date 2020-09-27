use std::convert::TryFrom;
use std::io::Read;

use crate::tes3::{Skill, Tes3Field, Tes3Record};
use crate::{
    decode_failed, decode_failed_because, extract, Attribute, EffectRange, Field, Form, Record,
    TesError,
};

use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum MagicEffect {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum SpellType {
    Spell,
    Ability,
    Blight,
    Disease,
    Curse,
    Power,
}

bitflags! {
    pub struct SpellFlags: u32 {
        const AUTO_CALC = 0x01;
        const PC_START_SPELL = 0x02;
        const ALWAYS_SUCCEEDS = 0x04;
    }
}

/// An individual effect of a spell
#[derive(Debug)]
pub struct SpellEffect {
    effect: MagicEffect,
    skill: Option<Skill>,
    attribute: Option<Attribute>,
    range: EffectRange,
    area: u32,
    duration: u32,
    min_magnitude: u32,
    max_magnitude: u32,
}

/// A spell, ability, or disease
#[derive(Debug)]
pub struct Spell {
    id: String,
    name: String,
    spell_type: SpellType,
    cost: u32,
    flags: SpellFlags,
    effects: Vec<SpellEffect>,
}

impl Form for Spell {
    type Field = Tes3Field;
    type Record = Tes3Record;

    fn record_type() -> &'static [u8; 4] {
        b"SPEL"
    }

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Spell::assert(&record)?;

        let mut spell = Spell {
            id: String::new(),
            name: String::new(),
            spell_type: SpellType::Spell,
            cost: 0,
            flags: SpellFlags::empty(),
            effects: vec![],
        };

        for field in record.iter() {
            match field.name() {
                b"NAME" => spell.id = String::from(field.get_zstring()?),
                b"FNAM" => spell.name = String::from(field.get_zstring()?),
                b"SPDT" => {
                    let mut reader = field.reader();
                    spell.spell_type = SpellType::try_from(extract!(reader as u32)? as u8)
                        .map_err(|e| decode_failed_because("Invalid spell type", e))?;
                    spell.cost = extract!(reader as u32)?;
                    spell.flags = SpellFlags::from_bits(extract!(reader as u32)?)
                        .ok_or_else(|| decode_failed("Invalid spell flags"))?;
                }
                b"ENAM" => {
                    let mut reader = field.reader();
                    spell.effects.push(SpellEffect {
                        effect: MagicEffect::try_from(extract!(reader as u16)? as u8)
                            .map_err(|e| decode_failed_because("Invalid magic effect", e))?,
                        skill: {
                            let skill = extract!(reader as u8)?;
                            if skill == 0xff {
                                None
                            } else {
                                Some(
                                    Skill::try_from(skill).map_err(|e| {
                                        decode_failed_because("Invalid skill value", e)
                                    })?,
                                )
                            }
                        },
                        attribute: {
                            let attribute = extract!(reader as u8)?;
                            if attribute == 0xff {
                                None
                            } else {
                                Some(Attribute::try_from(attribute).map_err(|e| {
                                    decode_failed_because("Invalid attribute value", e)
                                })?)
                            }
                        },
                        range: EffectRange::try_from(extract!(reader as u32)? as u8)
                            .map_err(|e| decode_failed_because("Invalid effect range", e))?,
                        area: extract!(reader as u32)?,
                        duration: extract!(reader as u32)?,
                        min_magnitude: extract!(reader as u32)?,
                        max_magnitude: extract!(reader as u32)?,
                    });
                }
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
