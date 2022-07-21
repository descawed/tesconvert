use crate::tes4::{FormId, Item, Magic, SpellEffect, Tes4Field, Tes4Record, MAGIC_EFFECTS};
use std::io::{Cursor, Read, Write};

use crate::{Field, Form, Record, TesError};
use binrw::{BinReaderExt, BinWriterExt};

/// An Oblivion potion
#[derive(Debug)]
pub struct Potion {
    editor_id: String,
    name: String,
    model: Option<String>,
    bound_radius: Option<f32>,
    texture_hash: Option<Vec<u8>>,
    icon: Option<String>,
    script: Option<FormId>,
    pub weight: f32,
    pub value: u32,
    pub is_auto_calc: bool,
    pub is_food_item: bool,
    unknown: [u8; 3],
    effects: Vec<SpellEffect>,
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

    fn name(&self) -> Option<&str> {
        Some(self.name.as_str())
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name.unwrap_or_else(String::new);
    }
}

impl Item for Potion {
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
        self.value = value
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

impl Potion {
    pub fn new(editor_id: String, name: String) -> Self {
        Potion {
            editor_id,
            name,
            ..Potion::default()
        }
    }

    /// Set the model and texture info appropriately for a user-created potion
    pub fn use_potion_graphics(&mut self) {
        // values copied from PotionCureDisease in Oblivion.esm
        self.model = Some(String::from(r"Clutter\Potions\Potion01.NIF"));
        self.bound_radius = Some(8.10082);
        self.texture_hash = Some(vec![
            0xB1, 0xB0, 0x08, 0x70, 0x68, 0xE9, 0x92, 0x4A, 0x31, 0x30, 0x08, 0x70, 0x6D, 0xE9,
            0x92, 0x4A, 0x73, 0x6E, 0x18, 0x74, 0xE8, 0x78, 0x8E, 0xC2,
        ]);
        self.icon = Some(String::from(r"Clutter\Potions\IconPotion01.dds"));
    }

    /// Set the model and texture info appropriately for a user-created poison
    pub fn use_poison_graphics(&mut self) {
        // values copied from PotionBurden in Oblivion.esm
        self.model = Some(String::from(r"Clutter\Potions\PotionPoison.NIF"));
        self.bound_radius = Some(8.08878);
        self.texture_hash = Some(vec![
            0xEE, 0xEF, 0x0E, 0x70, 0xB2, 0xD6, 0xB7, 0x70, 0x6E, 0x6F, 0x0E, 0x70, 0xB7, 0xD6,
            0xB7, 0x70, 0x73, 0x6E, 0x18, 0x74, 0xE8, 0x78, 0x8E, 0xC2,
        ]);
        self.icon = Some(String::from(r"Clutter\Potions\IconPotionPoison01.dds"));
    }

    /// Is this potion a poison?
    pub fn is_poison(&self) -> bool {
        self.effects.iter().all(|e| {
            MAGIC_EFFECTS[e.effect_type()].is_hostile()
                || e.script_effect().map_or(false, |s| s.is_hostile)
        })
    }

    /// Automatically set the model and texture info for a user-created potion of this type
    pub fn use_auto_graphics(&mut self) {
        if self.is_poison() {
            self.use_poison_graphics();
        } else {
            self.use_potion_graphics();
        }
    }
}

impl Default for Potion {
    fn default() -> Self {
        Potion {
            editor_id: String::new(),
            name: String::new(),
            model: None,
            bound_radius: None,
            texture_hash: None,
            icon: None,
            script: None,
            weight: 0.,
            value: 0,
            is_auto_calc: true,
            is_food_item: false,
            unknown: [0; 3],
            effects: vec![],
        }
    }
}

impl Form for Potion {
    type Field = Tes4Field;
    type Record = Tes4Record;

    const RECORD_TYPE: &'static [u8; 4] = b"ALCH";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Potion::assert(&record)?;

        let mut potion = Potion::default();
        for field in record.iter() {
            match field.name() {
                b"DATA" => potion.weight = field.get_f32()?,
                b"ENIT" => {
                    let mut reader = field.reader();
                    potion.value = reader.read_le()?;
                    let flags: u8 = reader.read_le()?;
                    potion.is_auto_calc = flags & 1 != 0;
                    potion.is_food_item = flags & 2 != 0;
                    reader.read_exact(&mut potion.unknown)?;
                }
                b"EFID" | b"EFIT" | b"SCIT" | b"FULL" => potion.read_magic_field(field)?,
                _ => potion.read_item_field(field)?,
            }
        }

        Ok(potion)
    }

    fn write(&self, mut record: &mut Self::Record) -> Result<(), TesError> {
        Potion::assert(record)?;

        record.clear();

        self.write_item_fields(&mut record, &[b"EDID"])?;
        record.add_field(Tes4Field::new_zstring(b"FULL", self.name.clone())?);
        self.write_item_fields(&mut record, &[b"MODL", b"MODB", b"MODT", b"ICON", b"SCRI"])?;
        record.add_field(Tes4Field::new_f32(b"DATA", self.weight));

        let mut buf = vec![];
        let mut cursor = Cursor::new(&mut buf);
        cursor.write_le(&self.value)?;
        let flags =
            if self.is_auto_calc { 1u8 } else { 0 } | if self.is_food_item { 2u8 } else { 0 };
        cursor.write_le(&flags)?;
        cursor.write_all(&self.unknown)?;
        record.add_field(Tes4Field::new(b"ENIT", buf)?);

        self.write_magic_effects(&mut record)?;

        Ok(())
    }
}
