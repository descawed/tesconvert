use std::cell::Ref;
use std::fs;
use std::path::Path;

use super::plugin::*;
use super::save::*;
use crate::{Form, TesError, World};

static BASE_GAME: &str = "oblivion.esm";
static PLUGIN_DIR: &str = "Data";

/// The full set of objects in the game world
///
/// The World type manages the current load order of plugins and allows looking up records from
/// the appropriate plugin based on load order.
#[derive(Debug)]
pub struct Tes4World {
    plugins: Vec<(String, Tes4Plugin)>,
    save: Option<Save>,
}

impl Tes4World {
    /// Loads the world from the Oblivion game directory and Plugins.txt
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading Plugins.txt or a plugin file,
    /// or if a plugin file contains invalid data.
    pub fn load_world<P: AsRef<Path>>(game_dir: P, plugins_path: P) -> Result<Tes4World, TesError> {
        let mut plugin_names: Vec<String> = fs::read_to_string(plugins_path)?
            .lines()
            .map(|s| s.to_lowercase())
            .collect();
        if plugin_names.iter().position(|s| s == BASE_GAME).is_none() {
            // Oblivion.esm always gets loaded even if it's not in plugins.txt, so insert it if we didn't find it
            plugin_names.push(String::from(BASE_GAME));
        }

        let plugin_dir = game_dir.as_ref().join(PLUGIN_DIR);
        let plugins = Tes4World::load_plugins(plugin_dir, plugin_names.into_iter())?;

        Ok(Tes4World {
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
    pub fn load_save<P: AsRef<Path>>(game_dir: P, save: Save) -> Result<Tes4World, TesError> {
        let plugin_dir = game_dir.as_ref().join(PLUGIN_DIR);
        let plugins = Tes4World::load_plugins(plugin_dir, save.iter_plugins())?;

        Ok(Tes4World {
            plugins,
            save: Some(save),
        })
    }

    /// Gets a record by form ID
    pub fn get_record(&self, form_id: FormId) -> Option<Ref<Tes4Record>> {
        let index = form_id.index() as usize;
        if index >= self.plugins.len() {
            return None;
        }

        // a record in one plugin may have been overridden by a record in another plugin later in the
        // load order which has that plugin as a master. to handle this, we first figure out which plugin
        // the record originates from. then we iterate through every plugin at a position >= that in the
        // load order in reverse order, looking for the latest plugin that contains that record from that
        // master.
        let target_name = self.plugins[index].0.to_lowercase();
        for (name, plugin) in self.plugins.iter().skip(index).rev() {
            let master: Option<&str> = if *name == target_name {
                None
            } else {
                Some(target_name.as_ref())
            };

            if let Some(record) = plugin.get_record_by_master(master, form_id) {
                return Some(record);
            }
        }

        None
    }

    /// Gets a float game setting by name
    pub fn get_float_setting(&self, name: &str, default: f32) -> Result<f32, TesError> {
        for (_, plugin) in self.plugins.iter().rev() {
            if let Some(value) = plugin.get_float_setting(name)? {
                return Ok(value);
            }
        }

        Ok(default)
    }

    /// Gets a form by form ID
    pub fn get<T: Form<Field = Tes4Field, Record = Tes4Record>>(
        &self,
        form_id: FormId,
    ) -> Result<Option<T>, TesError> {
        match self.get_record(form_id) {
            Some(record) => Ok(Some(T::read(&*record)?)),
            None => Ok(None),
        }
    }
}

impl World for Tes4World {
    type Plugin = Tes4Plugin;
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
        let world = Tes4World::load_world(&game_dir, &plugin_path).unwrap();
        assert_eq!(world.plugins.len(), 2);
    }
}
