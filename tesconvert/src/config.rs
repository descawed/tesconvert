use std::cmp;
use std::fs;
use std::ops::{Add, Div};
use std::path::Path;

use anyhow::{Context, Result};
use clap::Result as ClapResult;
use clap::{App, Arg, SubCommand};
use ini::Ini;
use num::{Float, One};

/// The command to be executed
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Convert a Morrowind character to Oblivion
    MorrowindToOblivion,
    /// Convert an Oblivion character to Morrowind
    OblivionToMorrowind,
}

/// Strategy to use when combining values
///
/// The same set of skills is not present in all games. Sometimes, what were multiple skills in one
/// game are consolidated into a single skill in the next game. In this case, we have to decide how
/// to calculate the value of the new skill from the value of old skill. This enum holds the different
/// strategies by which this may be accomplished.
#[derive(Debug, PartialEq)]
pub enum CombineStrategy {
    /// Use the value of the highest skill
    Highest,
    /// Average the value of all skills
    Average,
    /// Use the value of the lowest skill
    Lowest,
}

impl CombineStrategy {
    /// Combines two values using the appropriate strategy
    pub fn combine<T: Ord + Add<Output = T> + Div<Output = T> + One>(&self, x: T, y: T) -> T {
        match self {
            CombineStrategy::Highest => cmp::max(x, y),
            // I wasn't able to find a better way to do this. Using a literal 2 fails because
            // there's no way to constrain T to a type that we can convert a literal 2 to. The
            // Google results I found indicated that you generally can't do math involving a literal
            // and a generic type and all recommended to use the num crate.
            CombineStrategy::Average => (x + y) / (T::one() + T::one()),
            CombineStrategy::Lowest => cmp::min(x, y),
        }
    }

    /// Combines two values with float values using the appropriate strategy
    pub fn combine_float<T: Float>(&self, x: T, y: T) -> T {
        match self {
            CombineStrategy::Highest => x.max(y),
            CombineStrategy::Average => (x + y) / (T::one() + T::one()),
            CombineStrategy::Lowest => x.min(y),
        }
    }
}

/// Iterate through INI files in a given directory
pub fn iter_form_map<P: AsRef<Path>>(ini_dir: P) -> Result<impl Iterator<Item = Ini>> {
    let mut files = vec![];
    let ini_dir = ini_dir.as_ref();
    for entry in
        fs::read_dir(ini_dir).with_context(|| format!("Error reading directory {:?}", ini_dir))?
    {
        files.push(Ini::load_from_file(entry?.path())?);
    }

    Ok(files.into_iter())
}

/// Configuration options for a conversion
#[derive(Debug)]
pub struct Config {
    /// The conversion command to execute
    pub command: Command,
    /// Path to the save file that the character is being taken from
    pub source_path: String,
    /// Path to the save file that the character is being added to
    pub target_path: String,
    /// Path to the new save file that will be created
    pub output_path: String,
    /// Path to the directory where our configuration files are stored
    pub config_path: String,
    /// Path to the Morrowind directory
    pub mw_path: Option<String>,
    /// Path to the Oblivion directory
    pub ob_path: Option<String>,
    /// Strategy to use when combining skills
    pub combine_strategy: CombineStrategy,
}

impl Config {
    fn get(maybe_options: Option<Vec<&str>>, safe: bool) -> ClapResult<Config> {
        let app = App::new("tesconvert")
            .author("descawed <tesutil@descawed.com>")
            .version("0.1")
            .about("Converts characters between Elder Scrolls games")
            .subcommand_required(true)
            .arg(
                Arg::with_name("mw_path")
                    .short('m')
                    .long("morrowind-path")
                    .takes_value(true)
                    .value_name("PATH")
                    .help("Path to the Morrowind directory")
            )
            .arg(
                Arg::with_name("ob_path")
                    .short('o')
                    .long("oblivion-path")
                    .takes_value(true)
                    .value_name("PATH")
                    .help("Path to the Oblivion directory")
            )
            .arg(
                Arg::with_name("combine")
                    .short('c')
                    .long("combine")
                    .takes_value(true)
                    .value_name("STRATEGY")
                    .possible_values(&["highest", "average", "lowest"])
                    .help("Strategy for combining skills that were consolidated between games")
                    .long_help(
                        "Certain skills that exist in one game have been combined into a single skill in later games, \
                    such as Morrowind's Short Blade and Long Blade being combined into just Blade in Oblivion. When \
                    that happens, this setting determines how the new skill is calculated from the old ones. 'highest', \
                    the default, uses the value of the highest old skill as the value of the new skill. 'average' averages \
                    the old skills to come up with the value of the new skill. 'lowest' uses the value of the lowest old \
                    skill."
                    )
            )
            .subcommand(
                SubCommand::with_name("mw2ob")
                    .about("Converts a Morrowind character to Oblivion")
                    .arg(
                        Arg::with_name("SOURCE_PATH")
                            .required(true)
                            .help("Path to the Morrowind save file")
                    )
                    .arg(
                        Arg::with_name("TARGET_PATH")
                            .required(true)
                            .help("Path to the input Oblivion save file")
                    )
                    .arg(
                        Arg::with_name("OUTPUT_PATH")
                            .required(true)
                            .help("Path to the output Oblivion save file")
                    )
            );

        let matches = match maybe_options {
            Some(options) if safe => app.get_matches_from_safe(options)?,
            Some(options) if !safe => app.get_matches_from(options),
            None if safe => app.get_matches_safe()?,
            None if !safe => app.get_matches(),
            _ => unreachable!(), // the match is exhaustive even without this arm, but the compiler doesn't understand that
        };

        let (sub_command, sub_matches) = matches.subcommand().unwrap();

        Ok(Config {
            command: match sub_command {
                "mw2ob" => Command::MorrowindToOblivion,
                _ => unreachable!(),
            },
            source_path: String::from(sub_matches.value_of("SOURCE_PATH").unwrap()),
            target_path: String::from(sub_matches.value_of("TARGET_PATH").unwrap()),
            output_path: String::from(sub_matches.value_of("OUTPUT_PATH").unwrap()),
            config_path: String::from("."),
            mw_path: matches.value_of("mw_path").map(String::from),
            ob_path: matches.value_of("ob_path").map(String::from),
            combine_strategy: match matches.value_of("combine").unwrap_or("highest") {
                "highest" => CombineStrategy::Highest,
                "average" => CombineStrategy::Average,
                "lowest" => CombineStrategy::Lowest,
                _ => unreachable!(),
            },
        })
    }

    /// Gets configuration from the command line
    ///
    /// # Panics
    ///
    /// Panics if the command line is invalid.
    pub fn get_from_cli() -> Config {
        Config::get(None, false).unwrap()
    }

    /// Gets configuration from the provided strings
    ///
    /// # Panics
    ///
    /// Panics if the provided command line is invalid.
    pub fn get_from_strings(options: Vec<&str>) -> Config {
        Config::get(Some(options), false).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args() {
        let config = Config::get(
            Some(vec![
                "tesconvert",
                "--combine",
                "lowest",
                "mw2ob",
                "source",
                "target",
                "output",
            ]),
            true,
        )
        .unwrap();
        assert_eq!(config.command, Command::MorrowindToOblivion);
        assert_eq!(config.combine_strategy, CombineStrategy::Lowest);
        assert_eq!(config.source_path, "source");
        assert_eq!(config.target_path, "target");
        assert_eq!(config.output_path, "output");
    }

    #[test]
    fn test_empty_args() {
        assert!(Config::get(Some(vec!["tesconvert"]), true).is_err());
    }

    #[test]
    fn test_bogus_args() {
        assert!(Config::get(
            Some(vec![
                "tesconvert",
                "-x",
                "4",
                "efwef",
                "path1",
                "path2",
                "path3"
            ]),
            true
        )
        .is_err());
    }

    #[test]
    fn test_insufficient_args() {
        assert!(Config::get(Some(vec!["tesconvert", "mw2ob", "source"]), true).is_err());
    }

    #[test]
    fn combine_highest() {
        let strat = CombineStrategy::Highest;
        assert_eq!(strat.combine(32, 47), 47);
    }

    #[test]
    fn combine_lowest() {
        let strat = CombineStrategy::Lowest;
        assert_eq!(strat.combine(32, 47), 32);
    }

    #[test]
    fn combine_average() {
        let strat = CombineStrategy::Average;
        assert_eq!(strat.combine(32, 47), 39);
    }
}
