use std::fs;
use std::path::Path;

use ini::Ini;

use super::plugin::*;
use crate::{decode_failed, TesError};

/// The full set of objects in the game world
///
/// The World type manages the current load order of plugins and allows looking up records from
/// the appropriate plugin based on load order.
#[derive(Debug)]
pub struct World {
    plugins: Vec<Plugin>,
}

impl World {
    /// Loads the world from the Morrowind game directory
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading Morrowind.ini or a plugin file,
    /// if Morrowind.ini contains invalid data, or if a plugin file contains invalid data.
    pub fn load_world<P: AsRef<Path>>(game_dir: P) -> Result<World, TesError> {
        let path = game_dir.as_ref();
        let ini_path = path.join("Morrowind.ini");
        let ini = Ini::load_from_file(ini_path)?;
        let game_files = ini
            .section(Some("Game Files"))
            .ok_or_else(|| decode_failed("No Game Files section in Morrowind.ini"))?;

        let mut files = Vec::with_capacity(game_files.len());
        for (_, filename) in game_files.iter() {
            let plugin_path = path.join("Data Files").join(filename);
            let meta = fs::metadata(&plugin_path)?;
            files.push((plugin_path, meta.modified()?));
        }
        // we sort in reverse order here because we want to search plugins latest in the load order
        // first
        files.sort_by(|a, b| b.1.cmp(&a.1));

        let mut plugins = Vec::with_capacity(files.len());
        for (filename, _) in &files {
            // I would do a map but ? in the closure wouldn't have the correct effect
            plugins.push(Plugin::load_file(filename)?);
        }

        Ok(World { plugins })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_GAME_DIR: &str = "src/tes3/plugin/test";

    #[test]
    fn test_load() {
        // it's important to use this environment variable instead of a relative path because, at
        // least in CLion, the working directory is not the same when running and debugging the
        // configuration
        let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let game_dir = base_dir.join(TEST_GAME_DIR);
        let world = World::load_world(&game_dir).unwrap();
        assert_eq!(world.plugins.len(), 2);
    }
}
