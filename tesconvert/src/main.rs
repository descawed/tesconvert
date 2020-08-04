use tesutil::plugin::tes3::*;

fn main() {
    let plugin = Plugin::load_file(r"F:\Steam\steamapps\common\Morrowind\Data Files\Morrowind.esm").unwrap();
    println!("Author: {}; Description: {}", plugin.author(), plugin.description());
}
