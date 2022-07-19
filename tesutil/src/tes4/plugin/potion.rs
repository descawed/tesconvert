use crate::tes4::{FormId, Item, SpellEffect, Tes4Field, Tes4Record, TextureHash, MAGIC_EFFECTS};
use std::io::{Cursor, Read, Write};

use crate::{decode_failed, Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt, BinWriterExt};

/// An Oblivion potion
#[derive(Debug)]
pub struct Potion {
    editor_id: String,
    name: String,
    model: Option<String>,
    bound_radius: Option<f32>,
    texture_hash: Option<TextureHash>,
    icon: Option<String>,
    script: Option<FormId>,
    pub weight: f32,
    pub value: u32,
    pub is_auto_calc: bool,
    pub is_food_item: bool,
    unknown: [u8; 3],
    effects: Vec<SpellEffect>,
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

    fn texture_hash(&self) -> Option<&TextureHash> {
        self.texture_hash.as_ref()
    }

    fn set_texture_hash(&mut self, texture_hash: Option<TextureHash>) {
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

    /// Adds an effect to this potion
    pub fn add_effect(&mut self, effect: SpellEffect) {
        self.effects.push(effect);
    }

    /// Iterates over this potion's effects
    pub fn effects(&self) -> impl Iterator<Item = &SpellEffect> {
        self.effects.iter()
    }

    /// Set the model and texture info appropriately for a user-created potion
    pub fn use_potion_graphics(&mut self) {
        // values copied from PotionCureDisease in Oblivion.esm
        self.model = Some(String::from(r"Clutter\Potions\Potion01.NIF"));
        self.bound_radius = Some(8.10082);
        self.texture_hash = Some(TextureHash {
            file_hash_pc: 0x4A92E9687008B0B1,
            file_hash_console: 0x4A92E96D70083031,
            folder_hash: 0xC28E78E874186E73,
        });
        self.icon = Some(String::from(r"Clutter\Potions\IconPotion01.dds"));
    }

    /// Set the model and texture info appropriately for a user-created poison
    pub fn use_poison_graphics(&mut self) {
        // values copied from PotionBurden in Oblivion.esm
        self.model = Some(String::from(r"Clutter\Potions\PotionPoison.NIF"));
        self.bound_radius = Some(8.08878);
        self.texture_hash = Some(TextureHash {
            file_hash_pc: 0x70B7D6B2700EEFEE,
            file_hash_console: 0x70B7D6B7700E6F6E,
            folder_hash: 0xC28E78E874186E73,
        });
        self.icon = Some(String::from(r"Clutter\Potions\IconPotionPoison01.dds"));
    }

    /// Is this potion a poison?
    pub fn is_poison(&self) -> bool {
        self.effects.iter().all(|e| {
            MAGIC_EFFECTS[e.effect].is_hostile()
                || e.script_effect.as_ref().map_or(false, |s| s.is_hostile)
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
                b"EFID" => {
                    let mut effect = SpellEffect::default();
                    effect.load_from_field(&field)?;
                    potion.effects.push(effect);
                }
                b"EFIT" | b"SCIT" => {
                    if let Some(last_effect) = potion.effects.iter_mut().last() {
                        last_effect.load_from_field(&field)?;
                    } else {
                        return Err(decode_failed(format!(
                            "Orphaned {} field in ALCH record",
                            field.name_as_str()
                        )));
                    }
                }
                b"FULL" => {
                    if let Some(last_effect) = potion.effects.iter_mut().last() {
                        last_effect.load_from_field(&field)?;
                    } else {
                        potion.name = String::from(field.get_zstring()?);
                    }
                }
                _ => potion.read_item_field(field)?,
            }
        }

        Ok(potion)
    }

    fn write(&self, mut record: &mut Self::Record) -> Result<(), TesError> {
        Potion::assert(record)?;

        record.clear();

        self.write_scalar_fields(&mut record, &[b"EDID"])?;
        record.add_field(Tes4Field::new_zstring(b"FULL", self.name.clone())?);
        self.write_scalar_fields(&mut record, &[b"MODL", b"MODB", b"MODT", b"ICON", b"SCRI"])?;
        record.add_field(Tes4Field::new_f32(b"DATA", self.weight));

        let mut buf = vec![];
        let mut cursor = Cursor::new(&mut buf);
        cursor.write_le(&self.value)?;
        let flags =
            if self.is_auto_calc { 1u8 } else { 0 } | if self.is_food_item { 2u8 } else { 0 };
        cursor.write_le(&flags)?;
        cursor.write_all(&self.unknown)?;
        record.add_field(Tes4Field::new(b"ENIT", buf)?);

        for effect in &self.effects {
            for field in effect.to_fields()? {
                record.add_field(field);
            }
        }

        Ok(())
    }
}
