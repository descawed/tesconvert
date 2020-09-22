use std::path::Path;

use tesutil::tes3;
use tesutil::tes4;
use tesutil::tes4::Tes4World;

use anyhow::*;
#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

/// Container for Oblivion-related state and functionality
#[derive(Debug)]
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

    /// Gets the Morrowind skill equivalent to a given Oblivion skill, if one exists
    pub fn morrowind_skill(skill: tes4::Skill) -> tes3::Skill {
        match skill {
            tes4::Skill::Block => tes3::Skill::Block,
            tes4::Skill::Armorer => tes3::Skill::Armorer,
            tes4::Skill::HeavyArmor => tes3::Skill::HeavyArmor,
            tes4::Skill::Blunt => tes3::Skill::Blunt,
            tes4::Skill::Blade => tes3::Skill::LongBlade,
            tes4::Skill::Athletics => tes3::Skill::Athletics,
            tes4::Skill::Destruction => tes3::Skill::Destruction,
            tes4::Skill::Alteration => tes3::Skill::Alteration,
            tes4::Skill::Illusion => tes3::Skill::Illusion,
            tes4::Skill::Conjuration => tes3::Skill::Conjuration,
            tes4::Skill::Mysticism => tes3::Skill::Mysticism,
            tes4::Skill::Restoration => tes3::Skill::Restoration,
            tes4::Skill::Alchemy => tes3::Skill::Alchemy,
            tes4::Skill::Security => tes3::Skill::Security,
            tes4::Skill::Sneak => tes3::Skill::Sneak,
            tes4::Skill::Acrobatics => tes3::Skill::Acrobatics,
            tes4::Skill::LightArmor => tes3::Skill::LightArmor,
            tes4::Skill::Marksman => tes3::Skill::Marksman,
            tes4::Skill::Mercantile => tes3::Skill::Mercantile,
            tes4::Skill::Speechcraft => tes3::Skill::Speechcraft,
            tes4::Skill::HandToHand => tes3::Skill::HandToHand,
        }
    }

    /// Calculates the XP required to level a skill up
    pub fn calculate_skill_xp<T: Into<f32>>(
        &self,
        skill: tes4::Skill,
        level: T,
        class: &tes4::Class,
    ) -> f32 {
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
