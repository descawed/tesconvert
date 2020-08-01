#![macro_use]

use std::ffi::CStr;
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
    };
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
pub trait WriteExact {
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

pub fn extract_string<T: Read>(size: usize, f: &mut T) -> io::Result<String> {
    let mut buf = vec![0u8; size];
    f.read_exact(&mut buf)?;
    // ensure there is exactly one null byte at the end of the string
    let chars: Vec<u8> = buf.into_iter().take_while(|b| *b != 0).chain(iter::once(0)).collect();
    let cs = CStr::from_bytes_with_nul(&chars).map_err(|e| io_error(&format!("Invalid null-terminated string: {}", e)))?;
    match cs.to_str() {
        Ok(s) => Ok(String::from(s)),
        Err(e) => Err(io_error(&format!("Failed to decode string: {}", e))),
    }
}

pub fn serialize_str<T: Write>(s: &str, size: usize, f: &mut T) -> io::Result<()> {
    let mut buf = vec![0u8; size];
    buf[..s.len()].copy_from_slice(s.as_bytes());
    f.write_exact(&buf)
}

pub fn io_error(msg: &str) -> Error {
    Error::new(ErrorKind::InvalidData, msg)
}