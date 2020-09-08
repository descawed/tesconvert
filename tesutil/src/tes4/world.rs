use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Weak;

use super::plugin::*;
use super::save::*;
use crate::{decode_failed, TesError};

/// The full set of objects in the game world
///
/// The World type manages the current load order of plugins and allows looking up records from
/// the appropriate plugin based on load order.
#[derive(Debug)]
pub struct World {
    plugins: Vec<Plugin>,
    id_map: HashMap<FormId, Weak<RefCell<Record>>>,
    save: Option<Save>,
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
        // TODO: make Plugin and World traits and use those to generically implement this method
        //  across TES3 and TES4
        unimplemented!()
    }
}
