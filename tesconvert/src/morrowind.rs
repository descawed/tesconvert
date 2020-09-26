use std::collections::HashMap;
use std::path::Path;
use std::thread;

use tesutil::tes3;
use tesutil::tes3::{SkillType, Tes3World};
use tesutil::tes4;
use tesutil::tes4::save::*;
use tesutil::tes4::{FindForm, FormId, Tes4World};
use tesutil::{Attributes, Form};

use crate::config::*;
use crate::oblivion::Oblivion;

use anyhow::*;
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
    form_map: HashMap<String, FormId>,
    player_base: tes3::Npc,
    player_ref: tes3::PlayerReference,
    player_data: tes3::PlayerData,
    class: tes3::Class,
}

impl MorrowindToOblivion {
    fn load_map<P: AsRef<Path>>(
        config_dir: P,
        world: &Tes4World,
    ) -> Result<HashMap<String, FormId>> {
        let mut map = HashMap::new();
        for ini in iter_form_map(config_dir.as_ref().join("mwob"))? {
            for (plugin, values) in &ini {
                let plugin = plugin.unwrap_or("").to_lowercase();
                for (mw, ob) in values.iter() {
                    let search =
                        FindForm::ByMaster(Some(plugin.as_str()), u32::from_str_radix(ob, 16)?);
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

        let mw = mw_thread
            .join()
            .map_err(|_| anyhow!("Morrowind load failed"))??;
        let ob = ob_thread
            .join()
            .map_err(|_| anyhow!("Oblivion load failed"))??;
        let form_map = MorrowindToOblivion::load_map(&config.config_path, &ob.world)?;

        // the compiler actually requires a type here but not on player_ref or class
        let player_base: tes3::Npc = mw
            .world
            .get("player")?
            .ok_or_else(|| anyhow!("Missing player record in Morrowind save"))?;
        let player_ref = mw
            .world
            .get("PlayerSaveGame")?
            .ok_or_else(|| anyhow!("Missing player reference record in Morrowind save"))?;

        let player_data = {
            let save = mw.world.get_save().unwrap();
            let record = save
                .get_records_by_type(b"PCDT")
                .ok_or_else(|| anyhow!("Missing player data record (PCDT) in Morrowind save"))?
                .next()
                .ok_or_else(|| anyhow!("Missing player data record (PCDT) in Morrowind save"))?;
            tes3::PlayerData::read(&record)?
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
            player_data,
            class,
        })
    }

    fn convert_race(&self) -> Result<FormId> {
        let mw_race = self.player_base.race();
        self.form_map.get(mw_race).copied().ok_or_else(|| {
            anyhow!(format!(
                "Could not find Oblivion equivalent for race {}",
                mw_race
            ))
        })
    }

    fn convert_class(&self) -> Result<(tes4::Class, FormId)> {
        Ok(match self.form_map.get(self.class.id()) {
            Some(class_form_id) => {
                let search = FindForm::ByIndex(*class_form_id);
                // we know this form ID is good because it wouldn't be in the map otherwise
                (self.ob.world.get(&search)?.unwrap(), *class_form_id)
            }
            None => {
                // the Morrowind class needs to be converted as a custom class
                let mut new_class = tes4::Class::new(String::from(self.class.name()))?;
                new_class.set_primary_attributes(self.class.primary_attribute())?;
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

    fn convert_stats(
        &self,
        ob_player_base: &mut ActorChange,
        ob_player_ref: &mut PlayerReferenceChange,
        ob_class: &tes4::Class,
    ) -> Result<()> {
        ob_player_ref.is_female = self.player_base.is_female();

        // set attributes
        let attributes = ob_player_base
            .attributes_mut()
            .ok_or_else(|| anyhow!("Oblivion player base has no attributes"))?;
        for (attribute, value) in attributes.iter_mut() {
            *value = self.player_ref.attributes[attribute].base as u8;
        }

        // set skills
        let skills = ob_player_base
            .skills_mut()
            .ok_or_else(|| anyhow!("Oblivion player base has no skills"))?;
        for (skill, value) in skills.iter_mut() {
            *value = match Oblivion::morrowind_skill(skill) {
                tes3::Skill::LongBlade => self.config.skill_combine_strategy.combine(
                    self.player_ref.skills[tes3::Skill::LongBlade].base,
                    self.player_ref.skills[tes3::Skill::ShortBlade].base,
                ),
                tes3::Skill::Blunt => self.config.skill_combine_strategy.combine(
                    self.player_ref.skills[tes3::Skill::Axe].base,
                    self.player_ref.skills[tes3::Skill::Blunt].base,
                ),
                mw_skill => self.player_ref.skills[mw_skill].base,
            } as u8;
        }

        // set level
        if ob_player_base.actor_base().is_none() {
            // can happen if the player is level 1
            let base = ActorBase::default();
            ob_player_base.set_actor_base(Some(base));
        }

        let mut base = ob_player_base.actor_base_mut().unwrap();
        base.level = self.player_base.level as i16;

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
        while attributes.values().sum::<u8>() > 0 {
            let mut advancement = Attributes::new();
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
        let mut mw_progress = tes3::Skills::new();
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
                tes3::Skill::LongBlade => self.config.skill_combine_strategy.combine_float(
                    mw_progress[tes3::Skill::LongBlade],
                    mw_progress[tes3::Skill::ShortBlade],
                ),
                tes3::Skill::Blunt => self.config.skill_combine_strategy.combine_float(
                    mw_progress[tes3::Skill::Axe],
                    mw_progress[tes3::Skill::Blunt],
                ),
                mw_skill => mw_progress[mw_skill],
            } * self
                .ob
                .calculate_skill_xp(skill, ob_skills[skill], &ob_class);
        }

        Ok(())
    }

    /// Perform a Morrowind-to-Oblivion conversion
    pub fn convert(&mut self) -> Result<()> {
        // load initial data from the Oblivion save
        let ob_save = self.ob.world.get_save().unwrap();

        let mut ob_player_base = ob_save
            .get_form_change(FORM_PLAYER)?
            .ok_or_else(|| anyhow!("Missing player change record in Oblivion save"))?;

        let mut ob_player_ref = ob_save
            .get_form_change(FORM_PLAYER_REF)?
            .ok_or_else(|| anyhow!("Missing player reference change record in Oblivion save"))?;

        // convert data
        let ob_race_form_id = self.convert_race()?;
        let (ob_class, ob_class_form_id) = self.convert_class()?;
        self.convert_stats(&mut ob_player_base, &mut ob_player_ref, &ob_class)?;

        // apply changes to save
        let ob_save = self.ob.world.get_save_mut().unwrap();

        // finalize converted forms
        let ob_race_iref = ob_save.insert_form_id(ob_race_form_id);
        let ob_class_iref = ob_save.insert_form_id(ob_class_form_id);
        ob_player_ref.set_class(
            if ob_class_form_id == FORM_PLAYER_CUSTOM_CLASS {
                Some(ob_class)
            } else {
                None
            },
            ob_class_iref,
        );
        ob_player_ref.set_race(ob_race_iref);

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
    }
}
