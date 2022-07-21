use crate::tes4::{FormId, Item, Tes4Field, Tes4Record};
use crate::{decode_failed_because, Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt};
use enum_map::Enum;
use num_enum::TryFromPrimitive;

#[binrw]
#[derive(Debug, Default, Enum, TryFromPrimitive)]
#[repr(u8)]
#[brw(repr = u8)]
pub enum SoulType {
    #[default]
    None,
    Petty,
    Lesser,
    Common,
    Greater,
    Grand,
}

#[derive(Debug, Default)]
pub struct SoulGem {
    editor_id: String,
    name: String,
    value: u32,
    weight: f32,
    contained_soul: SoulType,
    max_soul: SoulType,
    model: Option<String>,
    bound_radius: Option<f32>,
    texture_hash: Option<Vec<u8>>,
    icon: Option<String>,
    script: Option<FormId>,
}

impl Item for SoulGem {
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
        self.value
    }

    fn set_value(&mut self, value: u32) {
        self.value = value;
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
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

    fn texture_hash(&self) -> Option<&[u8]> {
        self.texture_hash.as_deref()
    }

    fn set_texture_hash(&mut self, texture_hash: Option<Vec<u8>>) {
        self.texture_hash = texture_hash;
    }
}

impl Form for SoulGem {
    type Field = Tes4Field;
    type Record = Tes4Record;
    const RECORD_TYPE: &'static [u8; 4] = b"SLGM";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        SoulGem::assert(&record)?;

        let mut soul_gem = SoulGem::default();
        for field in record.iter() {
            match field.name() {
                b"DATA" => {
                    let mut reader = field.reader();
                    soul_gem.value = reader.read_le()?;
                    soul_gem.weight = reader.read_le()?;
                }
                b"SOUL" => {
                    soul_gem.contained_soul = SoulType::try_from(field.get_u8()?).map_err(|e| {
                        decode_failed_because("Invalid contained soul type in soul game", e)
                    })?
                }
                b"SLCP" => {
                    soul_gem.max_soul = SoulType::try_from(field.get_u8()?).map_err(|e| {
                        decode_failed_because("Invalid max soul type in soul game", e)
                    })?
                }
                _ => soul_gem.read_item_field(&field)?,
            }
        }

        Ok(soul_gem)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}
