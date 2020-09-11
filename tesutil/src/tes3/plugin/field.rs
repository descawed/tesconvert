use std::io;
use std::io::{Read, Seek, Write};
use std::mem::size_of;

use crate::plugin::Field;
use crate::*;

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
pub struct Tes3Field {
    name: [u8; 4],
    data: Vec<u8>,
}

impl Field for Tes3Field {
    /// Creates a new field with the specified data
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if `data` is larger than [`MAX_DATA`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::*;
    /// use tesutil::tes3::plugin::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let field = Tes3Field::new(b"DATA", vec![/* binary gobbledygook */])?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    fn new(name: &[u8; 4], data: Vec<u8>) -> Result<Tes3Field, TesError> {
        check_size(&data, MAX_DATA, "field data too large")?;
        Ok(Tes3Field { name: *name, data })
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
    /// use tesutil::tes3::plugin::*;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let data = b"NAME\x09\0\0\0GameHour\0";
    /// let field = Tes3Field::read(&mut data.as_ref())?;
    /// assert_eq!(field.name(), b"NAME");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    fn read(f: &mut dyn Read) -> io::Result<Tes3Field> {
        let mut name = [0u8; 4];
        f.read_exact(&mut name)?;

        let size = extract!(f as u32)? as usize;
        let mut data = vec![0u8; size];

        f.read_exact(&mut data)?;

        Ok(Tes3Field { name, data })
    }

    /// Returns the field name
    ///
    /// This is always a 4-byte ASCII identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::*;
    /// use tesutil::tes3::plugin::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let field = Tes3Field::new(b"NAME", vec![])?;
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
    /// use tesutil::*;
    /// use tesutil::plugin::*;
    /// use tesutil::tes3::plugin::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let field = Tes3Field::new(b"DATA", vec![1, 2, 3])?;
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
    /// use tesutil::*;
    /// use tesutil::plugin::*;
    /// use tesutil::tes3::plugin::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let field = Tes3Field::new(b"DATA", vec![1, 2, 3])?;
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
    /// use tesutil::*;
    /// use tesutil::plugin::*;
    /// use tesutil::tes3::plugin::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let mut field = Tes3Field::new(b"DATA", vec![])?;
    /// field.set(b"new data to use".to_vec())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    fn set(&mut self, data: Vec<u8>) -> Result<(), TesError> {
        check_size(&data, MAX_DATA, "field data too large")?;
        self.data = data;
        Ok(())
    }

    /// Calculates the size in bytes of this field
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::*;
    /// use tesutil::plugin::*;
    /// use tesutil::tes3::plugin::*;
    ///
    /// # fn main() -> Result<(), TesError> {
    /// let field = Tes3Field::new(b"NAME", vec![1, 2, 3])?;
    /// assert_eq!(field.size(), 11); // 4 bytes for the name + 4 bytes for the length + 3 bytes of data
    /// # Ok(())
    /// # }
    /// ```
    fn size(&self) -> usize {
        self.name.len() + size_of::<u32>() + self.data.len()
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
    /// use tesutil::tes3::plugin::*;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut buf: Vec<u8> = vec![];
    /// let field = Tes3Field::new(b"NAME", vec![1, 2, 3])?;
    /// field.write(&mut &mut buf)?;
    /// assert!(buf.len() > 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    fn write(&self, mut f: &mut dyn Write) -> io::Result<()> {
        let len = self.data.len();

        f.write_exact(&self.name)?;
        f.write_exact(&(len as u32).to_le_bytes())?;
        f.write_exact(&self.data)?;

        Ok(())
    }
}

impl Tes3Field {
    /// Gets a reader over the contents of this field
    pub fn reader(&self) -> impl Read + Seek + '_ {
        io::Cursor::new(self.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_field() {
        Tes3Field::new(b"NAME", vec![]).unwrap();
    }

    #[test]
    fn read_field() {
        let data = b"NAME\x09\0\0\0GameHour\0";
        let field = Tes3Field::read(&mut data.as_ref()).unwrap();
        assert_eq!(field.name, *b"NAME");
        assert_eq!(field.data, b"GameHour\0");
        assert_eq!(field.size(), data.len());
    }

    #[test]
    fn read_field_empty() {
        let data = b"";
        if Tes3Field::read(&mut data.as_ref()).is_ok() {
            panic!("Read of empty field succeeded");
        }
    }

    #[test]
    fn read_field_invalid_len() {
        let data = b"NAME\x0f\0\0\0GameHour\0";
        if Tes3Field::read(&mut data.as_ref()).is_ok() {
            panic!("Read of field with invalid length succeeded");
        }
    }

    #[test]
    fn write_field() {
        let field = Tes3Field::new(b"NAME", b"PCHasCrimeGold\0".to_vec()).unwrap();
        let mut buf = vec![];
        field.write(&mut buf).unwrap();
        assert_eq!(buf, *b"NAME\x0f\0\0\0PCHasCrimeGold\0");
    }

    #[test]
    fn read_zstring_field() {
        let data = b"NAME\x09\0\0\0GameHour\0";
        let field = Tes3Field::read(&mut data.as_ref()).unwrap();
        let s = field.get_zstring().unwrap();
        assert_eq!(s, "GameHour");
    }

    #[test]
    fn read_string_field() {
        let data = b"BNAM\x15\0\0\0shield_nordic_leather";
        let field = Tes3Field::read(&mut data.as_ref()).unwrap();
        let s = field.get_string().unwrap();
        assert_eq!(s, "shield_nordic_leather");
    }

    #[test]
    fn read_raw_field() {
        let data = b"ALDT\x0c\0\0\0\0\0\xa0\x40\x0a\0\0\0\0\0\0\0";
        let field = Tes3Field::read(&mut data.as_ref()).unwrap();
        let d = field.get();
        assert_eq!(d, *b"\0\0\xa0\x40\x0a\0\0\0\0\0\0\0");
    }

    #[test]
    fn read_numeric_field() {
        let data = b"DATA\x08\0\0\0\x75\x39\xc2\x04\0\0\0\0";
        let field = Tes3Field::read(&mut data.as_ref()).unwrap();
        let v = field.get_u64().unwrap();
        assert_eq!(v, 0x4c23975u64);
    }

    #[test]
    fn set_zstring_field() {
        let mut field = Tes3Field::new(b"NAME", vec![]).unwrap();
        field.set_zstring(String::from("sWerewolfRefusal")).unwrap();
        assert_eq!(field.data, *b"sWerewolfRefusal\0");
    }

    #[test]
    fn set_string_field() {
        let mut field = Tes3Field::new(b"BNAM", vec![]).unwrap();
        field.set_string(String::from("a_steel_helmet")).unwrap();
        assert_eq!(field.data, *b"a_steel_helmet");
    }

    #[test]
    fn set_raw_field() {
        let mut field = Tes3Field::new(b"ALDT", vec![]).unwrap();
        field
            .set(b"\0\0\xa0\x40\x0a\0\0\0\0\0\0\0".to_vec())
            .unwrap();
        assert_eq!(field.data, *b"\0\0\xa0\x40\x0a\0\0\0\0\0\0\0");
    }

    #[test]
    fn set_numeric_field() {
        let mut field = Tes3Field::new(b"XSCL", vec![]).unwrap();
        field.set_f32(0.75);
        assert_eq!(field.data, *b"\0\0\x40\x3f");
    }
}
