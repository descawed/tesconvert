use std::error;
use std::ffi::{CStr, CString, NulError};
use std::io;
use std::io::{Read, Write};
use std::mem::size_of;
use std::str;

use crate::common::*;

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
        pub fn $name(&self) -> Option<$type> {
            if self.data.len() != size_of::<$type>() {
                return None;
            }
            let mut buf = [0u8; size_of::<$type>()];
            buf.copy_from_slice(&self.data[..]);
            Some(<$type>::from_le_bytes(buf))
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

impl Field {
    pub fn new(name: &[u8; 4], data: Vec<u8>) -> Field {
        Field {
            name: name.clone(),
            data,
        }
    }

    pub fn new_string(name: &[u8; 4], data: String) -> Field {
        Field {
            name: name.clone(),
            data: data.into_bytes(),
        }
    }

    pub fn new_zstring(name: &[u8; 4], data: String) -> Result<Field, NulError> {
        let zstr = CString::new(data)?;
        Ok(Field {
            name: name.clone(),
            data: zstr.into_bytes_with_nul(),
        })
    }

    pub fn read<T: Read>(f: &mut T) -> io::Result<Field> {
        let mut name = [0u8; 4];
        f.read_exact(&mut name)?;

        let size = extract!(f as u32)? as usize;
        let mut data = vec![0u8; size];

        f.read_exact(&mut data)?;

        Ok(Field { name, data })
    }

    pub fn name(&self) -> &[u8] {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        str::from_utf8(&self.name).unwrap_or("<invalid>")
    }

    pub fn size(&self) -> usize {
        self.name.len() + size_of::<u32>() + self.data.len()
    }

    pub fn write<T: Write>(&self, f: &mut T) -> io::Result<()> {
        let len = self.data.len();

        if len > u32::MAX as usize {
            return Err(io_error("Field data too long to be serialized"));
        }

        f.write_exact(&self.name)?;
        f.write_exact(&(len as u32).to_le_bytes())?;
        f.write_exact(&self.data)?;

        Ok(())
    }

    pub fn get(&self) -> &[u8] {
        &self.data[..]
    }

    pub fn consume(self) -> Vec<u8> {
        self.data
    }

    pub fn set(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    // FIXME: the below string functions will fail on non-English versions of the game
    pub fn get_string(&self) -> Result<&str, str::Utf8Error> {
        str::from_utf8(&self.data[..])
    }

    pub fn set_string(&mut self, data: String) {
        self.data = data.into_bytes();
    }

    pub fn get_zstring(&self) -> Result<&str, Box<dyn error::Error>> {
        let zstr = CStr::from_bytes_with_nul(&self.data[..])?;
        let s = zstr.to_str()?;
        Ok(s)
    }

    pub fn set_zstring(&mut self, data: String) -> Result<(), NulError> {
        let zstr = CString::new(data)?;
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

pub struct Record{
    name: [u8; 4],
    pub is_deleted: bool,
    pub is_persistent: bool,
    pub is_initially_disabled: bool,
    pub is_blocked: bool,
    fields: Vec<Field>,
}

const DELETED_FIELD_SIZE: usize = 12;

impl Record {
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

    pub fn read<T: Read>(f: &mut T) -> io::Result<Record> {
        let mut name = [0u8; 4];
        f.read_exact(&mut name)?;

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

        // TODO: all these Field constructions are going to make copies of the stuff in data,
        //  but we could actually just hand over ownership of the contents of this vector.
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

        Ok(record)
    }

    pub fn write<T: Write>(&self, f: &mut T) -> io::Result<()> {
        let size = self.field_size();

        if size > u32::MAX as usize {
            return Err(io_error("Record data too long to be serialized"));
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
            field.write(f)?;
        }

        if self.is_deleted {
            let del = Field::new(b"DELE", vec![0; 4]);
            del.write(f)?;
        }

        Ok(())
    }

    pub fn name(&self) -> &[u8] {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        str::from_utf8(&self.name).unwrap_or("<invalid>")
    }

    pub fn id(&self) -> Option<&str> {
        match &self.name {
            b"CELL" | b"MGEF" | b"INFO" | b"LAND" | b"PGRD" | b"SCPT" | b"SKIL" | b"SSCR" | b"TES3" => None,
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

    pub fn size(&self) -> usize {
        self.name.len()
            + size_of::<u32>()*3 // 3 = size + dummy + flags
            + self.field_size()
    }

    pub fn into_iter(self) -> impl Iterator<Item = Field> {
        self.fields.into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Field> {
        self.fields.iter_mut()
    }

    pub fn clear(&mut self) {
        self.fields.clear();
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn create_field() {
        Field::new(b"NAME", vec![]);
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
        let field = Field::new(b"NAME", b"PCHasCrimeGold\0".to_vec());
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
        let mut field = Field::new(b"NAME", vec![]);
        field.set_zstring(String::from("sWerewolfRefusal")).unwrap();
        assert_eq!(field.data, *b"sWerewolfRefusal\0");
    }

    #[test]
    fn set_string_field() {
        let mut field = Field::new(b"BNAM", vec![]);
        field.set_string(String::from("a_steel_helmet"));
        assert_eq!(field.data, *b"a_steel_helmet");
    }

    #[test]
    fn set_raw_field() {
        let mut field = Field::new(b"ALDT", vec![]);
        field.set(b"\0\0\xa0\x40\x0a\0\0\0\0\0\0\0".to_vec());
        assert_eq!(field.data, *b"\0\0\xa0\x40\x0a\0\0\0\0\0\0\0");
    }

    #[test]
    fn set_numeric_field() {
        let mut field = Field::new(b"XSCL", vec![]);
        field.set_f32(0.75);
        assert_eq!(field.data, *b"\0\0\x40\x3f");
    }

    #[test]
    fn read_record() {
        let data = b"GLOB\x27\0\0\0\0\0\0\0\0\0\0\0NAME\x0a\0\0\0TimeScale\0FNAM\x01\0\0\0fFLTV\x04\0\0\0\0\0\x20\x41";
        let record = Record::read(&mut data.as_ref()).unwrap();
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
        let record = Record::read(&mut data.as_ref()).unwrap();
        assert!(record.is_deleted);
        assert_eq!(record.size(), data.len());
    }

    #[test]
    fn write_record() {
        let mut record = Record::new(b"DIAL");
        record.is_deleted = true;
        record.add_field(Field::new(b"NAME", b"Berel Sala\0".to_vec()));
        record.add_field(Field::new(b"DATA", vec![0; 4]));

        let mut buf = vec![];
        record.write(&mut buf).unwrap();
        assert_eq!(buf, b"DIAL\x2b\0\0\0\0\0\0\0\x20\0\0\0NAME\x0b\0\0\0Berel Sala\0DATA\x04\0\0\0\0\0\0\0DELE\x04\0\0\0\0\0\0\0".to_vec());
    }
}