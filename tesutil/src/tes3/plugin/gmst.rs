use super::*;
use crate::Form;

/// Game setting variant - string, int, or float
#[derive(Debug)]
pub enum GameSettingValue {
    String(String),
    Int(i32),
    Float(f32),
}

/// Game setting
#[derive(Debug)]
pub struct GameSetting {
    id: String,
    value: GameSettingValue,
}

impl Form for GameSetting {
    type Field = Tes3Field;
    type Record = Tes3Record;

    fn record_type() -> &'static [u8; 4] {
        b"GMST"
    }

    /// Reads a game setting from a raw record
    ///
    /// # Errors
    ///
    /// Fails if the record is not a `b"GMST"` record or if the data is invalid.
    fn read(record: &Tes3Record) -> Result<GameSetting, TesError> {
        GameSetting::assert(record)?;

        let mut setting = GameSetting {
            id: String::new(),
            value: GameSettingValue::String(String::new()),
        };

        for field in record.iter() {
            match field.name() {
                b"NAME" => setting.id = String::from(field.get_string()?),
                b"STRV" => {
                    setting.value = GameSettingValue::String(String::from(field.get_string()?))
                }
                b"FLTV" => setting.value = GameSettingValue::Float(field.get_f32()?),
                b"INTV" => setting.value = GameSettingValue::Int(field.get_i32()?),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(setting)
    }

    fn write(&self, _: &mut Tes3Record) -> Result<(), TesError> {
        unimplemented!()
    }
}

impl GameSetting {
    /// Gets the value as a float if appropriate
    pub fn get_float(&self) -> Option<f32> {
        if let GameSettingValue::Float(value) = self.value {
            Some(value)
        } else {
            None
        }
    }

    /// Gets the value as an integer if appropriate
    pub fn get_int(&self) -> Option<i32> {
        if let GameSettingValue::Int(value) = self.value {
            Some(value)
        } else {
            None
        }
    }

    /// Gets the value as a string if appropriate
    pub fn get_string(&self) -> Option<&str> {
        if let GameSettingValue::String(ref value) = self.value {
            Some(value)
        } else {
            None
        }
    }
}
