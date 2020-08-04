use std::io;
use std::io::{Read, Write};
use std::mem::size_of;
use std::str;

use crate::*;
use crate::plugin::{PluginError, MAX_DATA, Field};

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
            let field = Field::read(&mut data_ref, Game::Morrowind)?;
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
            field.write(&mut f, Game::Morrowind)?;
        }

        if self.is_deleted {
            let del = Field::new(b"DELE", vec![0; 4]).unwrap();
            del.write(&mut f, Game::Morrowind)?;
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