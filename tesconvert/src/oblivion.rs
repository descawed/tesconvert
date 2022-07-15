use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use tesutil::tes3;
use tesutil::tes4;
use tesutil::tes4::Tes4World;
use tesutil::EffectRange;

use anyhow::{Result, anyhow};
#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

/// Container for Oblivion-related state and functionality
#[derive(Debug)]
pub struct Oblivion {
    world: RefCell<Tes4World>,
    // skill XP settings
    skill_use_exp: f32,
    skill_use_factor: f32,
    major_skill_mult: f32,
    minor_skill_mult: f32,
    spec_skill_mult: f32,
    // magic cost settings
    base_cost_mult: f32,
    cost_scale: f32,
    area_cost_mult: f32,
    range_cost_mult: f32,
    // magic tier settings
    apprentice_min: f32,
    journeyman_min: f32,
    expert_min: f32,
    master_min: f32,
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

        // the defaults here are the hard-coded defaults in the exe, as you can see when opening
        // the CS without any plugins loaded.
        let skill_use_exp = world.get_float_setting("fSkillUseExp", 1.0)?;
        let skill_use_factor = world.get_float_setting("fSkillUseFactor", 1.0)?;
        let major_skill_mult = world.get_float_setting("fSkillUseMajorMult", 0.75)?;
        let minor_skill_mult = world.get_float_setting("fSkillUseMinorMult", 1.25)?;
        let spec_skill_mult = world.get_float_setting("fSkillUseSpecMult", 0.75)?;

        let base_cost_mult = world.get_float_setting("fMagicDurMagBaseCostMult", 0.1)?;
        let cost_scale = world.get_float_setting("fMagicCostScale", 1.25)?;
        let area_cost_mult = world.get_float_setting("fMagicAreaBaseCostMult", 0.15)?;
        let range_cost_mult = world.get_float_setting("fMagicRangeTargetCostMult", 1.5)?;

        let apprentice_min = world.get_float_setting("fMagicSpellLevelApprenticeMin", 25.0)?;
        let journeyman_min = world.get_float_setting("fMagicSpellLevelJourneymanMin", 50.0)?;
        let expert_min = world.get_float_setting("fMagicSpellLevelExpertMin", 75.0)?;
        let master_min = world.get_float_setting("fMagicSpellLevelMasterMin", 100.0)?;

        Ok(Oblivion {
            world: RefCell::new(world),
            skill_use_exp,
            skill_use_factor,
            major_skill_mult,
            minor_skill_mult,
            spec_skill_mult,
            base_cost_mult,
            cost_scale,
            area_cost_mult,
            range_cost_mult,
            apprentice_min,
            journeyman_min,
            expert_min,
            master_min,
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

    /// Auto-calculates a spell's cost
    pub fn calculate_spell_cost(&self, spell: &mut tes4::Spell) -> Result<()> {
        use tes4::SpellLevel::*;

        let cost = if spell.is_auto_calc() {
            let world = self.world.borrow();
            let mut total_cost = 0f32;

            for effect in spell.effects() {
                let base_effect = world.get_magic_effect(effect.effect_type())?;

                let effect_factor = base_effect.base_cost() * self.base_cost_mult;
                let magnitude_factor = (effect.magnitude() as f32).powf(self.cost_scale).max(1.);
                let duration_factor = (effect.duration() as f32).max(1.);
                let area_factor = (effect.area() as f32 * self.area_cost_mult).max(1.);
                let range_factor = if effect.range() == EffectRange::Target {
                    self.range_cost_mult
                } else {
                    1.
                };

                total_cost +=
                    effect_factor * magnitude_factor * duration_factor * area_factor * range_factor;
            }

            spell.cost = total_cost as u32;
            total_cost
        } else {
            spell.cost as f32
        };

        spell.level = if cost >= self.master_min {
            Master
        } else if cost >= self.expert_min {
            Expert
        } else if cost >= self.journeyman_min {
            Journeyman
        } else if cost >= self.apprentice_min {
            Apprentice
        } else {
            Novice
        };

        Ok(())
    }

    /// Gets the Oblivion world
    pub fn world(&self) -> impl Deref<Target = Tes4World> + '_ {
        self.world.borrow()
    }

    /// Gets the Oblivion world mutably
    pub fn world_mut(&self) -> impl Deref<Target = Tes4World> + DerefMut<Target = Tes4World> + '_ {
        self.world.borrow_mut()
    }
}
