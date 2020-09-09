use std::fs;
use std::path::Path;

use super::plugin::*;
use super::save::*;
use crate::{TesError, WorldInterface};

static BASE_GAME: &str = "oblivion.esm";
static PLUGIN_DIR: &str = "Data";

/// The full set of objects in the game world
///
/// The World type manages the current load order of plugins and allows looking up records from
/// the appropriate plugin based on load order.
#[derive(Debug)]
pub struct World {
    plugins: Vec<(String, Plugin)>,
    save: Option<Save>,
}

impl World {
    /// Loads the world from the Oblivion game directory and Plugins.txt
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading Plugins.txt or a plugin file,
    /// or if a plugin file contains invalid data.
    pub fn load_world<P: AsRef<Path>>(game_dir: P, plugins_path: P) -> Result<World, TesError> {
        let mut plugin_names: Vec<String> = fs::read_to_string(plugins_path)?
            .lines()
            .map(|s| s.to_lowercase())
            .collect();
        if plugin_names.iter().position(|s| s == BASE_GAME).is_none() {
            // Oblivion.esm always gets loaded even if it's not in plugins.txt, so insert it if we didn't find it
            plugin_names.push(String::from(BASE_GAME));
        }

        let plugin_dir = game_dir.as_ref().join(PLUGIN_DIR);
        let plugins = World::load_plugins(plugin_dir, plugin_names.into_iter())?;

        Ok(World {
            plugins,
            save: None,
        })
    }

    /// Loads the world from the Oblivion game directory and a save
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading a plugin file or if a plugin file
    /// contains invalid data.
    pub fn load_save<P: AsRef<Path>>(game_dir: P, save: Save) -> Result<World, TesError> {
        let plugin_dir = game_dir.as_ref().join(PLUGIN_DIR);
        let plugins = World::load_plugins(plugin_dir, save.iter_plugins())?;

        Ok(World {
            plugins,
            save: Some(save),
        })
    }
}

impl WorldInterface for World {
    type Plugin = Plugin;
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_GAME_DIR: &str = "src/tes4/plugin/test";

    #[test]
    fn test_load() {
        // it's important to use this environment variable instead of a relative path because, at
        // least in CLion, the working directory is not the same when running and debugging the
        // configuration
        let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let game_dir = base_dir.join(TEST_GAME_DIR);
        let plugin_path = game_dir.join("Plugins.txt");
        let world = World::load_world(&game_dir, &plugin_path).unwrap();
        assert_eq!(world.plugins.len(), 2);
    }
}
