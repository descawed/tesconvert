use std::process;

use tesconvert::*;

fn main() {
    let config = Config::get_from_cli();
    let result = match config.command {
        Command::MorrowindToOblivion => morrowind_to_oblivion(&config),
        _ => unimplemented!(),
    };

    if let Err(e) = result {
        eprintln!("Conversion failed: {}", e);
        process::exit(2);
    }
}
