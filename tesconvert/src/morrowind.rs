use tesutil::tes3::plugin::*;
use tesutil::tes3::*;

use anyhow::*;

/// Calculates the XP required to level a skill up
pub fn calculate_skill_xp<T: Into<f32>>(
    skill: Skill,
    level: T,
    class: &Class,
    world: &Tes3World,
) -> Result<f32> {
    let level = level.into();
    let bonus_setting: GameSetting = world
        .get(match class.get_skill_type(skill) {
            SkillType::Major => "fMajorSkillBonus",
            SkillType::Minor => "fMinorSkillBonus",
            SkillType::Miscellaneous => "fMiscSkillBonus",
        })?
        .ok_or_else(|| anyhow!("Missing skill bonus game setting"))?;
    let bonus_value = bonus_setting
        .get_float()
        .ok_or_else(|| anyhow!("Invalid game setting value"))?;
    let xp = level * bonus_value;

    Ok(if skill.specialization() == class.specialization {
        let bonus_setting: GameSetting = world
            .get("fSpecialSkillBonus")?
            .ok_or_else(|| anyhow!("Missing specialization GMST"))?;
        let bonus_value = bonus_setting
            .get_float()
            .ok_or_else(|| anyhow!("Invalid specialization game setting value"))?;
        xp * bonus_value
    } else {
        xp
    })
}
