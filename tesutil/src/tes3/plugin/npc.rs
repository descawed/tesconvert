use std::collections::HashMap;
use std::io::Seek;

use super::field::Tes3Field;
use super::package::Package;
use super::record::Tes3Record;
use crate::plugin::Field;
use crate::tes3::Skills;
use crate::*;

use binrw::BinReaderExt;
use bitflags::bitflags;

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

impl Form for Npc {
    type Field = Tes3Field;
    type Record = Tes3Record;

    const RECORD_TYPE: &'static [u8; 4] = b"NPC_";

    /// Reads NPC data from a raw record
    ///
    /// # Errors
    ///
    /// Fails if the provided record is not an `b"NPC_"` record or if the record data is invalid.
    fn read(record: &Tes3Record) -> Result<Npc, TesError> {
        Npc::assert(record)?;

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
            attributes: Attributes::default(),
            skills: Skills::default(),
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
                    let data = field.get();
                    let len = data.len();
                    let mut reader = field.reader();
                    npc.level = reader.read_le()?;
                    if len == 12 {
                        // auto-calculated; many fields are not present
                        npc.disposition = reader.read_le()?;
                        npc.reputation = reader.read_le()?;
                        npc.rank = reader.read_le()?;
                        // UESP says these next 3 bytes are junk and OpenMW labels them as unknown,
                        // so we're going to ignore them
                        reader.seek(SeekFrom::Current(3))?;
                        npc.gold = reader.read_le()?;
                    } else {
                        // not auto-calculated; all fields are present
                        for attribute in npc.attributes.values_mut() {
                            *attribute = reader.read_le()?;
                        }

                        for skill in npc.skills.values_mut() {
                            *skill = reader.read_le()?;
                        }

                        npc.health = reader.read_le()?;
                        npc.magicka = reader.read_le()?;
                        npc.fatigue = reader.read_le()?;

                        npc.disposition = reader.read_le()?;
                        npc.reputation = reader.read_le()?;
                        npc.rank = reader.read_le()?;
                        reader.seek(SeekFrom::Current(1))?; // skip dummy byte
                        npc.gold = reader.read_le()?;
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
                    let mut reader = field.reader();
                    let count = reader.read_le()?;
                    let id = read_string::<NPC_STRING_LENGTH, _>(&mut reader)?;
                    npc.inventory.insert(id, count);
                }
                b"NPCS" => {
                    let spell = read_string::<NPC_STRING_LENGTH, _>(&mut field.get())
                        .map_err(|e| decode_failed_because("Could not parse NPCS", e))?;
                    npc.spells.push(spell);
                }
                b"AIDT" => {
                    let mut reader = field.reader();
                    npc.hello = reader.read_le()?;
                    npc.fight = reader.read_le()?;
                    npc.flee = reader.read_le()?;
                    npc.alarm = reader.read_le()?;
                    npc.ai_unknown1 = reader.read_le()?;
                    npc.ai_unknown2 = reader.read_le()?;
                    npc.ai_unknown3 = reader.read_le()?;
                    // according to UESP, the remaining flag bits are "filled with junk data",
                    // so we mask them out to prevent an error when reading the flags
                    let flags = reader.read_le::<u32>()? & 0x3ffff;
                    npc.services = ServiceFlags::from_bits(flags).unwrap();
                }
                b"DODT" => {
                    let mut reader = field.reader();
                    let pos_x = reader.read_le()?;
                    let pos_y = reader.read_le()?;
                    let pos_z = reader.read_le()?;
                    let rot_x = reader.read_le()?;
                    let rot_y = reader.read_le()?;
                    let rot_z = reader.read_le()?;
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
                            return Err(decode_failed("Orphaned DNAM field"));
                        }
                    } else {
                        return Err(decode_failed("Orphaned DNAM field"));
                    }
                }
                b"AI_A" | b"AI_E" | b"AI_F" | b"AI_T" | b"AI_W" => {
                    npc.packages.push(Package::read(&field)?)
                }
                b"CNDT" => Package::read_cell_name(npc.packages.last_mut(), &field)?,
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(npc)
    }

    fn write(&self, _: &mut Tes3Record) -> Result<(), TesError> {
        unimplemented!()
    }
}

impl Npc {
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

    /// Gets whether the character is female
    pub fn is_female(&self) -> bool {
        self.flags.contains(NpcFlags::FEMALE)
    }

    /// Sets whether the character is female
    pub fn set_is_female(&mut self, is_female: bool) {
        self.flags.set(NpcFlags::FEMALE, is_female);
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

    /// Gets the character's spells
    pub fn spells(&self) -> impl Iterator<Item = &str> {
        self.spells.iter().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    static NPC_RECORD: &[u8] = include_bytes!("test/npc_record.bin");

    #[test]
    fn parse_record() {
        let mut record_ref = NPC_RECORD.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let record = Tes3Record::read(cursor).unwrap();
        let npc = Npc::read(&record).unwrap();
        assert_eq!(npc.id, "player");
        assert_eq!(npc.name.unwrap(), "Cirfenath");
        assert_eq!(npc.class, "NEWCLASSID_CHARGEN");
        assert!(npc.inventory.len() > 0);
    }
}
