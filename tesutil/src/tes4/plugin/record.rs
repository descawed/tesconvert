use std::io::{Read, Seek, Write};

use super::field::Tes4Field;
use super::group::Group;
use super::FormId;
use crate::plugin::*;
use crate::*;

use bitflags::bitflags;

use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;

macro_rules! flag_property {
    ($get:ident, $set:ident, $flag:ident) => {
        pub fn $get(&self) -> bool {
            self.flags.contains(RecordFlags::$flag)
        }

        pub fn $set(&mut self, value: bool) {
            if value {
                self.flags |= RecordFlags::$flag;
            } else {
                self.flags -= RecordFlags::$flag;
            }
        }
    };
}

bitflags! {
    struct RecordFlags: u32 {
        const MASTER = 0x00001;
        const DELETED = 0x00020;
        const BORDER_REGION = 0x00040;
        const TURN_OFF_FIRE = 0x00080;
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

const COMPRESSION_LEVEL: u32 = 6;

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
pub struct Tes4Record {
    name: [u8; 4],
    flags: RecordFlags,
    form_id: FormId,
    vcs_info: u32,
    status: RecordStatus,
    raw_data: Vec<u8>,
    changed: bool,
    fields: Vec<Tes4Field>,
    groups: Vec<Group>,
}

impl IntoIterator for Tes4Record {
    type Item = Tes4Field;
    type IntoIter = <Vec<Tes4Field> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.require_finalized();
        self.fields.into_iter()
    }
}

impl Record<Tes4Field> for Tes4Record {
    /// Reads a record from a binary stream and returns it with the number of bytes read
    ///
    /// Reads a record from any type that implements [`Read`] or a mutable reference to such a type.
    /// On success, this function returns a `(Record, usize)`, where the `usize` is the number of
    /// bytes read. This function takes the record `name` instead of reading it because the caller
    /// must have already verified that this is a record and not a [`Group`].
    ///
    /// # Errors
    ///
    /// Returns a [`std::io::Error`] if an I/O error occurs.
    ///
    /// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [`Group`']: struct.Group.html
    fn read_lazy<T: Read>(mut f: T) -> Result<Tes4Record, TesError> {
        let mut name = [0u8; 4];
        f.read_exact(&mut name)?;

        if name == *b"GRUP" {
            return Err(decode_failed("Expected record, found group"));
        }

        Tes4Record::read_lazy_with_name(f, name)
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
            let mut raw_reader: &[u8] = self.raw_data.as_ref();
            let mut size = if self.uses_compression() {
                match extract!(raw_reader as u32) {
                    Ok(size) => size as usize,
                    Err(e) => {
                        self.status = RecordStatus::Failed;
                        return Err(decode_failed_because(
                            "Failed to read compressed data size",
                            e,
                        ));
                    }
                }
            } else {
                self.raw_data.len()
            };

            let mut zlib_reader =
                ZlibDecoder::new(<Vec<u8> as AsRef<[u8]>>::as_ref(&self.raw_data));

            while size > 0 {
                let result = if self.uses_compression() {
                    Tes4Field::read(&mut zlib_reader)
                } else {
                    Tes4Field::read(&mut raw_reader)
                };

                match result {
                    Ok(field) => {
                        let field_size = field.size();
                        if field_size > size {
                            self.status = RecordStatus::Failed;
                            return Err(decode_failed("Field size exceeds record size"));
                        }

                        size -= field.size();
                        self.fields.push(field);
                    }
                    Err(e) => {
                        self.status = RecordStatus::Failed;
                        return Err(decode_failed_because("Failed to decode record field", e));
                    }
                }
            }

            self.status = RecordStatus::Finalized;
            self.changed = false;
        }

        Ok(())
    }

    /// Returns an iterator over this record's fields
    ///
    /// # Panics
    ///
    /// Panics if this record has not been finalized.
    fn iter(&self) -> Box<dyn Iterator<Item = &Tes4Field> + '_> {
        self.require_finalized();
        Box::new(self.fields.iter())
    }

    /// Returns a mutable iterator over this record's fields
    ///
    /// # Panics
    ///
    /// Panics if this record has not been finalized.
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut Tes4Field> + '_> {
        self.require_finalized();
        Box::new(self.fields.iter_mut())
    }

    /// Writes the record to the provided writer
    ///
    /// Writes a record to any type that implements [`Write`] and [`Seek`] or a mutable reference to such a type.
    /// On success, returns number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs.
    ///
    /// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`Seek`]: https://doc.rust-lang.org/std/io/trait.Seek.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    fn write<T: Write + Seek>(&self, f: &mut T) -> Result<(), TesError> {
        f.write_exact(&self.name)?;

        if !self.changed {
            f.write_exact(&(self.raw_data.len() as u32).to_le_bytes())?;
            f.write_exact(&self.flags.bits.to_le_bytes())?;
            f.write_exact(&self.form_id.0.to_le_bytes())?;
            f.write_exact(&self.vcs_info.to_le_bytes())?;
            f.write_exact(&self.raw_data)?;
        } else {
            let size = self.field_size();

            // technically, if the record is compressed, we should do this check on the compressed data,
            // but I don't think this will ever happen anyway
            if size > MAX_DATA {
                return Err(TesError::LimitExceeded {
                    description: String::from("Record data too long to be serialized"),
                    max_size: MAX_DATA,
                    actual_size: size,
                });
            }

            if self.flags.contains(RecordFlags::COMPRESSED) {
                let mut raw_buf: Vec<u8> = Vec::with_capacity(size);
                for field in self.fields.iter() {
                    field.write(&mut &mut raw_buf)?;
                }

                let mut buf_reader: &[u8] = raw_buf.as_ref();
                let mut encoder =
                    ZlibEncoder::new(&mut buf_reader, Compression::new(COMPRESSION_LEVEL));
                let mut comp_buf: Vec<u8> = vec![];
                encoder.read_to_end(&mut comp_buf)?;

                f.write_exact(&(comp_buf.len() as u32).to_le_bytes())?;
                f.write_exact(&self.flags.bits.to_le_bytes())?;
                f.write_exact(&self.form_id.0.to_le_bytes())?;
                f.write_exact(&self.vcs_info.to_le_bytes())?;
                f.write_exact(&(size as u32).to_le_bytes())?;
                f.write_exact(&comp_buf)?;
            } else {
                f.write_exact(&(size as u32).to_le_bytes())?;
                f.write_exact(&self.flags.bits.to_le_bytes())?;
                f.write_exact(&self.form_id.0.to_le_bytes())?;
                f.write_exact(&self.vcs_info.to_le_bytes())?;

                for field in self.fields.iter() {
                    field.write(&mut *f)?;
                }
            }
        }

        for group in self.groups.iter() {
            group.write(&mut *f)?;
        }

        Ok(())
    }
}

// FIXME: change Record so that add_field can efficiently check if adding the field would exceed
//  the maximum record size and then remove the check from write. obstacles: size method is O(n);
//  a caller could edit fields with iter_mut() and Record won't be notified of the new size.
impl Tes4Record {
    /// Creates a new, empty record
    pub fn new(name: &[u8; 4]) -> Tes4Record {
        Tes4Record {
            name: *name,
            flags: RecordFlags::empty(),
            form_id: FormId(0),
            vcs_info: 0,
            status: RecordStatus::Finalized,
            raw_data: vec![],
            changed: true,
            fields: vec![],
            groups: vec![],
        }
    }

    /// Reads a record from a binary stream with the name already provided
    ///
    /// Reads a record from any type that implements [`Read`] or a mutable reference to such a type.
    /// On success, this function returns a `Record`. This function takes the record `name` instead
    /// of reading it because the caller must have already verified that this is a record and not
    /// a [`Group`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs.
    ///
    /// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
    /// [`std::io::Error`]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [`Group`']: struct.Group.html
    pub fn read_lazy_with_name<T: Read>(mut f: T, name: [u8; 4]) -> Result<Tes4Record, TesError> {
        let size = extract!(f as u32)? as usize;

        let mut buf = [0u8; 4];
        f.read_exact(&mut buf)?;

        let flags = RecordFlags::from_bits(u32::from_le_bytes(buf))
            .ok_or_else(|| decode_failed("Invalid record flags"))?;

        let form_id = FormId(extract!(f as u32)?);
        let vcs_info = extract!(f as u32)?;

        let mut data = vec![0u8; size];
        // read in the field data
        f.read_exact(&mut data)?;

        Ok(Tes4Record {
            name,
            flags,
            form_id,
            vcs_info,
            status: RecordStatus::Initialized,
            raw_data: data,
            changed: false,
            fields: vec![],
            groups: vec![],
        })
    }

    fn require_finalized(&self) {
        if self.status != RecordStatus::Finalized {
            panic!("Attempted to access partially loaded record data");
        }
    }

    flag_property!(is_master, set_master, MASTER);
    flag_property!(is_deleted, set_deleted, DELETED);
    flag_property!(casts_shadows, set_casts_shadows, SHADOWS);
    flag_property!(is_persistent, set_persistent, PERSISTENT);
    flag_property!(is_quest_item, set_quest_item, PERSISTENT); // same flag; meaning is contextual
    flag_property!(
        is_initially_disabled,
        set_initially_disabled,
        INITIALLY_DISABLED
    );
    flag_property!(is_ignored, set_ignored, IGNORED);
    flag_property!(
        is_visible_when_distant,
        set_visible_when_distant,
        VISIBLE_WHEN_DISTANT
    );
    flag_property!(is_dangerous, set_dangerous, OFF_LIMITS);
    flag_property!(is_off_limits, set_off_limits, OFF_LIMITS);

    pub fn uses_compression(&self) -> bool {
        self.flags.contains(RecordFlags::COMPRESSED)
    }

    /// Sets whether this record's data should be compressed
    ///
    /// # Panics
    ///
    /// Panics if the compression status of the record is changed and the record has not been
    /// finalized.
    pub fn set_compression(&mut self, value: bool) {
        if self.uses_compression() != value {
            self.require_finalized();
            self.changed = true;
        }

        if value {
            self.flags |= RecordFlags::COMPRESSED;
        } else {
            self.flags -= RecordFlags::COMPRESSED;
        }
    }

    pub fn can_wait(&self) -> bool {
        !self.flags.contains(RecordFlags::CANT_WAIT)
    }

    pub fn set_can_wait(&mut self, value: bool) {
        if value {
            self.flags -= RecordFlags::CANT_WAIT;
        } else {
            self.flags |= RecordFlags::CANT_WAIT;
        }
    }

    /// Returns the form ID
    pub fn id(&self) -> FormId {
        self.form_id
    }

    /// Adds a field to this record
    ///
    /// # Panics
    ///
    /// Panics if this record has not been finalized.
    pub fn add_field(&mut self, field: Tes4Field) {
        self.require_finalized();
        self.changed = true;
        self.fields.push(field);
    }

    fn field_size(&self) -> usize {
        self.fields.iter().map(|f| f.size()).sum::<usize>()
    }

    /// Adds an associated group to this record
    ///
    /// Examples of associated groups would be like a CELL record's Cell Children group or a DIAL
    /// record's Topic Children group.
    pub fn add_group(&mut self, group: Group) {
        self.groups.push(group);
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
    ///
    /// # Panics
    ///
    /// Panics if this record has not been finalized.
    pub fn clear(&mut self) {
        self.require_finalized();
        self.changed = true;
        self.fields.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_record() {
        let data = b"GLOB\x21\0\0\0\0\0\0\0\x3a\0\0\0\0\0\0\0EDID\x0a\0TimeScale\0FNAM\x01\0sFLTV\x04\0\0\0\xf0\x41".to_vec();
        let cursor = io::Cursor::new(data);
        let mut record = Tes4Record::read_lazy(cursor).unwrap();
        record.finalize().unwrap();
        assert_eq!(record.name, *b"GLOB");
        assert!(!record.is_deleted());
        assert!(!record.is_persistent());
        assert!(!record.is_initially_disabled());
        assert_eq!(record.fields.len(), 3);
    }

    #[test]
    fn write_record() {
        let mut record = Tes4Record::new(b"DIAL");
        record.form_id = FormId(0xaa);
        record.vcs_info = 0x181f1c;
        record.add_field(Tes4Field::new(b"EDID", b"ADMIREHATE\0".to_vec()).unwrap());
        record.add_field(Tes4Field::new_u32(b"QSTI", 0x1e722));
        record.add_field(Tes4Field::new_u32(b"QSTI", 0x10602));
        record.add_field(Tes4Field::new(b"FULL", b"ADMIRE_HATE\0".to_vec()).unwrap());
        record.add_field(Tes4Field::new_u8(b"DATA", 3));

        let mut writer = Cursor::new(vec![]);
        record.write(&mut writer).unwrap();
        assert_eq!(writer.into_inner(), b"DIAL\x3e\0\0\0\0\0\0\0\xaa\0\0\0\x1c\x1f\x18\0EDID\x0b\0ADMIREHATE\0QSTI\x04\0\x22\xe7\x01\0QSTI\x04\0\x02\x06\x01\0FULL\x0c\0ADMIRE_HATE\0DATA\x01\0\x03".to_vec());
    }
}
