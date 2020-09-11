use tesutil::tes4::plugin::*;
use tesutil::tes4::*;

use anyhow::*;

/// Container for Oblivion-related state and functionality
pub struct Oblivion {
    skill_use_exp: f32,
    skill_use_factor: f32,
    major_skill_mult: f32,
    minor_skill_mult: f32,
    spec_skill_mult: f32,
}

impl Oblivion {
    /// Capture Oblivion state
    pub fn load(world: &Tes4World) -> Result<Oblivion> {
        // the defaults of 1.0 here are the hard-coded defaults in the exe, as you can see when opening
        // the CS without any plugins loaded.
        Ok(Oblivion {
            skill_use_exp: world.get_float_setting("fSkillUseExp", 1.0)?,
            skill_use_factor: world.get_float_setting("fSkillUseFactor", 1.0)?,
            major_skill_mult: world.get_float_setting("fSkillUseMajorMult", 0.75)?,
            minor_skill_mult: world.get_float_setting("fSkillUseMinorMult", 1.25)?,
            spec_skill_mult: world.get_float_setting("fSkillUseSpecMult", 0.75)?,
        })
    }

    /// Calculates the XP required to level a skill up
    pub fn calculate_skill_xp<T: Into<f32>>(
        &self,
        skill: Skill,
        level: T,
        class: &Class,
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

        mult*(self.skill_use_factor*level).powf(self.skill_use_exp)
    }
}
