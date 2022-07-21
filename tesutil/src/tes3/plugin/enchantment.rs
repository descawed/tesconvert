use crate::tes3::{Magic, SpellEffect, Tes3Field, Tes3Record};

use crate::{decode_failed, Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt};

#[binrw]
#[derive(Debug, Default, Eq, PartialEq)]
#[repr(u32)]
#[brw(repr = u32)]
pub enum EnchantmentType {
    #[default]
    CastOnce,
    CastWhenStrikes,
    CastWhenUsed,
    ConstantEffect,
}

#[binrw]
#[derive(Debug, Default)]
pub struct EnchantmentData {
    pub enchantment_type: EnchantmentType,
    pub cost: u32,
    pub charge: u32,
    #[br(map = |v: u32| v & 1 != 0)]
    #[bw(map = |v| if *v { 1u32 } else { 0 })]
    pub is_auto_calc: bool,
}

#[derive(Debug, Default)]
pub struct Enchantment {
    pub id: String,
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
}

impl Form for Enchantment {
    type Field = Tes3Field;
    type Record = Tes3Record;
    const RECORD_TYPE: &'static [u8; 4] = b"ENCH";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Enchantment::assert(&record)?;

        let mut enchantment = Enchantment::default();
        for field in record.iter() {
            match field.name() {
                b"NAME" => enchantment.id = String::from(field.get_zstring()?),
                b"ENDT" => enchantment.data = field.reader().read_le()?,
                b"ENAM" => enchantment.add_effect(field.reader().read_le()?),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected {} field in ENCH record",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(enchantment)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}
