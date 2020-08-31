use anyhow::*;

use tesutil::tes3::plugin::*;
use tesutil::tes3::Skill as Skill3;
use tesutil::tes4::save::*;
use tesutil::tes4::Skill as Skill4;
use tesutil::Attributes;

mod config;
pub use config::*;

pub fn morrowind_to_oblivion(config: &Config) -> Result<()> {
    let mw_save = Plugin::load_file(&config.source_path)?;
    let mut ob_save = Save::load_file(&config.target_path)?;

    // set name that appears in the save list
    let mw_save_info = mw_save
        .get_save_info()
        .ok_or_else(|| anyhow!("Morrowind plugin did not contain save information"))?;
    ob_save.set_player_name(String::from(mw_save_info.player_name()))?;

    // get Morrowind player information
    let mw_record = mw_save
        .get_record("player")?
        .ok_or_else(|| anyhow!("Missing player record in Morrowind save"))?;
    let mw_player_base = Npc::read(&mw_record)?;

    let mw_record = mw_save
        .get_record_with_type("PlayerSaveGame", b"REFR")
        .ok_or_else(|| anyhow!("Missing player reference record in Morrowind save"))?;
    let mw_player_ref = PlayerReference::read(&mw_record)?;

    let mw_record = mw_save
        .get_records_by_type(b"PCDT")
        .ok_or_else(|| anyhow!("Missing player data record (PCDT) in Morrowind save"))?
        .next()
        .ok_or_else(|| anyhow!("Missing player data record (PCDT) in Morrowind save"))?;
    let mw_player_data = PlayerData::read(&mw_record)?;

    // get Oblivion player information
    let mut ob_record_base = ob_save
        .get_change_record_mut(FORM_PLAYER)
        .ok_or_else(|| anyhow!("Missing player change record in Oblivion save"))?;
    let mut ob_player_base = ActorChange::read(&ob_record_base)?;
    // set attributes
    let attributes = ob_player_base
        .attributes_mut()
        .ok_or_else(|| anyhow!("Oblivion player base has no attributes"))?;
    for (attribute, value) in attributes.iter_mut() {
        *value = mw_player_ref.attributes[attribute].base as u8;
    }

    // set skills
    let skills = ob_player_base
        .skills_mut()
        .ok_or_else(|| anyhow!("Oblivion player base has no skills"))?;
    skills[Skill4::Armorer] = mw_player_ref.skills[Skill3::Armorer].base as u8;
    skills[Skill4::Athletics] = mw_player_ref.skills[Skill3::Athletics].base as u8;
    skills[Skill4::Blade] = config.skill_combine_strategy.combine(
        mw_player_ref.skills[Skill3::LongBlade].base,
        mw_player_ref.skills[Skill3::ShortBlade].base,
    ) as u8;
    skills[Skill4::Block] = mw_player_ref.skills[Skill3::Block].base as u8;
    skills[Skill4::Blunt] = config.skill_combine_strategy.combine(
        mw_player_ref.skills[Skill3::Axe].base,
        mw_player_ref.skills[Skill3::Blunt].base,
    ) as u8;
    skills[Skill4::HandToHand] = mw_player_ref.skills[Skill3::HandToHand].base as u8;
    skills[Skill4::HeavyArmor] = mw_player_ref.skills[Skill3::HeavyArmor].base as u8;
    skills[Skill4::Alchemy] = mw_player_ref.skills[Skill3::Alchemy].base as u8;
    skills[Skill4::Alteration] = mw_player_ref.skills[Skill3::Alteration].base as u8;
    skills[Skill4::Conjuration] = mw_player_ref.skills[Skill3::Conjuration].base as u8;
    skills[Skill4::Destruction] = mw_player_ref.skills[Skill3::Destruction].base as u8;
    skills[Skill4::Illusion] = mw_player_ref.skills[Skill3::Illusion].base as u8;
    skills[Skill4::Mysticism] = mw_player_ref.skills[Skill3::Mysticism].base as u8;
    skills[Skill4::Restoration] = mw_player_ref.skills[Skill3::Restoration].base as u8;
    skills[Skill4::Acrobatics] = mw_player_ref.skills[Skill3::Acrobatics].base as u8;
    skills[Skill4::LightArmor] = mw_player_ref.skills[Skill3::LightArmor].base as u8;
    skills[Skill4::Marksman] = mw_player_ref.skills[Skill3::Marksman].base as u8;
    skills[Skill4::Mercantile] = mw_player_ref.skills[Skill3::Mercantile].base as u8;
    skills[Skill4::Security] = mw_player_ref.skills[Skill3::Security].base as u8;
    skills[Skill4::Sneak] = mw_player_ref.skills[Skill3::Sneak].base as u8;
    skills[Skill4::Speechcraft] = mw_player_ref.skills[Skill3::Speechcraft].base as u8;

    // set level
    if ob_player_base.actor_base().is_none() {
        // can happen if the player is level 1
        let base = ActorBase::default();
        ob_player_base.set_actor_base(Some(base));
    }

    let mut base = ob_player_base.actor_base_mut().unwrap();
    base.level = mw_player_base.level as i16;

    // save skills and attributes
    ob_player_base.write(&mut ob_record_base)?;

    // set name and level/skill progress
    let mut ob_record_ref = ob_save
        .get_change_record_mut(FORM_PLAYER_REF)
        .ok_or_else(|| anyhow!("Missing player reference change record in Oblivion save"))?;
    let mut ob_player_ref = PlayerReferenceChange::read(&ob_record_ref)?;
    ob_player_ref.set_name(String::from(mw_player_base.name().unwrap_or("")))?;

    ob_player_ref.major_skill_advancements = mw_player_data.level_progress;

    for (spec, value) in ob_player_ref.spec_increases.iter_mut() {
        *value = mw_player_data.spec_increases[spec];
    }

    let mut advancements = vec![];
    // Morrowind doesn't track advancements by level like Oblivion does, so we have to fake it here.
    // I don't actually know how Oblivion would handle an advancement greater than 10, but it never
    // happens in normal gameplay, so I figure it's best to enforce it here.
    let mut attributes = mw_player_data.attribute_progress.clone();
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

    // TODO: skill usage and XP

    ob_player_ref.write(&mut ob_record_ref)?;

    // save all the changes
    ob_save.save_file(&config.output_path)?;

    Ok(())
}
