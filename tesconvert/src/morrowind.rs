use std::path::Path;

use tesutil::tes3::*;

use anyhow::*;
#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

/// Container for Morrowind-related state and functionality
pub struct Morrowind {
    pub world: Tes3World,
    major_skill_bonus: f32,
    minor_skill_bonus: f32,
    misc_skill_bonus: f32,
    spec_skill_bonus: f32,
}

impl Morrowind {
    fn get_float_setting(world: &Tes3World, name: &str, default: f32) -> Result<f32> {
        Ok(match world.get::<GameSetting>(name)? {
            Some(setting) => setting
                .get_float()
                .ok_or_else(|| anyhow!("Invalid game setting value"))?,
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

    /// Calculates the XP required to level a skill up
    pub fn calculate_skill_xp<T: Into<f32>>(&self, skill: Skill, level: T, class: &Class) -> f32 {
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
