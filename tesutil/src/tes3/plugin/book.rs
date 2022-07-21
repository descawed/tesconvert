use crate::tes3::{Enchantable, Item, Skill, Tes3Field, Tes3Record};
use crate::{decode_failed, Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt};

#[binrw]
#[derive(Debug, Default)]
pub struct BookData {
    pub weight: f32,
    pub value: u32,
    #[br(map = |v: u32| v & 1 != 0)]
    #[bw(map = |v| if *v { 1u32 } else { 0 })]
    pub is_scroll: bool,
    #[br(try_map = |s: u8| if s == 0xff { Ok(None) } else { Skill::try_from(s).map(|v| Some(v)) })]
    #[bw(map = |s| s.map_or(0xff, |v| v as u8))]
    pub skill: Option<Skill>,
    pub enchantment_points: u32,
}

#[derive(Debug, Default)]
pub struct Book {
    id: String,
    model: String,
    name: Option<String>,
    pub data: BookData,
    script: Option<String>,
    icon: Option<String>,
    pub text: String,
    enchantment: Option<String>,
}

impl Enchantable for Book {
    fn enchantment(&self) -> Option<&str> {
        self.enchantment.as_deref()
    }

    fn set_enchantment(&mut self, enchantment: Option<String>) {
        self.enchantment = enchantment;
    }

    fn enchantment_points(&self) -> u32 {
        self.data.enchantment_points
    }

    fn set_enchantment_points(&mut self, enchantment_points: u32) {
        self.data.enchantment_points = enchantment_points;
    }
}

impl Item for Book {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }

    fn model(&self) -> Option<&str> {
        Some(self.model.as_str())
    }

    fn set_model(&mut self, model: Option<String>) {
        self.model = model.unwrap_or_else(String::new);
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    fn weight(&self) -> f32 {
        self.data.weight
    }

    fn set_weight(&mut self, weight: f32) {
        self.data.weight = weight;
    }

    fn value(&self) -> u32 {
        self.data.value
    }

    fn set_value(&mut self, value: u32) {
        self.data.value = value;
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

impl Form for Book {
    type Field = Tes3Field;
    type Record = Tes3Record;
    const RECORD_TYPE: &'static [u8; 4] = b"BOOK";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Book::assert(record)?;

        let mut book = Book::default();
        for field in record.iter() {
            match field.name() {
                b"NAME" => book.id = String::from(field.get_zstring()?),
                b"MODL" => book.model = String::from(field.get_zstring()?),
                b"FNAM" => book.name = Some(String::from(field.get_zstring()?)),
                b"BKDT" => book.data = field.reader().read_le()?,
                b"SCRI" => book.script = Some(String::from(field.get_zstring()?)),
                b"ITEX" => book.icon = Some(String::from(field.get_zstring()?)),
                b"TEXT" => book.text = String::from(field.get_string()?),
                b"ENAM" => book.enchantment = Some(String::from(field.get_zstring()?)),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected {} field in BOOK record",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(book)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}
