//! Utilities for working with the files of The Elder Scrolls III and IV
//!
//! This crate contains utilities for reading and writing file formats associated with The Elder
//! Scrolls III: Morrowind and The Elder Scrolls IV: Oblivion. Currently, only plugin files (.esm,
//! .esp, .ess) are implemented, but support for archives (.bsa) will be added in the future, and
//! potentially other formats as well.

pub mod plugin;
mod save;

use std::error;
use std::ffi::CStr;
use std::fmt;
use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::iter;
use std::str;

use len_trait::len::Len;

#[macro_use]
extern crate bitflags;

// have to use a macro instead of a generic because from_le_bytes isn't a trait method
#[macro_export]
macro_rules! extract {
    ($f:ident as $t:ty) => {
        {
            let mut buf = [0u8; std::mem::size_of::<$t>()];
            $f.read_exact(&mut buf).map(move |_| <$t>::from_le_bytes(buf))
        }
    }
}

#[macro_export]
macro_rules! serialize {
    ($v:expr => $f:ident) => {
        {
            let value = $v;
            $f.write(&value.to_le_bytes())
        }
    }
}

// doing only a partial write could result in invalid plugins, so we want to treat this as an error
trait WriteExact {
    fn write_exact(&mut self, buf: &[u8]) -> io::Result<()>;
}

impl<T: Write> WriteExact for T {
    fn write_exact(&mut self, buf: &[u8]) -> io::Result<()> {
        match self.write(buf) {
            Ok(num_bytes) => if num_bytes == buf.len() {
                Ok(())
            } else {
                Err(Error::new(ErrorKind::UnexpectedEof, format!("Attempted to write {} bytes but could only write {}", buf.len(), num_bytes)))
            },
            Err(e) => Err(e),
        }
    }
}

// used for reading record types
trait ReadAllOrNone {
    fn read_all_or_none(&mut self, buf: &mut [u8]) -> io::Result<bool>;
}

impl<T: Read> ReadAllOrNone for T {
    fn read_all_or_none(&mut self, buf: &mut [u8]) -> io::Result<bool> {
        // there are a few reasons why we have to use this roundabout solution. there is a "number
        // of records" field in the plugin header, but it's not guaranteed to be accurate, so we
        // have to just keep reading records until EOF. unfortunately, the Read trait has no easy
        // way to check for EOF. we could check for EOF more easily if T were Read + Seek, but that
        // would prevent us from reading from byte arrays, which is handy for testing. instead, we
        // have to use read instead of read_exact and check if it returns 0. if it doesn't return 0,
        // we need to check that we got as many bytes as we were expecting. but that's not
        // straightforward either, because it's possible read might read less than 4 bytes even if
        // we're not at EOF (specifically, if we're at the end of BufReader's buffer), so we have to
        // keep reading in a loop until we reach the number of bytes we need or read 0 bytes.
        let mut total_bytes_read = 0;
        while total_bytes_read < buf.len() {
            let bytes_read = self.read(&mut buf[total_bytes_read..])?;
            if bytes_read == 0 {
                if total_bytes_read == 0 {
                    return Ok(false);
                } else if total_bytes_read < buf.len() {
                    return Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
                }
            }

            total_bytes_read += bytes_read;
        }

        Ok(true)
    }
}

fn extract_string<T: Read>(size: usize, mut f: T) -> io::Result<String> {
    let mut buf = vec![0u8; size];
    f.read_exact(&mut buf)?;
    // ensure there is exactly one null byte at the end of the string
    let chars: Vec<u8> = buf.into_iter().take_while(|b| *b != 0).chain(iter::once(0)).collect();
    let cs = CStr::from_bytes_with_nul(&chars).map_err(|e| io_error(format!("Invalid null-terminated string: {}", e)))?;
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

fn extract_bstring<T: Read>(mut f: T) -> io::Result<String> {
    let buf = extract_bstring_raw(f)?;
    let s = str::from_utf8(&buf).map_err(|e| io_error(e))?;
    Ok(String::from(s))
}

fn extract_bzstring<T: Read>(mut f: T) -> io::Result<String> {
    let buf = extract_bstring_raw(f)?;
    let cs = CStr::from_bytes_with_nul(&buf).map_err(|e| io_error(e))?;
    match cs.to_str() {
        Ok(s) => Ok(String::from(s)),
        Err(e) => Err(io_error(e)),
    }
}

fn serialize_bstring<T: Write>(mut f: T, data: &str) -> io::Result<()> {
    if data.len() > u8::MAX as usize {
        return Err(io_error("bstring too large"));
    }

    serialize!(data.len() as u8 => f)?;
    f.write_exact(data.as_bytes())
}

fn serialize_bzstring<T: Write>(mut f: T, data: &str) -> io::Result<()> {
    let size = data.len() + 1; // +1 for null
    if size > u8::MAX as usize {
        return Err(io_error("bstring too large"));
    }

    serialize!(size as u8 => f)?;
    f.write_exact(data.as_bytes())?;
    serialize!(0u8 => f)?;
    Ok(())
}

fn io_error<E>(e: E) -> Error
where E: Into<Box<dyn error::Error + Send + Sync>>
{
    Error::new(ErrorKind::InvalidData, e)
}

/// Maximum size in bytes of a record or field
pub const MAX_DATA: usize = u32::MAX as usize;

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

// this answer has a good explanation for why the 'static lifetime is required here: https://users.rust-lang.org/t/box-with-a-trait-object-requires-static-lifetime/35261
fn decode_failed<T: error::Error + Send + Sync + 'static>(msg: &str, e: T) -> TesError {
    TesError::DecodeFailed {
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
pub enum TesError {
    /// Multiple records in the plugin have the same ID string
    DuplicateId(String),
    /// The same master file is referenced by the plugin multiple times
    DuplicateMaster(String),
    /// A size limit, e.g. on a record or field, has been exceeded
    LimitExceeded { description: String, max_size: usize, actual_size: usize },
    /// Failed to decode binary data as the expected type or format
    DecodeFailed { description: String, cause: Option<Box<dyn error::Error + Send + Sync>> },
}

impl fmt::Display for TesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TesError::DuplicateId(id) => write!(f, "ID {} already in use", id),
            TesError::DuplicateMaster(name) => write!(f, "Master {} already present", name),
            TesError::LimitExceeded {
                description, max_size, actual_size
            } => write!(f, "Limit exceeded: {}. Max size {}, actual size {}", description, max_size, actual_size),
            TesError::DecodeFailed {
                description, cause
            } => match cause {
                Some(cause) => write!(f, "Decode failed: {}. Caused by: {}", description, cause),
                None => write!(f, "Decode failed: {}", description),
            },
        }
    }
}

impl error::Error for TesError {}

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
