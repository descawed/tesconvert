use std::io::{Read, Write};
use std::mem::size_of;
use std::str;

use super::field::Tes3Field;
use crate::plugin::*;
use crate::*;

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
pub struct Tes3Record {
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
    status: RecordStatus,
    raw_data: Vec<u8>,
    changed: bool,
    fields: Vec<Tes3Field>,
}

const DELETED_FIELD_SIZE: usize = 12;

impl IntoIterator for Tes3Record {
    type Item = Tes3Field;
    type IntoIter = <Vec<Tes3Field> as IntoIterator>::IntoIter;

    /// Consumes the record and returns an iterator over its fields
    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

impl Record<Tes3Field> for Tes3Record {
    fn read_lazy<T: Read>(mut f: T) -> Result<Tes3Record, TesError> {
        let mut name = [0u8; 4];
        f.read_exact(&mut name)?;

        let size = extract!(f as u32)? as usize;

        let mut buf = [0u8; 4];
        // the next field is useless, but skipping bytes is apparently needlessly complicated
        // via the Read trait? so we'll just do a dummy read into buf and then do the real read
        f.read_exact(&mut buf)?;
        f.read_exact(&mut buf)?;

        let flags = u32::from_le_bytes(buf);

        let mut data = vec![0u8; size];
        // read in the field data
        f.read_exact(&mut data)?;

        Ok(Tes3Record {
            name,
            is_deleted: flags & FLAG_DELETED != 0,
            is_persistent: flags & FLAG_PERSISTENT != 0,
            is_initially_disabled: flags & FLAG_INITIALLY_DISABLED != 0,
            is_blocked: flags & FLAG_BLOCKED != 0,
            status: RecordStatus::Initialized,
            raw_data: data,
            changed: false,
            fields: vec![],
        })
    }

    /// Returns a reference to the record name
    fn name(&self) -> &[u8; 4] {
        &self.name
    }

    fn status(&self) -> RecordStatus {
        self.status
    }

    fn finalize(&mut self) -> Result<(), TesError> {
        if self.status == RecordStatus::Initialized {
            let mut size = self.raw_data.len();
            let mut reader: &mut &[u8] = &mut self.raw_data.as_ref();
            while size > 0 {
                match Tes3Field::read(&mut reader) {
                    Ok(field) => {
                        let field_size = field.size();
                        if field_size > size {
                            self.status = RecordStatus::Failed;
                            return Err(decode_failed("Field size exceeds record size"));
                        }

                        size -= field.size();
                        // calling add_field does some checks that are unncessary now, and also
                        // causes problems with the borrow checker
                        if field.name() == b"DELE" {
                            // we'll add this field automatically based on the deleted flag, so don't add it to
                            // the fields vector
                            self.is_deleted = true;
                        } else {
                            self.fields.push(field);
                        }
                    }
                    Err(e) => {
                        self.status = RecordStatus::Failed;
                        return Err(decode_failed_because("Failed decoding field", e));
                    }
                }
            }

            self.status = RecordStatus::Finalized;
            self.changed = false;
        }

        Ok(())
    }

    /// Returns an iterator over this record's fields
    fn iter(&self) -> Box<dyn Iterator<Item = &Tes3Field> + '_> {
        self.require_finalized();
        Box::new(self.fields.iter())
    }

    /// Returns a mutable iterator over this record's fields
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut Tes3Field> + '_> {
        self.require_finalized();
        Box::new(self.fields.iter_mut())
    }

    /// Writes the record to the provided writer
    ///
    /// Writes a record to any type that implements [`Write`] or a mutable reference to such a type.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs.
    ///
    /// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    fn write<T: Write>(&self, mut f: &mut T) -> Result<(), TesError> {
        let flags = if self.is_deleted { FLAG_DELETED } else { 0 }
            | if self.is_persistent {
                FLAG_PERSISTENT
            } else {
                0
            }
            | if self.is_initially_disabled {
                FLAG_INITIALLY_DISABLED
            } else {
                0
            }
            | if self.is_blocked { FLAG_BLOCKED } else { 0 };

        f.write_exact(&self.name)?;

        if !self.changed {
            f.write_exact(&(self.raw_data.len() as u32).to_le_bytes())?;
            f.write_exact(b"\0\0\0\0")?; // dummy field
            f.write_exact(&flags.to_le_bytes())?;
            f.write_exact(&self.raw_data)?;
        } else {
            let size = self.field_size();

            if size > MAX_DATA {
                return Err(TesError::LimitExceeded {
                    description: String::from("Record data too long to be serialized"),
                    max_size: MAX_DATA,
                    actual_size: size,
                });
            }

            f.write_exact(&(size as u32).to_le_bytes())?;
            f.write_exact(b"\0\0\0\0")?; // dummy field
            f.write_exact(&flags.to_le_bytes())?;

            for field in self.fields.iter() {
                field.write(&mut f)?;
            }

            if self.is_deleted {
                let del = Tes3Field::new(b"DELE", vec![0; 4]).unwrap();
                del.write(&mut f)?;
            }
        }

        Ok(())
    }
}

// FIXME: change Record so that add_field can efficiently check if adding the field would exceed
//  the maximum record size and then remove the check from write. obstacles: size method is O(n);
//  a caller could edit fields with iter_mut() and Record won't be notified of the new size.
impl Tes3Record {
    /// Creates a new, empty record
    pub fn new(name: &[u8; 4]) -> Tes3Record {
        Tes3Record {
            name: *name,
            is_deleted: false,
            is_persistent: false,
            is_initially_disabled: false,
            is_blocked: false,
            status: RecordStatus::Finalized,
            raw_data: vec![],
            changed: false,
            fields: vec![],
        }
    }

    /// Returns whether this is a record type that has an ID
    ///
    /// If this returns true, it does not guarantee that the `id` method will not return `None`.
    /// That can still happen if the `b"NAME"` field is missing. This only indicates whether this
    /// is a record type that is expected to have an ID.
    pub fn has_id(&self) -> bool {
        !matches!(
            &self.name,
            b"CELL"
                | b"DIAL"
                | b"MGEF"
                | b"INFO"
                | b"LAND"
                | b"PGRD"
                | b"SCPT"
                | b"SKIL"
                | b"SSCR"
                | b"TES3"
        )
    }

    /// Returns the record ID
    ///
    /// Not all record types have an ID; in this case, this method will return `None`. For record
    /// types that do have an ID, this method may also return `None` if the `b"NAME"` field
    /// containing the ID has not yet been added to the record.
    ///
    /// # Panics
    ///
    /// Panics if this is a record type that has an ID and the record has not been finalized.
    pub fn id(&self) -> Option<&str> {
        if self.has_id() {
            self.require_finalized();
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
        } else {
            None
        }
    }

    fn require_finalized(&self) {
        if self.status != RecordStatus::Finalized {
            panic!("Attempted to access partially loaded record data");
        }
    }

    /// Adds a field to this record
    ///
    /// Note: you should use the [`is_deleted`] member instead of directly adding `b"DELE"` fields
    /// to records. If you do attempt to add a `b"DELE"` field, [`is_deleted`] will be set to true
    /// instead.
    ///
    /// [`is_deleted`]: #structfield.is_deleted
    pub fn add_field(&mut self, field: Tes3Field) {
        self.require_finalized();
        self.changed = true;
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
            + if self.is_deleted {
                DELETED_FIELD_SIZE
            } else {
                0
            }
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

    /// Returns whether this record is empty (contains no fields)
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Removes all fields from this record
    pub fn clear(&mut self) {
        self.require_finalized();
        self.changed = true;
        self.fields.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_record() {
        let data = b"GLOB\x27\0\0\0\0\0\0\0\0\0\0\0NAME\x0a\0\0\0TimeScale\0FNAM\x01\0\0\0fFLTV\x04\0\0\0\0\0\x20\x41".to_vec();
        let cursor = io::Cursor::new(data);
        let record = Tes3Record::read(cursor).unwrap();
        assert_eq!(record.name, *b"GLOB");
        assert!(!record.is_deleted);
        assert!(!record.is_persistent);
        assert!(!record.is_initially_disabled);
        assert!(!record.is_blocked);
        assert_eq!(record.fields.len(), 3);
    }

    #[test]
    fn read_deleted_record() {
        let data = b"DIAL\x2b\0\0\0\0\0\0\0\x20\0\0\0NAME\x0b\0\0\0Berel Sala\0DATA\x04\0\0\0\0\0\0\0DELE\x04\0\0\0\0\0\0\0".to_vec();
        let len = data.len();
        let cursor = io::Cursor::new(data);
        let record = Tes3Record::read(cursor).unwrap();
        assert!(record.is_deleted);
        assert_eq!(record.size(), len);
    }

    #[test]
    fn write_record() {
        let mut record = Tes3Record::new(b"DIAL");
        record.is_deleted = true;
        record.add_field(Tes3Field::new(b"NAME", b"Berel Sala\0".to_vec()).unwrap());
        record.add_field(Tes3Field::new(b"DATA", vec![0; 4]).unwrap());

        let buf = vec![];
        let mut cursor = io::Cursor::new(buf);
        record.write(&mut cursor).unwrap();
        assert_eq!(cursor.into_inner(), b"DIAL\x2b\0\0\0\0\0\0\0\x20\0\0\0NAME\x0b\0\0\0Berel Sala\0DATA\x04\0\0\0\0\0\0\0DELE\x04\0\0\0\0\0\0\0".to_vec());
    }
}
