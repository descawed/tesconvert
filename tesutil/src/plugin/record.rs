use std::ffi::{CStr, CString};
use std::io;
use std::io::{Read, Write};
use std::mem::size_of;
use std::str;

use crate::*;
use super::{PluginError, check_size, decode_failed};

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

// unfortunately, to_le_bytes and from_le_bytes are not trait methods, but instead are implemented
// directly on the integer types, which means we can't use generics to write a single method for
// converting field data to and from integers. instead, we'll use this macro. I would have the
// macro generate the function names as well, but it looks like I would have to take the type as
// an identifier instead of a type, and even then, pasting identifiers requires third-party crates
// or nightly Rust.
macro_rules! to_num {
    ($type:ty, $name:ident) => (
        pub fn $name(&self) -> Result<$type, PluginError> {
            if self.data.len() != size_of::<$type>() {
                return Err(PluginError::DecodeFailed {
                    description: format!("expected {} bytes for {}, found {}", size_of::<$type>(), stringify!($type), self.data.len()),
                    cause: None,
                });
            }
            let mut buf = [0u8; size_of::<$type>()];
            buf.copy_from_slice(&self.data[..]);
            Ok(<$type>::from_le_bytes(buf))
        }
    )
}

macro_rules! from_num {
    ($type:ty, $name:ident, $new_name:ident) => (
        pub fn $name(&mut self, v: $type) {
            self.data = v.to_le_bytes().to_vec();
        }

        pub fn $new_name(name: &[u8; 4], data: $type) -> Field {
            Field {
                name: name.clone(),
                data: data.to_le_bytes().to_vec(),
            }
        }
    )
}

/// Maximum size in bytes of a record or field
pub const MAX_DATA: usize = u32::MAX as usize;

impl Field {
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
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"DATA", vec![/* binary gobbledygook */])?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    pub fn new(name: &[u8; 4], data: Vec<u8>) -> Result<Field, PluginError> {
        check_size(&data, MAX_DATA, "field data too large")?;
        Ok(Field {
            name: name.clone(),
            data,
        })
    }

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
    /// let field = Field::new_string(b"NAME", String::from("Flora_kelp_01"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    pub fn new_string(name: &[u8; 4], data: String) -> Result<Field, PluginError> {
        check_size(&data, MAX_DATA, "field data too large")?;
        Ok(Field {
            name: name.clone(),
            data: data.into_bytes(),
        })
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
    /// let field = Field::new_zstring(b"NAME", String::from("Flora_kelp_01"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`new_string`]: #method.new_string
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    /// [`std::ffi::NulError`]: https://doc.rust-lang.org/std/ffi/struct.NulError.html
    pub fn new_zstring(name: &[u8; 4], data: String) -> Result<Field, PluginError> {
        let zstr = CString::new(data).map_err(|e| decode_failed("Failed to decode as zstring", e))?;
        check_size(zstr.as_bytes_with_nul(), MAX_DATA, "field data too large")?;
        Ok(Field {
            name: name.clone(),
            data: zstr.into_bytes_with_nul(),
        })
    }

    /// Reads a field from a binary stream
    ///
    /// Reads a field from any type that implements [`Read`] or a mutable reference to such a type.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if an I/O error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
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
    pub fn read<T: Read>(mut f: T) -> io::Result<Field> {
        let mut name = [0u8; 4];
        f.read_exact(&mut name)?;

        let size = extract!(f as u32)? as usize;
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
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"NAME", vec![])?;
    /// assert_eq!(field.name(), b"NAME");
    /// # Ok(())
    /// # }
    /// ```
    pub fn name(&self) -> &[u8] {
        &self.name
    }

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
    /// let field = Field::new(b"NAME", vec![])?;
    /// assert_eq!(field.display_name(), "NAME");
    /// # Ok(())
    /// # }
    /// ```
    pub fn display_name(&self) -> &str {
        str::from_utf8(&self.name).unwrap_or("<invalid>")
    }

    /// Calculates the size in bytes of this field
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"NAME", vec![1, 2, 3])?;
    /// assert_eq!(field.size(), 11); // 4 bytes for the name + 4 bytes for the length + 3 bytes of data
    /// # Ok(())
    /// # }
    /// ```
    pub fn size(&self) -> usize {
        self.name.len() + size_of::<u32>() + self.data.len()
    }

    /// Writes the field to the provided writer
    ///
    /// Writes a field to any type that implements [`Write`] or a mutable reference to such a type.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if an I/O error occurs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
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
    pub fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
        let len = self.data.len();

        f.write_exact(&self.name)?;
        f.write_exact(&(len as u32).to_le_bytes())?;
        f.write_exact(&self.data)?;

        Ok(())
    }

    /// Returns a reference to the field's data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"DATA", vec![1, 2, 3])?;
    /// assert_eq!(*field.get(), [1, 2, 3]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&self) -> &[u8] {
        &self.data[..]
    }

    /// Consumes the field and takes ownership of its data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tesutil::plugin::*;
    ///
    /// # fn main() -> Result<(), PluginError> {
    /// let field = Field::new(b"DATA", vec![1, 2, 3])?;
    /// let data = field.consume();
    /// assert_eq!(data[..], [1, 2, 3]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn consume(self) -> Vec<u8> {
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
    pub fn set(&mut self, data: Vec<u8>) -> Result<(), PluginError> {
        check_size(&data, MAX_DATA, "field data too large")?;
        self.data = data;
        Ok(())
    }

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
    /// let field = Field::read(&mut data.as_ref())?;
    /// let name = field.get_string()?;
    /// assert_eq!(name, "sSkillClassMajor");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`PluginError::DecodeFailed`]: enum.PluginError.html#variant.DecodeFailed
    // FIXME: the below string functions will fail on non-English versions of the game
    pub fn get_string(&self) -> Result<&str, PluginError> {
        str::from_utf8(&self.data[..]).map_err(|e| decode_failed("failed to decode string", e))
    }

    /// Sets the field's data from a string
    ///
    /// # Errors
    ///
    /// Returns a [`PluginError::LimitExceeded`] if the size of `data` exceeds [`MAX_DATA`].
    ///
    /// [`PluginError::LimitExceeded`]: enum.PluginError.html#variant.LimitExceeded
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    pub fn set_string(&mut self, data: String) -> Result<(), PluginError> {
        check_size(&data, MAX_DATA, "field data too large")?;
        self.data = data.into_bytes();
        Ok(())
    }

    /// Gets a reference to the fields data as a null-terminated string
    ///
    /// The data must include a terminating null byte, and the null will not be included in the
    /// result.
    ///
    /// # Errors
    ///
    /// Returns an error if the data includes internal null bytes or if the data is not valid UTF-8.
    pub fn get_zstring(&self) -> Result<&str, PluginError> {
        let zstr = CStr::from_bytes_with_nul(&self.data[..]).map_err(|e| decode_failed("string contained internal nulls", e))?;
        let s = zstr.to_str().map_err(|e| decode_failed("failed to decode string", e))?;
        Ok(s)
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
    pub fn set_zstring(&mut self, data: String) -> Result<(), PluginError> {
        let zstr = CString::new(data).map_err(|e| decode_failed("string contained internal nulls", e))?;
        check_size(zstr.as_bytes_with_nul(), MAX_DATA, "field data too large")?;
        self.data = zstr.into_bytes_with_nul();
        Ok(())
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

const FLAG_DELETED: u32 = 0x0020;
const FLAG_PERSISTENT: u32 = 0x0400;
const FLAG_INITIALLY_DISABLED: u32 = 0x0800;
const FLAG_BLOCKED: u32 = 0x2000;

/// A game object in a plugin
///
/// A record represents an object in the game, such as an NPC, container, global variable, etc.
/// Each record consists of a 4-byte ASCII identifier, such as `b"CREA"`, and a series of fields
/// defining the object's attributes.
///
/// Note that [`Field::new`] takes the name by reference and copies it. This is because new records
/// are (almost?) always constructed from hard-coded names such as `b"NPC_"` or `b"GLOB"`, and it
/// would be cumbersome to have to explicitly clone these everywhere.
///
/// [`Field::new`]: #method.new
#[derive(Debug)]
pub struct Record{
    name: [u8; 4],
    /// Whether the record is deleted
    ///
    /// This is different from actually removing the record from a plugin. If a plugin includes
    /// a record that is also present in a master file, that record will override the one from the
    /// master. Removing the record from the plugin will cause the record from the master to appear
    /// in the game unmodified. Including the record in the plugin but marking it deleted will cause
    /// the record to be deleted from the game.
    pub is_deleted: bool,
    /// Whether references to this object persist
    pub is_persistent: bool,
    /// Whether this object starts disabled
    pub is_initially_disabled: bool,
    // TODO: figure out what this does
    pub is_blocked: bool,
    fields: Vec<Field>,
}

const DELETED_FIELD_SIZE: usize = 12;

// FIXME: change Record so that add_field can efficiently check if adding the field would exceed
//  the maximum record size and then remove the check from write. obstacles: size method is O(n);
//  a caller could edit fields with iter_mut() and Record won't be notified of the new size.
impl Record {
    /// Creates a new, empty record
    pub fn new(name: &[u8; 4]) -> Record {
        Record {
            name: name.clone(),
            is_deleted: false,
            is_persistent: false,
            is_initially_disabled: false,
            is_blocked: false,
            fields: vec![],
        }
    }

    /// Reads a record from a binary stream
    ///
    /// Reads a record from any type that implements [`Read`] or a mutable reference to such a type.
    /// On success, this function returns an `Option<Record>`. A value of `None` indicates that the
    /// stream was at EOF; otherwise, it will be `Some(Record)`. This is necessary because EOF
    /// indicates that the end of a plugin file has been reached.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if an I/O error occurs.
    ///
    /// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    pub fn read<T: Read>(mut f: T) -> io::Result<Option<Record>> {
        let mut name = [0u8; 4];

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
        while total_bytes_read < name.len() {
            let bytes_read = f.read(&mut name[total_bytes_read..])?;
            if bytes_read == 0 {
                if total_bytes_read == 0 {
                    return Ok(None);
                } else if total_bytes_read < name.len() {
                    return Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
                }
            }

            total_bytes_read += bytes_read;
        }

        let mut size = extract!(f as u32)? as usize;

        let mut buf = [0u8; 4];
        // the next field is useless, but skipping bytes is apparently needlessly complicated
        // via the Read trait? so we'll just do a dummy read into buf and then do the real read
        f.read_exact(&mut buf)?;
        f.read_exact(&mut buf)?;

        let flags = u32::from_le_bytes(buf);

        let mut record = Record {
            name,
            is_deleted: flags & FLAG_DELETED != 0,
            is_persistent: flags & FLAG_PERSISTENT != 0,
            is_initially_disabled: flags & FLAG_INITIALLY_DISABLED != 0,
            is_blocked: flags & FLAG_BLOCKED != 0,
            fields: vec![],
        };

        let mut data = vec![0u8; size];
        // read in the field data
        f.read_exact(&mut data)?;

        let mut data_ref: &[u8] = data.as_ref();
        while size > 0 {
            let field = Field::read(&mut data_ref)?;
            let field_size = field.size();
            if field_size > size {
                return Err(io_error("Field size exceeds record size"));
            }

            size -= field.size();
            record.add_field(field);
        }

        Ok(Some(record))
    }

    /// Writes the record to the provided writer
    ///
    /// Writes a record to any type that implements [`Write`] or a mutable reference to such a type.
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if an I/O error occurs.
    ///
    /// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    pub fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
        let size = self.field_size();

        if size > MAX_DATA {
            return Err(io_error(PluginError::LimitExceeded {
                description: String::from("Record data too long to be serialized"),
                max_size: MAX_DATA,
                actual_size: size,
            }));
        }

        let flags = if self.is_deleted { FLAG_DELETED } else { 0 }
            | if self.is_persistent { FLAG_PERSISTENT } else { 0 }
            | if self.is_initially_disabled { FLAG_INITIALLY_DISABLED } else { 0 }
            | if self.is_blocked { FLAG_BLOCKED } else { 0 };

        f.write_exact(&self.name)?;
        f.write_exact(&(size as u32).to_le_bytes())?;
        f.write_exact(b"\0\0\0\0")?; // dummy field
        f.write_exact(&flags.to_le_bytes())?;

        for field in self.fields.iter() {
            field.write(&mut f)?;
        }

        if self.is_deleted {
            let del = Field::new(b"DELE", vec![0; 4]).unwrap();
            del.write(&mut f)?;
        }

        Ok(())
    }

    /// Returns a reference to the record name
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Returns the record name as a string
    ///
    /// If the record name cannot be decoded as UTF-8 (which will never happen in a valid plugin
    /// file), the string `"<invalid>"` will be returned.
    pub fn display_name(&self) -> &str {
        str::from_utf8(&self.name).unwrap_or("<invalid>")
    }

    /// Returns the record ID
    ///
    /// Not all record types have an ID; in this case, this method will return `None`. For record
    /// types that do have an ID, this method may also return `None` if the `b"NAME"` field
    /// containing the ID has not yet been added to the record.
    pub fn id(&self) -> Option<&str> {
        match &self.name {
            b"CELL" | b"DIAL" | b"MGEF" | b"INFO" | b"LAND" | b"PGRD" | b"SCPT" | b"SKIL" | b"SSCR" | b"TES3" => None,
            _ => {
                let mut id = None;
                for field in self.fields.iter() {
                    if field.name() == b"NAME" {
                        id = match &self.name {
                            b"GMST" | b"WEAP" => field.get_string().ok(),
                            _ => field.get_zstring().ok(),
                        };
                        break;
                    }
                }
                id
            },
        }
    }

    /// Adds a field to this record
    ///
    /// Note: you should use the [`is_deleted`] member instead of directly adding `b"DELE"` fields
    /// to records. If you do attempt to add a `b"DELE"` field, [`is_deleted`] will be set to true
    /// instead.
    ///
    /// [`is_deleted`]: #structfield.is_deleted
    pub fn add_field(&mut self, field: Field) {
        if field.name() == b"DELE" {
            // we'll add this field automatically based on the deleted flag, so don't add it to
            // the fields vector
            self.is_deleted = true;
        } else {
            self.fields.push(field);
        }
    }

    fn field_size(&self) -> usize {
        self.fields.iter().map(|f| f.size()).sum::<usize>()
            + if self.is_deleted { DELETED_FIELD_SIZE } else { 0 }
    }

    /// Calculates the size in bytes of this record
    pub fn size(&self) -> usize {
        self.name.len()
            + size_of::<u32>()*3 // 3 = size + dummy + flags
            + self.field_size()
    }

    /// Returns the number of fields currently in the record
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Consumes the record and returns an iterator over its fields
    pub fn into_iter(self) -> impl Iterator<Item = Field> {
        self.fields.into_iter()
    }

    /// Returns an iterator over this record's fields
    pub fn iter(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter()
    }

    /// Returns a mutable iterator over this record's fields
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Field> {
        self.fields.iter_mut()
    }

    /// Removes all fields from this record
    pub fn clear(&mut self) {
        self.fields.clear();
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn create_field() {
        Field::new(b"NAME", vec![]).unwrap();
    }

    #[test]
    fn read_field() {
        let data = b"NAME\x09\0\0\0GameHour\0";
        let field = Field::read(&mut data.as_ref()).unwrap();
        assert_eq!(field.name, *b"NAME");
        assert_eq!(field.data, b"GameHour\0");
        assert_eq!(field.size(), data.len());
    }

    #[test]
    fn read_field_empty() {
        let data = b"";
        if Field::read(&mut data.as_ref()).is_ok() {
            panic!("Read of empty field succeeded");
        }
    }

    #[test]
    fn read_field_invalid_len() {
        let data = b"NAME\x0f\0\0\0GameHour\0";
        if Field::read(&mut data.as_ref()).is_ok() {
            panic!("Read of field with invalid length succeeded");
        }
    }

    #[test]
    fn write_field() {
        let field = Field::new(b"NAME", b"PCHasCrimeGold\0".to_vec()).unwrap();
        let mut buf = vec![];
        field.write(&mut buf).unwrap();
        assert_eq!(buf, *b"NAME\x0f\0\0\0PCHasCrimeGold\0");
    }

    #[test]
    fn read_zstring_field() {
        let data = b"NAME\x09\0\0\0GameHour\0";
        let field = Field::read(&mut data.as_ref()).unwrap();
        let s = field.get_zstring().unwrap();
        assert_eq!(s, "GameHour");
    }

    #[test]
    fn read_string_field() {
        let data = b"BNAM\x15\0\0\0shield_nordic_leather";
        let field = Field::read(&mut data.as_ref()).unwrap();
        let s = field.get_string().unwrap();
        assert_eq!(s, "shield_nordic_leather");
    }

    #[test]
    fn read_raw_field() {
        let data = b"ALDT\x0c\0\0\0\0\0\xa0\x40\x0a\0\0\0\0\0\0\0";
        let field = Field::read(&mut data.as_ref()).unwrap();
        let d = field.get();
        assert_eq!(d, *b"\0\0\xa0\x40\x0a\0\0\0\0\0\0\0");
    }

    #[test]
    fn read_numeric_field() {
        let data = b"DATA\x08\0\0\0\x75\x39\xc2\x04\0\0\0\0";
        let field = Field::read(&mut data.as_ref()).unwrap();
        let v = field.get_u64().unwrap();
        assert_eq!(v, 0x4c23975u64);
    }

    #[test]
    fn set_zstring_field() {
        let mut field = Field::new(b"NAME", vec![]).unwrap();
        field.set_zstring(String::from("sWerewolfRefusal")).unwrap();
        assert_eq!(field.data, *b"sWerewolfRefusal\0");
    }

    #[test]
    fn set_string_field() {
        let mut field = Field::new(b"BNAM", vec![]).unwrap();
        field.set_string(String::from("a_steel_helmet")).unwrap();
        assert_eq!(field.data, *b"a_steel_helmet");
    }

    #[test]
    fn set_raw_field() {
        let mut field = Field::new(b"ALDT", vec![]).unwrap();
        field.set(b"\0\0\xa0\x40\x0a\0\0\0\0\0\0\0".to_vec()).unwrap();
        assert_eq!(field.data, *b"\0\0\xa0\x40\x0a\0\0\0\0\0\0\0");
    }

    #[test]
    fn set_numeric_field() {
        let mut field = Field::new(b"XSCL", vec![]).unwrap();
        field.set_f32(0.75);
        assert_eq!(field.data, *b"\0\0\x40\x3f");
    }

    #[test]
    fn read_record() {
        let data = b"GLOB\x27\0\0\0\0\0\0\0\0\0\0\0NAME\x0a\0\0\0TimeScale\0FNAM\x01\0\0\0fFLTV\x04\0\0\0\0\0\x20\x41";
        let record = Record::read(&mut data.as_ref()).unwrap().unwrap();
        assert_eq!(record.name, *b"GLOB");
        assert!(!record.is_deleted);
        assert!(!record.is_persistent);
        assert!(!record.is_initially_disabled);
        assert!(!record.is_blocked);
        assert_eq!(record.fields.len(), 3);
    }

    #[test]
    fn read_deleted_record() {
        let data = b"DIAL\x2b\0\0\0\0\0\0\0\x20\0\0\0NAME\x0b\0\0\0Berel Sala\0DATA\x04\0\0\0\0\0\0\0DELE\x04\0\0\0\0\0\0\0";
        let record = Record::read(&mut data.as_ref()).unwrap().unwrap();
        assert!(record.is_deleted);
        assert_eq!(record.size(), data.len());
    }

    #[test]
    fn write_record() {
        let mut record = Record::new(b"DIAL");
        record.is_deleted = true;
        record.add_field(Field::new(b"NAME", b"Berel Sala\0".to_vec()).unwrap());
        record.add_field(Field::new(b"DATA", vec![0; 4]).unwrap());

        let mut buf = vec![];
        record.write(&mut buf).unwrap();
        assert_eq!(buf, b"DIAL\x2b\0\0\0\0\0\0\0\x20\0\0\0NAME\x0b\0\0\0Berel Sala\0DATA\x04\0\0\0\0\0\0\0DELE\x04\0\0\0\0\0\0\0".to_vec());
    }
}