use std::collections::HashMap;
use std::io::Seek;

use super::field::Tes3Field;
use super::record::Tes3Record;
use crate::plugin::Field;
use crate::tes3::{Actor, ActorState, AiSettings, Destination, Package, Skills};
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

/// Maximum length of certain strings in an NPC record
pub const NPC_STRING_LENGTH: usize = 32;

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
    ai_settings: AiSettings,
    destinations: Vec<Destination>,
    packages: Vec<Package>,
}

impl ActorState for Npc {
    fn iter_packages(&self) -> Box<dyn Iterator<Item = &Package> + '_> {
        Box::new(self.packages.iter())
    }

    fn iter_packages_mut(&mut self) -> Box<dyn Iterator<Item = &mut Package> + '_> {
        Box::new(self.packages.iter_mut())
    }

    fn add_package(&mut self, package: Package) {
        self.packages.push(package);
    }

    fn iter_inventory(&self) -> Box<dyn Iterator<Item = (&str, u32)> + '_> {
        Box::new(
            self.inventory
                .iter()
                .map(|(id, count)| (id.as_str(), *count)),
        )
    }

    fn iter_inventory_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut u32)> + '_> {
        Box::new(
            self.inventory
                .iter_mut()
                .map(|(id, count)| (id.as_str(), count)),
        )
    }

    fn add_item(&mut self, item_id: String, count: u32) {
        self.inventory.insert(item_id, count);
    }
}

impl Actor for Npc {
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    fn set_model(&mut self, model: Option<String>) {
        self.model = model;
    }

    fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    fn set_script(&mut self, script: Option<String>) {
        self.script = script;
    }

    fn iter_spells(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(self.spells.iter().map(|s| s.as_str()))
    }

    fn add_spell(&mut self, spell_id: String) {
        self.spells.push(spell_id);
    }

    fn ai_settings(&self) -> &AiSettings {
        &self.ai_settings
    }

    fn ai_settings_mut(&mut self) -> &mut AiSettings {
        &mut self.ai_settings
    }

    fn set_ai_settings(&mut self, settings: AiSettings) {
        self.ai_settings = settings;
    }

    fn iter_destinations(&self) -> Box<dyn Iterator<Item = &Destination> + '_> {
        Box::new(self.destinations.iter())
    }

    fn iter_destinations_mut(&mut self) -> Box<dyn Iterator<Item = &mut Destination> + '_> {
        Box::new(self.destinations.iter_mut())
    }

    fn add_destination(&mut self, destination: Destination) {
        self.destinations.push(destination);
    }
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
            ai_settings: AiSettings::default(),
            destinations: vec![],
            packages: vec![],
        };

        for field in record.iter() {
            match field.name() {
                b"NAME" => npc.id = String::from(field.get_zstring()?),
                b"RNAM" => npc.race = String::from(field.get_zstring()?),
                b"CNAM" => npc.class = String::from(field.get_zstring()?),
                b"ANAM" => npc.faction = String::from(field.get_zstring()?),
                b"BNAM" => npc.head = String::from(field.get_zstring()?),
                b"KNAM" => npc.hair = String::from(field.get_zstring()?),
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
                _ => npc.read_actor_field(&field)?,
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
