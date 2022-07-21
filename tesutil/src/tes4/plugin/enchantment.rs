use crate::tes4::{Magic, SpellEffect, Tes4Field, Tes4Record};
use crate::{decode_failed, Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt, BinWriterExt};
use std::io::Cursor;

#[binrw]
#[derive(Debug, Default, Eq, PartialEq)]
#[repr(u32)]
#[brw(repr = u32)]
pub enum EnchantmentType {
    #[default]
    Scroll,
    Staff,
    Weapon,
    Apparel,
}

#[binrw]
#[derive(Debug, Default)]
pub struct EnchantmentData {
    pub enchantment_type: EnchantmentType,
    pub charge: u32,
    pub cost: u32,
    #[br(map = |v: u32| v & 1 == 0)]
    #[bw(map = |v| if *v { 0u32 } else { 1 })]
    pub is_auto_calc: bool,
}

#[derive(Debug, Default)]
pub struct Enchantment {
    editor_id: String,
    name: Option<String>,
    pub data: EnchantmentData,
    effects: Vec<SpellEffect>,
}

impl Magic for Enchantment {
    fn iter_effects(&self) -> Box<dyn Iterator<Item = &SpellEffect> + '_> {
        Box::new(self.effects.iter())
    }

    fn iter_effects_mut(&mut self) -> Box<dyn Iterator<Item = &mut SpellEffect> + '_> {
        Box::new(self.effects.iter_mut())
    }

    fn add_effect(&mut self, effect: SpellEffect) {
        self.effects.push(effect);
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }
}

impl Form for Enchantment {
    type Field = Tes4Field;
    type Record = Tes4Record;
    const RECORD_TYPE: &'static [u8; 4] = b"ENCH";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Enchantment::assert(&record)?;

        let mut enchantment = Enchantment::default();
        for field in record.iter() {
            match field.name() {
                b"EDID" => enchantment.editor_id = String::from(field.get_zstring()?),
                b"ENIT" => enchantment.data = field.reader().read_le()?,
                _ => enchantment.read_magic_field(field)?,
            }
        }

        Ok(enchantment)
    }

    fn write(&self, record: &mut Self::Record) -> Result<(), TesError> {
        Enchantment::assert(&record)?;

        record.clear();

        record.add_field(Tes4Field::new_zstring(b"EDID", self.editor_id.clone())?);
        if let Some(ref name) = self.name {
            record.add_field(Tes4Field::new_zstring(b"FULL", name.clone())?);
        }

        let mut buf = vec![];
        let mut cursor = Cursor::new(&mut buf);
        cursor.write_le(&self.data)?;
        record.add_field(Tes4Field::new(b"ENIT", buf)?);

        self.write_magic_effects(record)?;

        Ok(())
    }
}

impl Enchantment {
    pub fn new(editor_id: String) -> Enchantment {
        Enchantment {
            editor_id,
            ..Enchantment::default()
        }
    }
}
