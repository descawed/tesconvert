use std::env;
use std::process;

use tesconvert::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: tesconvert <mw save> <ob save> <output path>");
        process::exit(1);
    }

    if let Err(e) = morrowind_to_oblivion(&args[1], &args[2], &args[3]) {
        eprintln!("Conversion failed: {}", e);
        process::exit(2);
    }
}
