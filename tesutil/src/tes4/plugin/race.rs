use crate::tes4::{FormId, Tes4Field, Tes4Record};
use crate::{Field, Form, Record, TesError};

#[derive(Debug)]
pub struct Race {
    editor_id: String,
    name: Option<String>,
    description: String,
    spells: Vec<FormId>,
    // FIXME: fill in the rest
}

impl Race {
    /// Gets the spells associated with this race
    pub fn spells(&self) -> impl Iterator<Item = FormId> + '_ {
        self.spells.iter().copied()
    }
}

impl Form for Race {
    type Field = Tes4Field;
    type Record = Tes4Record;

    fn record_type() -> &'static [u8; 4] {
        b"RACE"
    }

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Race::assert(&record)?;

        let mut race = Race {
            editor_id: String::new(),
            name: None,
            description: String::new(),
            spells: vec![],
        };

        for field in record.iter() {
            match field.name() {
                b"EDID" => race.editor_id = String::from(field.get_zstring()?),
                b"FULL" => race.name = Some(String::from(field.get_zstring()?)),
                b"DESC" => race.description = String::from(field.get_zstring()?),
                b"SPLO" => race.spells.push(FormId(field.get_u32()?)),
                _ => (), // FIXME: implement remaining fields
            }
        }

        Ok(race)
    }

    fn write(&self, _record: &mut Self::Record) -> Result<(), TesError> {
        unimplemented!()
    }
}
