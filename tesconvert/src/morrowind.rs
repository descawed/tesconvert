use tesutil::tes3::plugin::*;
use tesutil::tes3::*;

use anyhow::*;

/// Container for Morrowind-related state and functionality
pub struct Morrowind {
    major_skill_bonus: f32,
    minor_skill_bonus: f32,
    misc_skill_bonus: f32,
    spec_skill_bonus: f32,
}

impl Morrowind {
    fn get_float_setting(world: &Tes3World, name: &str, default: f32) -> Result<f32> {
        Ok(match world.get::<GameSetting>(name)? {
            Some(setting) => setting.get_float().ok_or_else(|| anyhow!("Invalid game setting value"))?,
            None => default,
        })
    }

    /// Capture Morrowind state
    pub fn load(world: &Tes3World) -> Result<Morrowind> {
        Ok(Morrowind {
            major_skill_bonus: Morrowind::get_float_setting(world, "fMajorSkillBonus", 0.75)?,
            minor_skill_bonus: Morrowind::get_float_setting(world, "fMinorSkillBonus", 1.0)?,
            misc_skill_bonus: Morrowind::get_float_setting(world, "fMiscSkillBonus", 1.25)?,
            spec_skill_bonus: Morrowind::get_float_setting(world, "fSpecialSkillBonus", 0.8)?,
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
