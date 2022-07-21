use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use super::cosave::*;
use super::plugin::*;
use super::save::*;
use super::{FindForm, FormId, MagicEffectType, SpellEffect, MAGIC_EFFECTS};
use crate::{Form, OwnedOrRef, Record, TesError, World};

static BASE_GAME: &str = "Oblivion.esm";
static PLUGIN_DIR: &str = "Data";

/// The full set of objects in the game world
///
/// The World type manages the current load order of plugins and allows looking up records from
/// the appropriate plugin based on load order.
#[derive(Debug)]
pub struct Tes4World {
    plugins: Vec<(String, Tes4Plugin)>,
    save: Option<(Save, CoSave)>,
}

impl Tes4World {
    /// Loads the world from the Oblivion game directory and Plugins.txt
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading Plugins.txt or a plugin file,
    /// or if a plugin file contains invalid data.
    pub fn load_world<P, Q>(game_dir: P, plugins_path: Q) -> Result<Tes4World, TesError>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut plugin_names: Vec<String> = fs::read_to_string(plugins_path)?
            .lines()
            .map(String::from)
            .collect();
        let ob_lowercase = BASE_GAME.to_lowercase();
        if !plugin_names
            .iter()
            .any(|s| s.to_lowercase() == ob_lowercase)
        {
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
    pub fn load_from_save<P, Q>(game_dir: P, save_path: Q) -> Result<Tes4World, TesError>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let cosave_path = save_path.as_ref().with_extension("obse");
        let save = Save::load_file(save_path)?;
        let cosave = CoSave::load_file(cosave_path)?;
        let plugin_dir = game_dir.as_ref().join(PLUGIN_DIR);
        let plugins = Tes4World::load_plugins(plugin_dir, save.iter_plugins())?;

        Ok(Tes4World {
            plugins,
            save: Some((save, cosave)),
        })
    }

    /// Gets the currently loaded save, if there is one
    pub fn get_save(&self) -> Option<&Save> {
        self.save.as_ref().map(|(s, _)| s)
    }

    /// Gets the currently load save mutably, if there is one
    pub fn get_save_mut(&mut self) -> Option<&mut Save> {
        self.save.as_mut().map(|(s, _)| s)
    }

    /// Gets the currently loaded co-save, if there is one
    pub fn get_cosave(&self) -> Option<&CoSave> {
        self.save.as_ref().map(|(_, c)| c)
    }

    /// Gets the currently load co-save mutably, if there is one
    pub fn get_cosave_mut(&mut self) -> Option<&mut CoSave> {
        self.save.as_mut().map(|(_, c)| c)
    }

    /// Gets the form ID matching a search
    pub fn get_form_id(&self, search: &FindForm) -> Option<FormId> {
        search.form_id(self.plugins.iter().map(|(s, _)| s.as_str()))
    }

    /// Gets a record by form ID
    pub fn get_record(&self, search: &FindForm) -> Option<impl Deref<Target = Tes4Record> + '_> {
        let form_id = self.get_form_id(search)?;
        let index = form_id.index() as usize;
        if index == 0xff {
            if let Some((ref save, _)) = self.save {
                return save.get_record(form_id);
            }
        }

        if index >= self.plugins.len() {
            return None;
        }

        // a record in one plugin may have been overridden by a record in another plugin later in the
        // load order which has that plugin as a master. to handle this, we first figure out which plugin
        // the record originates from. then we iterate through every plugin at a position >= that in the
        // load order in reverse order, looking for the latest plugin that contains that record from that
        // master.
        let target_name = &self.plugins[index].0;
        let self_search = FindForm::ByMaster(None, form_id.0);
        for (name, plugin) in self.plugins.iter().skip(index).rev() {
            if let Some(record) = plugin.get_record(if name == target_name {
                &self_search
            } else {
                search
            }) {
                return Some(record);
            }
        }

        None
    }

    /// Gets a record by form ID
    pub fn get_record_mut(
        &self,
        search: &FindForm,
    ) -> Option<impl Deref<Target = Tes4Record> + DerefMut<Target = Tes4Record> + '_> {
        let form_id = self.get_form_id(search)?;
        let index = form_id.index() as usize;
        if index == 0xff {
            if let Some((ref save, _)) = self.save {
                return save.get_record_mut(form_id);
            }
        }

        if index >= self.plugins.len() {
            return None;
        }

        // a record in one plugin may have been overridden by a record in another plugin later in the
        // load order which has that plugin as a master. to handle this, we first figure out which plugin
        // the record originates from. then we iterate through every plugin at a position >= that in the
        // load order in reverse order, looking for the latest plugin that contains that record from that
        // master.
        let target_name = &self.plugins[index].0;
        let self_search = FindForm::ByMaster(None, form_id.0);
        for (name, plugin) in self.plugins.iter().skip(index).rev() {
            if let Some(record) = plugin.get_record_mut(if name == target_name {
                &self_search
            } else {
                search
            }) {
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

    /// Gets a magic effect by effect type
    pub fn get_magic_effect(
        &self,
        effect_type: MagicEffectType,
    ) -> Result<impl Deref<Target = MagicEffect>, TesError> {
        for (_, plugin) in self.plugins.iter().rev() {
            if let Some(value) = plugin.get_magic_effect(effect_type)? {
                return Ok(OwnedOrRef::Owned(value));
            }
        }

        Ok(OwnedOrRef::Ref(&MAGIC_EFFECTS[effect_type]))
    }

    /// Gets a form by form ID
    pub fn get<T: Form<Field = Tes4Field, Record = Tes4Record>>(
        &self,
        search: &FindForm,
    ) -> Result<Option<T>, TesError> {
        match self.get_record(search) {
            Some(record) => Ok(Some(T::read(&*record)?)),
            None => Ok(None),
        }
    }

    /// Loads a form by ID and type
    ///
    /// # Errors
    ///
    /// Fails if the matching record contains invalid data or no matching record is found.
    pub fn require<T: Form<Field = Tes4Field, Record = Tes4Record>>(
        &self,
        search: &FindForm,
    ) -> Result<T, TesError> {
        self.get(search)?.ok_or_else(|| search.err())
    }

    /// Updates a form by form ID
    pub fn update<T>(&self, form: &T, search: &FindForm) -> Result<(), TesError>
    where
        T: Form<Field = Tes4Field, Record = Tes4Record>,
    {
        match self.get_record_mut(search) {
            Some(mut record) => form.write(&mut record),
            None => Err(search.err()),
        }
    }

    /// Gets an item from the given record
    ///
    /// # Errors
    ///
    /// Fails if the matching record contains invalid data
    pub fn get_item_from_record(&self, record: &Tes4Record) -> Result<Box<dyn Item>, TesError> {
        match record.name() {
            Ammo::RECORD_TYPE => Ok(Box::new(Ammo::read(record)?)),
            Book::RECORD_TYPE => Ok(Box::new(Book::read(record)?)),
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
    pub fn get_item(&self, search: &FindForm) -> Result<Option<Box<dyn Item>>, TesError> {
        Ok(match self.get_record(search) {
            Some(record) => Some(self.get_item_from_record(&record)?),
            None => None,
        })
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
