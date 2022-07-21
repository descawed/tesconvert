//! Types for manipulating plugin files
//!
//! This module contains common types for reading and writing plugin files (.esm, .esp, and .ess) in
//! different games. [`FieldInterface`] contains common operations for reading and writing fields in
//! plugin files.
//!
//! [`FieldInterface`]: trait.FieldInterface.html

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;

use crate::TesError;

mod field;
pub use field::*;

mod record;
pub use record::*;

/// Common functionality between different games' plugin implementations
pub trait Plugin: Sized + Send + Sync {
    fn read<T: Read + Seek>(f: T) -> Result<Self, TesError>;

    /// Loads a plugin from a file
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs or if the plugin file is invalid.
    fn load_file<P: AsRef<Path>>(path: P) -> Result<Self, TesError> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        Self::read(reader)
    }

    fn is_master(&self) -> bool;
    fn set_is_master(&mut self, is_master: bool);

    fn iter_masters(&self) -> Box<dyn Iterator<Item = &str> + '_>;

    fn write<T: Write + Seek>(&self, f: T) -> Result<(), TesError>;

    /// Saves a plugin to a file
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs
    fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<(), TesError> {
        let f = File::create(path)?;
        let writer = BufWriter::new(f);
        self.write(writer)
    }
}
