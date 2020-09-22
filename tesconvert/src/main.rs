use anyhow::*;

use tesconvert::*;

fn main() -> Result<()> {
    let config = Config::get_from_cli();
    convert(config)
}
