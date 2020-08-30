//! Types for manipulating plugin files
//!
//! This module contains common types for reading and writing plugin files (.esm, .esp, and .ess) in
//! different games. [`FieldInterface`] contains common operations for reading and writing fields in
//! plugin files.
//!
//! [`FieldInterface`]: trait.FieldInterface.html

mod field;
pub use field::*;