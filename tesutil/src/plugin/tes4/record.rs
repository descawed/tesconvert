use std::io;
use std::io::{Read, Write};
use std::mem::size_of;
use std::str;

use crate::*;
use crate::plugin::{PluginError, MAX_DATA, FieldInterface};
use super::field::Field;

// this line is only to help the IDE
use bitflags;

use flate2::bufread::ZlibDecoder;



bitflags! {
    struct RecordFlags: u32 {
        const MASTER = 0x00001;
        const DELETED = 0x00020;
        const SHADOWS = 0x00200;
        const PERSISTENT = 0x00400;
        const INITIALLY_DISABLED = 0x00800;
        const IGNORED = 0x01000;
        const VISIBLE_WHEN_DISTANT = 0x08000;
        const OFF_LIMITS = 0x20000;
        const COMPRESSED = 0x40000;
        const CANT_WAIT = 0x80000;
    }
}

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
    flags: RecordFlags,
    form_id: u32,
    vcs_info: u32,
    fields: Vec<Field>,
}

// FIXME: change Record so that add_field can efficiently check if adding the field would exceed
//  the maximum record size and then remove the check from write. obstacles: size method is O(n);
//  a caller could edit fields with iter_mut() and Record won't be notified of the new size.
impl Record {
    /// Creates a new, empty record
    pub fn new(name: &[u8; 4]) -> Record {
        Record {
            name: name.clone(),
            flags: RecordFlags::empty(),
            form_id: 0,
            vcs_info: 0,
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
        if !f.read_all_or_none(&mut name)? {
            return Ok(None);
        }

        let mut size = extract!(f as u32)? as usize;

        let mut buf = [0u8; 4];
        f.read_exact(&mut buf)?;

        let flags = RecordFlags::from_bits(u32::from_le_bytes(buf)).ok_or(io_error(PluginError::DecodeFailed {
            description: String::from("Invalid record flags"),
            cause: None,
        }))?;

        let form_id = extract!(f as u32)?;
        let vcs_info = extract!(f as u32)?;

        let mut record = Record {
            name,
            flags,
            form_id,
            vcs_info,
            fields: vec![],
        };

        let mut data = vec![0u8; size];
        // read in the field data
        f.read_exact(&mut data)?;

        let mut field_reader: dyn Read = if flags.contains(RecordFlags::COMPRESSED) {
            // 4 bytes to skip the size of the decoded data, which we don't need
            ZlibDecoder::new(&mut &data[4..])
        } else {
            &mut data.as_ref()
        };

        while size > 0 {
            let field = Field::read(&mut field_reader)?;
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
        self.fields.push(field);
    }

    fn field_size(&self) -> usize {
        self.fields.iter().map(|f| f.size()).sum::<usize>()
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