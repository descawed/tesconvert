//! Utilities for working with the files of The Elder Scrolls III and IV
//!
//! This crate contains utilities for reading and writing file formats associated with The Elder
//! Scrolls III: Morrowind and The Elder Scrolls IV: Oblivion. Currently, only plugin files (.esm,
//! .esp, .ess) are implemented, but support for archives (.bsa) will be added in the future, and
//! potentially other formats as well.

pub mod plugin;

use std::error;
use std::ffi::CStr;
use std::fmt;
use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::iter;

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

/// Indicates the game whose data is being manipulated
///
/// Used by code that can operate on either game to know in which format to read/write data.
pub enum Game {
    Morrowind,
    Oblivion,
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

fn io_error<E>(e: E) -> Error
where E: Into<Box<dyn error::Error + Send + Sync>>
{
    Error::new(ErrorKind::InvalidData, e)
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