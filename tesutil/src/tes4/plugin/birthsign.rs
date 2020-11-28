use crate::tes4::{FormId, Tes4Field, Tes4Record};
use crate::{decode_failed, Field, Form, Record, TesError};

#[derive(Debug)]
pub struct Birthsign {
    editor_id: Option<String>,
    name: Option<String>,
    icon: Option<String>,
    description: String,
    spells: Vec<FormId>,
}

impl Birthsign {
    /// Gets the spells associated with this birthsign
    pub fn spells(&self) -> impl Iterator<Item = FormId> + '_ {
        self.spells.iter().copied()
    }
}

impl Form for Birthsign {
    type Field = Tes4Field;
    type Record = Tes4Record;

    fn record_type() -> &'static [u8; 4] {
        b"BSGN"
    }

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Birthsign::assert(&record)?;

        let mut birthsign = Birthsign {
            editor_id: None,
            name: None,
            icon: None,
            description: String::new(),
            spells: vec![],
        };

        for field in record.iter() {
            match field.name() {
                b"EDID" => birthsign.editor_id = Some(String::from(field.get_zstring()?)),
                b"FULL" => birthsign.name = Some(String::from(field.get_zstring()?)),
                b"ICON" => birthsign.icon = Some(String::from(field.get_zstring()?)),
                b"DESC" => birthsign.description = String::from(field.get_zstring()?),
                b"SPLO" => birthsign.spells.push(FormId(field.get_u32()?)),
                _ => {
                    return Err(decode_failed(format!(
                        "Unepxected field {} in BSGN",
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
