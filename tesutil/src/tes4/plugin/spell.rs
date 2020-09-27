use std::convert::TryFrom;
use std::io::{Read, Write};

use crate::tes4::{ActorValue, FormId, Tes4Field, Tes4Record};
use crate::{
    decode_failed, decode_failed_because, extract, serialize, EffectRange, Field, Form,
    MagicSchool, Record, TesError, WriteExact,
};

use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MagicEffect {
    AbsorbAttribute,
    AbsorbFatigue,
    AbsorbHealth,
    AbsorbSkill,
    AbsorbMagicka,
    BoundBoots,
    BoundCuirass,
    BoundGauntlets,
    BoundGreaves,
    BoundHelmet,
    BoundShield,
    Burden,
    BoundOrderWeapon1,
    BoundOrderWeapon2,
    BoundOrderWeapon3,
    BoundOrderWeapon4,
    BoundOrderWeapon5,
    BoundOrderWeapon6,
    SummonStaffOfSheogorath,
    BoundPriestDagger,
    BoundAxe,
    BoundBow,
    BoundDagger,
    BoundMace,
    BoundSword,
    Calm,
    Chameleon,
    Charm,
    CommandCreature,
    CommandHumanoid,
    CureDisease,
    CureParalysis,
    CurePoison,
    Darkness,
    Demoralize,
    DamageAttribute,
    DamageFatigue,
    DamageHealth,
    DamageMagicka,
    DisintegrateArmor,
    DiseaseInfo,
    DisintegrateWeapon,
    DrainAttribute,
    DrainFatigue,
    DrainHealth,
    DrainSkill,
    DrainMagicka,
    Dispel,
    DetectLife,
    MehrunesDagonCustomEffect,
    FireDamage,
    FireShield,
    FortifyAttribute,
    FortifyFatigue,
    FortifyHealth,
    FortifyMagickaMultiplier,
    FortifySkill,
    FortifyMagicka,
    FrostDamage,
    Frenzy,
    FrostShield,
    Feather,
    Invisibility,
    Light,
    ShockShield,
    Lock,
    SummonMythicDawnHelm,
    SummonMythicDawnArmor,
    NightEye,
    Open,
    Paralyze,
    PoisonInfo,
    Rally,
    Reanimate,
    RestoreAttribute,
    ReflectDamage,
    RestoreFatigue,
    RestoreHealth,
    RestoreMagicka,
    ReflectSpell,
    ResistDisease,
    ResistFire,
    ResistFrost,
    ResistMagic,
    ResistNormalWeapons,
    ResistParalysis,
    ResistPoison,
    ResistShock,
    ResistWaterDamage,
    SpellAbsorption,
    ScriptEffect,
    ShockDamage,
    Shield,
    Silence,
    StuntedMagicka,
    SoulTrap,
    SunDamage,
    Telekinesis,
    TurnUndead,
    Vampirism,
    WaterBreathing,
    WaterWalking,
    WeaknessToDisease,
    WeaknessToFire,
    WeaknessToFrost,
    WeaknessToMagic,
    WeaknessToNormalWeapons,
    WeaknessToPoison,
    WeaknessToShock,
    SummonRufiosGhost,
    SummonAncestorGuardian,
    SummonSpiderling,
    SummonFleshAtronach,
    SummonBear,
    SummonGluttonousHunger,
    SummonRavenousHunger,
    SummonVoraciousHunger,
    SummonDarkSeducer,
    SummonGoldenSaint,
    WabbaSummon,
    SummonDecrepitShambles,
    SummonShambles,
    SummonRepleteShambles,
    SummonHunger,
    SummonMangledFleshAtronach,
    SummonTornFleshAtronach,
    SummonStitchedFleshAtronach,
    SummonSewnFleshAtronach,
    ExtraSummon20,
    SummonClannfear,
    SummonDaedroth,
    SummonDremora,
    SummonDremoraLord,
    SummonFlameAtronach,
    SummonFrostAtronach,
    SummonGhost,
    SummonHeadlessZombie,
    SummonLich,
    SummonScamp,
    SummonSkeletonGuardian,
    SummonSkeletonChampion,
    SummonSkeleton,
    SummonSkeletonHero,
    SummonSpiderDaedra,
    SummonStormAtronach,
    SummonFadedWraith,
    SummonGloomWraith,
    SummonXivilai,
    SummonZombie,
}

impl MagicEffect {
    /// Gets the 4-byte effect ID for this magic effect
    pub fn id(&self) -> [u8; 4] {
        use MagicEffect::*;

        *match self {
            AbsorbAttribute => b"ABAT",
            AbsorbFatigue => b"ABFA",
            AbsorbHealth => b"ABHE",
            AbsorbSkill => b"ABSK",
            AbsorbMagicka => b"ABSP",
            BoundBoots => b"BABO",
            BoundCuirass => b"BACU",
            BoundGauntlets => b"BAGA",
            BoundGreaves => b"BAGR",
            BoundHelmet => b"BAHE",
            BoundShield => b"BASH",
            Burden => b"BRDN",
            BoundOrderWeapon1 => b"BW01",
            BoundOrderWeapon2 => b"BW02",
            BoundOrderWeapon3 => b"BW03",
            BoundOrderWeapon4 => b"BW04",
            BoundOrderWeapon5 => b"BW05",
            BoundOrderWeapon6 => b"BW06",
            SummonStaffOfSheogorath => b"BW07",
            BoundPriestDagger => b"BW08",
            BoundAxe => b"BWAX",
            BoundBow => b"BWBO",
            BoundDagger => b"BWDA",
            BoundMace => b"BWMA",
            BoundSword => b"BWSW",
            Calm => b"CALM",
            Chameleon => b"CHML",
            Charm => b"CHRM",
            CommandCreature => b"COCR",
            CommandHumanoid => b"COHU",
            CureDisease => b"CUDI",
            CureParalysis => b"CUPA",
            CurePoison => b"CUPO",
            Darkness => b"DARK",
            Demoralize => b"DEMO",
            DamageAttribute => b"DGAT",
            DamageFatigue => b"DGFA",
            DamageHealth => b"DGHE",
            DamageMagicka => b"DGSP",
            DisintegrateArmor => b"DIAR",
            DiseaseInfo => b"DISE",
            DisintegrateWeapon => b"DIWE",
            DrainAttribute => b"DRAT",
            DrainFatigue => b"DRFA",
            DrainHealth => b"DRHE",
            DrainSkill => b"DRSK",
            DrainMagicka => b"DRSP",
            Dispel => b"DSPL",
            DetectLife => b"DTCT",
            MehrunesDagonCustomEffect => b"DUMY",
            FireDamage => b"FIDG",
            FireShield => b"FISH",
            FortifyAttribute => b"FOAT",
            FortifyFatigue => b"FOFA",
            FortifyHealth => b"FOHE",
            FortifyMagickaMultiplier => b"FOMM",
            FortifySkill => b"FOSK",
            FortifyMagicka => b"FOSP",
            FrostDamage => b"FRDG",
            Frenzy => b"FRNZ",
            FrostShield => b"FRSH",
            Feather => b"FTHR",
            Invisibility => b"INVI",
            Light => b"LGHT",
            ShockShield => b"LISH",
            Lock => b"LOCK",
            SummonMythicDawnHelm => b"MYHL",
            SummonMythicDawnArmor => b"MYTH",
            NightEye => b"NEYE",
            Open => b"OPEN",
            Paralyze => b"PARA",
            PoisonInfo => b"POSN",
            Rally => b"RALY",
            Reanimate => b"REAN",
            RestoreAttribute => b"REAT",
            ReflectDamage => b"REDG",
            RestoreFatigue => b"REFA",
            RestoreHealth => b"REHE",
            RestoreMagicka => b"RESP",
            ReflectSpell => b"RFLC",
            ResistDisease => b"RSDI",
            ResistFire => b"RSFI",
            ResistFrost => b"RSFR",
            ResistMagic => b"RSMA",
            ResistNormalWeapons => b"RSNW",
            ResistParalysis => b"RSPA",
            ResistPoison => b"RSPO",
            ResistShock => b"RSSH",
            ResistWaterDamage => b"RSWD",
            SpellAbsorption => b"SABS",
            ScriptEffect => b"SEFF",
            ShockDamage => b"SHDG",
            Shield => b"SHLD",
            Silence => b"SLNC",
            StuntedMagicka => b"STMA",
            SoulTrap => b"STRP",
            SunDamage => b"SUDG",
            Telekinesis => b"TELE",
            TurnUndead => b"TURN",
            Vampirism => b"VAMP",
            WaterBreathing => b"WABR",
            WaterWalking => b"WAWA",
            WeaknessToDisease => b"WKDI",
            WeaknessToFire => b"WKFI",
            WeaknessToFrost => b"WKFR",
            WeaknessToMagic => b"WKMA",
            WeaknessToNormalWeapons => b"WKNW",
            WeaknessToPoison => b"WKPO",
            WeaknessToShock => b"WKSH",
            SummonRufiosGhost => b"Z001",
            SummonAncestorGuardian => b"Z002",
            SummonSpiderling => b"Z003",
            SummonFleshAtronach => b"Z004",
            SummonBear => b"Z005",
            SummonGluttonousHunger => b"Z006",
            SummonRavenousHunger => b"Z007",
            SummonVoraciousHunger => b"Z008",
            SummonDarkSeducer => b"Z009",
            SummonGoldenSaint => b"Z010",
            WabbaSummon => b"Z011",
            SummonDecrepitShambles => b"Z012",
            SummonShambles => b"Z013",
            SummonRepleteShambles => b"Z014",
            SummonHunger => b"Z015",
            SummonMangledFleshAtronach => b"Z016",
            SummonTornFleshAtronach => b"Z017",
            SummonStitchedFleshAtronach => b"Z018",
            SummonSewnFleshAtronach => b"Z019",
            ExtraSummon20 => b"Z020",
            SummonClannfear => b"ZCLA",
            SummonDaedroth => b"ZDAE",
            SummonDremora => b"ZDRE",
            SummonDremoraLord => b"ZDRL",
            SummonFlameAtronach => b"ZFIA",
            SummonFrostAtronach => b"ZFRA",
            SummonGhost => b"ZGHO",
            SummonHeadlessZombie => b"ZHDZ",
            SummonLich => b"ZLIC",
            SummonScamp => b"ZSCA",
            SummonSkeletonGuardian => b"ZSKA",
            SummonSkeletonChampion => b"ZSKC",
            SummonSkeleton => b"ZSKE",
            SummonSkeletonHero => b"ZSKH",
            SummonSpiderDaedra => b"ZSPD",
            SummonStormAtronach => b"ZSTA",
            SummonFadedWraith => b"ZWRA",
            SummonGloomWraith => b"ZWRL",
            SummonXivilai => b"ZXIV",
            SummonZombie => b"ZZOM",
        }
    }

    /// Gets a magic effect from a 4-byte ID, if the ID is valid
    pub fn from_id(id: &[u8; 4]) -> Option<MagicEffect> {
        use MagicEffect::*;

        match id {
            b"ABAT" => Some(AbsorbAttribute),
            b"ABFA" => Some(AbsorbFatigue),
            b"ABHE" => Some(AbsorbHealth),
            b"ABSK" => Some(AbsorbSkill),
            b"ABSP" => Some(AbsorbMagicka),
            b"BABO" => Some(BoundBoots),
            b"BACU" => Some(BoundCuirass),
            b"BAGA" => Some(BoundGauntlets),
            b"BAGR" => Some(BoundGreaves),
            b"BAHE" => Some(BoundHelmet),
            b"BASH" => Some(BoundShield),
            b"BRDN" => Some(Burden),
            b"BW01" => Some(BoundOrderWeapon1),
            b"BW02" => Some(BoundOrderWeapon2),
            b"BW03" => Some(BoundOrderWeapon3),
            b"BW04" => Some(BoundOrderWeapon4),
            b"BW05" => Some(BoundOrderWeapon5),
            b"BW06" => Some(BoundOrderWeapon6),
            b"BW07" => Some(SummonStaffOfSheogorath),
            b"BW08" => Some(BoundPriestDagger),
            b"BWAX" => Some(BoundAxe),
            b"BWBO" => Some(BoundBow),
            b"BWDA" => Some(BoundDagger),
            b"BWMA" => Some(BoundMace),
            b"BWSW" => Some(BoundSword),
            b"CALM" => Some(Calm),
            b"CHML" => Some(Chameleon),
            b"CHRM" => Some(Charm),
            b"COCR" => Some(CommandCreature),
            b"COHU" => Some(CommandHumanoid),
            b"CUDI" => Some(CureDisease),
            b"CUPA" => Some(CureParalysis),
            b"CUPO" => Some(CurePoison),
            b"DARK" => Some(Darkness),
            b"DEMO" => Some(Demoralize),
            b"DGAT" => Some(DamageAttribute),
            b"DGFA" => Some(DamageFatigue),
            b"DGHE" => Some(DamageHealth),
            b"DGSP" => Some(DamageMagicka),
            b"DIAR" => Some(DisintegrateArmor),
            b"DISE" => Some(DiseaseInfo),
            b"DIWE" => Some(DisintegrateWeapon),
            b"DRAT" => Some(DrainAttribute),
            b"DRFA" => Some(DrainFatigue),
            b"DRHE" => Some(DrainHealth),
            b"DRSK" => Some(DrainSkill),
            b"DRSP" => Some(DrainMagicka),
            b"DSPL" => Some(Dispel),
            b"DTCT" => Some(DetectLife),
            b"DUMY" => Some(MehrunesDagonCustomEffect),
            b"FIDG" => Some(FireDamage),
            b"FISH" => Some(FireShield),
            b"FOAT" => Some(FortifyAttribute),
            b"FOFA" => Some(FortifyFatigue),
            b"FOHE" => Some(FortifyHealth),
            b"FOMM" => Some(FortifyMagickaMultiplier),
            b"FOSK" => Some(FortifySkill),
            b"FOSP" => Some(FortifyMagicka),
            b"FRDG" => Some(FrostDamage),
            b"FRNZ" => Some(Frenzy),
            b"FRSH" => Some(FrostShield),
            b"FTHR" => Some(Feather),
            b"INVI" => Some(Invisibility),
            b"LGHT" => Some(Light),
            b"LISH" => Some(ShockShield),
            b"LOCK" => Some(Lock),
            b"MYHL" => Some(SummonMythicDawnHelm),
            b"MYTH" => Some(SummonMythicDawnArmor),
            b"NEYE" => Some(NightEye),
            b"OPEN" => Some(Open),
            b"PARA" => Some(Paralyze),
            b"POSN" => Some(PoisonInfo),
            b"RALY" => Some(Rally),
            b"REAN" => Some(Reanimate),
            b"REAT" => Some(RestoreAttribute),
            b"REDG" => Some(ReflectDamage),
            b"REFA" => Some(RestoreFatigue),
            b"REHE" => Some(RestoreHealth),
            b"RESP" => Some(RestoreMagicka),
            b"RFLC" => Some(ReflectSpell),
            b"RSDI" => Some(ResistDisease),
            b"RSFI" => Some(ResistFire),
            b"RSFR" => Some(ResistFrost),
            b"RSMA" => Some(ResistMagic),
            b"RSNW" => Some(ResistNormalWeapons),
            b"RSPA" => Some(ResistParalysis),
            b"RSPO" => Some(ResistPoison),
            b"RSSH" => Some(ResistShock),
            b"RSWD" => Some(ResistWaterDamage),
            b"SABS" => Some(SpellAbsorption),
            b"SEFF" => Some(ScriptEffect),
            b"SHDG" => Some(ShockDamage),
            b"SHLD" => Some(Shield),
            b"SLNC" => Some(Silence),
            b"STMA" => Some(StuntedMagicka),
            b"STRP" => Some(SoulTrap),
            b"SUDG" => Some(SunDamage),
            b"TELE" => Some(Telekinesis),
            b"TURN" => Some(TurnUndead),
            b"VAMP" => Some(Vampirism),
            b"WABR" => Some(WaterBreathing),
            b"WAWA" => Some(WaterWalking),
            b"WKDI" => Some(WeaknessToDisease),
            b"WKFI" => Some(WeaknessToFire),
            b"WKFR" => Some(WeaknessToFrost),
            b"WKMA" => Some(WeaknessToMagic),
            b"WKNW" => Some(WeaknessToNormalWeapons),
            b"WKPO" => Some(WeaknessToPoison),
            b"WKSH" => Some(WeaknessToShock),
            b"Z001" => Some(SummonRufiosGhost),
            b"Z002" => Some(SummonAncestorGuardian),
            b"Z003" => Some(SummonSpiderling),
            b"Z004" => Some(SummonFleshAtronach),
            b"Z005" => Some(SummonBear),
            b"Z006" => Some(SummonGluttonousHunger),
            b"Z007" => Some(SummonRavenousHunger),
            b"Z008" => Some(SummonVoraciousHunger),
            b"Z009" => Some(SummonDarkSeducer),
            b"Z010" => Some(SummonGoldenSaint),
            b"Z011" => Some(WabbaSummon),
            b"Z012" => Some(SummonDecrepitShambles),
            b"Z013" => Some(SummonShambles),
            b"Z014" => Some(SummonRepleteShambles),
            b"Z015" => Some(SummonHunger),
            b"Z016" => Some(SummonMangledFleshAtronach),
            b"Z017" => Some(SummonTornFleshAtronach),
            b"Z018" => Some(SummonStitchedFleshAtronach),
            b"Z019" => Some(SummonSewnFleshAtronach),
            b"Z020" => Some(ExtraSummon20),
            b"ZCLA" => Some(SummonClannfear),
            b"ZDAE" => Some(SummonDaedroth),
            b"ZDRE" => Some(SummonDremora),
            b"ZDRL" => Some(SummonDremoraLord),
            b"ZFIA" => Some(SummonFlameAtronach),
            b"ZFRA" => Some(SummonFrostAtronach),
            b"ZGHO" => Some(SummonGhost),
            b"ZHDZ" => Some(SummonHeadlessZombie),
            b"ZLIC" => Some(SummonLich),
            b"ZSCA" => Some(SummonScamp),
            b"ZSKA" => Some(SummonSkeletonGuardian),
            b"ZSKC" => Some(SummonSkeletonChampion),
            b"ZSKE" => Some(SummonSkeleton),
            b"ZSKH" => Some(SummonSkeletonHero),
            b"ZSPD" => Some(SummonSpiderDaedra),
            b"ZSTA" => Some(SummonStormAtronach),
            b"ZWRA" => Some(SummonFadedWraith),
            b"ZWRL" => Some(SummonGloomWraith),
            b"ZXIV" => Some(SummonXivilai),
            b"ZZOM" => Some(SummonZombie),
            _ => None,
        }
    }
}

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
    Apprentice,
    Novice,
    Journeyman,
    Expert,
    Master,
}

bitflags! {
    pub struct SpellFlags: u8 {
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
    visual_effect: Option<MagicEffect>,
    pub is_hostile: bool,
    name: String,
}

/// An individual effect of a spell
#[derive(Debug)]
pub struct SpellEffect {
    effect: MagicEffect,
    magnitude: u32,
    area: u32,
    duration: u32,
    range: EffectRange,
    actor_value: ActorValue,
    script_effect: Option<ScriptEffect>,
}

impl SpellEffect {
    /// Creates a new spell effect
    pub fn new(effect: MagicEffect) -> SpellEffect {
        SpellEffect {
            effect,
            magnitude: 0,
            area: 0,
            duration: 0,
            range: EffectRange::Self_,
            actor_value: ActorValue::Vampirism, // spells that don't affect an actor value seem to use either this or Health
            script_effect: None,
        }
    }

    // TODO: add some checks to make sure the magnitude, area, duration, and range make sense with
    //  the effect
    /// Gets the effect's magnitude
    pub fn magnitude(&self) -> u32 {
        self.magnitude
    }

    /// Sets the effect's magnitude
    pub fn set_magnitude(&mut self, value: u32) -> Result<(), TesError> {
        self.magnitude = value;
        Ok(())
    }

    /// Gets the effect's area
    pub fn area(&self) -> u32 {
        self.area
    }

    /// Sets the effect's area
    pub fn set_area(&mut self, value: u32) -> Result<(), TesError> {
        if self.range == EffectRange::Self_ && value > 0 {
            Err(TesError::LimitExceeded {
                description: String::from("Cast-on-self spells cannot have an area"),
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
        self.duration = value;
        Ok(())
    }

    /// Gets the effect's range
    pub fn range(&self) -> EffectRange {
        self.range
    }

    /// Sets the effect's range
    pub fn set_range(&mut self, range: EffectRange) {
        if range == EffectRange::Self_ {
            self.area = 0;
        }

        self.range = range;
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
                    spell.spell_type = SpellType::try_from(extract!(reader as u32)? as u8)
                        .map_err(|e| decode_failed_because("Invalid spell type", e))?;
                    spell.cost = extract!(reader as u32)?;
                    spell.level = SpellLevel::try_from(extract!(reader as u32)? as u8)
                        .map_err(|e| decode_failed_because("Invalid spell level", e))?;
                    spell.flags = SpellFlags::from_bits(extract!(reader as u32)? as u8)
                        .ok_or_else(|| decode_failed("Invalid spell flags"))?;
                }
                b"EFID" => (), // the effect ID is also contained in the EFIT field, so we'll just get it from there
                b"EFIT" => {
                    let mut reader = field.reader();
                    spell.effects.push(SpellEffect {
                        effect: {
                            let mut id = [0u8; 4];
                            reader.read_exact(&mut id)?;
                            MagicEffect::from_id(&id).ok_or_else(|| {
                                decode_failed(format!("Unexpected effect ID {:?}", id))
                            })?
                        },
                        magnitude: extract!(reader as u32)?,
                        area: extract!(reader as u32)?,
                        duration: extract!(reader as u32)?,
                        range: EffectRange::try_from(extract!(reader as u32)? as u8)
                            .map_err(|e| decode_failed_because("Invalid effect range", e))?,
                        actor_value: ActorValue::try_from(extract!(reader as u32)? as u8)
                            .map_err(|e| decode_failed_because("Invalid effect actor value", e))?,
                        script_effect: None,
                    })
                }
                b"SCIT" => {
                    if let Some(last_effect) = spell.effects.iter_mut().last() {
                        let mut reader = field.reader();
                        last_effect.script_effect = Some(ScriptEffect {
                            script: FormId(extract!(reader as u32)?),
                            school: MagicSchool::try_from(extract!(reader as u32)? as u8).map_err(
                                |e| decode_failed_because("Invalid script effect magic school", e),
                            )?,
                            visual_effect: {
                                let id = extract!(reader as u32)?;
                                if id == 0 {
                                    None
                                } else {
                                    let id_bytes = id.to_le_bytes();
                                    Some(MagicEffect::from_id(&id_bytes).ok_or_else(|| {
                                        decode_failed(format!("Unexpected effect ID {:?}", id))
                                    })?)
                                }
                            },
                            is_hostile: extract!(reader as u32)? & 1 != 0, // other "flag" bits are garbage?
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
        let writer = &mut buf;
        let spell_type: u8 = self.spell_type.into();
        let spell_level: u8 = self.level.into();

        serialize!(spell_type as u32 => writer)?;
        serialize!(self.cost => writer)?;
        serialize!(spell_level as u32 => writer)?;
        serialize!(self.flags.bits => writer)?;

        record.add_field(Tes4Field::new(b"SPIT", buf)?);

        for effect in &self.effects {
            let effect_id = effect.effect.id();
            record.add_field(Tes4Field::new(b"EFID", effect_id.to_vec())?);

            let mut buf = Vec::with_capacity(24);
            let writer = &mut buf;
            let range: u8 = effect.range.into();
            let av: u8 = effect.actor_value.into();
            writer.write_exact(&effect_id)?;
            serialize!(effect.magnitude => writer)?;
            serialize!(effect.area => writer)?;
            serialize!(effect.duration => writer)?;
            serialize!(range as u32 => writer)?;
            serialize!(av as u32 => writer)?;

            record.add_field(Tes4Field::new(b"EFIT", buf)?);

            if let Some(ref script_effect) = effect.script_effect {
                let mut buf = Vec::with_capacity(16);
                let writer = &mut buf;
                let school: u8 = script_effect.school.into();
                let flags = if script_effect.is_hostile { 1u32 } else { 0 };
                serialize!(script_effect.script.0 => writer)?;
                serialize!(school as u32 => writer)?;
                match script_effect.visual_effect {
                    Some(ref effect) => {
                        writer.write_exact(&effect.id())?;
                    }
                    None => {
                        serialize!(0u32 => writer)?;
                    }
                }
                serialize!(flags => writer)?;

                record.add_field(Tes4Field::new(b"SCIT", buf)?);
                record.add_field(Tes4Field::new_zstring(b"FULL", script_effect.name.clone())?);
            }
        }

        Ok(())
    }
}
