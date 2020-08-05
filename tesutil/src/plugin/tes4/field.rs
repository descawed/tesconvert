use std::io;
use std::mem::size_of;

use crate::*;
use crate::plugin::FieldInterface;
use crate::plugin::{PluginError, check_size, MAX_DATA};

/// An attribute of a record
///
/// A record consists of one or more fields which describe the attributes of that record. Each field
/// consists of a 4-byte ASCII identifier, such as `b"NAME"`, and the field data. A field can hold
/// data of any type, including integers, floats, strings, and structs. The type of data a
/// particular field depends on the identifier (referred to here as `name`) and the record it
/// belongs to.
///
/// Note that all of the various `new` functions for `Field` take the name by reference and copy it.
/// This is because new fields are (almost?) always constructed from hard-coded names such as
/// `b"STRV"` or `b"DATA"`, and it would be cumbersome to have to explicitly clone these everywhere.
/// The data, on the other hand, is taken as an owned value, because this is much more likely to be
/// dynamic.
#[derive(Debug)]
pub struct Field{
    name: [u8; 4],
    data: Vec<u8>,
}

impl FieldInterface for Field {
    /// Creates a new field with the specified data
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if `data` is larger than [`MAX_DATA`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes4::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"DATA", vec![/* binary gobbledygook */])?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    fn new(name: &[u8; 4], data: Vec<u8>) -> Result<Field, PluginError> {
        check_size(&data, MAX_DATA, "field data too large")?;
        Ok(Field {
            name: name.clone(),
            data,
        })
    }

    /// Reads a field from a binary stream
    ///
    /// Reads a field from any type that implements [`Read`] or a mutable reference to such a type.
    /// `game` indicates which type of plugin file the field is being read from.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if an I/O error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes4::*;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let data = b"NAME\x09\0\0\0GameHour\0";
    /// let field = Field::read(&mut data.as_ref())?;
    /// assert_eq!(field.name(), b"NAME");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    fn read(f: &mut dyn Read) -> io::Result<Field> {
        let mut name = [0u8; 4];
        f.read_exact(&mut name)?;

        let size = if name == *b"XXXX" {
            extract!(f as u16)?; // always 4 bytes
            let real_size = extract!(f as u32)?;
            // now fetch the actual record
            f.read_exact(&mut name)?;
            extract!(f as u16)?; // regular size is irrelevant here
            real_size as usize
        } else {
            extract!(f as u16)? as usize
        };

        let mut data = vec![0u8; size];

        f.read_exact(&mut data)?;

        Ok(Field { name, data })
    }

    /// Returns the field name
    ///
    /// This is always a 4-byte ASCII identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes4::Field;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"NAME", vec![])?;
    /// assert_eq!(field.name(), b"NAME");
    /// # Ok(())
    /// # }
    /// ```
    fn name(&self) -> &[u8] {
        &self.name
    }

    /// Returns a reference to the field's data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes4::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"DATA", vec![1, 2, 3])?;
    /// assert_eq!(*field.get(), [1, 2, 3]);
    /// # Ok(())
    /// # }
    /// ```
    fn get(&self) -> &[u8] {
        &self.data[..]
    }

    /// Consumes the field and takes ownership of its data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes4::Field;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"DATA", vec![1, 2, 3])?;
    /// let data = field.consume();
    /// assert_eq!(data[..], [1, 2, 3]);
    /// # Ok(())
    /// # }
    /// ```
    fn consume(self) -> Vec<u8> {
        self.data
    }

    /// Sets the field's data
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if the size of `data` exceeds [`MAX_DATA`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes4::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let mut field = Field::new(b"DATA", vec![])?;
    /// field.set(b"new data to use".to_vec())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    fn set(&mut self, data: Vec<u8>) -> Result<(), PluginError> {
        check_size(&data, MAX_DATA, "field data too large")?;
        self.data = data;
        Ok(())
    }

    /// Calculates the size in bytes of this field
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes4::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"NAME", vec![1, 2, 3])?;
    /// assert_eq!(field.size(), 11); // 4 bytes for the name + 4 bytes for the length + 3 bytes of data
    /// # Ok(())
    /// # }
    /// ```
    fn size(&self) -> usize {
        self.name.len() + size_of::<u16>() + self.data.len()
            // 10 = 4 byte XXXX + 2 byte length + 4 byte data
            + if self.data.len() > u16::MAX as usize { 10 } else { 0 }
    }

    /// Writes the field to the provided writer
    ///
    /// Writes a field to any type that implements [`Write`] or a mutable reference to such a type.
    /// `game` indicates which game the field is being written for.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if an I/O error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    /// use tesutil::plugin::tes4::*;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut buf: Vec<u8> = vec![];
    /// let field = Field::new(b"NAME", vec![1, 2, 3])?;
    /// field.write(&mut &mut buf)?;
    /// assert!(buf.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    fn write(&self, f: &mut dyn Write) -> io::Result<()> {
        let mut len = self.data.len();

        if len > u16::MAX as usize {
            f.write_exact(b"XXXX\x04\0")?;
            serialize!(len as u32 => f)?;
            len = 0;
        }

        f.write_exact(&self.name)?;
        f.write_exact(&(len as u16).to_le_bytes())?;
        f.write_exact(&self.data)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_tes4_field() {
        let field = Field::new(b"EDID", b"sNoTalkFleeing\0".to_vec()).unwrap();
        let mut buf = vec![];
        field.write(&mut buf).unwrap();
        assert_eq!(buf, *b"EDID\x0f\0sNoTalkFleeing\0");
    }

    #[test]
    fn read_tes4_field() {
        let data = b"EDID\x13\0fDialogSpeachDelay\0";
        let field = Field::read(&mut data.as_ref()).unwrap();
        let s = field.get_zstring().unwrap();
        assert_eq!(s, "fDialogSpeachDelay");
    }

    #[test]
    fn read_long_tes4_field() {
        let data = b"XXXX\x04\0\x51\0\0\0DATA\0\0Choose your 7 major skills. You will start at 25 (Apprentice Level) in each one.\0";
        let field = Field::read(&mut data.as_ref()).unwrap();
        let s = field.get_zstring().unwrap();
        assert_eq!(field.name, *b"DATA");
        assert_eq!(s, "Choose your 7 major skills. You will start at 25 (Apprentice Level) in each one.");
    }
}