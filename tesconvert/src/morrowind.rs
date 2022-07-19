use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::iter::repeat;
use std::path::Path;
use std::thread;

use tesutil::tes3::{InventoryItem, SkillType, Tes3World};
use tesutil::tes4::cosave::{CoSave, ObConvert, OPCODE_BASE};
use tesutil::tes4::save::*;
use tesutil::tes4::{ActorValue, FindForm, FormId, Tes4World};
use tesutil::{tes3, EffectRange};
use tesutil::{tes4, Record};
use tesutil::{Attribute, Attributes, Form, TesError};

use crate::config::*;
use crate::oblivion::Oblivion;

use anyhow::{anyhow, Context, Result};
#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

/// Container for Morrowind-related state and functionality
#[derive(Debug)]
pub struct Morrowind {
    pub world: Tes3World,
    major_skill_bonus: f32,
    minor_skill_bonus: f32,
    misc_skill_bonus: f32,
    spec_skill_bonus: f32,
}

impl Morrowind {
    fn get_float_setting(world: &Tes3World, name: &str, default: f32) -> Result<f32> {
        Ok(match world.get::<tes3::GameSetting>(name)? {
            Some(setting) => setting
                .get_float()
                .ok_or_else(|| anyhow!("Invalid value for Morrowind game setting {}", name))?,
            None => default,
        })
    }

    #[cfg(windows)]
    fn detect_dir() -> Result<String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey_with_flags(
            r"SOFTWARE\Bethesda Softworks\Morrowind",
            KEY_READ | KEY_WOW64_32KEY,
        ) {
            if let Ok(path) = key.get_value("Installed Path") {
                return Ok(path);
            }
        }

        if let Ok(key) =
            hklm.open_subkey_with_flags(r"SOFTWARE\OpenMW.org", KEY_READ | KEY_WOW64_32KEY)
        {
            // key seems to be of the form "OpenMW x.xx.x". we'll just sort the keys and take the
            // last one. this could cause problems one day when we go from 1.9 to 1.10 (or 0.99 to
            // 0.100), but that's a problem for future me.
            let mut last_key = String::new();
            for version in key.enum_keys() {
                let version = version?;
                if version > last_key {
                    last_key = version;
                }
            }

            let key_to_use = key.open_subkey(last_key)?;
            return Ok(key_to_use.get_value("")?);
        }

        Err(anyhow!("Could not detect Morrowind install path"))
    }

    #[cfg(not(windows))]
    fn detect_dir() -> Result<String> {
        // TODO: refer to OpenMW code for detecting Wine installations. can we detect OpenMW installs?
        Err(anyhow!("Could not detect Morrowind install path"))
    }

    /// Capture Morrowind state
    pub fn load<P, Q>(game_dir: Option<P>, save_path: Q) -> Result<Morrowind>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let world = match game_dir {
            // two calls because we're calling different functions here if P is not String
            Some(path) => Tes3World::load_from_save(path.as_ref(), save_path),
            None => Tes3World::load_from_save(Morrowind::detect_dir()?, save_path),
        }?;
        let major_skill_bonus = Morrowind::get_float_setting(&world, "fMajorSkillBonus", 0.75)?;
        let minor_skill_bonus = Morrowind::get_float_setting(&world, "fMinorSkillBonus", 1.0)?;
        let misc_skill_bonus = Morrowind::get_float_setting(&world, "fMiscSkillBonus", 1.25)?;
        let spec_skill_bonus = Morrowind::get_float_setting(&world, "fSpecialSkillBonus", 0.8)?;

        Ok(Morrowind {
            world,
            major_skill_bonus,
            minor_skill_bonus,
            misc_skill_bonus,
            spec_skill_bonus,
        })
    }

    /// Gets the Oblivion skill equivalent to a given Morrowind skill, if one exists
    pub fn oblivion_skill(skill: tes3::Skill) -> Option<tes4::Skill> {
        match skill {
            tes3::Skill::Block => Some(tes4::Skill::Block),
            tes3::Skill::Armorer => Some(tes4::Skill::Armorer),
            tes3::Skill::HeavyArmor => Some(tes4::Skill::HeavyArmor),
            tes3::Skill::Blunt => Some(tes4::Skill::Blunt),
            tes3::Skill::LongBlade => Some(tes4::Skill::Blade),
            tes3::Skill::Axe => Some(tes4::Skill::Blunt),
            tes3::Skill::Athletics => Some(tes4::Skill::Athletics),
            tes3::Skill::Destruction => Some(tes4::Skill::Destruction),
            tes3::Skill::Alteration => Some(tes4::Skill::Alteration),
            tes3::Skill::Illusion => Some(tes4::Skill::Illusion),
            tes3::Skill::Conjuration => Some(tes4::Skill::Conjuration),
            tes3::Skill::Mysticism => Some(tes4::Skill::Mysticism),
            tes3::Skill::Restoration => Some(tes4::Skill::Restoration),
            tes3::Skill::Alchemy => Some(tes4::Skill::Alchemy),
            tes3::Skill::Security => Some(tes4::Skill::Security),
            tes3::Skill::Sneak => Some(tes4::Skill::Sneak),
            tes3::Skill::Acrobatics => Some(tes4::Skill::Acrobatics),
            tes3::Skill::LightArmor => Some(tes4::Skill::LightArmor),
            tes3::Skill::ShortBlade => Some(tes4::Skill::Blade),
            tes3::Skill::Marksman => Some(tes4::Skill::Marksman),
            tes3::Skill::Mercantile => Some(tes4::Skill::Mercantile),
            tes3::Skill::Speechcraft => Some(tes4::Skill::Speechcraft),
            tes3::Skill::HandToHand => Some(tes4::Skill::HandToHand),
            _ => None,
        }
    }

    /// Gets the Oblivion magic effect type equivalent to a given Morrowind magic effect type, if one exists
    pub fn oblivion_effect(effect: tes3::MagicEffectType) -> Option<tes4::MagicEffectType> {
        use tes3::MagicEffectType as MGEF3;
        use tes4::MagicEffectType as MGEF4;

        match effect {
            MGEF3::WaterBreathing => Some(MGEF4::WaterBreathing),
            MGEF3::WaterWalking => Some(MGEF4::WaterWalking),
            MGEF3::Shield => Some(MGEF4::Shield),
            MGEF3::FireShield => Some(MGEF4::FireShield),
            MGEF3::LightningShield => Some(MGEF4::ShockShield),
            MGEF3::FrostShield => Some(MGEF4::FrostShield),
            MGEF3::Burden => Some(MGEF4::Burden),
            MGEF3::Feather => Some(MGEF4::Feather),
            MGEF3::Lock => Some(MGEF4::Lock),
            MGEF3::Open => Some(MGEF4::Open),
            MGEF3::FireDamage => Some(MGEF4::FireDamage),
            MGEF3::ShockDamage => Some(MGEF4::ShockDamage),
            MGEF3::FrostDamage => Some(MGEF4::FrostDamage),
            MGEF3::DrainAttribute => Some(MGEF4::DrainAttribute),
            MGEF3::DrainHealth => Some(MGEF4::DrainHealth),
            MGEF3::DrainMagicka => Some(MGEF4::DrainMagicka),
            MGEF3::DrainFatigue => Some(MGEF4::DrainFatigue),
            MGEF3::DrainSkill => Some(MGEF4::DrainSkill),
            MGEF3::DamageAttribute => Some(MGEF4::DamageAttribute),
            MGEF3::DamageHealth => Some(MGEF4::DamageHealth),
            MGEF3::DamageMagicka => Some(MGEF4::DamageMagicka),
            MGEF3::DamageFatigue => Some(MGEF4::DamageFatigue),
            MGEF3::WeaknessToFire => Some(MGEF4::WeaknessToFire),
            MGEF3::WeaknessToFrost => Some(MGEF4::WeaknessToFrost),
            MGEF3::WeaknessToShock => Some(MGEF4::WeaknessToShock),
            MGEF3::WeaknessToMagicka => Some(MGEF4::WeaknessToMagic),
            MGEF3::WeaknessToCommonDisease => Some(MGEF4::WeaknessToDisease),
            MGEF3::WeaknessToPoison => Some(MGEF4::WeaknessToPoison),
            MGEF3::WeaknessToNormalWeapons => Some(MGEF4::WeaknessToNormalWeapons),
            MGEF3::DisintegrateWeapon => Some(MGEF4::DisintegrateWeapon),
            MGEF3::DisintegrateArmor => Some(MGEF4::DisintegrateArmor),
            MGEF3::Invisibility => Some(MGEF4::Invisibility),
            MGEF3::Chameleon => Some(MGEF4::Chameleon),
            MGEF3::Light => Some(MGEF4::Light),
            MGEF3::NightEye => Some(MGEF4::NightEye),
            MGEF3::Charm => Some(MGEF4::Charm),
            MGEF3::Paralyze => Some(MGEF4::Paralyze),
            MGEF3::Silence => Some(MGEF4::Silence),
            MGEF3::CalmHumanoid => Some(MGEF4::Calm),
            MGEF3::CalmCreature => Some(MGEF4::Calm),
            MGEF3::FrenzyHumanoid => Some(MGEF4::Frenzy),
            MGEF3::FrenzyCreature => Some(MGEF4::Frenzy),
            MGEF3::DemoralizeHumanoid => Some(MGEF4::Demoralize),
            MGEF3::DemoralizeCreature => Some(MGEF4::Demoralize),
            MGEF3::RallyHumanoid => Some(MGEF4::Rally),
            MGEF3::RallyCreature => Some(MGEF4::Rally),
            MGEF3::Dispel => Some(MGEF4::Dispel),
            MGEF3::Soultrap => Some(MGEF4::SoulTrap),
            MGEF3::Telekinesis => Some(MGEF4::Telekinesis),
            MGEF3::DetectAnimal => Some(MGEF4::DetectLife),
            MGEF3::SpellAbsorption => Some(MGEF4::SpellAbsorption),
            MGEF3::Reflect => Some(MGEF4::ReflectSpell),
            MGEF3::CureCommonDisease => Some(MGEF4::CureDisease),
            MGEF3::CurePoison => Some(MGEF4::CurePoison),
            MGEF3::CureParalyzation => Some(MGEF4::CureParalysis),
            MGEF3::RestoreAttribute => Some(MGEF4::RestoreAttribute),
            MGEF3::RestoreHealth => Some(MGEF4::RestoreHealth),
            MGEF3::RestoreMagicka => Some(MGEF4::RestoreMagicka),
            MGEF3::RestoreFatigue => Some(MGEF4::RestoreFatigue),
            MGEF3::FortifyAttribute => Some(MGEF4::FortifyAttribute),
            MGEF3::FortifyHealth => Some(MGEF4::FortifyHealth),
            MGEF3::FortifyMagicka => Some(MGEF4::FortifyMagicka),
            MGEF3::FortifyMaximumMagicka => Some(MGEF4::FortifyMagickaMultiplier),
            MGEF3::FortifyFatigue => Some(MGEF4::FortifyFatigue),
            MGEF3::FortifySkill => Some(MGEF4::FortifySkill),
            MGEF3::AbsorbAttribute => Some(MGEF4::AbsorbAttribute),
            MGEF3::AbsorbHealth => Some(MGEF4::AbsorbHealth),
            MGEF3::AbsorbMagicka => Some(MGEF4::AbsorbMagicka),
            MGEF3::AbsorbFatigue => Some(MGEF4::AbsorbFatigue),
            MGEF3::AbsorbSkill => Some(MGEF4::AbsorbSkill),
            MGEF3::ResistFire => Some(MGEF4::ResistFire),
            MGEF3::ResistFrost => Some(MGEF4::ResistFrost),
            MGEF3::ResistShock => Some(MGEF4::ResistShock),
            MGEF3::ResistMagicka => Some(MGEF4::ResistMagic),
            MGEF3::ResistCommonDisease => Some(MGEF4::ResistDisease),
            MGEF3::ResistPoison => Some(MGEF4::ResistPoison),
            MGEF3::ResistNormalWeapons => Some(MGEF4::ResistNormalWeapons),
            MGEF3::ResistParalysis => Some(MGEF4::ResistParalysis),
            MGEF3::TurnUndead => Some(MGEF4::TurnUndead),
            MGEF3::SummonScamp => Some(MGEF4::SummonScamp),
            MGEF3::SummonClannfear => Some(MGEF4::SummonClannfear),
            MGEF3::SummonDaedroth => Some(MGEF4::SummonDaedroth),
            MGEF3::SummonDremora => Some(MGEF4::SummonDremora),
            MGEF3::SummonAncestralGhost => Some(MGEF4::SummonGhost),
            MGEF3::SummonSkeletalMinion => Some(MGEF4::SummonSkeleton),
            MGEF3::SummonHunger => Some(MGEF4::SummonHunger),
            MGEF3::SummonGoldenSaint => Some(MGEF4::SummonGoldenSaint),
            MGEF3::SummonFlameAtronach => Some(MGEF4::SummonFlameAtronach),
            MGEF3::SummonFrostAtronach => Some(MGEF4::SummonFrostAtronach),
            MGEF3::SummonStormAtronach => Some(MGEF4::SummonStormAtronach),
            MGEF3::CommandCreature => Some(MGEF4::CommandCreature),
            MGEF3::CommandHumanoid => Some(MGEF4::CommandHumanoid),
            MGEF3::BoundDagger => Some(MGEF4::BoundDagger),
            MGEF3::BoundLongsword => Some(MGEF4::BoundSword),
            MGEF3::BoundMace => Some(MGEF4::BoundMace),
            MGEF3::BoundBattleAxe => Some(MGEF4::BoundAxe),
            MGEF3::BoundLongbow => Some(MGEF4::BoundBow),
            MGEF3::BoundCuirass => Some(MGEF4::BoundCuirass),
            MGEF3::BoundHelm => Some(MGEF4::BoundHelmet),
            MGEF3::BoundBoots => Some(MGEF4::BoundBoots),
            MGEF3::BoundShield => Some(MGEF4::BoundShield),
            MGEF3::BoundGloves => Some(MGEF4::BoundGauntlets),
            MGEF3::Vampirism => Some(MGEF4::Vampirism),
            MGEF3::SunDamage => Some(MGEF4::SunDamage),
            MGEF3::StuntedMagicka => Some(MGEF4::StuntedMagicka),
            MGEF3::CallBear => Some(MGEF4::SummonBear),
            _ => None,
        }
    }

    /// Calculates the XP required to level a skill up
    pub fn calculate_skill_xp<T: Into<f32>>(
        &self,
        skill: tes3::Skill,
        level: T,
        class: &tes3::Class,
    ) -> f32 {
        let level = level.into();
        let bonus_value = match class.get_skill_type(skill) {
            SkillType::Major => self.major_skill_bonus,
            SkillType::Minor => self.minor_skill_bonus,
            SkillType::Miscellaneous => self.misc_skill_bonus,
        };
        let xp = level * bonus_value;

        if skill.specialization() == class.specialization {
            xp * self.spec_skill_bonus
        } else {
            xp
        }
    }
}

/// Container for Morrowind-to-Oblivion conversion state and functionality
#[derive(Debug)]
pub struct MorrowindToOblivion {
    config: Config,
    mw: Morrowind,
    ob: Oblivion,
    form_map: RefCell<HashMap<String, FormId>>,
    player_base: tes3::Npc,
    player_ref: tes3::PlayerReference,
    player_change: tes3::NpcChange,
    player_data: tes3::PlayerData,
    class: tes3::Class,
    active_spells: tes3::ActiveSpellList,
}

impl MorrowindToOblivion {
    fn load_map<P: AsRef<Path>>(
        config_dir: P,
        world: &Tes4World,
    ) -> Result<HashMap<String, FormId>> {
        let mut map = HashMap::new();
        let inis = iter_form_map(config_dir.as_ref().join("mwob"))
            .with_context(|| "Failed to load Morrowind-to-Oblivion mapping")?;
        for ini in inis {
            for (plugin, values) in &ini {
                let plugin = plugin.unwrap_or("").to_lowercase();
                for (mw, ob) in values.iter() {
                    let form_id = u32::from_str_radix(ob, 16).with_context(|| {
                        format!("Invalid form ID {} in Morrowind-to-Oblivion mapping", ob)
                    })?;
                    let search = FindForm::ByMaster(Some(plugin.as_str()), form_id);
                    if let Some(form_id) = world.get_form_id(&search) {
                        map.insert(String::from(mw), form_id);
                    }
                }
            }
        }

        Ok(map)
    }

    /// Prepare a Morrowind-to-Oblivion conversion based on the provided configuration
    pub fn load(config: Config) -> Result<MorrowindToOblivion> {
        // the compiler can't tell that config lives long enough for our threads to take references
        // to these values, so we have to clone them so we can give owned values to the threads.
        let mw_path = config.mw_path.clone();
        let ob_path = config.ob_path.clone();
        let source_path = config.source_path.clone();
        let target_path = config.target_path.clone();

        let mw_thread = thread::spawn(|| Morrowind::load(mw_path, source_path));
        let ob_thread = thread::spawn(|| Oblivion::load(ob_path, target_path));

        // the map_err handles the case where join() failed and the with_context adds context to the
        // case where the load failed
        let mw = mw_thread
            .join()
            .map_err(|_| anyhow!("Morrowind load failed"))?
            .with_context(|| "Morrowind load failed")?;
        let ob = ob_thread
            .join()
            .map_err(|_| anyhow!("Oblivion load failed"))?
            .with_context(|| "Oblivion load failed")?;
        let form_map = RefCell::new(MorrowindToOblivion::load_map(
            &config.config_path,
            &ob.world(),
        )?);

        let player_base: tes3::Npc = mw
            .world
            .get("player")?
            .ok_or_else(|| anyhow!("Missing player record in Morrowind save"))?;
        // types aren't necessary here but I've included them to make it clearer why these two
        // identical statements do different things
        let player_ref: tes3::PlayerReference = mw
            .world
            .get("PlayerSaveGame")?
            .ok_or_else(|| anyhow!("Missing player reference record in Morrowind save"))?;
        let player_change: tes3::NpcChange = mw
            .world
            .get("PlayerSaveGame")?
            .ok_or_else(|| anyhow!("Missing player change record in Morrowind save"))?;

        let player_data = {
            let save = mw.world.get_save().unwrap();
            let record = save
                .get_records_by_type(b"PCDT")
                .ok_or_else(|| anyhow!("Missing player data record (PCDT) in Morrowind save"))?
                .next()
                .ok_or_else(|| anyhow!("Missing player data record (PCDT) in Morrowind save"))?;
            tes3::PlayerData::read(&record)?
        };

        let active_spells = {
            let save = mw.world.get_save().unwrap();
            let record = save
                .get_records_by_type(b"SPLM")
                .ok_or_else(|| anyhow!("Missing active spells record (SPLM) in Morrowind save"))?
                .next()
                .ok_or_else(|| anyhow!("Missing active spells record (SPLM) in Morrowind save"))?;
            tes3::ActiveSpellList::read(&record)?
        };

        let class = mw
            .world
            .get(player_base.class())?
            .ok_or_else(|| anyhow!("Invalid Morrowind player class"))?;

        Ok(MorrowindToOblivion {
            config,
            mw,
            ob,
            form_map,
            player_base,
            player_ref,
            player_change,
            player_data,
            class,
            active_spells,
        })
    }

    fn with_save<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&Save) -> T,
    {
        let ob_world = self.ob.world();
        f(ob_world.get_save().unwrap())
    }

    fn with_save_mut<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&mut Save) -> T,
    {
        let mut ob_world = self.ob.world_mut();
        f(ob_world.get_save_mut().unwrap())
    }

    fn with_cosave_mut<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&mut CoSave) -> T,
    {
        let mut ob_world = self.ob.world_mut();
        f(ob_world.get_cosave_mut().unwrap())
    }

    fn convert_race(&self, ob_player_ref: &mut PlayerReferenceChange) -> Result<()> {
        let mw_race = self.player_base.race();
        let form_id = self
            .form_map
            .borrow()
            .get(mw_race)
            .copied()
            .ok_or_else(|| {
                anyhow!(format!(
                    "Could not find Oblivion equivalent for race {}",
                    mw_race
                ))
            })?;

        // TODO: UESP says this is optional, but can the player not have a birthsign? probably not.
        //  at a minimum we should warn if there is no birthsign
        let bs_form_id = self
            .player_data
            .birthsign()
            .and_then(|id| self.form_map.borrow().get(id).copied());

        self.with_save_mut(|ob_save| {
            let iref = ob_save.insert_form_id(form_id);
            ob_player_ref.set_race(iref);

            if let Some(form_id) = bs_form_id {
                let iref = ob_save.insert_form_id(form_id);
                ob_player_ref.set_birthsign(iref);
            }
        });

        Ok(())
    }

    fn convert_class(&self) -> Result<(tes4::Class, FormId)> {
        Ok(match self.form_map.borrow().get(self.class.id()) {
            Some(class_form_id) => {
                let search = FindForm::ByIndex(*class_form_id);
                // we know this form ID is good because it wouldn't be in the map otherwise
                (self.ob.world().get(&search)?.unwrap(), *class_form_id)
            }
            None => {
                // the Morrowind class needs to be converted as a custom class
                let mut new_class = tes4::Class::new(String::from(self.class.name()))?;
                new_class.set_primary_attributes(self.class.primary_attributes())?;
                new_class.specialization = self.class.specialization;

                // we need to map the Morrowind major and minor skills to Oblivion major skills. because
                // Oblivion has 7 major skills to Morrowind's 5, we will need to take at least some
                // minor skills as major skills in Oblivion. because Oblivion removed and consolidated a
                // number of skills, we may also end up needing to take some miscellaneous skills. we
                // will sort the minor and miscellaneous skills in order of level, then take the highest
                // ones.
                let mut minor_skills = self.class.minor_skills().to_vec();
                minor_skills.sort_by(|a, b| {
                    self.player_ref.skills[*a]
                        .base
                        .cmp(&self.player_ref.skills[*b].base)
                });

                let mut misc_skills: Vec<_> = tes3::Skill::iter()
                    .filter(|s| self.class.get_skill_type(*s) == SkillType::Miscellaneous)
                    .collect();
                misc_skills.sort_by(|a, b| {
                    self.player_ref.skills[*a]
                        .base
                        .cmp(&self.player_ref.skills[*b].base)
                });

                let num_skills = new_class.major_skills().len();
                let mut new_skills = Vec::with_capacity(num_skills);
                for skill in self
                    .class
                    .major_skills()
                    .iter()
                    .chain(&minor_skills)
                    .chain(&misc_skills)
                {
                    // if this skill has an Oblivion equivalent
                    if let Some(ob_skill) = Morrowind::oblivion_skill(*skill) {
                        // if this skill is not already in the list of major skills
                        if !new_skills.iter().any(|s| *s == ob_skill) {
                            new_skills.push(ob_skill);

                            if new_skills.len() == num_skills {
                                break;
                            }
                        }
                    }
                }

                new_class.set_major_skills(new_skills.as_ref())?;

                new_class.is_playable = true;
                (new_class, FORM_PLAYER_CUSTOM_CLASS)
            }
        })
    }

    fn convert_effect(&self, effect: &tes3::SpellEffect) -> Result<Option<tes4::SpellEffect>> {
        Ok(
            if let Some(effect_type) = Morrowind::oblivion_effect(effect.effect()) {
                // TODO: what should we do if a spell includes both e.g. a Calm Creature and Calm Humanoid effect?
                let mut ob_effect = tes4::SpellEffect::new(effect_type);

                // Night-Eye has a magnitude in Morrowind, but not Oblivion, so we'll just leave it at the default of 0
                if effect_type != tes4::MagicEffectType::NightEye {
                    let (min, max) = effect.magnitude();
                    ob_effect.set_magnitude(self.config.combine_strategy.combine(min, max))?;
                }

                ob_effect.set_range(match effect_type {
                    // in Morrowind, Telekinesis always has Self range, but in Oblivion, it's always Target
                    tes4::MagicEffectType::Telekinesis => EffectRange::Target,
                    // for some reason, Oblivion doesn't allow Absorb Skill to be cast on Target
                    tes4::MagicEffectType::AbsorbSkill => EffectRange::Touch,
                    _ => effect.range(),
                })?;
                ob_effect.set_duration(effect.duration())?;
                ob_effect.set_area(effect.area())?;

                ob_effect.set_actor_value(if let Some(mw_skill) = effect.skill() {
                    tes4::ActorValue::from(match Morrowind::oblivion_skill(mw_skill) {
                        Some(skill) => skill,
                        None => return Ok(None),
                    })
                } else if let Some(attribute) = effect.attribute() {
                    tes4::ActorValue::from(attribute)
                } else {
                    effect_type.default_actor_value()
                })?;

                Some(ob_effect)
            } else {
                None
            },
        )
    }

    fn convert_spell(&self, mw_spell: &tes3::Spell) -> Result<Option<tes4::Spell>> {
        let mut ob_spell = tes4::Spell::new(None, Some(String::from(mw_spell.name())));
        if mw_spell.is_auto_calc() {
            ob_spell.set_auto_calc(true);
        } else {
            ob_spell.set_auto_calc(false);
            ob_spell.cost = mw_spell.cost();
        }
        // choosing not to set this flag right now because I don't think it's desirable to add new player start spells in the save
        // ob_spell.set_player_start_spell(spell.is_player_start_spell());
        ob_spell.spell_type = match mw_spell.spell_type() {
            tes3::SpellType::Spell => tes4::SpellType::Spell,
            tes3::SpellType::Ability => tes4::SpellType::Ability,
            tes3::SpellType::Power => tes4::SpellType::Power,
            tes3::SpellType::Disease => tes4::SpellType::Disease,
            _ => return Ok(None),
        };

        let mut converted_any = false;
        for effect in mw_spell.effects() {
            if let Some(ob_effect) = self.convert_effect(effect)? {
                ob_spell.add_effect(ob_effect);
                converted_any = true;
            }
        }

        // only add the spell if we successfully converted at least one effect
        if converted_any {
            self.ob.calculate_spell_cost(&mut ob_spell)?;
            Ok(Some(ob_spell))
        } else {
            Ok(None)
        }
    }

    fn convert_spells(
        &self,
        ob_player_base: &mut ActorChange,
        ob_player_ref: &mut PlayerReferenceChange,
    ) -> Result<()> {
        let mw_race: tes3::Race = self
            .mw
            .world
            .get(self.player_base.race())?
            .ok_or_else(|| anyhow!("Could not find Morrowind player race"))?;
        let mw_sign: Option<tes3::Birthsign> = match self.player_data.birthsign() {
            Some(birthsign) => self.mw.world.get(birthsign)?,
            None => None,
        };
        let spells_to_suppress: HashSet<_> = if let Some(ref sign) = mw_sign {
            mw_race.specials().chain(sign.spells()).collect()
        } else {
            mw_race.specials().collect()
        };

        // TODO: make spell conversion errors warnings
        let ob_spells: HashMap<_, _> = self
            .player_base
            .spells()
            .filter_map(|id| {
                if spells_to_suppress.contains(id) {
                    None
                } else {
                    Some((
                        id,
                        self.convert_spell(&self.mw.world.get(id).ok()??).unwrap()?,
                    ))
                }
            })
            .collect();
        let known_effects: HashSet<_> = ob_spells
            .iter()
            .filter(|(_, s)| s.spell_type == tes4::SpellType::Spell)
            .flat_map(|(_, s)| s.effects().map(|e| e.effect_type()))
            .collect();

        // set known magic effects
        // TODO: do powers contribute to known effects?
        ob_player_ref.set_known_magic_effects(known_effects.iter().map(|e| e.id()).collect());

        let ob_race: tes4::Race = self
            .with_save(|save| {
                let form_id = save.iref_to_form_id(ob_player_ref.race()).unwrap();
                self.ob.world().get(&FindForm::ByIndex(form_id))
            })?
            .unwrap();

        // FIXME: these unwraps are not safe
        let ob_birthsign: tes4::Birthsign = self
            .with_save(|save| {
                let form_id = save.iref_to_form_id(ob_player_ref.birthsign()).unwrap();
                self.ob.world().get(&FindForm::ByIndex(form_id))
            })?
            .unwrap();

        // we have to collect the specials here because if we leave them as an iterator we'll end up
        // trying to borrow the world while it's already mutably borrowed below
        let specials: Vec<_> = ob_race
            .spells()
            .chain(ob_birthsign.spells())
            .filter(|id| {
                let spell: tes4::Spell = self
                    .ob
                    .world()
                    .get(&FindForm::ByIndex(*id))
                    .unwrap()
                    .unwrap();
                matches!(
                    spell.spell_type,
                    tes4::SpellType::Power | tes4::SpellType::LesserPower | tes4::SpellType::Spell
                )
            })
            .collect();

        self.with_save_mut(|save| {
            let mut spell_irefs = Vec::with_capacity(ob_spells.len());
            for (id, spell) in ob_spells {
                // we don't put abilities and diseases in the spell list because those need to be added to the player by the OBSE plugin
                if matches!(
                    spell.spell_type,
                    tes4::SpellType::Power | tes4::SpellType::LesserPower | tes4::SpellType::Spell
                ) {
                    let iref = save.add_form(&spell)?;
                    let form_id = save.iref_to_form_id(iref).unwrap();
                    spell_irefs.push(iref);
                    self.form_map.borrow_mut().insert(String::from(id), form_id);
                }
            }

            for special in specials {
                let iref = save.insert_form_id(special);
                spell_irefs.push(iref);
            }

            ob_player_base.set_spells(spell_irefs);

            Ok(())
        })
    }

    fn convert_stats(
        &self,
        ob_player_base: &mut ActorChange,
        ob_player_ref: &mut PlayerReferenceChange,
        ob_class: &tes4::Class,
    ) -> Result<()> {
        let mw_race: tes3::Race = self
            .mw
            .world
            .get(self.player_base.race())?
            .ok_or_else(|| anyhow!("Could not find Morrowind player race"))?;
        let mw_sign: Option<tes3::Birthsign> = match self.player_data.birthsign() {
            Some(birthsign) => self.mw.world.get(birthsign)?,
            None => None,
        };
        let spells_to_suppress: HashSet<_> = if let Some(ref sign) = mw_sign {
            mw_race.specials().chain(sign.spells()).collect()
        } else {
            mw_race.specials().collect()
        };

        // we're going to start by pulling in the active spells. we need this here so we can subtract
        // effects before we convert the stats.
        let mut base_attribute_modifiers: Attributes<f32> = Attributes::default();
        let mut base_skill_modifiers: tes3::Skills<i32> = tes3::Skills::default();
        let mut base_health_modifier = 0f32;
        let mut base_magicka_modifier = 0f32;
        let mut base_fatigue_modifier = 0f32;

        let mut current_attribute_modifiers = Attributes::default();
        let mut current_skill_modifiers = tes3::Skills::default();
        let mut current_health_modifier = 0.;
        let mut current_magicka_modifier = 0.;
        let mut current_fatigue_modifier = 0.;

        let active_player_spells = self
            .active_spells
            .iter()
            .filter(|s| s.effects().any(|e| e.affected_actor() == "PlayerSaveGame"));
        let mut new_active_spells = HashMap::new();
        for active_spell in active_player_spells {
            let id = active_spell.id();

            let mw_spell: tes3::Spell = match self.mw.world.get(id)? {
                Some(spell) => spell,
                None => continue,
            };

            if !spells_to_suppress.contains(id) {
                let form_id = match self.form_map.borrow().get(active_spell.id()) {
                    Some(form_id) => *form_id,
                    None => {
                        if let Some(ob_spell) = self.convert_spell(&mw_spell)? {
                            self.with_save_mut::<Result<FormId, TesError>, _>(|save| {
                                let iref = save.add_form(&ob_spell)?;
                                Ok(save.iref_to_form_id(iref).unwrap())
                            })?
                        } else {
                            continue;
                        }
                    }
                };

                let seconds_active = active_spell.effects().last().unwrap().seconds_active();
                new_active_spells.insert(form_id, seconds_active);
            }

            let (attribute_modifiers, skill_modifiers) =
                if mw_spell.spell_type() == tes3::SpellType::Ability {
                    (&mut base_attribute_modifiers, &mut base_skill_modifiers)
                } else {
                    (
                        &mut current_attribute_modifiers,
                        &mut current_skill_modifiers,
                    )
                };

            for effect in active_spell
                .effects()
                .filter(|e| e.affected_actor() == "PlayerSaveGame")
            {
                if let Some(base_effect) = mw_spell
                    .effects()
                    .enumerate()
                    .filter(|(p, _)| *p as i32 == effect.index())
                    .map(|(_, e)| e)
                    .last()
                {
                    match base_effect.effect() {
                        tes3::MagicEffectType::FortifyAttribute => {
                            let magnitude = effect.magnitude() as f32;
                            let attribute = base_effect.attribute().unwrap();
                            attribute_modifiers[attribute] -= magnitude;
                            match attribute {
                                Attribute::Strength
                                | Attribute::Willpower
                                | Attribute::Agility
                                | Attribute::Endurance => base_fatigue_modifier -= magnitude,
                                Attribute::Intelligence => base_magicka_modifier -= magnitude,
                                _ => (),
                            }
                        }
                        tes3::MagicEffectType::FortifySkill => {
                            skill_modifiers[base_effect.skill().unwrap()] -= effect.magnitude()
                        }
                        tes3::MagicEffectType::AbsorbAttribute
                        | tes3::MagicEffectType::DrainAttribute => {
                            let magnitude = effect.magnitude() as f32;
                            let attribute = base_effect.attribute().unwrap();
                            attribute_modifiers[attribute] += magnitude;
                            match attribute {
                                Attribute::Strength
                                | Attribute::Willpower
                                | Attribute::Agility
                                | Attribute::Endurance => base_fatigue_modifier += magnitude,
                                Attribute::Intelligence => base_magicka_modifier += magnitude,
                                _ => (),
                            }
                        }
                        tes3::MagicEffectType::AbsorbSkill | tes3::MagicEffectType::DrainSkill => {
                            skill_modifiers[base_effect.skill().unwrap()] += effect.magnitude()
                        }
                        tes3::MagicEffectType::FortifyHealth => {
                            *if mw_spell.spell_type() == tes3::SpellType::Ability {
                                &mut base_health_modifier
                            } else {
                                &mut current_health_modifier
                            } -= effect.magnitude() as f32
                        }
                        tes3::MagicEffectType::DrainHealth => {
                            *if mw_spell.spell_type() == tes3::SpellType::Ability {
                                &mut base_health_modifier
                            } else {
                                &mut current_health_modifier
                            } += effect.magnitude() as f32
                        }
                        tes3::MagicEffectType::FortifyMaximumMagicka => {
                            *if mw_spell.spell_type() == tes3::SpellType::Ability {
                                &mut base_magicka_modifier
                            } else {
                                &mut current_magicka_modifier
                            } -= (effect.magnitude() as f32) / 10.
                                * self.player_ref.attributes[Attribute::Intelligence].current
                        }
                        tes3::MagicEffectType::FortifyMagicka => {
                            *if mw_spell.spell_type() == tes3::SpellType::Ability {
                                &mut base_magicka_modifier
                            } else {
                                &mut current_magicka_modifier
                            } -= effect.magnitude() as f32
                        }
                        tes3::MagicEffectType::DrainMagicka => {
                            *if mw_spell.spell_type() == tes3::SpellType::Ability {
                                &mut base_magicka_modifier
                            } else {
                                &mut current_magicka_modifier
                            } += effect.magnitude() as f32
                        }
                        tes3::MagicEffectType::FortifyFatigue => {
                            *if mw_spell.spell_type() == tes3::SpellType::Ability {
                                &mut base_fatigue_modifier
                            } else {
                                &mut current_fatigue_modifier
                            } -= effect.magnitude() as f32
                        }
                        tes3::MagicEffectType::DrainFatigue => {
                            *if mw_spell.spell_type() == tes3::SpellType::Ability {
                                &mut base_fatigue_modifier
                            } else {
                                &mut current_fatigue_modifier
                            } += effect.magnitude() as f32
                        }
                        _ => (),
                    }
                }
            }
        }

        let ob_race: tes4::Race = self
            .with_save(|save| {
                let form_id = save.iref_to_form_id(ob_player_ref.race()).unwrap();
                self.ob.world().get(&FindForm::ByIndex(form_id))
            })?
            .unwrap();

        // FIXME: these unwraps are not safe
        let ob_birthsign: tes4::Birthsign = self
            .with_save(|save| {
                let form_id = save.iref_to_form_id(ob_player_ref.birthsign()).unwrap();
                self.ob.world().get(&FindForm::ByIndex(form_id))
            })?
            .unwrap();

        for special in ob_race.spells().chain(ob_birthsign.spells()).filter(|id| {
            let spell: tes4::Spell = self
                .ob
                .world()
                .get(&FindForm::ByIndex(*id))
                .ok()
                .unwrap()
                .unwrap();
            !matches!(
                spell.spell_type,
                tes4::SpellType::Power | tes4::SpellType::LesserPower | tes4::SpellType::Spell
            )
        }) {
            // seconds active doesn't matter for abilities
            new_active_spells.insert(special, 0.);
        }

        self.with_cosave_mut(|cosave| {
            let plugin = cosave.get_plugin_by_opcode(OPCODE_BASE).unwrap();
            let mut obconvert = ObConvert::read(plugin)?;
            obconvert.set_active_spells(new_active_spells);
            let plugin = cosave.get_plugin_by_opcode_mut(OPCODE_BASE).unwrap();
            obconvert.write(plugin)
        })?;

        ob_player_ref.is_female = self.player_base.is_female();

        let damage = ob_player_ref.damage_modifiers_mut();
        for (_, value) in damage.iter_mut() {
            *value = 0.;
        }

        // set attributes
        let attributes = ob_player_base
            .attributes_mut()
            .ok_or_else(|| anyhow!("Oblivion player base has no attributes"))?;
        for (attribute, value) in attributes.iter_mut() {
            let base =
                self.player_ref.attributes[attribute].base + base_attribute_modifiers[attribute];
            let current = self.player_ref.attributes[attribute].current
                + base_attribute_modifiers[attribute]
                + current_attribute_modifiers[attribute];
            *value = base as u8;
            // FIXME: should these be negative?
            damage[ActorValue::from(attribute)] = current - base;
        }

        // set skills
        let skills = ob_player_base
            .skills_mut()
            .ok_or_else(|| anyhow!("Oblivion player base has no skills"))?;
        for (skill, value) in skills.iter_mut() {
            *value = match Oblivion::morrowind_skill(skill) {
                tes3::Skill::LongBlade => {
                    let base = self.config.combine_strategy.combine(
                        self.player_ref.skills[tes3::Skill::LongBlade].base
                            + base_skill_modifiers[tes3::Skill::LongBlade],
                        self.player_ref.skills[tes3::Skill::ShortBlade].base
                            + base_skill_modifiers[tes3::Skill::ShortBlade],
                    );
                    let current = self.config.combine_strategy.combine(
                        self.player_ref.skills[tes3::Skill::LongBlade].current
                            + base_skill_modifiers[tes3::Skill::LongBlade]
                            + current_skill_modifiers[tes3::Skill::LongBlade],
                        self.player_ref.skills[tes3::Skill::ShortBlade].current
                            + base_skill_modifiers[tes3::Skill::ShortBlade]
                            + current_skill_modifiers[tes3::Skill::ShortBlade],
                    );
                    // FIXME: should these be negative?
                    damage[ActorValue::Blade] = (current - base) as f32;
                    base
                }
                tes3::Skill::Blunt => {
                    let base = self.config.combine_strategy.combine(
                        self.player_ref.skills[tes3::Skill::Axe].base
                            + base_skill_modifiers[tes3::Skill::Axe],
                        self.player_ref.skills[tes3::Skill::Blunt].base
                            + base_skill_modifiers[tes3::Skill::Blunt],
                    );
                    let current = self.config.combine_strategy.combine(
                        self.player_ref.skills[tes3::Skill::Axe].current
                            + base_skill_modifiers[tes3::Skill::Axe]
                            + current_skill_modifiers[tes3::Skill::Axe],
                        self.player_ref.skills[tes3::Skill::Blunt].current
                            + base_skill_modifiers[tes3::Skill::Blunt]
                            + current_skill_modifiers[tes3::Skill::Blunt],
                    );
                    // FIXME: should these be negative?
                    damage[ActorValue::Blunt] = (current - base) as f32;
                    base
                }
                mw_skill => {
                    let base =
                        self.player_ref.skills[mw_skill].base + base_skill_modifiers[mw_skill];
                    let current = self.player_ref.skills[mw_skill].current
                        + base_skill_modifiers[mw_skill]
                        + current_skill_modifiers[mw_skill];
                    // FIXME: should these be negative?
                    damage[ActorValue::from(skill)] = (current - base) as f32;
                    base
                }
            } as u8;
        }

        // remove any active effects
        let active_effect_modifiers = ob_player_ref.active_effect_modifiers_mut();
        for (_, value) in active_effect_modifiers.iter_mut() {
            *value = 0.;
        }
        ob_player_ref.clear_active_magic_effects();

        // set level, fatigue, and magicka
        if ob_player_base.actor_base().is_none() {
            // can happen if the player is level 1
            let base = ActorBase::default();
            ob_player_base.set_actor_base(Some(base));
        }

        // calculate fatigue and magicka
        let attributes = ob_player_base.attributes().unwrap();
        let max_fatigue = attributes[Attribute::Endurance] as f32
            + attributes[Attribute::Strength] as f32
            + attributes[Attribute::Agility] as f32
            + attributes[Attribute::Willpower] as f32;
        let fatigue_ratio =
            (self.player_ref.fatigue.current + base_fatigue_modifier + current_fatigue_modifier)
                / (self.player_ref.fatigue.base + base_fatigue_modifier);
        let fatigue_delta = max_fatigue * (1. - fatigue_ratio);
        ob_player_ref.set_fatigue_delta(-fatigue_delta);

        let max_magicka = attributes[Attribute::Intelligence] as f32
            * (self
                .ob
                .world()
                .get_float_setting("fPCBaseMagickaMult", 1.)?
                + 1.);
        let magicka_ratio =
            (self.player_ref.magicka.current + base_magicka_modifier + current_magicka_modifier)
                / (self.player_ref.magicka.base + base_magicka_modifier);
        let magicka_delta = max_magicka * (1. - magicka_ratio);
        ob_player_ref.set_magicka_delta(-magicka_delta);

        // calculate health
        let (mut starting_strength, mut starting_endurance) = if ob_player_ref.is_female {
            (
                mw_race.attribute_female(Attribute::Strength),
                mw_race.attribute_female(Attribute::Endurance),
            )
        } else {
            (
                mw_race.attribute_male(Attribute::Strength),
                mw_race.attribute_male(Attribute::Endurance),
            )
        };
        for attribute in self.class.primary_attributes() {
            match attribute {
                Attribute::Strength => starting_strength += 10,
                Attribute::Endurance => starting_endurance += 10,
                _ => (),
            }
        }
        let starting_health = (starting_strength + starting_endurance) / 2;
        let mw_base_max_health = self.player_ref.health.base + base_health_modifier;
        // amount of health gained from level-ups
        let level_health = mw_base_max_health - starting_health as f32;

        let ob_base_max_health = attributes[Attribute::Endurance] as f32
            * self.ob.world().get_float_setting("fPCBaseHealthMult", 2.)?
            * self
                .ob
                .world()
                .get_float_setting("fStatsHealthStartMult", 1.)?;
        let max_health = ob_base_max_health + level_health;
        let health_ratio =
            (self.player_ref.health.current + base_health_modifier + current_health_modifier)
                / mw_base_max_health;
        let health_delta = max_health * (1. - health_ratio);
        ob_player_ref.set_health_delta(-health_delta);

        ob_player_base.set_base_health(Some(level_health as u32));
        let mut base = ob_player_base.actor_base_mut().unwrap();
        base.level = self.player_base.level as i16;
        // this will be recalculated once we add back racial and birthsign bonuses
        base.magicka = 0;
        base.fatigue = 0;

        ob_player_ref.set_name(String::from(self.player_base.name().unwrap_or("")))?;

        ob_player_ref.major_skill_advancements = self.player_data.level_progress;

        for (spec, value) in ob_player_ref.spec_increases.iter_mut() {
            *value = self.player_data.spec_increases[spec];
        }

        let mut advancements = vec![];
        // Morrowind doesn't track advancements by level like Oblivion does, so we have to fake it here.
        // I don't actually know how Oblivion would handle an advancement greater than 10, but it never
        // happens in normal gameplay, so I figure it's best to enforce it here.
        // IDE falsely claims the error "Cannot move" here on self, but attribute_progress is Copy, so
        // there is no move.
        let mut attributes = self.player_data.attribute_progress;
        while attributes.values().any(|v| *v > 0) {
            let mut advancement = Attributes::default();
            for (attribute, value) in advancement.iter_mut() {
                *value = attributes[attribute] % 10;
            }

            for (attribute, value) in attributes.iter_mut() {
                *value -= advancement[attribute];
            }

            advancements.push(advancement);
        }
        ob_player_ref.advancements = advancements;

        // skill XP
        let mut mw_progress = tes3::Skills::default();
        for (skill, value) in mw_progress.iter_mut() {
            *value = self.player_data.skill_progress[skill]
                / self.mw.calculate_skill_xp(
                    skill,
                    self.player_ref.skills[skill].base as u16,
                    &self.class,
                );
        }

        let ob_skills = ob_player_base.skills().unwrap(); // we know this is safe or we would have failed earlier
        for (skill, value) in ob_player_ref.skill_xp.iter_mut() {
            *value = match Oblivion::morrowind_skill(skill) {
                tes3::Skill::LongBlade => self.config.combine_strategy.combine_float(
                    mw_progress[tes3::Skill::LongBlade],
                    mw_progress[tes3::Skill::ShortBlade],
                ),
                tes3::Skill::Blunt => self.config.combine_strategy.combine_float(
                    mw_progress[tes3::Skill::Axe],
                    mw_progress[tes3::Skill::Blunt],
                ),
                mw_skill => mw_progress[mw_skill],
            } * self
                .ob
                .calculate_skill_xp(skill, ob_skills[skill], ob_class);
        }

        Ok(())
    }

    fn convert_potion(&self, mw_potion: &tes3::Potion) -> Result<Option<tes4::Potion>> {
        let mut ob_potion = tes4::Potion::new(
            mw_potion.id.clone(),
            mw_potion.name.clone().unwrap_or_else(String::new),
        );

        ob_potion.is_auto_calc = mw_potion.alchemy_data.is_auto_calc;
        ob_potion.value = mw_potion.alchemy_data.value;
        ob_potion.weight = mw_potion.alchemy_data.weight;

        let mut converted_any = false;
        for effect in &mw_potion.effects {
            if let Some(ob_effect) = self.convert_effect(effect)? {
                ob_potion.add_effect(ob_effect);
                converted_any = true;
            }
        }

        // only add the spell if we successfully converted at least one effect
        Ok(if converted_any {
            ob_potion.use_auto_graphics();
            Some(ob_potion)
        } else {
            None
        })
    }

    fn convert_inventory(&self, ob_player_ref: &mut PlayerReferenceChange) -> Result<()> {
        let ob_player_npc: tes4::Npc = self
            .ob
            .world()
            .get(&FindForm::ByIndex(FORM_PLAYER))?
            .ok_or_else(|| anyhow!("Missing Oblivion player NPC record"))?;

        let mut stacks = HashMap::new();
        let mut created_forms = HashMap::new();
        ob_player_ref.clear_inventory();
        let mut mw_inventory: Vec<(&InventoryItem, bool)> = self
            .player_change
            .iter_inventory()
            .zip(repeat(false))
            .collect();
        for (mw_item, was_converted) in &mut mw_inventory {
            if mw_item.script.is_some() {
                continue; // can't convert scripted items
            }

            let iref = if let Some(iref) = created_forms.get(mw_item.id.as_str()).copied() {
                iref
            } else if let Some(form_id) = self.form_map.borrow().get(&mw_item.id).copied() {
                self.with_save_mut(|ob_save| ob_save.insert_form_id(form_id))
            } else if let Some(record) = self.mw.world.get_record(&mw_item.id)? {
                match record.name() {
                    tes3::Potion::RECORD_TYPE => {
                        let mw_potion = tes3::Potion::read(&*record)?;
                        if let Some(ob_potion) = self.convert_potion(&mw_potion)? {
                            let iref =
                                self.with_save_mut(|ob_save| ob_save.add_form(&ob_potion))?;

                            created_forms.insert(mw_item.id.as_str(), iref);

                            iref
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                }
            } else {
                continue;
            };

            if !stacks.contains_key(&iref) {
                stacks.insert(iref, Item::new(iref, 0));
            }

            let ob_item = stacks.get_mut(&iref).unwrap();
            ob_item.stack_count += mw_item.count as i32;

            let mut properties = vec![];
            if mw_item.is_equipped {
                // FIXME: look up clothing type of Oblivion form and use EquippedAccessory if ring or amulet
                properties.push(Property::EquippedItem);
            }

            if mw_item.count > 1 {
                properties.push(Property::AffectedItemCount(mw_item.count as u16));
            }

            if let Some(durability) = mw_item.remaining_durability {
                // TODO: convert Morrowind integer health to Oblivion float health
            }

            if let Some(charge) = mw_item.enchantment_charge {
                properties.push(Property::EnchantmentPoints(charge));
            }

            if let Some(ref soul) = mw_item.soul {
                // TODO: look up soul type from Morrowind
            }

            if !properties.is_empty() {
                ob_item.add_change(properties);
            }

            *was_converted = true;
        }

        // remove default items
        for (form_id, count) in ob_player_npc.iter_inventory() {
            let iref = self.with_save_mut(|ob_save| ob_save.insert_form_id(form_id));
            if !stacks.contains_key(&iref) {
                stacks.insert(iref, Item::new(iref, -count));
            } else {
                let item = stacks.get_mut(&iref).unwrap();
                item.stack_count -= count;
                if item.stack_count == 0 && !item.has_changes() {
                    stacks.remove(&iref);
                }
            }
        }

        // update inventory
        for (_, item) in stacks {
            ob_player_ref.add_item(item);
        }

        // store a list of all the inventory we couldn't convert
        self.with_cosave_mut(|cosave| {
            let plugin = cosave.get_plugin_by_opcode(OPCODE_BASE).unwrap();
            let mut obconvert = ObConvert::read(plugin)?;
            for item in
                mw_inventory
                    .into_iter()
                    .filter_map(|(i, was_converted)| if was_converted { None } else { Some(i) })
            {
                obconvert.add_morrowind_item(item.clone());
            }
            let plugin = cosave.get_plugin_by_opcode_mut(OPCODE_BASE).unwrap();
            obconvert.write(plugin)
        })?;

        Ok(())
    }

    /// Perform a Morrowind-to-Oblivion conversion
    pub fn convert(&self) -> Result<()> {
        let (mut ob_player_base, mut ob_player_ref) = {
            // load initial data from the Oblivion save
            let ob_world = self.ob.world();
            let ob_save = ob_world.get_save().unwrap();

            let ob_player_base = ob_save
                .get_form_change(FORM_PLAYER)?
                .ok_or_else(|| anyhow!("Missing player change record in Oblivion save"))?;

            let ob_player_ref = ob_save.get_form_change(FORM_PLAYER_REF)?.ok_or_else(|| {
                anyhow!("Missing player reference change record in Oblivion save")
            })?;

            (ob_player_base, ob_player_ref)
        };

        // convert data
        self.convert_race(&mut ob_player_ref)?;
        let (ob_class, ob_class_form_id) = self.convert_class()?;
        self.convert_spells(&mut ob_player_base, &mut ob_player_ref)?;
        self.convert_stats(&mut ob_player_base, &mut ob_player_ref, &ob_class)?;
        self.convert_inventory(&mut ob_player_ref)?;

        // apply changes to save
        self.with_save_mut::<Result<()>, _>(|ob_save| {
            // finalize converted class (we have to wait and do this here because this might take
            // ownership of the class)
            let ob_class_iref = ob_save.insert_form_id(ob_class_form_id);
            ob_player_ref.set_class(
                if ob_class_form_id == FORM_PLAYER_CUSTOM_CLASS {
                    Some(ob_class)
                } else {
                    None
                },
                ob_class_iref,
            );

            // copy save metadata
            let mw_save = self.mw.world.get_save().unwrap();

            // set name that appears in the save list
            let mw_save_info = mw_save
                .get_save_info()
                .ok_or_else(|| anyhow!("Morrowind plugin did not contain save information"))?;
            ob_save.set_player_name(String::from(mw_save_info.player_name()))?;

            ob_save.update_form_change(&ob_player_base, FORM_PLAYER)?;
            ob_save.update_form_change(&ob_player_ref, FORM_PLAYER_REF)?;

            ob_save.save_file(&self.config.output_path)?;

            Ok(())
        })?;

        self.with_cosave_mut(|cosave| {
            let cosave_path = Path::new(&self.config.output_path).with_extension("obse");
            cosave.save_file(cosave_path)?;

            Ok(())
        })
    }
}
