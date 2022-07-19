use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use super::Plugin;
use crate::tes3::{InventoryItem, Script};
use crate::tes4::FormId;
use crate::{decode_failed, read_bstring, write_bstring, TesError};

use crate::tes4::cosave::Chunk;
use binrw::{BinReaderExt, BinWriterExt};

const FORM_MAP_VERSION: u32 = 0;
const ACTIVE_SPELL_VERSION: u32 = 0;
const MW_INVENTORY_VERSION: u32 = 0;

#[derive(Debug)]
pub struct ObConvert {
    form_map: HashMap<String, FormId>,
    active_spells: HashMap<FormId, f32>,
    // inconvertible inventory items from other games
    morrowind_inventory: Vec<InventoryItem>,
}

impl Default for ObConvert {
    fn default() -> Self {
        ObConvert::new()
    }
}

impl ObConvert {
    pub fn new() -> ObConvert {
        ObConvert {
            form_map: HashMap::new(),
            active_spells: HashMap::new(),
            morrowind_inventory: vec![],
        }
    }

    fn read_morrowind_item<T: Read + Seek>(mut f: T) -> Result<InventoryItem, TesError> {
        let id = read_bstring(&mut f)?;
        let count: u32 = f.read_le()?;
        let is_equipped = f.read_le::<u8>()? != 0;
        let soul = read_bstring(&mut f)?;

        let has_charge = f.read_le::<u8>()? != 0;
        let charge = if has_charge {
            Some(f.read_le::<f32>()?)
        } else {
            None
        };

        let has_durability = f.read_le::<u8>()? != 0;
        let durability = if has_durability {
            Some(f.read_le::<u32>()?)
        } else {
            None
        };

        let script_id = read_bstring(&mut f)?;
        let script = if script_id.len() > 0 {
            let num_shorts: u32 = f.read_le()?;
            let mut shorts = vec![0i16; num_shorts as usize];
            for short in &mut shorts {
                *short = f.read_le()?;
            }

            let num_longs: u32 = f.read_le()?;
            let mut longs = vec![0i32; num_longs as usize];
            for long in &mut longs {
                *long = f.read_le()?;
            }

            let num_floats: u32 = f.read_le()?;
            let mut floats = vec![0f32; num_floats as usize];
            for float in &mut floats {
                *float = f.read_le()?;
            }

            Some(Script {
                name: script_id,
                shorts,
                longs,
                floats,
            })
        } else {
            None
        };

        Ok(InventoryItem {
            id,
            count,
            is_equipped,
            soul: if soul.len() > 0 { Some(soul) } else { None },
            enchantment_charge: charge,
            remaining_durability: durability,
            script,
        })
    }

    fn write_morrowind_item<T: Write + Seek>(
        mut f: T,
        item: &InventoryItem,
    ) -> Result<(), TesError> {
        write_bstring(&mut f, &item.id)?;
        f.write_le(&item.count)?;
        f.write_le(&(item.is_equipped as u8))?;
        write_bstring(&mut f, (&item.soul).as_ref().map_or("", |s| s.as_str()))?;

        if let Some(charge) = item.enchantment_charge {
            f.write_le(&1u8)?;
            f.write_le(&charge)?;
        } else {
            f.write_le(&0u8)?;
        }

        if let Some(durability) = item.remaining_durability {
            f.write_le(&1u8)?;
            f.write_le(&durability)?;
        } else {
            f.write_le(&0u8)?;
        }

        if let Some(ref script) = item.script {
            write_bstring(&mut f, &script.name)?;

            f.write_le(&(script.shorts.len() as u32))?;
            for short in &script.shorts {
                f.write_le(short)?;
            }

            f.write_le(&(script.longs.len() as u32))?;
            for long in &script.longs {
                f.write_le(long)?;
            }

            f.write_le(&(script.floats.len() as u32))?;
            for float in &script.floats {
                f.write_le(float)?;
            }
        } else {
            write_bstring(&mut f, "")?;
        }

        Ok(())
    }

    pub fn read(plugin: &Plugin) -> Result<ObConvert, TesError> {
        let mut convert = ObConvert::new();

        for chunk in plugin.iter() {
            match &chunk.tag {
                b"FMAP" => {
                    if chunk.version != FORM_MAP_VERSION {
                        return Err(decode_failed(format!(
                            "Unexpected form map version {}",
                            chunk.version
                        )));
                    }

                    let data_len = chunk.data.len() as u64;
                    let mut reader = Cursor::new(&chunk.data);
                    while reader.seek(SeekFrom::Current(0))? < data_len {
                        let mw_id = read_bstring(&mut reader)?;
                        let form_id = FormId(reader.read_le()?);
                        convert.form_map.insert(mw_id, form_id);
                    }
                }
                b"ASPL" => {
                    if chunk.version != ACTIVE_SPELL_VERSION {
                        return Err(decode_failed(format!(
                            "Unexpected active spell version {}",
                            chunk.version
                        )));
                    }

                    let data_len = chunk.data.len() as u64;
                    let mut reader = Cursor::new(&chunk.data);
                    while reader.seek(SeekFrom::Current(0))? < data_len {
                        let form_id = FormId(reader.read_le()?);
                        let seconds_active = reader.read_le()?;
                        convert.active_spells.insert(form_id, seconds_active);
                    }
                }
                b"MWIN" => {
                    if chunk.version != MW_INVENTORY_VERSION {
                        return Err(decode_failed(format!(
                            "Unexpected Morrowind inventory version {}",
                            chunk.version
                        )));
                    }

                    let mut reader = Cursor::new(&chunk.data);
                    let inventory_count = reader.read_le::<u32>()? as usize;
                    for _ in 0..inventory_count {
                        convert
                            .morrowind_inventory
                            .push(Self::read_morrowind_item(&mut reader)?);
                    }
                }
                _ => return Err(decode_failed("Unexpected chunk")),
            }
        }

        Ok(convert)
    }

    pub fn set_active_spells(&mut self, active_spells: HashMap<FormId, f32>) {
        self.active_spells = active_spells;
    }

    pub fn add_active_spell(&mut self, form_id: FormId, seconds_active: f32) {
        self.active_spells.insert(form_id, seconds_active);
    }

    pub fn clear_active_spells(&mut self) {
        self.active_spells.clear();
    }

    pub fn set_morrowind_inventory(&mut self, inventory: Vec<InventoryItem>) {
        self.morrowind_inventory = inventory;
    }

    pub fn add_morrowind_item(&mut self, item: InventoryItem) {
        self.morrowind_inventory.push(item);
    }

    pub fn clear_morrowind_inventory(&mut self) {
        self.morrowind_inventory.clear();
    }

    pub fn write(&self, plugin: &mut Plugin) -> Result<(), TesError> {
        let missing_tags = [b"FMAP", b"ASPL", b"MWIN"]
            .iter()
            .filter(|t| !plugin.iter().any(|c| c.tag == ***t))
            .collect::<Vec<&&[u8; 4]>>();
        for tag in missing_tags {
            plugin.add_chunk(Chunk::new(**tag));
        }

        for chunk in plugin.iter_mut() {
            match &chunk.tag {
                b"FMAP" => {
                    let mut data = vec![];
                    let mut writer = Cursor::new(&mut data);
                    for (mw_id, form_id) in &self.form_map {
                        write_bstring(&mut writer, mw_id)?;
                        writer.write_le(&form_id.0)?;
                    }
                    chunk.set_data(data);
                }
                b"ASPL" => {
                    let mut data = vec![];
                    let mut writer = Cursor::new(&mut data);
                    for (form_id, seconds_active) in &self.active_spells {
                        writer.write_le(&form_id.0)?;
                        writer.write_le(&seconds_active)?;
                    }
                    chunk.set_data(data);
                }
                b"MWIN" => {
                    let mut data = vec![];
                    let mut writer = Cursor::new(&mut data);
                    writer.write_le(&(self.morrowind_inventory.len() as u32))?;
                    for item in &self.morrowind_inventory {
                        Self::write_morrowind_item(&mut writer, item)?;
                    }
                    chunk.set_data(data);
                }
                _ => (), // ignore
            }
        }

        Ok(())
    }
}
