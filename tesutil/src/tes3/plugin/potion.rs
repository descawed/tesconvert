use crate::tes3::{MagicEffectType, SpellEffect, Tes3Field, Tes3Record};
use crate::{decode_failed, Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt};

#[binrw]
#[derive(Debug, Default)]
pub struct AlchemyData {
    pub weight: f32,
    pub value: u32,
    #[br(map = |v: u32| v != 0)]
    #[bw(map = |v| if *v { 1u32 } else { 0u32 })]
    pub is_auto_calc: bool,
}

/// A Morrowind potion
#[derive(Debug, Default)]
pub struct Potion {
    pub id: String,
    pub model: Option<String>,
    pub icon: Option<String>,
    pub script: Option<String>,
    pub name: Option<String>,
    pub alchemy_data: AlchemyData,
    pub effects: Vec<SpellEffect>,
}

impl Form for Potion {
    type Field = Tes3Field;
    type Record = Tes3Record;

    const RECORD_TYPE: &'static [u8; 4] = b"ALCH";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Potion::assert(&record)?;

        let mut potion = Potion::default();
        for field in record.iter() {
            match field.name() {
                b"NAME" => potion.id = String::from(field.get_zstring()?),
                b"MODL" => potion.model = Some(String::from(field.get_zstring()?)),
                b"TEXT" => potion.icon = Some(String::from(field.get_zstring()?)),
                b"SCRI" => potion.script = Some(String::from(field.get_zstring()?)),
                b"FNAM" => potion.name = Some(String::from(field.get_zstring()?)),
                b"ALDT" => potion.alchemy_data = field.reader().read_le()?,
                b"ENAM" => potion.effects.push(field.reader().read_le()?),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {} in ALCH record",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(potion)
    }

    fn write(&self, record: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}
