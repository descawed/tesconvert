use crate::tes3::{Item, Magic, MagicEffectType, SpellEffect, Tes3Field, Tes3Record};
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

impl Magic for Potion {
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

impl Item for Potion {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }

    fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    fn set_model(&mut self, model: Option<String>) {
        self.model = model;
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    fn weight(&self) -> f32 {
        self.alchemy_data.weight
    }

    fn set_weight(&mut self, weight: f32) {
        self.alchemy_data.weight = weight;
    }

    fn value(&self) -> u32 {
        self.alchemy_data.value
    }

    fn set_value(&mut self, value: u32) {
        self.alchemy_data.value = value;
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
