use std::cmp;
use std::fs;
use std::ops::{Add, Div};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{Arg, Command};
use ini::Ini;
use num::{Float, One};

/// The conversion action to be executed
#[derive(Debug, PartialEq)]
pub enum Action {
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
    pub command: Action,
    /// Path to the save file that the character is being taken from
    pub source_path: PathBuf,
    /// Path to the save file that the character is being added to
    pub target_path: PathBuf,
    /// Path to the new save file that will be created
    pub output_path: PathBuf,
    /// Path to the directory where our configuration files are stored
    pub config_path: PathBuf,
    /// Path to the Morrowind directory
    pub mw_path: Option<PathBuf>,
    /// Path to the Oblivion directory
    pub ob_path: Option<PathBuf>,
    /// Strategy to use when combining skills
    pub combine_strategy: CombineStrategy,
    /// MW:OB equipment durability ratio
    pub equipment_durability_ratio: f32,
}

impl Config {
    fn get(maybe_options: Option<Vec<&str>>) -> Result<Config> {
        let app = Command::new("tesconvert")
            .author("descawed <tesutil@descawed.com>")
            .version("0.1")
            .about("Converts characters between Elder Scrolls games")
            .subcommand_required(true)
            .arg(
                Arg::new("mw_path")
                    .short('m')
                    .long("morrowind-path")
                    .value_parser(clap::value_parser!(PathBuf))
                    .value_name("PATH")
                    .help("Path to the Morrowind directory")
            )
            .arg(
                Arg::new("ob_path")
                    .short('o')
                    .long("oblivion-path")
                    .value_parser(clap::value_parser!(PathBuf))
                    .value_name("PATH")
                    .help("Path to the Oblivion directory")
            )
            .arg(
                Arg::new("combine")
                    .short('c')
                    .long("combine")
                    .value_name("STRATEGY")
                    .value_parser(["highest", "average", "lowest"])
                    .default_value("highest")
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
            .arg(
                Arg::new("durability_ratio")
                    .short('d')
                    .long("durability")
                    .value_parser(clap::value_parser!(f32))
                    .value_name("RATIO")
                    .help("Ratio of Morrowind equipment durability to Oblivion equipment durability")
                    .long_help(
                        "Morrowind uses a larger range for equipment durability values than Oblivion. For example, in Morrowind \
                        an Iron Dagger has 400 health and a Daedric Warhammer has 8000, whereas in Oblivion an Iron Dagger has \
                        70 health and a Daedric Warhammer has 784. For this reason, we divide (coming from Morrowind) or multiply \
                        (coming from Oblivion) by this ratio to scale the values from one game to be reasonable in the other."
                    )
            )
            .subcommand(
                Command::new("mw2ob")
                    .about("Converts a Morrowind character to Oblivion")
                    .arg(
                        Arg::new("SOURCE_PATH")
                            .required(true)
                            .help("Path to the Morrowind save file")
                            .value_parser(clap::value_parser!(PathBuf))
                    )
                    .arg(
                        Arg::new("TARGET_PATH")
                            .required(true)
                            .help("Path to the input Oblivion save file")
                            .value_parser(clap::value_parser!(PathBuf))
                    )
                    .arg(
                        Arg::new("OUTPUT_PATH")
                            .required(true)
                            .help("Path to the output Oblivion save file")
                            .value_parser(clap::value_parser!(PathBuf))
                    )
            );

        let mut matches = match maybe_options {
            Some(options) => app.get_matches_from(options),
            None => app.get_matches(),
        };

        let (sub_command, mut sub_matches) = matches.remove_subcommand().unwrap();

        Ok(Config {
            command: match sub_command.as_str() {
                "mw2ob" => Action::MorrowindToOblivion,
                _ => unreachable!(),
            },
            source_path: sub_matches.remove_one::<PathBuf>("SOURCE_PATH").expect("Source save path is required"),
            target_path: sub_matches.remove_one::<PathBuf>("TARGET_PATH").expect("Target save path is required"),
            output_path: sub_matches.remove_one::<PathBuf>("OUTPUT_PATH").expect("Output save path is required"),
            config_path: PathBuf::from("."),
            mw_path: matches.remove_one::<PathBuf>("mw_path"),
            ob_path: matches.remove_one::<PathBuf>("ob_path"),
            combine_strategy: match *matches.get_one::<&str>("combine").unwrap_or(&"highest") {
                "highest" => CombineStrategy::Highest,
                "average" => CombineStrategy::Average,
                "lowest" => CombineStrategy::Lowest,
                _ => unreachable!(),
            },
            equipment_durability_ratio: matches
                .get_one::<String>("durability_ratio")
                .map(|v| f32::from_str(v))
                .unwrap_or(Ok(5.))?,
        })
    }

    /// Gets configuration from the command line
    ///
    /// # Panics
    ///
    /// Panics if the command line is invalid.
    pub fn get_from_cli() -> Config {
        Config::get(None).unwrap()
    }

    /// Gets configuration from the provided strings
    ///
    /// # Panics
    ///
    /// Panics if the provided command line is invalid.
    pub fn get_from_strings(options: Vec<&str>) -> Config {
        Config::get(Some(options)).unwrap()
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
        )
        .unwrap();
        assert_eq!(config.command, Action::MorrowindToOblivion);
        assert_eq!(config.combine_strategy, CombineStrategy::Lowest);
        assert_eq!(config.source_path, PathBuf::from("source"));
        assert_eq!(config.target_path, PathBuf::from("target"));
        assert_eq!(config.output_path, PathBuf::from("output"));
    }

    #[test]
    fn test_empty_args() {
        assert!(Config::get(Some(vec!["tesconvert"])).is_err());
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
        )
        .is_err());
    }

    #[test]
    fn test_insufficient_args() {
        assert!(Config::get(Some(vec!["tesconvert", "mw2ob", "source"])).is_err());
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
