#![allow(clippy::single_component_path_imports)]

use std::collections::HashMap;

use super::record::Record;
use crate::plugin::FieldInterface;
use crate::tes3::Skills;
use crate::*;

// this line is only to help the IDE
use bitflags;

bitflags! {
    struct NpcFlags: u32 {
        const FEMALE = 0x0001;
        const ESSENTIAL = 0x0002;
        const RESPAWN = 0x0004;
        const UNKNOWN = 0x0008;
        const AUTO_CALC = 0x0010;
        const BLOOD_SKELETON = 0x0400;
        const BLOOD_SPARKS = 0x0800;
    }
}

bitflags! {
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

/// Maximum length of certain strings in an NPC record
pub const NPC_STRING_LENGTH: usize = 32;

/// NPC's cell travel destination
#[derive(Debug)]
pub struct Destination {
    position: (f32, f32, f32),
    rotation: (f32, f32, f32),
    cell_name: Option<String>,
}

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

/// An NPC (or the PC) in the game
#[derive(Debug)]
pub struct Npc {
    id: String,
    model: Option<String>,
    name: Option<String>,
    race: String,
    class: String,
    faction: String,
    head: String,
    hair: String,
    script: Option<String>,
    pub level: u16,
    pub attributes: Attributes<u8>,
    pub skills: Skills<u8>,
    health: u16,
    magicka: u16,
    fatigue: u16,
    disposition: u8,
    reputation: u8,
    rank: u8,
    gold: u32,
    flags: NpcFlags,
    inventory: HashMap<String, u32>,
    spells: Vec<String>,
    // <ai>
    hello: u16,
    fight: u8,
    flee: u8,
    alarm: u8,
    ai_unknown1: u8,
    ai_unknown2: u8,
    ai_unknown3: u8,
    services: ServiceFlags,
    // </ai>
    destinations: Vec<Destination>,
    packages: Vec<Package>,
}

impl Npc {
    /// Read NPC data from a raw record
    ///
    /// # Errors
    ///
    /// Fails if the provided record is not an `b"NPC_"` record or if the record data is invalid.
    pub fn read(record: &Record) -> Result<Npc, TesError> {
        if record.name() != b"NPC_" {
            return Err(decode_failed("Record was not an NPC_ record"));
        }

        // initialize an empty struct which we'll fill in based on what's available
        let mut npc = Npc {
            id: String::new(),
            model: None,
            name: None,
            race: String::new(),
            class: String::new(),
            faction: String::new(),
            head: String::new(),
            hair: String::new(),
            script: None,
            level: 0,
            attributes: Attributes::new(),
            skills: Skills::new(),
            health: 0,
            magicka: 0,
            fatigue: 0,
            disposition: 0,
            reputation: 0,
            rank: 0,
            gold: 0,
            flags: NpcFlags::empty(),
            inventory: HashMap::new(),
            spells: vec![],
            // <ai>
            hello: 0,
            fight: 0,
            flee: 0,
            alarm: 0,
            ai_unknown1: 0,
            ai_unknown2: 0,
            ai_unknown3: 0,
            services: ServiceFlags::empty(),
            // </ai>
            destinations: vec![],
            packages: vec![],
        };

        for field in record.iter() {
            match field.name() {
                b"NAME" => npc.id = String::from(field.get_zstring()?),
                b"MODL" => npc.model = Some(String::from(field.get_zstring()?)),
                b"FNAM" => npc.name = Some(String::from(field.get_zstring()?)),
                b"RNAM" => npc.race = String::from(field.get_zstring()?),
                b"CNAM" => npc.class = String::from(field.get_zstring()?),
                b"ANAM" => npc.faction = String::from(field.get_zstring()?),
                b"BNAM" => npc.head = String::from(field.get_zstring()?),
                b"KNAM" => npc.hair = String::from(field.get_zstring()?),
                b"SCRI" => npc.script = Some(String::from(field.get_zstring()?)),
                b"NPDT" => {
                    let mut data = field.get();
                    let len = data.len();
                    let reader = &mut data;
                    npc.level = extract!(reader as u16)?;
                    if len == 12 {
                        // auto-calculated; many fields are not present
                        npc.disposition = extract!(reader as u8)?;
                        npc.reputation = extract!(reader as u8)?;
                        npc.rank = extract!(reader as u8)?;
                        // UESP says these next 3 bytes are junk and OpenMW labels them as unknown,
                        // so we're going to ignore them
                        let mut buf = [0u8; 3];
                        reader.read_exact(&mut buf)?;
                        npc.gold = extract!(reader as u32)?;
                    } else {
                        // not auto-calculated; all fields are present
                        for attribute in npc.attributes.values_mut() {
                            *attribute = extract!(reader as u8)?;
                        }

                        for skill in npc.skills.values_mut() {
                            *skill = extract!(reader as u8)?;
                        }

                        npc.health = extract!(reader as u16)?;
                        npc.magicka = extract!(reader as u16)?;
                        npc.fatigue = extract!(reader as u16)?;

                        npc.disposition = extract!(reader as u8)?;
                        npc.reputation = extract!(reader as u8)?;
                        npc.rank = extract!(reader as u8)?;
                        extract!(reader as u8)?; // skip dummy byte
                        npc.gold = extract!(reader as u32)?;
                    }
                }
                b"FLAG" => {
                    npc.flags =
                        NpcFlags::from_bits(field.get_u32()?).ok_or(TesError::DecodeFailed {
                            description: String::from("Invalid NPC flags"),
                            source: None,
                        })?
                }
                b"NPCO" => {
                    let mut data = field.get();
                    let mut reader = &mut data;
                    let count = extract!(reader as u32)?;
                    let id = extract_string(NPC_STRING_LENGTH, &mut reader)?;
                    npc.inventory.insert(id, count);
                }
                b"NPCS" => {
                    let spell = extract_string(NPC_STRING_LENGTH, &mut field.get())
                        .map_err(|e| decode_failed_because("Could not parse NPCS", e))?;
                    npc.spells.push(spell);
                }
                b"AIDT" => {
                    let mut data = field.get();
                    let reader = &mut data;
                    npc.hello = extract!(reader as u16)?;
                    npc.fight = extract!(reader as u8)?;
                    npc.flee = extract!(reader as u8)?;
                    npc.alarm = extract!(reader as u8)?;
                    npc.ai_unknown1 = extract!(reader as u8)?;
                    npc.ai_unknown2 = extract!(reader as u8)?;
                    npc.ai_unknown3 = extract!(reader as u8)?;
                    // according to UESP, the remaining flag bits are "filled with junk data",
                    // so we mask them out to prevent an error when reading the flags
                    let flags = extract!(reader as u32)? & 0x3ffff;
                    npc.services = ServiceFlags::from_bits(flags).unwrap();
                }
                b"DODT" => {
                    let mut data = field.get();
                    let reader = &mut data;
                    let pos_x = extract!(reader as f32)?;
                    let pos_y = extract!(reader as f32)?;
                    let pos_z = extract!(reader as f32)?;
                    let rot_x = extract!(reader as f32)?;
                    let rot_y = extract!(reader as f32)?;
                    let rot_z = extract!(reader as f32)?;
                    npc.destinations.push(Destination {
                        position: (pos_x, pos_y, pos_z),
                        rotation: (rot_x, rot_y, rot_z),
                        cell_name: None,
                    });
                }
                b"DNAM" => {
                    if let Some(last_destination) = npc.destinations.last_mut() {
                        if last_destination.cell_name == None {
                            last_destination.cell_name = Some(String::from(field.get_zstring()?));
                        } else {
                            return Err(TesError::DecodeFailed {
                                description: String::from("Orphaned DNAM field"),
                                source: None,
                            });
                        }
                    } else {
                        return Err(TesError::DecodeFailed {
                            description: String::from("Orphaned DNAM field"),
                            source: None,
                        });
                    }
                }
                b"AI_A" => {
                    let mut data = field.get();
                    let mut reader = &mut data;
                    npc.packages.push(Package::Activate(extract_string(
                        NPC_STRING_LENGTH,
                        &mut reader,
                    )?));
                }
                b"AI_E" => {
                    let mut data = field.get();
                    let mut reader = &mut data;
                    let x = extract!(reader as f32)?;
                    let y = extract!(reader as f32)?;
                    let z = extract!(reader as f32)?;
                    let duration = extract!(reader as u16)?;
                    let id = extract_string(NPC_STRING_LENGTH, &mut reader)?;
                    npc.packages.push(Package::Escort {
                        x,
                        y,
                        z,
                        duration,
                        id,
                        cell: None,
                    });
                }
                b"AI_F" => {
                    let mut data = field.get();
                    let mut reader = &mut data;
                    let x = extract!(reader as f32)?;
                    let y = extract!(reader as f32)?;
                    let z = extract!(reader as f32)?;
                    let duration = extract!(reader as u16)?;
                    let id = extract_string(NPC_STRING_LENGTH, &mut reader)?;
                    npc.packages.push(Package::Follow {
                        x,
                        y,
                        z,
                        duration,
                        id,
                        cell: None,
                    });
                }
                b"AI_T" => {
                    let mut data = field.get();
                    let reader = &mut data;
                    let x = extract!(reader as f32)?;
                    let y = extract!(reader as f32)?;
                    let z = extract!(reader as f32)?;
                    npc.packages.push(Package::Travel { x, y, z });
                }
                b"AI_W" => {
                    let mut data = field.get();
                    let reader = &mut data;
                    let distance = extract!(reader as u16)?;
                    let duration = extract!(reader as u16)?;
                    let time_of_day = extract!(reader as u8)?;
                    let mut idles = [0u8; 8];
                    reader.read_exact(&mut idles)?;
                    npc.packages.push(Package::Wander {
                        distance,
                        duration,
                        time_of_day,
                        idles,
                    });
                }
                b"CNDT" => {
                    if let Some(last_package) = npc.packages.last_mut() {
                        let cell_field = Some(String::from(field.get_zstring()?));
                        match last_package {
                            Package::Escort { ref mut cell, .. } => match *cell {
                                Some(_) => {
                                    return Err(TesError::DecodeFailed {
                                        description: String::from("Extraneous CNDT field"),
                                        source: None,
                                    })
                                }
                                None => *cell = cell_field,
                            },
                            Package::Follow { ref mut cell, .. } => match *cell {
                                Some(_) => {
                                    return Err(TesError::DecodeFailed {
                                        description: String::from("Extraneous CNDT field"),
                                        source: None,
                                    })
                                }
                                None => *cell = cell_field,
                            },
                            _ => {
                                return Err(TesError::DecodeFailed {
                                    description: String::from("Orphaned CNDT field"),
                                    source: None,
                                })
                            }
                        }
                    } else {
                        return Err(TesError::DecodeFailed {
                            description: String::from("Orphaned CNDT field"),
                            source: None,
                        });
                    }
                }
                _ => {
                    return Err(TesError::DecodeFailed {
                        description: format!("Unexpected field {}", field.display_name()),
                        source: None,
                    })
                }
            }
        }

        Ok(npc)
    }

    /// Gets the character's name
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|v| &v[..])
    }

    /// Sets the character's name
    ///
    /// # Errors
    ///
    /// Fails if the string length exceeds [`MAX_DATA`]
    ///
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    pub fn set_name(&mut self, name: String) -> Result<(), TesError> {
        check_size(&name, MAX_DATA, "NPC name too long")?;
        self.name = Some(name);
        Ok(())
    }

    /// Gets the character's class
    pub fn class(&self) -> &str {
        &self.class
    }

    /// Sets the character's class
    ///
    /// # Errors
    ///
    /// Fails if the string length exceeds [`MAX_DATA`]
    ///
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    pub fn set_class(&mut self, class: String) -> Result<(), TesError> {
        check_size(&class, MAX_DATA, "NPC class too long")?;
        self.class = class;
        Ok(())
    }

    /// Gets the character's race
    pub fn race(&self) -> &str {
        &self.race
    }

    /// Sets the character's race
    ///
    /// # Errors
    ///
    /// Fails if the string length exceeds [`MAX_DATA`]
    ///
    /// [`MAX_DATA`]: constant.MAX_DATA.html
    pub fn set_race(&mut self, race: String) -> Result<(), TesError> {
        check_size(&race, MAX_DATA, "NPC race too long")?;
        self.race = race;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static NPC_RECORD: &[u8] = include_bytes!("test/npc_record.bin");

    #[test]
    fn parse_record() {
        let record = Record::read(&mut NPC_RECORD.as_ref()).unwrap();
        let npc = Npc::read(&record).unwrap();
        assert_eq!(npc.id, "player");
        assert_eq!(npc.name.unwrap(), "Cirfenath");
        assert_eq!(npc.class, "NEWCLASSID_CHARGEN");
        assert!(npc.inventory.len() > 0);
    }
}
