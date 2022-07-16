use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom};

use super::Plugin;
use crate::tes4::FormId;
use crate::{decode_failed, read_bstring, write_bstring, TesError};

use binrw::{BinReaderExt, BinWriterExt};

const FORM_MAP_VERSION: u32 = 0;
const ACTIVE_SPELL_VERSION: u32 = 0;

#[derive(Debug)]
pub struct ObConvert {
    form_map: HashMap<String, FormId>,
    active_spells: HashMap<FormId, f32>,
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
        }
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

    pub fn write(&self, plugin: &mut Plugin) -> Result<(), TesError> {
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
                _ => (), // ignore
            }
        }

        Ok(())
    }
}
