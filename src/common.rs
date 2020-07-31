#![macro_use]

use std::ffi::CStr;
use std::io;
use std::io::{Error, ErrorKind, Read};
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

pub fn io_error(msg: &str) -> Error {
    Error::new(ErrorKind::InvalidData, msg)
}