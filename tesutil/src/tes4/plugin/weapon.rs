use crate::tes4::{Enchantable, EnchantmentType, FormId, Item, Tes4Field, Tes4Record, TextureHash};
use crate::{Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt, BinWriterExt};
use std::io::Cursor;

#[binrw]
#[derive(Debug, Default)]
#[repr(u32)]
#[brw(repr = u32)]
pub enum WeaponType {
    #[default]
    BladeOneHand,
    BladeTwoHand,
    BluntOneHand,
    BluntTwoHand,
    Staff,
    Bow,
}

#[binrw]
#[derive(Debug, Default)]
pub struct WeaponData {
    pub weapon_type: WeaponType,
    pub speed: f32,
    pub reach: f32,
    #[br(map = |v: u32| v & 1 != 0)]
    #[bw(map = |v| if *v { 1u32 } else { 0 })]
    pub ignores_normal_weapon_resistance: bool,
    pub value: u32,
    pub health: u32,
    pub weight: f32,
    pub damage: u16,
}

#[derive(Debug, Default)]
pub struct Weapon {
    editor_id: String,
    name: String,
    script: Option<FormId>,
    model: Option<String>,
    bound_radius: Option<f32>,
    texture_hash: Option<TextureHash>,
    icon: Option<String>,
    enchantment_points: Option<u32>,
    enchantment: Option<FormId>,
    pub data: WeaponData,
}

impl Item for Weapon {
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

impl Form for Weapon {
    type Field = Tes4Field;
    type Record = Tes4Record;
    const RECORD_TYPE: &'static [u8; 4] = b"WEAP";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Weapon::assert(&record)?;

        let mut weapon = Weapon::default();
        for field in record.iter() {
            match field.name() {
                b"ANAM" => weapon.enchantment_points = Some(field.get_u32()?),
                b"ENAM" => weapon.enchantment = Some(FormId(field.get_u32()?)),
                b"DATA" => weapon.data = field.reader().read_le()?,
                _ => weapon.read_item_field(field)?,
            }
        }

        Ok(weapon)
    }

    fn write(&self, record: &mut Self::Record) -> Result<(), TesError> {
        Weapon::assert(&record)?;

        record.clear();

        self.write_item_fields(
            record,
            &[
                b"EDID", b"FULL", b"SCRI", b"MODL", b"MODB", b"MODT", b"ICON",
            ],
        )?;
        if let Some(enchantment_points) = self.enchantment_points {
            record.add_field(Tes4Field::new_u32(b"ANAM", enchantment_points));
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

impl Enchantable for Weapon {
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
        self.enchantment_points
    }

    fn set_enchantment_points(&mut self, enchantment_points: Option<u32>) {
        self.enchantment_points = enchantment_points;
    }

    fn enchantment_type(&self) -> EnchantmentType {
        match self.data.weapon_type {
            WeaponType::Staff => EnchantmentType::Staff,
            _ => EnchantmentType::Weapon,
        }
    }
}

impl Weapon {
    pub fn new(editor_id: String, name: String) -> Weapon {
        Weapon {
            editor_id,
            name,
            ..Weapon::default()
        }
    }
}
