use crate::tes4::{
    Enchantable, EnchantmentType, FormId, Item, Skill, Tes4Field, Tes4Record, TextureHash,
};
use crate::{Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt, BinWriterExt};
use bitflags::bitflags;
use std::io::Cursor;

bitflags! {
    #[derive(Default)]
    struct BookFlags: u8 {
        const SCROLL = 0x01;
        const CANT_BE_TAKEN = 0x02;
    }
}

#[binrw]
#[derive(Debug, Default)]
pub struct BookData {
    #[br(try_map = |f| BookFlags::from_bits(f).ok_or("Invalid book flags"))]
    #[bw(map = |f| f.bits)]
    flags: BookFlags,
    #[br(try_map = |s: u8| if s == 0xff { Ok(None) } else { Skill::try_from(s).map(|v| Some(v)) })]
    #[bw(map = |s| s.map_or(0xff, |v| v as u8))]
    pub skill: Option<Skill>,
    pub value: u32,
    pub weight: f32,
}

#[derive(Debug, Default)]
pub struct Book {
    editor_id: String,
    name: String,
    pub text: String,
    script: Option<FormId>,
    model: Option<String>,
    bound_radius: Option<f32>,
    texture_hash: Option<TextureHash>,
    icon: Option<String>,
    enchantment_points: Option<u16>,
    enchantment: Option<FormId>,
    pub data: BookData,
}

impl Enchantable for Book {
    fn enchantment(&self) -> Option<FormId> {
        self.enchantment
    }

    fn set_enchantment(&mut self, enchantment: Option<FormId>) {
        self.enchantment = enchantment;
    }

    fn enchantment_points(&self) -> Option<u32> {
        self.enchantment_points.map(|v| v as u32)
    }

    fn set_enchantment_points(&mut self, enchantment_points: Option<u32>) {
        self.enchantment_points = enchantment_points.map(|v| v as u16);
    }

    fn enchantment_type(&self) -> EnchantmentType {
        EnchantmentType::Scroll
    }
}

impl Item for Book {
    fn editor_id(&self) -> &str {
        self.editor_id.as_str()
    }

    fn set_editor_id(&mut self, id: String) {
        self.editor_id = id;
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn value(&self) -> u32 {
        self.data.value
    }

    fn set_value(&mut self, value: u32) {
        self.data.value = value;
    }

    fn weight(&self) -> f32 {
        self.data.weight
    }

    fn set_weight(&mut self, weight: f32) {
        self.data.weight = weight;
    }

    fn script(&self) -> Option<FormId> {
        self.script
    }

    fn set_script(&mut self, script: Option<FormId>) {
        self.script = script;
    }

    fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    fn set_model(&mut self, model: Option<String>) {
        self.model = model;
    }

    fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    fn set_icon(&mut self, icon: Option<String>) {
        self.icon = icon;
    }

    fn bound_radius(&self) -> Option<f32> {
        self.bound_radius
    }

    fn set_bound_radius(&mut self, bound_radius: Option<f32>) {
        self.bound_radius = bound_radius;
    }

    fn texture_hash(&self) -> Option<&TextureHash> {
        self.texture_hash.as_ref()
    }

    fn set_texture_hash(&mut self, texture_hash: Option<TextureHash>) {
        self.texture_hash = texture_hash;
    }
}

impl Form for Book {
    type Field = Tes4Field;
    type Record = Tes4Record;
    const RECORD_TYPE: &'static [u8; 4] = b"BOOK";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Book::assert(record)?;

        let mut book = Book::default();
        for field in record.iter() {
            match field.name() {
                b"DESC" => book.text = String::from(field.get_zstring()?),
                b"ANAM" => book.enchantment_points = Some(field.get_u16()?),
                b"ENAM" => book.enchantment = Some(FormId(field.get_u32()?)),
                b"DATA" => book.data = field.reader().read_le()?,
                _ => book.read_item_field(field)?,
            }
        }

        Ok(book)
    }

    fn write(&self, record: &mut Self::Record) -> Result<(), TesError> {
        Book::assert(record)?;

        self.write_item_fields(record, &[b"EDID", b"FULL"])?;
        record.add_field(Tes4Field::new_zstring(b"DESC", self.text.clone())?);
        self.write_item_fields(record, &[b"SCRI", b"MODL", b"MODB", b"MODT", b"ICON"])?;

        if let Some(enchantment_points) = self.enchantment_points {
            record.add_field(Tes4Field::new_u16(b"ANAM", enchantment_points));
        }
        if let Some(enchantment_id) = self.enchantment {
            record.add_field(Tes4Field::new_u32(b"ENAM", enchantment_id.0));
        }

        let mut buf = vec![];
        let mut cursor = Cursor::new(&mut buf);
        cursor.write_le(&self.data)?;
        record.add_field(Tes4Field::new(b"DATA", buf)?);

        Ok(())
    }
}

impl Book {
    pub fn new(editor_id: String, name: String, text: String) -> Book {
        Book {
            editor_id,
            name,
            text,
            ..Book::default()
        }
    }

    pub fn is_scroll(&self) -> bool {
        self.data.flags.contains(BookFlags::SCROLL)
    }

    pub fn set_is_scroll(&mut self, is_scroll: bool) {
        self.data.flags.set(BookFlags::SCROLL, is_scroll);
    }

    pub fn can_be_taken(&self) -> bool {
        !self.data.flags.contains(BookFlags::CANT_BE_TAKEN)
    }

    pub fn set_can_be_taken(&mut self, can_be_taken: bool) {
        self.data.flags.set(BookFlags::CANT_BE_TAKEN, !can_be_taken);
    }
}
