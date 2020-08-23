use std::cmp;
use std::ops::{Add, Div};

use clap::*;
use clap::Result as ClapResult;
use num::One;

/// The command to be executed
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Convert a Morrowind character to Oblivion
    MorrowindToOblivion,
    /// Convert an Oblivion character to Morrowind
    OblivionToMorrowind,
}

/// Strategy to use when combining skills
///
/// The same set of skills is not present in all games. Sometimes, what were multiple skills in one
/// game are consolidated into a single skill in the next game. In this case, we have to decide how
/// to calculate the value of the new skill from the value of old skill. This enum holds the different
/// strategies by which this may be accomplished.
#[derive(Debug, PartialEq)]
pub enum SkillCombineStrategy {
    /// Use the value of the highest skill
    Highest,
    /// Average the value of all skills
    Average,
    /// Use the value of the lowest skill
    Lowest,
}

impl SkillCombineStrategy {
    /// Combines two skills using the appropriate strategy
    pub fn combine<T: Ord + Add<Output=T> + Div<Output=T> + One>(&self, x: T, y: T) -> T {
        match self {
            SkillCombineStrategy::Highest => cmp::max(x, y),
            // I wasn't able to find a better way to do this. Using a literal 2 fails because
            // there's no way to constrain T to a type that we can convert a literal 2 to. The
            // Google results I found indicated that you generally can't do math involving a literal
            // and a generic type and all recommended to use the num crate.
            SkillCombineStrategy::Average => (x + y)/(T::one() + T::one()),
            SkillCombineStrategy::Lowest => cmp::min(x, y),
        }
    }
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
    /// Path to the Morrowind directory
    pub mw_path: String,
    /// Path to the Oblivion directory
    pub ob_path: String,
    /// Strategy to use when combining skills
    pub skill_combine_strategy: SkillCombineStrategy,
}

impl Config {
    fn get(maybe_options: Option<Vec<&str>>, safe: bool) -> ClapResult<Config> {
        let app = App::new("tesconvert")
            .author("descawed <descawed@gmail.com>")
            .version("0.1")
            .about("Converts characters between Elder Scrolls games")
            .setting(AppSettings::SubcommandRequired)
            .arg(
                Arg::with_name("mw_path")
                    .short("m")
                    .long("morrowind-path")
                    .takes_value(true)
                    .value_name("PATH")
                    .help("Path to the Morrowind directory")
            )
            .arg(
                Arg::with_name("ob_path")
                    .short("o")
                    .long("oblivion-path")
                    .takes_value(true)
                    .value_name("PATH")
                    .help("Path to the Oblivion directory")
            )
            .arg(
                Arg::with_name("combine")
                    .short("c")
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

        let (sub_command, sub_matches) = matches.subcommand();
        let sub_matches = sub_matches.unwrap();

        Ok(Config {
            command: match sub_command {
                "mw2ob" => Command::MorrowindToOblivion,
                _ => unreachable!(),
            },
            source_path: String::from(sub_matches.value_of("SOURCE_PATH").unwrap()),
            target_path: String::from(sub_matches.value_of("TARGET_PATH").unwrap()),
            output_path: String::from(sub_matches.value_of("OUTPUT_PATH").unwrap()),
            mw_path: String::from(""),
            ob_path: String::from(""),
            skill_combine_strategy: match matches.value_of("combine").unwrap_or("highest") {
                "highest" => SkillCombineStrategy::Highest,
                "average" => SkillCombineStrategy::Average,
                "lowest" => SkillCombineStrategy::Lowest,
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
        let config = Config::get(Some(vec!["tesconvert", "--combine", "lowest", "mw2ob", "source", "target", "output"]), true).unwrap();
        assert_eq!(config.command, Command::MorrowindToOblivion);
        assert_eq!(config.skill_combine_strategy, SkillCombineStrategy::Lowest);
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
        assert!(Config::get(Some(vec!["tesconvert", "-x", "4", "efwef", "path1", "path2", "path3"]), true).is_err());
    }

    #[test]
    fn test_insufficient_args() {
        assert!(Config::get(Some(vec!["tesconvert", "mw2ob", "source"]), true).is_err());
    }

    #[test]
    fn combine_highest() {
        let strat = SkillCombineStrategy::Highest;
        assert_eq!(strat.combine(32, 47), 47);
    }

    #[test]
    fn combine_lowest() {
        let strat = SkillCombineStrategy::Lowest;
        assert_eq!(strat.combine(32, 47), 32);
    }

    #[test]
    fn combine_average() {
        let strat = SkillCombineStrategy::Average;
        assert_eq!(strat.combine(32, 47), 39);
    }
}