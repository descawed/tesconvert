use crate::tes3::{Tes3Field, Tes3Record};
use crate::{
    decode_failed, decode_failed_because, make_str_vec, read_string, write_str, Field, TesError,
};
use binrw::{binrw, BinReaderExt, BinWriterExt};
use bitflags::bitflags;
use std::io::{Cursor, Read, Write};

/// Maximum length of certain strings on actor records
pub const ACTOR_STRING_LENGTH: usize = 32;

/// AI packages
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

/// Actor's cell travel destination
#[binrw]
#[derive(Debug, Default)]
pub struct Destination {
    position: (f32, f32, f32),
    rotation: (f32, f32, f32),
    #[brw(ignore)]
    cell_name: Option<String>,
}

/// Actor's AI settings
#[binrw]
#[derive(Debug, Default)]
pub struct AiSettings {
    hello: u16,
    fight: u8,
    flee: u8,
    alarm: u8,
    ai_unknown1: u8,
    ai_unknown2: u8,
    ai_unknown3: u8,
    // according to UESP, the remaining flag bits are "filled with junk data",
    // so we mask them out to prevent an error when reading the flags
    #[br(try_map = |s: u32| ServiceFlags::from_bits(s & 0x3ffff).ok_or("Invalid service flags"))]
    #[bw(map = |s| s.bits)]
    services: ServiceFlags,
}

bitflags! {
    #[derive(Default)]
    struct ServiceFlags: u32 {
        const WEAPON = 0x00001;
        const ARMOR = 0x00002;
        const CLOTHING = 0x00004;
        const BOOKS = 0x00008;
        const INGREDIENTS = 0x00010;
        const PICKS = 0x00020;
        const PROBES = 0x00040;
        const LIGHTS = 0x00080;
        const APPARATUS = 0x00100;
        const REPAIR_ITEMS = 0x00200;
        const MISC = 0x00400;
        const SPELLS = 0x00800;
        const MAGIC_ITEMS = 0x01000;
        const POTIONS = 0x02000;
        const TRAINING = 0x04000;
        const SPELLMAKING = 0x08000;
        const ENCHANTING = 0x10000;
        const REPAIR = 0x20000;
    }
}

/// A creature or NPC's modifiable state
pub trait ActorState {
    /// Iterate through this actor's AI packages
    fn iter_packages(&self) -> Box<dyn Iterator<Item = &Package> + '_>;

    /// Iterate through this actor's AI packages mutably
    fn iter_packages_mut(&mut self) -> Box<dyn Iterator<Item = &mut Package> + '_>;

    /// Add an AI package to this actor
    fn add_package(&mut self, package: Package);

    /// Iterate through this actor's inventory
    fn iter_inventory(&self) -> Box<dyn Iterator<Item = (&str, u32)> + '_>;

    /// Iterate through this actor's inventory mutably
    fn iter_inventory_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut u32)> + '_>;

    /// Add an item to this actor's inventory
    fn add_item(&mut self, item_id: String, count: u32);

    /// Read an actor data field and populate this actor appropriately
    fn read_actor_state_field(&mut self, field: &Tes3Field) -> Result<(), TesError> {
        match field.name() {
            b"NPCO" => {
                let mut reader = field.reader();
                let count = reader.read_le()?;
                let id = read_string::<ACTOR_STRING_LENGTH, _>(&mut reader)?;
                self.add_item(id, count);
            }
            b"AI_A" => self.add_package(Package::Activate(read_string::<ACTOR_STRING_LENGTH, _>(
                &mut field.get(),
            )?)),
            b"AI_E" => {
                let mut reader = field.reader();
                let x = reader.read_le()?;
                let y = reader.read_le()?;
                let z = reader.read_le()?;
                let duration = reader.read_le()?;
                let id = read_string::<ACTOR_STRING_LENGTH, _>(&mut reader)?;
                self.add_package(Package::Escort {
                    x,
                    y,
                    z,
                    duration,
                    id,
                    cell: None,
                });
            }
            b"AI_F" => {
                let mut reader = field.reader();
                let x = reader.read_le()?;
                let y = reader.read_le()?;
                let z = reader.read_le()?;
                let duration = reader.read_le()?;
                let id = read_string::<ACTOR_STRING_LENGTH, _>(&mut reader)?;
                self.add_package(Package::Follow {
                    x,
                    y,
                    z,
                    duration,
                    id,
                    cell: None,
                });
            }
            b"AI_T" => {
                let mut reader = field.reader();
                let x = reader.read_le()?;
                let y = reader.read_le()?;
                let z = reader.read_le()?;
                self.add_package(Package::Travel { x, y, z });
            }
            b"AI_W" => {
                let mut reader = field.reader();
                let distance = reader.read_le()?;
                let duration = reader.read_le()?;
                let time_of_day = reader.read_le()?;
                let mut idles = [0u8; 8];
                reader.read_exact(&mut idles)?;
                self.add_package(Package::Wander {
                    distance,
                    duration,
                    time_of_day,
                    idles,
                });
            }
            b"CNDT" => {
                let package = self
                    .iter_packages_mut()
                    .last()
                    .ok_or_else(|| decode_failed("Orphaned CNDT field"))?;
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
            }
            _ => {
                return Err(decode_failed(format!(
                    "Unknown actor state field {}",
                    field.name_as_str()
                )))
            }
        }

        Ok(())
    }

    /// Write the actor's inventory to the provided record
    fn write_inventory(&self, record: &mut Tes3Record) -> Result<(), TesError> {
        for (id, count) in self.iter_inventory() {
            let mut buf = vec![];
            let mut cursor = Cursor::new(&mut buf);
            cursor.write_le(&count)?;
            write_str::<ACTOR_STRING_LENGTH, _>(id, cursor)?;
            record.add_field(Tes3Field::new(b"NPCO", buf)?);
        }

        Ok(())
    }

    /// Write the actor's AI packages to the provided record
    fn write_packages(&self, record: &mut Tes3Record) -> Result<(), TesError> {
        for package in self.iter_packages() {
            match package {
                Package::Activate(activator) => {
                    let mut buf = vec![];
                    let mut cursor = Cursor::new(&mut buf);
                    write_str::<ACTOR_STRING_LENGTH, _>(activator, &mut cursor)?;
                    cursor.write_le(&1u8)?;
                    record.add_field(Tes3Field::new(b"AI_A", buf)?);
                }
                Package::Escort {
                    x,
                    y,
                    z,
                    duration,
                    id,
                    cell,
                }
                | Package::Follow {
                    x,
                    y,
                    z,
                    duration,
                    id,
                    cell,
                } => {
                    let mut buf = vec![];
                    let mut cursor = Cursor::new(&mut buf);
                    cursor.write_le(x)?;
                    cursor.write_le(y)?;
                    cursor.write_le(z)?;
                    cursor.write_le(duration)?;
                    write_str::<ACTOR_STRING_LENGTH, _>(id, &mut cursor)?;
                    cursor.write_le(&1u8)?;
                    cursor.write_le(&0u8)?;
                    record.add_field(Tes3Field::new(
                        if matches!(package, Package::Escort { .. }) {
                            b"AI_E"
                        } else {
                            b"AI_F"
                        },
                        buf,
                    )?);

                    if let Some(cell_name) = cell {
                        record.add_field(Tes3Field::new_zstring(b"CNDT", cell_name.clone())?);
                    }
                }
                Package::Travel { x, y, z } => {
                    let mut buf = vec![];
                    let mut cursor = Cursor::new(&mut buf);
                    cursor.write_le(x)?;
                    cursor.write_le(y)?;
                    cursor.write_le(z)?;
                    cursor.write_le(&1u8)?;
                    cursor.write_le(&0u8)?;
                    record.add_field(Tes3Field::new(b"AI_T", buf)?);
                }
                Package::Wander {
                    distance,
                    duration,
                    time_of_day,
                    idles,
                } => {
                    let mut buf = vec![];
                    let mut cursor = Cursor::new(&mut buf);
                    cursor.write_le(distance)?;
                    cursor.write_le(duration)?;
                    cursor.write_le(time_of_day)?;
                    cursor.write_all(idles)?;
                    cursor.write_le(&1u8)?;
                    record.add_field(Tes3Field::new(b"AI_W", buf)?);
                }
            }
        }

        Ok(())
    }
}

/// A creature or NPC
pub trait Actor: ActorState {
    /// Get this actor's name, if any
    fn name(&self) -> Option<&str>;

    /// Set this actor's name
    fn set_name(&mut self, name: Option<String>);

    /// Get this actor's NIF model, if any
    fn model(&self) -> Option<&str>;

    /// Set this actor's NIF model
    fn set_model(&mut self, model: Option<String>);

    /// Get this actor's script, if any
    fn script(&self) -> Option<&str>;

    /// Set this actor's script
    fn set_script(&mut self, script: Option<String>);

    /// Iterate through this actor's spells
    fn iter_spells(&self) -> Box<dyn Iterator<Item = &str> + '_>;

    /// Add a spell to this actor
    fn add_spell(&mut self, spell_id: String);

    /// Get this actor's AI settings
    fn ai_settings(&self) -> &AiSettings;

    /// Get this actor's AI settings mutably
    fn ai_settings_mut(&mut self) -> &mut AiSettings;

    /// Set this actor's AI settings
    fn set_ai_settings(&mut self, settings: AiSettings);

    /// Iterate through this actor's travel destinations
    fn iter_destinations(&self) -> Box<dyn Iterator<Item = &Destination> + '_>;

    /// Iterate through this actor's travel destinations mutably
    fn iter_destinations_mut(&mut self) -> Box<dyn Iterator<Item = &mut Destination> + '_>;

    /// Add a travel destination to this actor
    fn add_destination(&mut self, destination: Destination);

    /// Read an actor data field and populate this actor appropriately
    fn read_actor_field(&mut self, field: &Tes3Field) -> Result<(), TesError> {
        match field.name() {
            b"MODL" => self.set_model(Some(String::from(field.get_zstring()?))),
            b"FNAM" => self.set_name(Some(String::from(field.get_zstring()?))),
            b"SCRI" => self.set_script(Some(String::from(field.get_zstring()?))),
            b"NPCS" => {
                let spell = read_string::<ACTOR_STRING_LENGTH, _>(&mut field.get())
                    .map_err(|e| decode_failed_because("Could not parse NPCS", e))?;
                self.add_spell(spell);
            }
            b"AIDT" => self.set_ai_settings(field.reader().read_le()?),
            b"DODT" => self.add_destination(field.reader().read_le()?),
            b"DNAM" => {
                if let Some(last_destination) = self.iter_destinations_mut().last() {
                    if last_destination.cell_name == None {
                        last_destination.cell_name = Some(String::from(field.get_zstring()?));
                    } else {
                        return Err(decode_failed("Orphaned DNAM field"));
                    }
                } else {
                    return Err(decode_failed("Orphaned DNAM field"));
                }
            }
            _ => self.read_actor_state_field(field)?,
        }

        Ok(())
    }

    /// Write zero or more actor data fields to the provided record
    fn write_scalar_fields(
        &self,
        record: &mut Tes3Record,
        fields: &[&[u8; 4]],
    ) -> Result<(), TesError> {
        for field_name in fields.iter().copied() {
            let field = match field_name {
                b"MODL" => {
                    if let Some(model) = self.model() {
                        Tes3Field::new_zstring(field_name, String::from(model))?
                    } else {
                        continue;
                    }
                }
                b"FNAM" => {
                    if let Some(name) = self.name() {
                        Tes3Field::new_zstring(field_name, String::from(name))?
                    } else {
                        continue;
                    }
                }
                b"SCRI" => {
                    if let Some(script) = self.script() {
                        Tes3Field::new_zstring(field_name, String::from(script))?
                    } else {
                        continue;
                    }
                }
                b"AIDT" => {
                    let mut buf = vec![];
                    let mut cursor = Cursor::new(&mut buf);
                    cursor.write_le(self.ai_settings())?;
                    Tes3Field::new(field_name, buf)?
                }
                _ => {
                    return Err(TesError::RequirementFailed(format!(
                        "Unexpected actor scalar field {:?}",
                        field_name
                    )))
                }
            };

            record.add_field(field);
        }

        Ok(())
    }

    /// Write the actor's spells to the provided record
    fn write_spells(&self, record: &mut Tes3Record) -> Result<(), TesError> {
        for id in self.iter_spells() {
            record.add_field(Tes3Field::new(
                b"NPCS",
                make_str_vec(id, ACTOR_STRING_LENGTH),
            )?);
        }

        Ok(())
    }

    /// Write the actor's travel destinations to the provided record
    fn write_destinations(&self, record: &mut Tes3Record) -> Result<(), TesError> {
        for destination in self.iter_destinations() {
            let mut buf = vec![];
            let mut cursor = Cursor::new(&mut buf);
            cursor.write_le(destination)?;
            record.add_field(Tes3Field::new(b"DODT", buf)?);

            if let Some(ref cell_name) = destination.cell_name {
                record.add_field(Tes3Field::new_zstring(b"DNAM", cell_name.clone())?);
            }
        }

        Ok(())
    }
}
