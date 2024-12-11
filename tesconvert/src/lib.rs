use anyhow::*;

mod config;
pub use config::*;

mod morrowind;
mod oblivion;

use morrowind::*;

pub fn convert(config: Config) -> Result<()> {
    match config.command {
        Action::MorrowindToOblivion => {
            let mw2ob = MorrowindToOblivion::load(config)?;
            mw2ob.convert()
        }
        _ => unimplemented!(),
    }
}
