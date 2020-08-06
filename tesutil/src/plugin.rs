//! Types for manipulating plugin files
//!
//! This module contains types for reading and writing plugin files (.esm, .esp, and .ess).
//! [`Plugin`] represents a plugin file. [`Record`] represents an individual record in a plugin
//! file, and [`Field`] represents a field in a record.
//!
//! [`Plugin`]: struct.Plugin.html
//! [`Record`]: struct.Record.html
//! [`Field`]: struct.Field.html

pub mod tes3;
pub mod tes4;

mod field;
pub use field::*;