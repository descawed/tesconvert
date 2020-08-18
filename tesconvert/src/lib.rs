use std::fmt;

use tesutil::plugin::tes3::*;
use tesutil::save::*;

#[derive(Debug, Clone)]
pub struct ConversionError(String);

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConversionError {}

pub fn morrowind_to_oblivion(mw_path: &str, ob_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mw_save = Plugin::load_file(mw_path)?;
    let mut ob_save = Save::load_file(ob_path)?;

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
    ob_player_base.write(&mut ob_record_base)?;

    // set name that appears in-game
    let mut ob_record_ref = ob_save.get_change_record_mut(FORM_PLAYER_REF)
        .ok_or(ConversionError(String::from("Missing player reference change record in Oblivion save")))?;
    let mut ob_player_ref = PlayerReferenceChange::read(&ob_record_ref)?;
    ob_player_ref.set_name(String::from(mw_player_base.name().unwrap_or("")))?;
    ob_player_ref.write(&mut ob_record_ref)?;

    // save all the changes
    ob_save.save_file(output_path)?;
    
    Ok(())
}