use std::convert::{Into, TryFrom};

use crate::tes4::{ActorValue, FormId, Tes4Field, Tes4Record};
use crate::{
    decode_failed, decode_failed_because, EffectRange, Field, Form, MagicSchool, Record, TesError,
};

use binrw::BinReaderExt;
use bitflags::bitflags;
use enum_map::*;
use lazy_static::lazy_static;

#[derive(Debug, Copy, Clone, Eq, Enum, PartialEq, Hash)]
pub enum MagicEffectType {
    AbsorbAttribute,
    AbsorbFatigue,
    AbsorbHealth,
    AbsorbSkill,
    AbsorbMagicka,
    BoundArmorExtra01,
    BoundArmorExtra02,
    BoundArmorExtra03,
    BoundArmorExtra04,
    BoundArmorExtra05,
    BoundArmorExtra06,
    BoundArmorExtra07,
    BoundArmorExtra08,
    BoundArmorExtra09,
    BoundArmorExtra10,
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
    BoundWeaponExtra09,
    BoundWeaponExtra10,
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

impl MagicEffectType {
    /// Gets the 4-byte effect ID for this magic effect
    pub fn id(&self) -> [u8; 4] {
        use MagicEffectType::*;

        *match self {
            AbsorbAttribute => b"ABAT",
            AbsorbFatigue => b"ABFA",
            AbsorbHealth => b"ABHE",
            AbsorbSkill => b"ABSK",
            AbsorbMagicka => b"ABSP",
            BoundArmorExtra01 => b"BA01",
            BoundArmorExtra02 => b"BA02",
            BoundArmorExtra03 => b"BA03",
            BoundArmorExtra04 => b"BA04",
            BoundArmorExtra05 => b"BA05",
            BoundArmorExtra06 => b"BA06",
            BoundArmorExtra07 => b"BA07",
            BoundArmorExtra08 => b"BA08",
            BoundArmorExtra09 => b"BA09",
            BoundArmorExtra10 => b"BA10",
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
            BoundWeaponExtra09 => b"BW09",
            BoundWeaponExtra10 => b"BW10",
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
    pub fn from_id(id: &[u8]) -> Option<MagicEffectType> {
        use MagicEffectType::*;

        // sometimes effect IDs having a trailing null in the data. for convenience, we'll just
        // truncate anything past the 4th byte
        let id = &id[..4];

        match id {
            b"ABAT" => Some(AbsorbAttribute),
            b"ABFA" => Some(AbsorbFatigue),
            b"ABHE" => Some(AbsorbHealth),
            b"ABSK" => Some(AbsorbSkill),
            b"ABSP" => Some(AbsorbMagicka),
            b"BA01" => Some(BoundArmorExtra01),
            b"BA02" => Some(BoundArmorExtra02),
            b"BA03" => Some(BoundArmorExtra03),
            b"BA04" => Some(BoundArmorExtra04),
            b"BA05" => Some(BoundArmorExtra05),
            b"BA06" => Some(BoundArmorExtra06),
            b"BA07" => Some(BoundArmorExtra07),
            b"BA08" => Some(BoundArmorExtra08),
            b"BA09" => Some(BoundArmorExtra09),
            b"BA10" => Some(BoundArmorExtra10),
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
            b"BW09" => Some(BoundWeaponExtra09),
            b"BW10" => Some(BoundWeaponExtra10),
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

    /// Gets the default actor value associated with a given effect
    pub fn default_actor_value(&self) -> ActorValue {
        use ActorValue as AV;
        use MagicEffectType::*;

        match self {
            AbsorbAttribute => AV::Strength,
            AbsorbFatigue => AV::Fatigue,
            AbsorbHealth => AV::Health,
            AbsorbSkill => AV::Armorer,
            AbsorbMagicka => AV::Magicka,
            Burden => AV::Encumbrance,
            Calm => AV::Aggression,
            Chameleon => AV::Chameleon,
            Charm => AV::Personality,
            CureDisease => AV::Health,
            CureParalysis => AV::Health,
            CurePoison => AV::Health,
            Demoralize => AV::Confidence,
            DamageAttribute => AV::Strength,
            DamageFatigue => AV::Fatigue,
            DamageHealth => AV::Health,
            DamageMagicka => AV::Magicka,
            DisintegrateArmor => AV::Health,
            DisintegrateWeapon => AV::Health,
            DrainAttribute => AV::Strength,
            DrainFatigue => AV::Fatigue,
            DrainHealth => AV::Health,
            DrainSkill => AV::Armorer,
            DrainMagicka => AV::Magicka,
            Dispel => AV::Health,
            DetectLife => AV::DetectLifeRange,
            MehrunesDagonCustomEffect => AV::Health,
            FireDamage => AV::Health,
            FireShield => AV::ResistFire,
            FortifyAttribute => AV::Strength,
            FortifyFatigue => AV::Fatigue,
            FortifyHealth => AV::Health,
            FortifyMagickaMultiplier => AV::MagickaMultiplier,
            FortifySkill => AV::Armorer,
            FortifyMagicka => AV::Magicka,
            FrostDamage => AV::Health,
            Frenzy => AV::Aggression,
            FrostShield => AV::ResistFrost,
            Feather => AV::Encumbrance,
            Invisibility => AV::Invisibility,
            Light => AV::Health,
            ShockShield => AV::ResistShock,
            NightEye => AV::NightEyeBonus,
            Paralyze => AV::Paralysis,
            Rally => AV::Confidence,
            Reanimate => AV::Health,
            RestoreAttribute => AV::Strength,
            ReflectDamage => AV::ReflectDamage,
            RestoreFatigue => AV::Fatigue,
            RestoreHealth => AV::Health,
            RestoreMagicka => AV::Magicka,
            ReflectSpell => AV::SpellReflectChance,
            ResistDisease => AV::ResistDisease,
            ResistFire => AV::ResistFire,
            ResistFrost => AV::ResistFrost,
            ResistMagic => AV::ResistMagic,
            ResistNormalWeapons => AV::ResistNormalWeapons,
            ResistParalysis => AV::ResistParalysis,
            ResistPoison => AV::ResistPoison,
            ResistShock => AV::ResistShock,
            ResistWaterDamage => AV::ResistWaterDamage,
            SpellAbsorption => AV::SpellAbsorbChance,
            ScriptEffect => AV::Health,
            ShockDamage => AV::Health,
            Shield => AV::DefendBonus,
            Silence => AV::Silence,
            StuntedMagicka => AV::StuntedMagicka,
            SunDamage => AV::Health,
            Telekinesis => AV::Telekinesis,
            Vampirism => AV::Health,
            WaterBreathing => AV::WaterBreathing,
            WaterWalking => AV::WaterWalking,
            WeaknessToDisease => AV::ResistDisease,
            WeaknessToFire => AV::ResistFire,
            WeaknessToFrost => AV::ResistFrost,
            WeaknessToMagic => AV::ResistMagic,
            WeaknessToNormalWeapons => AV::ResistNormalWeapons,
            WeaknessToPoison => AV::ResistPoison,
            WeaknessToShock => AV::ResistShock,
            // summons and bound items, where an actor value doesn't make sense, seem to use either
            // Health or Vampirism. I haven't seen any pattern to when it uses one or the other, and
            // my guess is it doesn't actually matter, so we just default to Vampirism.
            _ => AV::Vampirism,
        }
    }
}

/// Type of projectile created when casting a spell with this effect
#[derive(Debug)]
pub enum ProjectileType {
    Ball,
    Bolt,
    Fog,
    Spray,
}

bitflags! {
    pub struct EffectFlags: u32 {
        const HOSTILE = 0x00000001;
        const RECOVER = 0x00000002;
        const DETRIMENTAL = 0x00000004;
        const MAGNITUDE_PERCENT = 0x00000008;
        const SELF = 0x00000010;
        const TOUCH = 0x00000020;
        const TARGET = 0x00000040;
        const NO_DURATION = 0x00000080;
        const NO_MAGNITUDE = 0x00000100;
        const NO_AREA = 0x00000200;
        const FX_PERSIST = 0x00000400;
        const SPELLMAKING = 0x00000800;
        const ENCHANTING = 0x00001000;
        const NO_INGREDIENT = 0x00002000;
        const UNKNOWN_14 = 0x00004000;
        const UNKNOWN_15 = 0x00008000;
        const USE_WEAPON = 0x00010000;
        const USE_ARMOR = 0x00020000;
        const USE_CREATURE = 0x00040000;
        const USE_SKILL = 0x00080000;
        const USE_ATTRIBUTE = 0x00100000;
        const UNKNOWN_21 = 0x00200000;
        const UNKNOWN_22 = 0x00400000;
        const UNKNOWN_23 = 0x00800000;
        const USE_ACTOR_VALUE = 0x01000000;
        const SPRAY_PROJECTILE_TYPE = 0x02000000;
        const BOLT_PROJECTILE_TYPE = 0x04000000;
        const NO_HIT_EFFECT = 0x08000000;
        const UNKNOWN_28 = 0x10000000;
        const UNKNOWN_29 = 0x20000000; // used by FIDG
        const UNKNOWN_30 = 0x40000000;
        const UNKNOWN_31 = 0x80000000;
    }
}

#[derive(Debug)]
pub struct MagicEffect {
    effect_type: MagicEffectType,
    name: String,
    description: String,
    icon: Option<String>,
    model: Option<(String, f32)>,
    flags: EffectFlags,
    base_cost: f32,
    associated_form: FormId,
    school: MagicSchool,
    resist_value: Option<ActorValue>,
    light: FormId,
    projectile_speed: f32,
    effect_shader: FormId,
    casting_sound: FormId,
    bolt_sound: FormId,
    hit_sound: FormId,
    area_sound: FormId,
    constant_effect_enchantment_factor: f32,
    constant_effect_barter_factor: f32,
    counter_effects: Vec<MagicEffectType>,
}

impl MagicEffect {
    /// Creates a new magic effect
    pub fn new<T: Into<String>>(
        effect_type: MagicEffectType,
        name: T,
        school: MagicSchool,
        resist_value: Option<ActorValue>,
        flags: EffectFlags,
        counter_effects: Vec<MagicEffectType>,
    ) -> MagicEffect {
        MagicEffect {
            effect_type,
            name: name.into(),
            description: String::new(),
            icon: None,
            model: None,
            flags,
            base_cost: 0.,
            associated_form: FormId(0),
            school,
            resist_value,
            light: FormId(0),
            projectile_speed: 0.,
            effect_shader: FormId(0),
            casting_sound: FormId(0),
            bolt_sound: FormId(0),
            hit_sound: FormId(0),
            area_sound: FormId(0),
            constant_effect_enchantment_factor: 0.,
            constant_effect_barter_factor: 0.,
            counter_effects,
        }
    }

    /// Gets this effect's base cost
    pub fn base_cost(&self) -> f32 {
        self.base_cost
    }

    /// Gets this effect's school
    pub fn school(&self) -> MagicSchool {
        self.school
    }

    /// Does this effect have a duration?
    pub fn has_duration(&self) -> bool {
        !self.flags.contains(EffectFlags::NO_DURATION)
    }

    /// Does this effect have a magnitude?
    pub fn has_magnitude(&self) -> bool {
        !self.flags.contains(EffectFlags::NO_MAGNITUDE)
    }

    /// Does this effect have an area?
    pub fn has_area(&self) -> bool {
        !self.flags.contains(EffectFlags::NO_AREA)
    }

    /// Is this effect's magnitude a percent?
    pub fn is_magnitude_percent(&self) -> bool {
        self.flags.contains(EffectFlags::MAGNITUDE_PERCENT)
    }

    /// Gets this effect's projectile type
    pub fn projectile_type(&self) -> ProjectileType {
        if self
            .flags
            .contains(EffectFlags::SPRAY_PROJECTILE_TYPE | EffectFlags::BOLT_PROJECTILE_TYPE)
        {
            ProjectileType::Fog
        } else if self.flags.contains(EffectFlags::SPRAY_PROJECTILE_TYPE) {
            ProjectileType::Spray
        } else if self.flags.contains(EffectFlags::BOLT_PROJECTILE_TYPE) {
            ProjectileType::Bolt
        } else {
            ProjectileType::Ball
        }
    }

    /// Does this effect allow the given effect range?
    pub fn allows_range(&self, range: EffectRange) -> bool {
        self.flags.contains(match range {
            EffectRange::Self_ => EffectFlags::SELF,
            EffectRange::Touch => EffectFlags::TOUCH,
            EffectRange::Target => EffectFlags::TARGET,
        })
    }
}

impl Form for MagicEffect {
    type Field = Tes4Field;
    type Record = Tes4Record;

    fn record_type() -> &'static [u8; 4] {
        b"MGEF"
    }

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        MagicEffect::assert(record)?;

        let mut effect = MagicEffect {
            effect_type: MagicEffectType::AbsorbAttribute,
            name: String::new(),
            description: String::new(),
            icon: None,
            model: None,
            flags: EffectFlags::empty(),
            base_cost: 0.,
            associated_form: FormId(0),
            school: MagicSchool::Alteration,
            resist_value: None,
            light: FormId(0),
            projectile_speed: 0.,
            effect_shader: FormId(0),
            casting_sound: FormId(0),
            bolt_sound: FormId(0),
            hit_sound: FormId(0),
            area_sound: FormId(0),
            constant_effect_enchantment_factor: 0.,
            constant_effect_barter_factor: 0.,
            counter_effects: vec![],
        };

        for field in record.iter() {
            match field.name() {
                b"EDID" => {
                    effect.effect_type = MagicEffectType::from_id(field.get())
                        .ok_or_else(|| decode_failed("Invalid magic effect type"))?
                }
                b"FULL" => effect.name = String::from(field.get_zstring()?),
                b"DESC" => effect.description = String::from(field.get_zstring()?),
                b"ICON" => effect.icon = Some(String::from(field.get_zstring()?)),
                b"MODL" => effect.model = Some((String::from(field.get_zstring()?), 0.)),
                b"MODB" => match effect.model {
                    Some((_, ref mut bound_radius)) => *bound_radius = field.get_f32()?,
                    None => return Err(decode_failed("MODB field without MODL in MGEF record")),
                },
                b"DATA" => {
                    let mut reader = field.reader();
                    effect.flags = EffectFlags::from_bits(reader.read_le()?)
                        .ok_or_else(|| decode_failed("Invalid magic effect flags"))?;
                    effect.base_cost = reader.read_le()?;
                    effect.associated_form = FormId(reader.read_le()?);
                    effect.school = MagicSchool::try_from(reader.read_le::<u32>()? as u8)
                        .map_err(|e| decode_failed_because("Invalid school in magic effect", e))?;
                    effect.resist_value = {
                        let resist: u32 = reader.read_le()?;
                        if resist == 0 {
                            None
                        } else {
                            ActorValue::try_from(resist as u8).ok()
                        }
                    };
                    effect
                        .counter_effects
                        .reserve(reader.read_le::<u32>()? as usize);
                    effect.light = FormId(reader.read_le()?);
                    effect.projectile_speed = reader.read_le()?;
                    effect.effect_shader = FormId(reader.read_le()?);
                    effect.casting_sound = FormId(reader.read_le()?);
                    effect.bolt_sound = FormId(reader.read_le()?);
                    effect.hit_sound = FormId(reader.read_le()?);
                    effect.area_sound = FormId(reader.read_le()?);
                    effect.constant_effect_enchantment_factor = reader.read_le()?;
                    effect.constant_effect_barter_factor = reader.read_le()?;
                }
                b"ESCE" => effect.counter_effects.push(
                    MagicEffectType::from_id(field.get())
                        .ok_or_else(|| decode_failed("Invalid counter effect type"))?,
                ),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {} in MGEF record",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(effect)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        unimplemented!()
    }
}

lazy_static! {
    pub static ref MAGIC_EFFECTS: EnumMap<MagicEffectType, MagicEffect> = {
        use MagicEffectType::*;
        use MagicSchool::*;

        enum_map! {
            AbsorbAttribute => MagicEffect::new(
                AbsorbAttribute, "Absorb Attribute", Restoration, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::USE_ATTRIBUTE,
                vec![AbsorbAttribute, DamageAttribute, DrainAttribute, Dispel]
            ),
            AbsorbFatigue => MagicEffect::new(
                AbsorbFatigue, "Absorb Fatigue", Restoration, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::TOUCH,
                vec![AbsorbFatigue, DamageFatigue, DrainFatigue, Dispel]
            ),
            AbsorbHealth => MagicEffect::new(
                AbsorbHealth, "Absorb Health", Restoration, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::TOUCH,
                vec![AbsorbHealth, DamageHealth, DrainHealth, Dispel]
            ),
            AbsorbSkill => MagicEffect::new(
                AbsorbSkill, "Absorb Skill", Restoration, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::USE_SKILL,
                vec![AbsorbSkill, DrainSkill, Dispel]
            ),
            AbsorbMagicka => MagicEffect::new(
                AbsorbMagicka, "Absorb Spell Points", Restoration, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::TOUCH,
                vec![AbsorbMagicka, DamageMagicka, DrainMagicka, Dispel]
            ),
            BoundArmorExtra01 => MagicEffect::new(
                BoundArmorExtra01, "Bound Armor Extra 01", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra02 => MagicEffect::new(
                BoundArmorExtra02, "Bound Armor Extra 02", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra03 => MagicEffect::new(
                BoundArmorExtra03, "Bound Armor Extra 03", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra04 => MagicEffect::new(
                BoundArmorExtra04, "Bound Armor Extra 04", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra05 => MagicEffect::new(
                BoundArmorExtra05, "Bound Armor Extra 05", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra06 => MagicEffect::new(
                BoundArmorExtra06, "Bound Armor Extra 06", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra07 => MagicEffect::new(
                BoundArmorExtra07, "Bound Armor Extra 07", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra08 => MagicEffect::new(
                BoundArmorExtra08, "Bound Armor Extra 08", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra09 => MagicEffect::new(
                BoundArmorExtra09, "Bound Armor Extra 09", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundArmorExtra10 => MagicEffect::new(
                BoundArmorExtra10, "Bound Armor Extra 10", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundBoots => MagicEffect::new(
                BoundBoots, "Bound Boots", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundCuirass => MagicEffect::new(
                BoundCuirass, "Bound Cuirass", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundGauntlets => MagicEffect::new(
                BoundGauntlets, "Bound Gauntlets", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundGreaves => MagicEffect::new(
                BoundGreaves, "Bound Greaves", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundHelmet => MagicEffect::new(
                BoundHelmet, "Bound Helmet", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            BoundShield => MagicEffect::new(
                BoundShield, "Bound Shield", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_ARMOR,
                vec![]
            ),
            Burden => MagicEffect::new(
                Burden, "Burden", Alteration, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, Feather]
            ),
            BoundOrderWeapon1 => MagicEffect::new(
                BoundOrderWeapon1, "Bound Weapon Extra 01", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundOrderWeapon2 => MagicEffect::new(
                BoundOrderWeapon2, "Bound Weapon Extra 02", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundOrderWeapon3 => MagicEffect::new(
                BoundOrderWeapon3, "Bound Weapon Extra 03", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundOrderWeapon4 => MagicEffect::new(
                BoundOrderWeapon4, "Bound Weapon Extra 04", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundOrderWeapon5 => MagicEffect::new(
                BoundOrderWeapon5, "Bound Weapon Extra 05", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundOrderWeapon6 => MagicEffect::new(
                BoundOrderWeapon6, "Bound Weapon Extra 06", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            SummonStaffOfSheogorath => MagicEffect::new(
                SummonStaffOfSheogorath, "Bound Weapon Extra 07", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundPriestDagger => MagicEffect::new(
                BoundPriestDagger, "Bound Weapon Extra 08", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundWeaponExtra09 => MagicEffect::new(
                BoundWeaponExtra09, "Bound Weapon Extra 09", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundWeaponExtra10 => MagicEffect::new(
                BoundWeaponExtra10, "Bound Weapon Extra 10", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundAxe => MagicEffect::new(
                BoundAxe, "Bound Axe", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundBow => MagicEffect::new(
                BoundBow, "Bound Bow", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundDagger => MagicEffect::new(
                BoundDagger, "Bound Dagger", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundMace => MagicEffect::new(
                BoundMace, "Bound Mace", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            BoundSword => MagicEffect::new(
                BoundSword, "Bound Sword", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE | EffectFlags::USE_WEAPON,
                vec![]
            ),
            Calm => MagicEffect::new(
                Calm, "Calm", Illusion, Some(ActorValue::ResistMagic),
                EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, Frenzy]
            ),
            Chameleon => MagicEffect::new(
                Chameleon, "Chameleon", Illusion, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            Charm => MagicEffect::new(
                Charm, "Charm", Illusion, Some(ActorValue::ResistMagic),
                EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel]
            ),
            CommandCreature => MagicEffect::new(
                CommandCreature, "Command Creature", Illusion, None,
                EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel]
            ),
            CommandHumanoid => MagicEffect::new(
                CommandHumanoid, "Command Humanoid", Illusion, None,
                EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel]
            ),
            CureDisease => MagicEffect::new(
                CureDisease, "Cure Disease", Restoration, None,
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_DURATION | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            CureParalysis => MagicEffect::new(
                CureParalysis, "Cure Paralysis", Restoration, None,
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_DURATION | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            CurePoison => MagicEffect::new(
                CurePoison, "Cure Poison", Restoration, None,
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_DURATION | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            Darkness => MagicEffect::new(
                Darkness, "Darkness", Illusion, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            Demoralize => MagicEffect::new(
                Demoralize, "Demoralize", Illusion, Some(ActorValue::ResistMagic),
                EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, Rally]
            ),
            DamageAttribute => MagicEffect::new(
                DamageAttribute, "Damage Attribute", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![AbsorbAttribute, Dispel, FortifyAttribute, RestoreAttribute]
            ),
            DamageFatigue => MagicEffect::new(
                DamageFatigue, "Damage Fatigue", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![AbsorbFatigue, Dispel, FortifyFatigue, RestoreFatigue]
            ),
            DamageHealth => MagicEffect::new(
                DamageHealth, "Damage Health", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![AbsorbHealth, Dispel, FortifyHealth, RestoreHealth]
            ),
            DamageMagicka => MagicEffect::new(
                DamageMagicka, "Damage Magicka", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![AbsorbMagicka, Dispel, FortifyMagicka, RestoreMagicka]
            ),
            DisintegrateArmor => MagicEffect::new(
                DisintegrateArmor, "Disintegrate Armor", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, FireShield, FrostShield, ShockShield, Shield]
            ),
            DiseaseInfo => MagicEffect::new(
                DiseaseInfo, "Disease Info", Destruction, Some(ActorValue::ResistDisease),
                EffectFlags::empty(),
                vec![CureDisease]
            ),
            DisintegrateWeapon => MagicEffect::new(
                DisintegrateWeapon, "Disintegrate Weapon", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, FireShield, FrostShield, ShockShield, Shield]
            ),
            DrainAttribute => MagicEffect::new(
                DrainAttribute, "Drain Attribute", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::USE_ATTRIBUTE,
                vec![AbsorbAttribute, Dispel, FortifyAttribute, RestoreAttribute]
            ),
            DrainFatigue => MagicEffect::new(
                DrainFatigue, "Drain Fatigue", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![AbsorbFatigue, Dispel, FortifyFatigue, RestoreFatigue]
            ),
            DrainHealth => MagicEffect::new(
                DrainHealth, "Drain Health", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![AbsorbHealth, Dispel, FortifyHealth, RestoreHealth]
            ),
            DrainSkill => MagicEffect::new(
                DrainSkill, "Drain Skill", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::USE_SKILL,
                vec![AbsorbSkill, Dispel, FortifySkill]
            ),
            DrainMagicka => MagicEffect::new(
                DrainMagicka, "Drain Spell Points", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![AbsorbMagicka, Dispel, FortifyMagicka, RestoreMagicka]
            ),
            Dispel => MagicEffect::new(
                Dispel, "Dispel", Mysticism, Some(ActorValue::ResistMagic),
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_DURATION,
                vec![]
            ),
            DetectLife => MagicEffect::new(
                DetectLife, "Detect Life", Mysticism, None,
                EffectFlags::RECOVER | EffectFlags::SELF,
                vec![]
            ),
            MehrunesDagonCustomEffect => MagicEffect::new(
                MehrunesDagonCustomEffect, "Mehrunes Dagon Custom Effect", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![AbsorbHealth, Dispel, FortifyHealth, RestoreHealth]
            ),
            FireDamage => MagicEffect::new(
                FireDamage, "Fire Damage", Destruction, Some(ActorValue::ResistFire),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::FX_PERSIST | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, FireShield, ResistFire]
            ),
            FireShield => MagicEffect::new(
                FireShield, "Fire Shield", Alteration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            FortifyAttribute => MagicEffect::new(
                FortifyAttribute, "Fortify Attribute", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            FortifyFatigue => MagicEffect::new(
                FortifyFatigue, "Fortify Fatigue", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            FortifyHealth => MagicEffect::new(
                FortifyHealth, "Fortify Health", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            FortifyMagickaMultiplier => MagicEffect::new(
                FortifyMagickaMultiplier, "Fortify Magicka Multiplier", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::SELF,
                vec![]
            ),
            FortifySkill => MagicEffect::new(
                FortifySkill, "Fortify Skill", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            FortifyMagicka => MagicEffect::new(
                FortifyMagicka, "Fortify Spell Points", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            FrostDamage => MagicEffect::new(
                FrostDamage, "Frost Damage", Destruction, Some(ActorValue::ResistFrost),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, FrostShield, ResistFrost]
            ),
            Frenzy => MagicEffect::new(
                Frenzy, "Frenzy", Illusion, Some(ActorValue::ResistMagic),
                EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Calm, Dispel]
            ),
            FrostShield => MagicEffect::new(
                FrostShield, "Frost Shield", Alteration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            Feather => MagicEffect::new(
                Feather, "Feather", Alteration, Some(ActorValue::ResistMagic),
                EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel]
            ),
            Invisibility => MagicEffect::new(
                Invisibility, "Invisibilty", Illusion, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            Light => MagicEffect::new(
                Light, "Light", Illusion, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ShockShield => MagicEffect::new(
                ShockShield, "Lightning Shield", Alteration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            Lock => MagicEffect::new(
                Lock, "Lock", Alteration, None,
                EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_DURATION,
                vec![]
            ),
            SummonMythicDawnHelm => MagicEffect::new(
                SummonMythicDawnHelm, "Summon Mythic Dawn Helm", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonMythicDawnArmor => MagicEffect::new(
                SummonMythicDawnArmor, "Summon Mythic Dawn Armor", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            NightEye => MagicEffect::new(
                NightEye, "Night-Eye", Illusion, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            Open => MagicEffect::new(
                Open, "Open", Alteration, None,
                EffectFlags::TARGET | EffectFlags::NO_DURATION,
                vec![]
            ),
            Paralyze => MagicEffect::new(
                Paralyze, "Paralyze", Illusion, Some(ActorValue::ResistParalysis),
                EffectFlags::HOSTILE | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_MAGNITUDE,
                vec![CureParalysis, Dispel]
            ),
            PoisonInfo => MagicEffect::new(
                PoisonInfo, "Poison Info", Destruction, Some(ActorValue::ResistPoison),
                EffectFlags::empty(),
                vec![CurePoison]
            ),
            Rally => MagicEffect::new(
                Rally, "Rally", Illusion, Some(ActorValue::ResistMagic),
                EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Demoralize, Dispel]
            ),
            Reanimate => MagicEffect::new(
                Reanimate, "Reanimate", Conjuration, None,
                EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_MAGNITUDE | EffectFlags::NO_AREA,
                vec![]
            ),
            RestoreAttribute => MagicEffect::new(
                RestoreAttribute, "Restore Attribute", Restoration, None,
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::USE_ATTRIBUTE,
                vec![]
            ),
            ReflectDamage => MagicEffect::new(
                ReflectDamage, "Reflect Damage", Mysticism, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF,
                vec![]
            ),
            RestoreFatigue => MagicEffect::new(
                RestoreFatigue, "Restore Fatigue", Restoration, None,
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            RestoreHealth => MagicEffect::new(
                RestoreHealth, "Restore Health", Restoration, None,
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            RestoreMagicka => MagicEffect::new(
                RestoreMagicka, "Restore Spell Points", Restoration, None,
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ReflectSpell => MagicEffect::new(
                ReflectSpell, "Reflect", Mysticism, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistDisease => MagicEffect::new(
                ResistDisease, "Resist Disease", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistFire => MagicEffect::new(
                ResistFire, "Resist Fire", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistFrost => MagicEffect::new(
                ResistFrost, "Resist Frost", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistMagic => MagicEffect::new(
                ResistMagic, "Resist Magic", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistNormalWeapons => MagicEffect::new(
                ResistNormalWeapons, "Resist Normal Weapons", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistParalysis => MagicEffect::new(
                ResistParalysis, "Resist Paralysis", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistPoison => MagicEffect::new(
                ResistPoison, "Resist Poison", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistShock => MagicEffect::new(
                ResistShock, "Resist Shock", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ResistWaterDamage => MagicEffect::new(
                ResistWaterDamage, "Resist Water Damage", Restoration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            SpellAbsorption => MagicEffect::new(
                SpellAbsorption, "Spell Absorption", Mysticism, Some(ActorValue::ResistMagic),
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            ScriptEffect => MagicEffect::new(
                ScriptEffect, "Script Effect", Alteration, None,
                EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_MAGNITUDE,
                vec![Dispel]
            ),
            ShockDamage => MagicEffect::new(
                ShockDamage, "Shock Damage", Destruction, Some(ActorValue::ResistShock),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, ShockShield, ResistShock]
            ),
            Shield => MagicEffect::new(
                Shield, "Shield", Alteration, None,
                EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![]
            ),
            Silence => MagicEffect::new(
                Silence, "Silence", Illusion, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_MAGNITUDE,
                vec![Dispel]
            ),
            StuntedMagicka => MagicEffect::new(
                StuntedMagicka, "Stunted Magicka", Destruction, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SoulTrap => MagicEffect::new(
                SoulTrap, "Soul Trap", Mysticism, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_MAGNITUDE,
                vec![Dispel]
            ),
            SunDamage => MagicEffect::new(
                SunDamage, "Sun Damage", Destruction, None,
                EffectFlags::DETRIMENTAL | EffectFlags::SELF,
                vec![]
            ),
            Telekinesis => MagicEffect::new(
                Telekinesis, "Telekinesis", Mysticism, None,
                EffectFlags::RECOVER | EffectFlags::TARGET | EffectFlags::NO_AREA,
                vec![]
            ),
            TurnUndead => MagicEffect::new(
                TurnUndead, "Turn Undead", Conjuration, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::RECOVER | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, Rally]
            ),
            Vampirism => MagicEffect::new(
                Vampirism, "Vampirism", Destruction, Some(ActorValue::ResistDisease),
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_DURATION,
                vec![]
            ),
            WaterBreathing => MagicEffect::new(
                WaterBreathing, "Water Breathing", Alteration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            WaterWalking => MagicEffect::new(
                WaterWalking, "Water Walking", Alteration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            WeaknessToDisease => MagicEffect::new(
                WeaknessToDisease, "Weakness to Disease", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, ResistDisease]
            ),
            WeaknessToFire => MagicEffect::new(
                WeaknessToFire, "Weakness to Fire", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, FireShield, ResistFire]
            ),
            WeaknessToFrost => MagicEffect::new(
                WeaknessToFrost, "Weakness to Frost", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, FrostShield, ResistFrost]
            ),
            WeaknessToMagic => MagicEffect::new(
                WeaknessToMagic, "Weakness to Magic", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, ResistMagic]
            ),
            WeaknessToNormalWeapons => MagicEffect::new(
                WeaknessToNormalWeapons, "Weakness to Normal Weapons", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, ResistNormalWeapons]
            ),
            WeaknessToPoison => MagicEffect::new(
                WeaknessToPoison, "Weakness to Poison", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, ResistPoison]
            ),
            WeaknessToShock => MagicEffect::new(
                WeaknessToShock, "Weakness to Shock", Destruction, Some(ActorValue::ResistMagic),
                EffectFlags::HOSTILE | EffectFlags::DETRIMENTAL | EffectFlags::RECOVER | EffectFlags::MAGNITUDE_PERCENT | EffectFlags::SELF | EffectFlags::TOUCH | EffectFlags::TARGET,
                vec![Dispel, ShockShield, ResistShock]
            ),
            SummonRufiosGhost => MagicEffect::new(
                SummonRufiosGhost, "Extra Summon 01", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonAncestorGuardian => MagicEffect::new(
                SummonAncestorGuardian, "Extra Summon 02", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonSpiderling => MagicEffect::new(
                SummonSpiderling, "Extra Summon 03", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonFleshAtronach => MagicEffect::new(
                SummonFleshAtronach, "Extra Summon 04", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonBear => MagicEffect::new(
                SummonBear, "Extra Summon 05", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonGluttonousHunger => MagicEffect::new(
                SummonGluttonousHunger, "Extra Summon 06", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonRavenousHunger => MagicEffect::new(
                SummonRavenousHunger, "Extra Summon 07", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonVoraciousHunger => MagicEffect::new(
                SummonVoraciousHunger, "Extra Summon 08", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonDarkSeducer => MagicEffect::new(
                SummonDarkSeducer, "Extra Summon 09", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonGoldenSaint => MagicEffect::new(
                SummonGoldenSaint, "Extra Summon 10", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            WabbaSummon => MagicEffect::new(
                WabbaSummon, "Extra Summon 11", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonDecrepitShambles => MagicEffect::new(
                SummonDecrepitShambles, "Extra Summon 12", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonShambles => MagicEffect::new(
                SummonShambles, "Extra Summon 13", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonRepleteShambles => MagicEffect::new(
                SummonRepleteShambles, "Extra Summon 14", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonHunger => MagicEffect::new(
                SummonHunger, "Extra Summon 15", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonMangledFleshAtronach => MagicEffect::new(
                SummonMangledFleshAtronach, "Extra Summon 16", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonTornFleshAtronach => MagicEffect::new(
                SummonTornFleshAtronach, "Extra Summon 17", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonStitchedFleshAtronach => MagicEffect::new(
                SummonStitchedFleshAtronach, "Extra Summon 18", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonSewnFleshAtronach => MagicEffect::new(
                SummonSewnFleshAtronach, "Extra Summon 19", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            ExtraSummon20 => MagicEffect::new(
                ExtraSummon20, "Extra Summon 20", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonClannfear => MagicEffect::new(
                SummonClannfear, "Summon Clannfear", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonDaedroth => MagicEffect::new(
                SummonDaedroth, "Summon Daedroth", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonDremora => MagicEffect::new(
                SummonDremora, "Summon Dremora", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonDremoraLord => MagicEffect::new(
                SummonDremoraLord, "Summon Dremora Lord", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonFlameAtronach => MagicEffect::new(
                SummonFlameAtronach, "Summon Fire Atronach", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonFrostAtronach => MagicEffect::new(
                SummonFrostAtronach, "Summon Frost Atronach", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonGhost => MagicEffect::new(
                SummonGhost, "Summon Ghost", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonHeadlessZombie => MagicEffect::new(
                SummonHeadlessZombie, "Summon Headless Zombie", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonLich => MagicEffect::new(
                SummonLich, "Summon Lich", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonScamp => MagicEffect::new(
                SummonScamp, "Summon Scamp", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonSkeletonGuardian => MagicEffect::new(
                SummonSkeletonGuardian, "Summon Skeleton Archer", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonSkeletonChampion => MagicEffect::new(
                SummonSkeletonChampion, "Summon Skeleton Champion", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonSkeleton => MagicEffect::new(
                SummonSkeleton, "Summon Skeleton", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonSkeletonHero => MagicEffect::new(
                SummonSkeletonHero, "Summon Skeleton Hero", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonSpiderDaedra => MagicEffect::new(
                SummonSpiderDaedra, "Summon Spider Daedra", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonStormAtronach => MagicEffect::new(
                SummonStormAtronach, "Summon Storm Atronach", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonFadedWraith => MagicEffect::new(
                SummonFadedWraith, "Summon Wraith", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonGloomWraith => MagicEffect::new(
                SummonGloomWraith, "Summon Wraith Lord", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonXivilai => MagicEffect::new(
                SummonXivilai, "Summon Xivilai", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
            SummonZombie => MagicEffect::new(
                SummonZombie, "Summon Zombie", Conjuration, None,
                EffectFlags::RECOVER | EffectFlags::SELF | EffectFlags::NO_MAGNITUDE,
                vec![]
            ),
        }
    };
}
