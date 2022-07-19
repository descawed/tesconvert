use crate::tes3::{Item, Tes3Field, Tes3Record};
use crate::{decode_failed, Field, Form, Record, TesError};
use binrw::BinReaderExt;

#[derive(Debug, Default)]
pub struct MiscItem {
    id: String,
    model: String,
    name: Option<String>,
    weight: f32,
    value: u32,
    unknown: u32,
    script: Option<String>,
    icon: Option<String>,
}

impl Item for MiscItem {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }

    fn model(&self) -> Option<&str> {
        Some(self.model.as_str())
    }

    fn set_model(&mut self, model: Option<String>) {
        self.model = model.unwrap_or_else(|| String::from(""));
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
    }

    fn value(&self) -> u32 {
        self.value
    }

    fn set_value(&mut self, value: u32) {
        self.value = value;
    }

    fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    fn set_script(&mut self, script: Option<String>) {
        self.script = script;
    }

    fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    fn set_icon(&mut self, icon: Option<String>) {
        self.icon = icon;
    }
}

impl Form for MiscItem {
    type Field = Tes3Field;
    type Record = Tes3Record;
    const RECORD_TYPE: &'static [u8; 4] = b"MISC";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        MiscItem::assert(&record)?;

        let mut item = MiscItem::default();
        for field in record.iter() {
            match field.name() {
                b"NAME" => item.id = String::from(field.get_zstring()?),
                b"MODL" => item.model = String::from(field.get_zstring()?),
                b"FNAM" => item.name = Some(String::from(field.get_zstring()?)),
                b"MCDT" => {
                    let mut reader = field.reader();
                    item.weight = reader.read_le()?;
                    item.value = reader.read_le()?;
                    item.unknown = reader.read_le()?;
                }
                b"SCRI" => item.script = Some(String::from(field.get_zstring()?)),
                b"ITEX" => item.icon = Some(String::from(field.get_zstring()?)),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {} in MISC record",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(item)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}
