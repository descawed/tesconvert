//! Utilities for working with the files of The Elder Scrolls III and IV
//!
//! This crate contains utilities for reading and writing file formats associated with The Elder
//! Scrolls III: Morrowind and The Elder Scrolls IV: Oblivion. Currently, only plugin files (.esm,
//! .esp, .ess) are implemented, but support for archives (.bsa) will be added in the future, and
//! potentially other formats as well.

pub mod plugin;
pub mod tes3;
pub mod tes4;

use std::error;
use std::ffi::CStr;
use std::io;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::iter;
use std::str;

use len_trait::len::Len;
use thiserror::*;

#[macro_use]
extern crate bitflags;

// have to use a macro instead of a generic because from_le_bytes isn't a trait method
#[macro_export]
macro_rules! extract {
    ($f:ident as $t:ty) => {{
        let mut buf = [0u8; std::mem::size_of::<$t>()];
        $f.read_exact(&mut buf)
            .map(move |_| <$t>::from_le_bytes(buf))
    }};
}

#[macro_export]
macro_rules! serialize {
    ($v:expr => $f:ident) => {{
        let value = $v;
        $f.write(&value.to_le_bytes())
    }};
}

// doing only a partial write could result in invalid plugins, so we want to treat this as an error
trait WriteExact {
    fn write_exact(&mut self, buf: &[u8]) -> io::Result<()>;
}

impl<T: Write> WriteExact for T {
    fn write_exact(&mut self, buf: &[u8]) -> io::Result<()> {
        match self.write(buf) {
            Ok(num_bytes) => {
                if num_bytes == buf.len() {
                    Ok(())
                } else {
                    Err(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        format!(
                            "Attempted to write {} bytes but could only write {}",
                            buf.len(),
                            num_bytes
                        ),
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }
}

fn extract_string<T: Read>(size: usize, mut f: T) -> io::Result<String> {
    let mut buf = vec![0u8; size];
    f.read_exact(&mut buf)?;
    // ensure there is exactly one null byte at the end of the string
    let chars: Vec<u8> = buf
        .into_iter()
        .take_while(|b| *b != 0)
        .chain(iter::once(0))
        .collect();
    let cs = CStr::from_bytes_with_nul(&chars)
        .map_err(|e| io_error(format!("Invalid null-terminated string: {}", e)))?;
    match cs.to_str() {
        Ok(s) => Ok(String::from(s)),
        Err(e) => Err(io_error(format!("Failed to decode string: {}", e))),
    }
}

fn serialize_str<T: Write>(s: &str, size: usize, mut f: T) -> io::Result<()> {
    let mut buf = vec![0u8; size];
    buf[..s.len()].copy_from_slice(s.as_bytes());
    f.write_exact(&buf)
}

fn extract_bstring_raw<T: Read>(mut f: T) -> io::Result<Vec<u8>> {
    let size = extract!(f as u8)? as usize;
    let mut buf = vec![0u8; size];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

fn extract_bstring<T: Read>(f: T) -> io::Result<String> {
    let buf = extract_bstring_raw(f)?;
    let s = str::from_utf8(&buf).map_err(io_error)?;
    Ok(String::from(s))
}

fn extract_bzstring<T: Read>(f: T) -> io::Result<String> {
    let buf = extract_bstring_raw(f)?;
    let cs = CStr::from_bytes_with_nul(&buf).map_err(io_error)?;
    match cs.to_str() {
        Ok(s) => Ok(String::from(s)),
        Err(e) => Err(io_error(e)),
    }
}

fn serialize_bstring<T: Write>(mut f: T, data: &str) -> io::Result<()> {
    if data.len() > MAX_BSTRING {
        return Err(io_error("bstring too large"));
    }

    serialize!(data.len() as u8 => f)?;
    f.write_exact(data.as_bytes())
}

fn serialize_bzstring<T: Write>(mut f: T, data: &str) -> io::Result<()> {
    let size = data.len() + 1; // +1 for null
    if size > MAX_BSTRING {
        return Err(io_error("bstring too large"));
    }

    serialize!(size as u8 => f)?;
    f.write_exact(data.as_bytes())?;
    serialize!(0u8 => f)?;
    Ok(())
}

fn io_error<E>(e: E) -> Error
where
    E: Into<Box<dyn error::Error + Send + Sync>>,
{
    io::Error::new(ErrorKind::InvalidData, e)
}

/// Maximum size in bytes of a record or field
pub const MAX_DATA: usize = u32::MAX as usize;
/// Maximum size of length-prefixed strings
pub const MAX_BSTRING: usize = u8::MAX as usize;

fn check_size<T: Len + ?Sized>(data: &T, max_size: usize, msg: &str) -> Result<(), TesError> {
    if data.len() > max_size {
        Err(TesError::LimitExceeded {
            description: String::from(msg),
            max_size,
            actual_size: data.len(),
        })
    } else {
        Ok(())
    }
}

fn check_range<T: Into<f64> + PartialOrd>(
    value: T,
    min: T,
    max: T,
    msg: &str,
) -> Result<(), TesError> {
    if value < min || value > max {
        Err(TesError::OutOfRange {
            description: String::from(msg),
            min: min.into(),
            max: max.into(),
            actual: value.into(),
        })
    } else {
        Ok(())
    }
}

// this answer has a good explanation for why the 'static lifetime is required here: https://users.rust-lang.org/t/box-with-a-trait-object-requires-static-lifetime/35261
fn decode_failed_because<T: Into<String>, E: error::Error + Send + Sync + 'static>(
    msg: T,
    e: E,
) -> TesError {
    TesError::DecodeFailed {
        description: msg.into(),
        source: Some(Box::new(e)),
    }
}

fn decode_failed<T: Into<String>>(msg: T) -> TesError {
    TesError::DecodeFailed {
        description: msg.into(),
        source: None,
    }
}

/// Error type for utility errors
///
/// A type for errors that may occur while manipulating game files.
#[derive(Error, Debug)]
pub enum TesError {
    /// Multiple records in the plugin have the same ID string
    #[error("ID {0} already in use")]
    DuplicateId(String),
    /// The same master file is referenced by the plugin multiple times
    #[error("Master {0} already present")]
    DuplicateMaster(String),
    /// A size limit, e.g. on a record or field, has been exceeded
    #[error("Limit exceeded: {description}. Max size {max_size}, actual size {actual_size}")]
    LimitExceeded {
        description: String,
        max_size: usize,
        actual_size: usize,
    },
    /// A value is not in the expected range
    #[error("Out of range: {description}. Min: {min}, max: {max}, actual: {actual}")]
    OutOfRange {
        description: String,
        min: f64,
        max: f64,
        actual: f64,
    },
    /// Failed to decode binary data as the expected type or format
    #[error("Decode failed: {description}")]
    DecodeFailed {
        description: String,
        #[source]
        source: Option<Box<dyn error::Error + Send + Sync>>,
    },
    /// Unexpected I/O error
    #[error(transparent)]
    IoError(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_string() {
        let data = b"abcd\0\0\0\0\0\0";
        let s = extract_string(10, &mut data.as_ref()).unwrap();
        assert_eq!(s, "abcd");
    }

    #[test]
    fn test_serialize_str() {
        let mut buf = [0u8; 10];
        serialize_str("abcd", 10, &mut buf.as_mut()).unwrap();
        assert_eq!(buf, *b"abcd\0\0\0\0\0\0");
    }
}
