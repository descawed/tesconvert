use tesutil::plugin::tes3::*;
use tesutil::save::*;

fn main() {
    let mw_save = Plugin::load_file(r"F:\Steam\steamapps\common\Morrowind\Saves\quiksave.ess").unwrap();
    let mut ob_save = Save::load_file(r"C:\Users\Jacob\Documents\My Games\Oblivion\Saves\autosave.ess").unwrap();

    let mw_save_info = mw_save.get_save_info().unwrap();
    ob_save.set_player_name(String::from(mw_save_info.player_name())).unwrap();
    ob_save.save_file(r"C:\Users\Jacob\Documents\My Games\Oblivion\Saves\mwconvert.ess").unwrap();
}
