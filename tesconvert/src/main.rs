use anyhow::*;

use tesconvert::*;

fn main() -> Result<()> {
    let config = Config::get_from_cli();
    match config.command {
        Command::MorrowindToOblivion => morrowind_to_oblivion(&config),
        _ => unimplemented!(),
    }
}
