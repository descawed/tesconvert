use anyhow::*;

use tesutil::tes3::plugin::*;
use tesutil::tes4::save::*;

mod config;
pub use config::*;

pub fn morrowind_to_oblivion(config: &Config) -> Result<()> {
    let mw_save = Plugin::load_file(&config.source_path)?;
    let mut ob_save = Save::load_file(&config.target_path)?;

    // set name that appears in the save list
    let mw_save_info = mw_save
        .get_save_info()
        .ok_or(anyhow!("Morrowind plugin did not contain save information"))?;
    ob_save.set_player_name(String::from(mw_save_info.player_name()))?;

    // get Morrowind player information
    let mw_record = mw_save
        .get_record("player")?
        .ok_or(anyhow!("Missing player record in Morrowind save"))?;
    let mw_player_base = Npc::read(&mw_record)?;

    let mw_record = mw_save
        .get_record_with_type("PlayerSaveGame", b"REFR")
        .ok_or(anyhow!("Missing player reference record in Morrowind save"))?;
    let mw_player_ref = PlayerReference::read(&mw_record)?;

    let mw_record = mw_save
        .get_records_by_type(b"PCDT")
        .ok_or(anyhow!(
            "Missing player data record (PCDT) in Morrowind save"
        ))?
        .next()
        .ok_or(anyhow!(
            "Missing player data record (PCDT) in Morrowind save"
        ))?;
    let mw_player_data = PlayerData::read(&mw_record)?;

    // get Oblivion player information
    let mut ob_record_base = ob_save
        .get_change_record_mut(FORM_PLAYER)
        .ok_or(anyhow!("Missing player change record in Oblivion save"))?;
    let mut ob_player_base = ActorChange::read(&ob_record_base)?;
    // set attributes
    let mut attributes = ob_player_base
        .attributes_mut()
        .ok_or(anyhow!("Oblivion player base has no attributes"))?;
    attributes.strength = mw_player_ref.strength.base as u8;
    attributes.intelligence = mw_player_ref.intelligence.base as u8;
    attributes.willpower = mw_player_ref.willpower.base as u8;
    attributes.agility = mw_player_ref.agility.base as u8;
    attributes.speed = mw_player_ref.speed.base as u8;
    attributes.endurance = mw_player_ref.endurance.base as u8;
    attributes.personality = mw_player_ref.personality.base as u8;
    attributes.luck = mw_player_ref.luck.base as u8;

    // set skills
    let mut skills = ob_player_base
        .skills_mut()
        .ok_or(anyhow!("Oblivion player base has no skills"))?;
    skills.armorer = mw_player_ref.armorer.base as u8;
    skills.athletics = mw_player_ref.athletics.base as u8;
    skills.blade = config.skill_combine_strategy.combine(
        mw_player_ref.long_blade.base,
        mw_player_ref.short_blade.base,
    ) as u8;
    skills.block = mw_player_ref.block.base as u8;
    skills.blunt = config
        .skill_combine_strategy
        .combine(mw_player_ref.axe.base, mw_player_ref.blunt.base) as u8;
    skills.hand_to_hand = mw_player_ref.hand_to_hand.base as u8;
    skills.heavy_armor = mw_player_ref.heavy_armor.base as u8;
    skills.alchemy = mw_player_ref.alchemy.base as u8;
    skills.alteration = mw_player_ref.alteration.base as u8;
    skills.conjuration = mw_player_ref.conjuration.base as u8;
    skills.destruction = mw_player_ref.destruction.base as u8;
    skills.illusion = mw_player_ref.illusion.base as u8;
    skills.mysticism = mw_player_ref.mysticism.base as u8;
    skills.restoration = mw_player_ref.restoration.base as u8;
    skills.acrobatics = mw_player_ref.acrobatics.base as u8;
    skills.light_armor = mw_player_ref.light_armor.base as u8;
    skills.marksman = mw_player_ref.marksman.base as u8;
    skills.mercantile = mw_player_ref.mercantile.base as u8;
    skills.security = mw_player_ref.security.base as u8;
    skills.sneak = mw_player_ref.sneak.base as u8;
    skills.speechcraft = mw_player_ref.speechcraft.base as u8;

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
        .ok_or(anyhow!(
            "Missing player reference change record in Oblivion save"
        ))?;
    let mut ob_player_ref = PlayerReferenceChange::read(&ob_record_ref)?;
    ob_player_ref.set_name(String::from(mw_player_base.name().unwrap_or("")))?;

    ob_player_ref.major_skill_advancements = mw_player_data.level_progress;

    ob_player_ref.combat_increases = mw_player_data.combat_increases;
    ob_player_ref.magic_increases = mw_player_data.magic_increases;
    ob_player_ref.stealth_increases = mw_player_data.stealth_increases;

    let mut advancements = vec![];
    // Morrowind doesn't track advancements by level like Oblivion does, so we have to fake it here.
    // I don't actually know how Oblivion would handle an advancement greater than 10, but it never
    // happens in normal gameplay, so I figure it's best to enforce it here.
    let mut attributes = Attributes {
        strength: mw_player_data.strength_progress,
        intelligence: mw_player_data.intelligence_progress,
        willpower: mw_player_data.willpower_progress,
        agility: mw_player_data.agility_progress,
        endurance: mw_player_data.endurance_progress,
        speed: mw_player_data.speed_progress,
        personality: mw_player_data.personality_progress,
        luck: mw_player_data.luck_progress,
    };
    while !attributes.are_all_zero() {
        let advancement = Attributes {
            strength: attributes.strength % 10,
            intelligence: attributes.intelligence % 10,
            willpower: attributes.willpower % 10,
            agility: attributes.agility % 10,
            endurance: attributes.endurance % 10,
            speed: attributes.speed % 10,
            personality: attributes.personality % 10,
            luck: attributes.luck % 10,
        };

        attributes.strength -= advancement.strength;
        attributes.intelligence -= advancement.intelligence;
        attributes.willpower -= advancement.willpower;
        attributes.agility -= advancement.agility;
        attributes.endurance -= advancement.endurance;
        attributes.speed -= advancement.speed;
        attributes.personality -= advancement.personality;
        attributes.luck -= advancement.luck;

        advancements.push(advancement);
    }
    ob_player_ref.advancements = advancements;

    // TODO: skill usage and XP

    ob_player_ref.write(&mut ob_record_ref)?;

    // save all the changes
    ob_save.save_file(&config.output_path)?;

    Ok(())
}
