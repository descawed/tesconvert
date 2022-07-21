use std::ops::Deref;
use std::path::Path;

use ini::Ini;

use super::plugin::*;
use crate::{decode_failed, Form, Plugin, Record, TesError, World};

const INI_FILE: &str = "Morrowind.ini";
const PLUGIN_DIR: &str = "Data Files";

/// The full set of objects in the game world
///
/// The World type manages the current load order of plugins and allows looking up records from
/// the appropriate plugin based on load order.
#[derive(Debug)]
pub struct Tes3World {
    plugins: Vec<Tes3Plugin>,
    has_save: bool, // if we have one, it's always the last plugin
}

impl Tes3World {
    fn load_from_plugins<'a, P, T>(game_dir: P, plugin_names: T) -> Result<Tes3World, TesError>
    where
        P: AsRef<Path>,
        T: Iterator<Item = &'a str>,
    {
        let plugin_dir = game_dir.as_ref().join(PLUGIN_DIR);
        let plugins = Tes3World::load_plugins(plugin_dir, plugin_names)?;
        let plugins = plugins.into_iter().map(|(_, p)| p).collect();
        Ok(Tes3World {
            plugins,
            has_save: false,
        })
    }

    /// Loads the world from the Morrowind game directory
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading Morrowind.ini or a plugin file,
    /// if Morrowind.ini contains invalid data, or if a plugin file contains invalid data.
    pub fn load_world<P: AsRef<Path>>(game_dir: P) -> Result<Tes3World, TesError> {
        let path = game_dir.as_ref();
        let ini_path = path.join(INI_FILE);
        let ini = Ini::load_from_file(ini_path)?;
        let game_files = ini
            .section(Some("Game Files"))
            .ok_or_else(|| decode_failed(format!("No Game Files section in {}", INI_FILE)))?;
        Tes3World::load_from_plugins(game_dir, game_files.iter().map(|(_, v)| v))
    }

    /// Loads the world from a save file
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading a plugin file or if a plugin file
    /// contains invalid data.
    pub fn load_from_save<P, Q>(game_dir: P, save_path: Q) -> Result<Tes3World, TesError>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let save = Tes3Plugin::load_file(save_path)?;
        let mut world = Tes3World::load_from_plugins(game_dir, save.iter_masters())?;
        world.plugins.push(save);
        world.has_save = true;
        Ok(world)
    }

    /// Gets the currently loaded save, if there is one
    pub fn get_save(&self) -> Option<&Tes3Plugin> {
        if self.has_save {
            self.plugins.iter().last()
        } else {
            None
        }
    }

    /// Gets the currently load save mutably, if there is one
    pub fn get_save_mut(&mut self) -> Option<&mut Tes3Plugin> {
        if self.has_save {
            self.plugins.iter_mut().last()
        } else {
            None
        }
    }

    /// Gets the active version of a record by ID
    ///
    /// Returns None if there is no record with the given ID.
    ///
    /// # Errors
    ///
    /// Fails if there is more than one record with the given ID.
    pub fn get_record(
        &self,
        id: &str,
    ) -> Result<Option<impl Deref<Target = Tes3Record> + '_>, TesError> {
        for plugin in self.plugins.iter().rev() {
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
    pub fn get_record_with_type(
        &self,
        id: &str,
        name: &[u8; 4],
    ) -> Option<impl Deref<Target = Tes3Record> + '_> {
        self.plugins
            .iter()
            .rev()
            .fold(None, |a, p| a.or_else(|| p.get_record_with_type(id, name)))
    }

    /// Loads a form by ID and type
    ///
    /// # Errors
    ///
    /// Fails if the matching record contains invalid data.
    pub fn get<T: Form<Field = Tes3Field, Record = Tes3Record>>(
        &self,
        id: &str,
    ) -> Result<Option<T>, TesError> {
        self.plugins.iter().rev().fold(Ok(None), |a, p| {
            if a.is_ok() && a.as_ref().unwrap().is_none() {
                Ok(p.get(id)?)
            } else {
                a
            }
        })
    }

    /// Loads a form by ID and type
    ///
    /// # Errors
    ///
    /// Fails if the matching record contains invalid data or no matching record is found.
    pub fn require<T: Form<Field = Tes3Field, Record = Tes3Record>>(
        &self,
        id: &str,
    ) -> Result<T, TesError> {
        self.get(id)?
            .ok_or_else(|| TesError::InvalidId(String::from(id)))
    }

    /// Gets an item from the given record
    ///
    /// # Errors
    ///
    /// Fails if the matching record contains invalid data
    pub fn get_item_from_record(&self, record: &Tes3Record) -> Result<Box<dyn Item>, TesError> {
        match record.name() {
            MiscItem::RECORD_TYPE => Ok(Box::new(MiscItem::read(record)?)),
            Potion::RECORD_TYPE => Ok(Box::new(Potion::read(record)?)),
            Weapon::RECORD_TYPE => Ok(Box::new(Weapon::read(record)?)),
            _ => Err(TesError::RequirementFailed(String::from(
                "The given record is not an item or its Form type is not implemented",
            ))),
        }
    }

    /// Loads an item by ID if such an item exists
    ///
    /// # Errors
    ///
    /// Fails if the matching record contains invalid data
    pub fn get_item(&self, id: &str) -> Result<Option<Box<dyn Item>>, TesError> {
        Ok(match self.get_record(id)? {
            Some(record) => Some(self.get_item_from_record(&record)?),
            None => None,
        })
    }
}

impl World for Tes3World {
    type Plugin = Tes3Plugin;
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
        let world = Tes3World::load_world(&game_dir).unwrap();
        assert_eq!(world.plugins.len(), 2);
    }

    #[test]
    fn test_explicit_plugins() {
        let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let game_dir = base_dir.join(TEST_GAME_DIR);
        let plugins = vec!["test1.esp", "test2.esp"];
        let world = Tes3World::load_from_plugins(&game_dir, plugins.into_iter()).unwrap();
        assert_eq!(world.plugins.len(), 2);
    }
}
