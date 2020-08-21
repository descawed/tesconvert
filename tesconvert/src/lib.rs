use std::fmt;

use tesutil::plugin::tes3::*;
use tesutil::save::*;

mod config;
pub use config::*;
use std::cmp;

#[derive(Debug, Clone)]
pub struct ConversionError(String);

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConversionError {}

pub fn morrowind_to_oblivion(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let mw_save = Plugin::load_file(&config.source_path)?;
    let mut ob_save = Save::load_file(&config.target_path)?;

    // set name that appears in the save list
    let mw_save_info = mw_save.get_save_info()
        .ok_or(ConversionError(String::from("Morrowind plugin did not contain save information")))?;
    ob_save.set_player_name(String::from(mw_save_info.player_name()))?;

    // get Morrowind player information
    let mw_record = mw_save.get_record("player")?
        .ok_or(ConversionError(String::from("Missing player record in Morrowind save")))?;
    let mw_player_base = Npc::read(&mw_record)?;

    let mw_record = mw_save.get_record_with_type("PlayerSaveGame", b"REFR")
        .ok_or(ConversionError(String::from("Missing player reference record in Morrowind save")))?;
    let mw_player_ref = PlayerReference::read(&mw_record)?;

    // get Oblivion player information
    let mut ob_record_base = ob_save.get_change_record_mut(FORM_PLAYER)
        .ok_or(ConversionError(String::from("Missing player change record in Oblivion save")))?;
    let mut ob_player_base = ActorChange::read(&ob_record_base)?;
    // set attributes
    let mut attributes = ob_player_base.attributes_mut().ok_or(ConversionError(String::from("Oblivion player base has no attributes")))?;
    attributes.strength = mw_player_ref.strength.base as u8;
    attributes.intelligence = mw_player_ref.intelligence.base as u8;
    attributes.willpower = mw_player_ref.willpower.base as u8;
    attributes.agility = mw_player_ref.agility.base as u8;
    attributes.speed = mw_player_ref.speed.base as u8;
    attributes.endurance = mw_player_ref.endurance.base as u8;
    attributes.personality = mw_player_ref.personality.base as u8;
    attributes.luck = mw_player_ref.luck.base as u8;

    // set skills
    let mut skills = ob_player_base.skills_mut().ok_or(ConversionError(String::from("Oblivion player base has no skills")))?;
    skills.armorer = mw_player_ref.armorer.base as u8;
    skills.athletics = mw_player_ref.athletics.base as u8;
    skills.block = mw_player_ref.block.base as u8;
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
    match config.skill_combine_strategy {
        SkillCombineStrategy::Highest => {
            skills.blade = cmp::max(mw_player_ref.long_blade.base, mw_player_ref.short_blade.base) as u8;
            skills.blunt = cmp::max(mw_player_ref.axe.base, mw_player_ref.blunt.base) as u8;
        },
        SkillCombineStrategy::Average => {
            skills.blade = ((mw_player_ref.long_blade.base + mw_player_ref.short_blade.base)/2) as u8;
            skills.blunt = ((mw_player_ref.axe.base + mw_player_ref.blunt.base)/2) as u8;
        },
        SkillCombineStrategy::Lowest => {
            skills.blade = cmp::min(mw_player_ref.long_blade.base, mw_player_ref.short_blade.base) as u8;
            skills.blunt = cmp::min(mw_player_ref.axe.base, mw_player_ref.blunt.base) as u8;
        },
    }

    // set level
    let mut base = ob_player_base.actor_base_mut().ok_or(ConversionError(String::from("Oblivion player base has no actor base")))?;
    // TODO: add a warning here if mw_player_base.level exceeds i16::MAX? I don't think that will ever actually happen, though
    base.level = mw_player_base.level as i16;

    // save skills and attributes
    ob_player_base.write(&mut ob_record_base)?;

    // set name that appears in-game
    let mut ob_record_ref = ob_save.get_change_record_mut(FORM_PLAYER_REF)
        .ok_or(ConversionError(String::from("Missing player reference change record in Oblivion save")))?;
    let mut ob_player_ref = PlayerReferenceChange::read(&ob_record_ref)?;
    ob_player_ref.set_name(String::from(mw_player_base.name().unwrap_or("")))?;
    ob_player_ref.write(&mut ob_record_ref)?;

    // save all the changes
    ob_save.save_file(&config.output_path)?;
    
    Ok(())
}