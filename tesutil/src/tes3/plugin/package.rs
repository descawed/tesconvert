use super::field::Tes3Field;
use crate::{decode_failed, read_string, Field, TesError};
use std::io::Read;

use binrw::BinReaderExt;

/// Maximum length of certain strings in an AI package
pub const PACKAGE_STRING_LENGTH: usize = 32;

/// NPC AI packages
#[derive(Debug)]
pub enum Package {
    Activate(String),
    Escort {
        x: f32,
        y: f32,
        z: f32,
        duration: u16,
        id: String,
        cell: Option<String>,
    },
    Follow {
        x: f32,
        y: f32,
        z: f32,
        duration: u16,
        id: String,
        cell: Option<String>,
    },
    Travel {
        x: f32,
        y: f32,
        z: f32,
    },
    Wander {
        distance: u16,
        duration: u16,
        time_of_day: u8,
        idles: [u8; 8],
    },
}

impl TryFrom<Option<Package>> for Package {
    type Error = TesError;

    fn try_from(value: Option<Package>) -> Result<Self, Self::Error> {
        value.ok_or_else(|| decode_failed("No package value"))
    }
}

impl<'a> TryFrom<Option<&'a Package>> for &'a Package {
    type Error = TesError;

    fn try_from(value: Option<&'a Package>) -> Result<Self, Self::Error> {
        value.ok_or_else(|| decode_failed("No package value"))
    }
}

impl<'a> TryFrom<Option<&'a mut Package>> for &'a mut Package {
    type Error = TesError;

    fn try_from(value: Option<&'a mut Package>) -> Result<Self, Self::Error> {
        value.ok_or_else(|| decode_failed("No package value"))
    }
}

impl Package {
    pub fn read(field: &Tes3Field) -> Result<Self, TesError> {
        let mut reader = field.reader();

        match field.name() {
            b"AI_A" => Ok(Package::Activate(read_string::<PACKAGE_STRING_LENGTH, _>(
                &mut reader,
            )?)),
            b"AI_E" => {
                let x = reader.read_le()?;
                let y = reader.read_le()?;
                let z = reader.read_le()?;
                let duration = reader.read_le()?;
                let id = read_string::<PACKAGE_STRING_LENGTH, _>(&mut reader)?;
                Ok(Package::Escort {
                    x,
                    y,
                    z,
                    duration,
                    id,
                    cell: None,
                })
            }
            b"AI_F" => {
                let x = reader.read_le()?;
                let y = reader.read_le()?;
                let z = reader.read_le()?;
                let duration = reader.read_le()?;
                let id = read_string::<PACKAGE_STRING_LENGTH, _>(&mut reader)?;
                Ok(Package::Follow {
                    x,
                    y,
                    z,
                    duration,
                    id,
                    cell: None,
                })
            }
            b"AI_T" => {
                let x = reader.read_le()?;
                let y = reader.read_le()?;
                let z = reader.read_le()?;
                Ok(Package::Travel { x, y, z })
            }
            b"AI_W" => {
                let distance = reader.read_le()?;
                let duration = reader.read_le()?;
                let time_of_day = reader.read_le()?;
                let mut idles = [0u8; 8];
                reader.read_exact(&mut idles)?;
                Ok(Package::Wander {
                    distance,
                    duration,
                    time_of_day,
                    idles,
                })
            }
            _ => Err(decode_failed(format!(
                "Unknown package type {}",
                field.name_as_str()
            ))),
        }
    }

    pub fn read_cell_name<'a, 'b, T: TryInto<&'a mut Package>>(
        package: T,
        field: &'b Tes3Field,
    ) -> Result<(), TesError> {
        let package = package
            .try_into()
            .map_err(|_| decode_failed("Orphaned CNDT field"))?;
        let cell_field = Some(String::from(field.get_zstring()?));
        match package {
            Package::Escort { ref mut cell, .. } => match *cell {
                Some(_) => return Err(decode_failed("Extraneous CNDT field")),
                None => *cell = cell_field,
            },
            Package::Follow { ref mut cell, .. } => match *cell {
                Some(_) => return Err(decode_failed("Extraneous CNDT field")),
                None => *cell = cell_field,
            },
            _ => return Err(decode_failed("Orphaned CNDT field")),
        }

        Ok(())
    }
}
