use crate::tes4::{Enchantable, EnchantmentType, FormId, Item, Tes4Field, Tes4Record};
use crate::{Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt, BinWriterExt};
use std::io::Cursor;

#[binrw]
#[derive(Debug, Default)]
pub struct AmmoData {
    pub speed: f32,
    #[br(map = |v: u32| v & 1 != 0)]
    #[bw(map = |v| if *v { 1u32 } else { 0 })]
    pub ignores_normal_weapon_resistance: bool,
    pub value: u32,
    pub weight: f32,
    pub damage: u16,
}

#[derive(Debug, Default)]
pub struct Ammo {
    editor_id: String,
    name: String,
    model: Option<String>,
    bound_radius: Option<f32>,
    texture_hash: Option<Vec<u8>>,
    icon: Option<String>,
    enchantment: Option<FormId>,
    enchantment_points: Option<u16>,
    pub data: AmmoData,
}

impl Enchantable for Ammo {
    fn enchantment(&self) -> Option<FormId> {
        self.enchantment
    }

    fn set_enchantment(&mut self, enchantment: Option<FormId>) {
        self.enchantment = enchantment;
        if self.enchantment_points.is_none() {
            self.enchantment_points = Some(0);
        }
    }

    fn enchantment_points(&self) -> Option<u32> {
        self.enchantment_points.map(|v| v as u32)
    }

    fn set_enchantment_points(&mut self, enchantment_points: Option<u32>) {
        self.enchantment_points = enchantment_points.map(|v| v as u16);
    }

    fn enchantment_type(&self) -> EnchantmentType {
        EnchantmentType::Weapon
    }
}

impl Item for Ammo {
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
        None
    }

    fn set_script(&mut self, script: Option<FormId>) {
        if script.is_some() {
            panic!("AMMO is not scriptable");
        }
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

impl Form for Ammo {
    type Field = Tes4Field;
    type Record = Tes4Record;
    const RECORD_TYPE: &'static [u8; 4] = b"AMMO";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Ammo::assert(record)?;

        let mut ammo = Ammo::default();
        for field in record.iter() {
            match field.name() {
                b"ENAM" => ammo.enchantment = Some(FormId(field.get_u32()?)),
                b"ANAM" => ammo.enchantment_points = Some(field.get_u16()?),
                b"DATA" => ammo.data = field.reader().read_le()?,
                _ => ammo.read_item_field(field)?,
            }
        }

        Ok(ammo)
    }

    fn write(&self, record: &mut Self::Record) -> Result<(), TesError> {
        Ammo::assert(record)?;

        self.write_item_fields(
            record,
            &[b"EDID", b"FULL", b"MODL", b"MODB", b"MODT", b"ICON"],
        )?;

        if let Some(enchantment_id) = self.enchantment {
            record.add_field(Tes4Field::new_u32(b"ENAM", enchantment_id.0));
        }
        if let Some(enchantment_points) = self.enchantment_points {
            record.add_field(Tes4Field::new_u16(b"ANAM", enchantment_points));
        }

        let mut buf = vec![];
        let mut cursor = Cursor::new(&mut buf);
        cursor.write_le(&self.data)?;
        record.add_field(Tes4Field::new(b"DATA", buf)?);

        Ok(())
    }
}

impl Ammo {
    pub fn new(editor_id: String, name: String) -> Ammo {
        Ammo {
            editor_id,
            name,
            ..Ammo::default()
        }
    }
}
