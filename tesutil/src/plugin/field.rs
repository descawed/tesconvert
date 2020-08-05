use std::ffi::{CStr, CString};
use std::io;
use std::io::{Read, Write};
use std::mem::size_of;
use std::str;

use super::{PluginError, decode_failed};

// unfortunately, to_le_bytes and from_le_bytes are not trait methods, but instead are implemented
// directly on the integer types, which means we can't use generics to write a single method for
// converting field data to and from integers. instead, we'll use this macro. I would have the
// macro generate the function names as well, but it looks like I would have to take the type as
// an identifier instead of a type, and even then, pasting identifiers requires third-party crates
// or nightly Rust.
macro_rules! to_num {
    ($type:ty, $name:ident) => (
        fn $name(&self) -> Result<$type, PluginError> {
            let data = self.get();
            if data.len() != size_of::<$type>() {
                return Err(PluginError::DecodeFailed {
                    description: format!("expected {} bytes for {}, found {}", size_of::<$type>(), stringify!($type), data.len()),
                    cause: None,
                });
            }
            let mut buf = [0u8; size_of::<$type>()];
            buf.copy_from_slice(&data[..]);
            Ok(<$type>::from_le_bytes(buf))
        }
    )
}

macro_rules! from_num {
    ($type:ty, $name:ident, $new_name:ident) => (
        fn $name(&mut self, v: $type) {
            self.set(v.to_le_bytes().to_vec());
        }

        fn $new_name(name: &[u8; 4], data: $type) -> Self {
            Self::new(name, data.to_le_bytes().to_vec()).unwrap()
        }
    )
}

/// An attribute of a record
///
/// This trait is a general interface to the field types of different games.
pub trait FieldInterface: Sized {
    fn new(name: &[u8; 4], data: Vec<u8>) -> Result<Self, PluginError>;

    /// Creates a new field with the specified string data
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if `data` is larger than [`MAX_DATA`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = FieldInterface::new_string(b"NAME", String::from("Flora_kelp_01"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    fn new_string(name: &[u8; 4], data: String) -> Result<Self, PluginError> {
        Self::new(name, data.into_bytes())
    }

    /// Creates a new field with the specified string data
    ///
    /// The difference between this and [`new_string`] is that `new_zstring` will store the string
    /// with a terminating null byte.
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if `data` plus the terminating null byte is larger
    /// than [`MAX_DATA`]. Returns a [`std::ffi::NulError`] if `data` contains internal nulls.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let field = FieldInterface::new_zstring(b"NAME", String::from("Flora_kelp_01"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`new_string`]: #method.new_string
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    /// [`std::ffi::NulError`]: https://doc.rust-lang.org/std/ffi/struct.NulError.html
    fn new_zstring(name: &[u8; 4], data: String) -> Result<Self, PluginError> {
        let zstr = CString::new(data).map_err(|e| decode_failed("Failed to decode as zstring", e))?;
        Self::new(name, zstr.into_bytes_with_nul())
    }

    fn read(f: &mut dyn Read) -> io::Result<Self>;

    fn name(&self) -> &[u8];

    /// Returns the field name as a string
    ///
    /// If the field name cannot be decoded as UTF-8 (which will never happen in a valid plugin
    /// file), the string `"<invalid>"` will be returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = FieldInterface::new(b"NAME", vec![])?;
    /// assert_eq!(field.display_name(), "NAME");
    /// # Ok(())
    /// # }
    /// ```
    fn display_name(&self) -> &str {
        str::from_utf8(self.name()).unwrap_or("<invalid>")
    }

    fn get(&self) -> &[u8];
    
    fn consume(self) -> Vec<u8>;
    
    fn set(&mut self, data: Vec<u8>) -> Result<(), PluginError>;
    
    fn size(&self) -> usize;

    fn write(&self, f: &mut dyn Write) -> io::Result<()>;

    /// Gets a reference to the field's data as a string
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::DecodeFailed`] if the data is not valid UTF-8. This means that, currently,
    /// this function only works correctly with English versions of the game. This will be updated
    /// in the future.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let data = b"NAME\x10\0\0\0sSkillClassMajor";
    /// let field = FieldInterface::read(&mut data.as_ref())?;
    /// let name = field.get_string()?;
    /// assert_eq!(name, "sSkillClassMajor");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::DecodeFailed`]: enum.PluginError.html#variant.DecodeFailed
    // FIXME: the below string functions will fail on non-English versions of the game
    fn get_string(&self) -> Result<&str, PluginError> {
        str::from_utf8(&self.get()[..]).map_err(|e| decode_failed("failed to decode string", e))
    }

    /// Sets the field's data from a string
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if the size of `data` exceeds [`MAX_DATA`].
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    fn set_string(&mut self, data: String) -> Result<(), PluginError> {
        self.set(data.into_bytes())
    }

    /// Gets a reference to the fields data as a null-terminated string
    ///
    /// The data must include a terminating null byte, and the null will not be included in the
    /// result.
    ///
    /// # Errors
    ///
    /// Returns an error if the data includes internal null bytes or if the data is not valid UTF-8.
    fn get_zstring(&self) -> Result<&str, PluginError> {
        let zstr = CStr::from_bytes_with_nul(&self.get()[..]).map_err(|e| decode_failed("string contained internal nulls", e))?;
        zstr.to_str().map_err(|e| decode_failed("failed to decode string", e))
    }

    /// Sets the field's data as a string
    ///
    /// The difference between this and [`set_string`] is that `set_zstring` will store the string
    /// with a terminating null byte.
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if `data` plus the terminating null byte is larger
    /// than [`MAX_DATA`]. Returns a [`PluginError::DecodeFailed`] if `data` contains internal nulls.
    ///
    /// [`set_string`]: #method.set_string
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    /// [`PluginError::DecodeFailed`]: enum.PluginError.html#variant.DecodeFailed
    fn set_zstring(&mut self, data: String) -> Result<(), PluginError> {
        let zstr = CString::new(data).map_err(|e| decode_failed("string contained internal nulls", e))?;
        self.set(zstr.into_bytes_with_nul())
    }

    to_num!(i8, get_i8);
    to_num!(u8, get_u8);

    to_num!(i16, get_i16);
    to_num!(u16, get_u16);

    to_num!(i32, get_i32);
    to_num!(u32, get_u32);

    to_num!(i64, get_i64);
    to_num!(u64, get_u64);

    to_num!(f32, get_f32);

    from_num!(i8, set_i8, new_i8);
    from_num!(u8, set_u8, new_u8);

    from_num!(i16, set_i16, new_i16);
    from_num!(u16, set_u16, new_u16);

    from_num!(i32, set_i32, new_i32);
    from_num!(u32, set_u32, new_u32);

    from_num!(i64, set_i64, new_i64);
    from_num!(u64, set_u64, new_u64);

    from_num!(f32, set_f32, new_f32);
}