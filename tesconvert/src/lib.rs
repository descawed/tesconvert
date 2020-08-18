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

    let mw_save_info = mw_save.get_save_info()
        .ok_or(ConversionError(String::from("Morrowind plugin did not contain save information")))?;
    // set name that appears in the save list
    ob_save.set_player_name(String::from(mw_save_info.player_name()))?;
    // set name that appears in-game
    let mw_record = mw_save.get_record("player")?
        .ok_or(ConversionError(String::from("Missing player record in Morrowind save")))?;
    // TODO: need to grab PlayerSaveGame REFR to get skills and attributes and NPCC to get inventory
    //  (going to need to update tes3::plugin to support multiple records with the same name)
    let mw_player = Npc::read(&mw_record)?;

    let mut ob_record = ob_save.get_change_record_mut(FORM_PLAYER_REF)
        .ok_or(ConversionError(String::from("Missing player change record in Oblivion save")))?;
    let mut ob_player = PlayerReferenceChange::read(&ob_record)?;
    ob_player.set_name(String::from(mw_player.name().unwrap_or("")))?;
    ob_player.write(&mut ob_record)?;

    ob_save.save_file(output_path)?;
    
    Ok(())
}