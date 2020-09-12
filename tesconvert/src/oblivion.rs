use std::path::Path;

use tesutil::tes4::plugin::*;
use tesutil::tes4::*;

use anyhow::*;
#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

/// Container for Oblivion-related state and functionality
pub struct Oblivion {
    pub world: Tes4World,
    skill_use_exp: f32,
    skill_use_factor: f32,
    major_skill_mult: f32,
    minor_skill_mult: f32,
    spec_skill_mult: f32,
}

impl Oblivion {
    /// Capture Oblivion state
    pub fn load<P, Q>(game_dir: Option<P>, save_path: Q) -> Result<Oblivion>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let world = match game_dir {
            Some(path) => Tes4World::load_from_save(path, save_path),
            None => Tes4World::load_from_save(Oblivion::detect_dir()?, save_path),
        }?;
        // the defaults of 1.0 here are the hard-coded defaults in the exe, as you can see when opening
        // the CS without any plugins loaded.
        let skill_use_exp = world.get_float_setting("fSkillUseExp", 1.0)?;
        let skill_use_factor = world.get_float_setting("fSkillUseFactor", 1.0)?;
        let major_skill_mult = world.get_float_setting("fSkillUseMajorMult", 0.75)?;
        let minor_skill_mult = world.get_float_setting("fSkillUseMinorMult", 1.25)?;
        let spec_skill_mult = world.get_float_setting("fSkillUseSpecMult", 0.75)?;

        Ok(Oblivion {
            world,
            skill_use_exp,
            skill_use_factor,
            major_skill_mult,
            minor_skill_mult,
            spec_skill_mult,
        })
    }

    #[cfg(windows)]
    fn detect_dir() -> Result<String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey_with_flags(
            r"SOFTWARE\Bethesda Softworks\Oblivion",
            KEY_READ | KEY_WOW64_32KEY,
        ) {
            if let Ok(path) = key.get_value("installed path") {
                return Ok(path);
            }
        }

        Err(anyhow!("Could not detect Oblivion install path"))
    }

    #[cfg(not(windows))]
    fn detect_dir() -> Result<String> {
        // TODO: refer to OpenMW code for detecting Wine installations
        Err(anyhow!("Could not detect Morrowind install path"))
    }

    /// Calculates the XP required to level a skill up
    pub fn calculate_skill_xp<T: Into<f32>>(&self, skill: Skill, level: T, class: &Class) -> f32 {
        let level = level.into();
        let mut mult = if class.is_major_skill(skill) {
            self.major_skill_mult
        } else {
            self.minor_skill_mult
        };

        if class.specialization == skill.specialization() {
            mult *= self.spec_skill_mult;
        }

        mult * (self.skill_use_factor * level).powf(self.skill_use_exp)
    }
}
