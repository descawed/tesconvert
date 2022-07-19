use crate::tes3::{Tes3Field, Tes3Record};
use crate::{decode_failed, read_string, Field, Form, Record, TesError};

const ID_LENGTH: usize = 32;

#[derive(Debug)]
pub struct Birthsign {
    id: String,
    name: Option<String>,
    spells: Vec<String>,
    texture: Option<String>,
    description: Option<String>,
}

impl Birthsign {
    /// Gets the spells associated with this birthsign
    pub fn spells(&self) -> impl Iterator<Item = &str> + '_ {
        self.spells.iter().map(String::as_str)
    }
}

impl Form for Birthsign {
    type Field = Tes3Field;
    type Record = Tes3Record;

    const RECORD_TYPE: &'static [u8; 4] = b"BSGN";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Birthsign::assert(record)?;

        let mut birthsign = Birthsign {
            id: String::new(),
            name: None,
            spells: vec![],
            texture: None,
            description: None,
        };

        for field in record.iter() {
            match field.name() {
                b"NAME" => birthsign.id = String::from(field.get_zstring()?),
                b"FNAM" => birthsign.name = Some(String::from(field.get_zstring()?)),
                b"NPCS" => birthsign
                    .spells
                    .push(read_string::<ID_LENGTH, _>(&mut field.get())?),
                b"TNAM" => birthsign.texture = Some(String::from(field.get_zstring()?)),
                b"DESC" => birthsign.description = Some(String::from(field.get_zstring()?)),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {} in BSGN",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(birthsign)
    }

    fn write(&self, _record: &mut Self::Record) -> Result<(), TesError> {
        unimplemented!()
    }
}
