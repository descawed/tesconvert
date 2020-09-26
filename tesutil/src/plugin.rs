//! Types for manipulating plugin files
//!
//! This module contains common types for reading and writing plugin files (.esm, .esp, and .ess) in
//! different games. [`FieldInterface`] contains common operations for reading and writing fields in
//! plugin files.
//!
//! [`FieldInterface`]: trait.FieldInterface.html

use std::path::Path;

use crate::TesError;

mod field;
pub use field::*;

mod record;
pub use record::*;

/// Common functionality between different games' plugin implementations
pub trait Plugin: Sized + Send + Sync {
    fn load_file<P: AsRef<Path>>(path: P) -> Result<Self, TesError>;

    fn is_master(&self) -> bool;
    fn set_is_master(&mut self, is_master: bool);

    fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<(), TesError>;
}
