use tesutil::plugin::*;

fn main() {
    let plugin = Plugin::load_file(r"F:\Steam\steamapps\common\Morrowind\Data Files\DaedricArmorGod.ESP").unwrap();
    println!("Author: {}; Description: {}", plugin.author(), plugin.description());
}
