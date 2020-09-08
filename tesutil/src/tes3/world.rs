use std::cell::Ref;
use std::fs;
use std::path::Path;

use ini::Ini;

use super::plugin::*;
use crate::{decode_failed, TesError};

static INI_FILE: &str = "Morrowind.ini";
static PLUGIN_DIR: &str = "Data Files";

/// The full set of objects in the game world
///
/// The World type manages the current load order of plugins and allows looking up records from
/// the appropriate plugin based on load order.
#[derive(Debug)]
pub struct World {
    plugins: Vec<Plugin>,
}

impl World {
    /// Loads the world from a provided list of plugins
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading a plugin file or if a plugin file
    /// contains invalid data.
    pub fn load_plugins<'a, P, T>(game_dir: P, plugin_names: T) -> Result<World, TesError>
    where
        P: AsRef<Path>,
        T: Iterator<Item = &'a str>,
    {
        let plugin_dir = game_dir.as_ref().join(PLUGIN_DIR);
        let mut files = vec![];
        for filename in plugin_names {
            let plugin_path = plugin_dir.join(filename);
            let meta = fs::metadata(&plugin_path)?;
            files.push((plugin_path, meta.modified()?));
        }

        // we sort in reverse order here because we want to search plugins latest in the load order
        // first
        files.sort_by(|a, b| b.1.cmp(&a.1));

        let mut plugins = Vec::with_capacity(files.len());
        for (filename, _) in &files {
            plugins.push(Plugin::load_file(filename)?);
        }

        Ok(World { plugins })
    }

    /// Loads the world from the Morrowind game directory
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading Morrowind.ini or a plugin file,
    /// if Morrowind.ini contains invalid data, or if a plugin file contains invalid data.
    pub fn load_world<P: AsRef<Path>>(game_dir: P) -> Result<World, TesError> {
        let path = game_dir.as_ref();
        let ini_path = path.join(INI_FILE);
        let ini = Ini::load_from_file(ini_path)?;
        let game_files = ini
            .section(Some("Game Files"))
            .ok_or_else(|| decode_failed(format!("No Game Files section in {}", INI_FILE)))?;
        World::load_plugins(game_dir, game_files.iter().map(|(_, v)| v))
    }

    /// Loads the world from a save file
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading a plugin file or if a plugin file
    /// contains invalid data.
    pub fn load_from_save<P: AsRef<Path>>(game_dir: P, save: &Plugin) -> Result<World, TesError> {
        World::load_plugins(game_dir, save.iter_masters())
    }

    /// Gets the active version of a record by ID
    ///
    /// Returns None if there is no record with the given ID.
    ///
    /// # Errors
    ///
    /// Fails if there is more than one record with the given ID.
    pub fn get_record(&self, id: &str) -> Result<Option<Ref<Record>>, TesError> {
        for plugin in &self.plugins {
            if let Some(record) = plugin.get_record(id)? {
                return Ok(Some(record));
            }
        }

        Ok(None)
    }

    /// Gets the active version of a record by ID and record type
    ///
    /// Returns None if there is no record with the given ID and type.
    ///
    /// # Errors
    ///
    /// Fails if there is more than one record with the given ID and type.
    pub fn get_record_with_type(&self, id: &str, name: &[u8; 4]) -> Option<Ref<Record>> {
        self.plugins
            .iter()
            .fold(None, |a, e| a.or_else(|| e.get_record_with_type(id, name)))
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

    #[test]
    fn test_explicit_plugins() {
        let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let game_dir = base_dir.join(TEST_GAME_DIR);
        let plugins = vec!["test1.esp", "test2.esp"];
        let world = World::load_plugins(&game_dir, plugins.into_iter()).unwrap();
        assert_eq!(world.plugins.len(), 2);
    }
}
