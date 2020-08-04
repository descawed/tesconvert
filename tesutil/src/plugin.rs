//! Types for manipulating plugin files
//!
//! This module contains types for reading and writing plugin files (.esm, .esp, and .ess).
//! [`Plugin`] represents a plugin file. [`Record`] represents an individual record in a plugin
//! file, and [`Field`] represents a field in a record.
//!
//! [`Plugin`]: struct.Plugin.html
//! [`Record`]: struct.Record.html
//! [`Field`]: struct.Field.html

use std::str;

use len_trait::len::Len;

pub mod tes3;
pub mod tes4;

mod field;
pub use field::*;

use crate::*;

/// Maximum size in bytes of a record or field
pub const MAX_DATA: usize = u32::MAX as usize;

fn check_size<T: Len + ?Sized>(data: &T, max_size: usize, msg: &str) -> Result<(), PluginError> {
    if data.len() > max_size {
        Err(PluginError::LimitExceeded {
            description: String::from(msg),
            max_size,
            actual_size: data.len(),
        })
    } else {
        Ok(())
    }
}

// this answer has a good explanation for why the 'static lifetime is required here: https://users.rust-lang.org/t/box-with-a-trait-object-requires-static-lifetime/35261
fn decode_failed<T: error::Error + Send + Sync + 'static>(msg: &str, e: T) -> PluginError {
    PluginError::DecodeFailed {
        description: String::from(msg),
        cause: Some(Box::new(e)),
    }
}

/// Error type for plugin errors
///
/// A type for errors that may occur while reading or writing plugin files. Methods that return a
/// [`std::io::Error`] will sometimes wrap a `PluginError` in that error.
///
/// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
#[derive(Debug)]
pub enum PluginError {
    /// Multiple records in the plugin have the same ID string
    DuplicateId(String),
    /// The same master file is referenced by the plugin multiple times
    DuplicateMaster(String),
    /// A size limit, e.g. on a record or field, has been exceeded
    LimitExceeded { description: String, max_size: usize, actual_size: usize },
    /// Failed to decode binary data as the expected type or format
    DecodeFailed { description: String, cause: Option<Box<dyn error::Error + Send + Sync>> },
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PluginError::DuplicateId(id) => write!(f, "ID {} already in use", id),
            PluginError::DuplicateMaster(name) => write!(f, "Master {} already present", name),
            PluginError::LimitExceeded {
                description, max_size, actual_size
            } => write!(f, "Limit exceeded: {}. Max size {}, actual size {}", description, max_size, actual_size),
            PluginError::DecodeFailed {
                description, cause
            } => match cause {
                Some(cause) => write!(f, "Decode failed: {}. Caused by: {}", description, cause),
                None => write!(f, "Decode failed: {}", description),
            },
        }
    }
}

impl error::Error for PluginError {}