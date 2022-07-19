use crate::tes3::{Actor, ActorState, AiSettings, Destination, Package, Tes3Field, Tes3Record};
use crate::{decode_failed, Attribute, Attributes, Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt};
use bitflags::bitflags;
use enum_map::Enum;

bitflags! {
    #[derive(Default)]
    struct CreatureFlags: u32 {
        const BIPED = 0x00000001;
        const RESPAWN = 0x00000002;
        const WEAPON_AND_SHIELD = 0x00000004;
        const NONE = 0x00000008;
        const SWIMS = 0x00000010;
        const FLIES = 0x00000020;
        const WALKS = 0x00000040;
        const ESSENTIAL = 0x00000080;
        // the blood type is actually a three-bit integer stored in these bits
        const BLOOD_TYPE_1 = 0x00000400;
        const BLOOD_TYPE_2 = 0x00000800;
        const BLOOD_TYPE_4 = 0x00001000;
    }
}

#[binrw]
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
#[brw(repr = u32)]
pub enum CreatureType {
    #[default]
    Creature,
    Daedra,
    Undead,
    Humanoid,
}

#[binrw]
#[derive(Debug, Default)]
pub struct CreatureData {
    pub creature_type: CreatureType,
    pub level: u32,
    #[br(map = |a| Attributes::from_array(a))]
    #[bw(map = |a| a.as_slice())]
    pub attributes: Attributes<u32>,
    pub health: u32,
    pub magicka: u32,
    pub fatigue: u32,
    pub soul: u32,
    pub combat: u32,
    pub magic: u32,
    pub stealth: u32,
    pub attack_min_1: u32,
    pub attack_max_1: u32,
    pub attack_min_2: u32,
    pub attack_max_2: u32,
    pub attack_min_3: u32,
    pub attack_max_3: u32,
    pub gold: u32,
}

#[derive(Debug, Default)]
pub struct Creature {
    id: String,
    model: String,
    sound_gen_creature: Option<String>,
    name: Option<String>,
    script: Option<String>,
    data: CreatureData,
    flags: CreatureFlags,
    scale: Option<f32>,
    inventory: Vec<(String, u32)>,
    spells: Vec<String>,
    ai_settings: AiSettings,
    destinations: Vec<Destination>,
    packages: Vec<Package>,
}

impl ActorState for Creature {
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
        Box::new(self.inventory.iter().map(|(s, n)| (s.as_str(), *n)))
    }

    fn iter_inventory_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut u32)> + '_> {
        Box::new(self.inventory.iter_mut().map(|(s, n)| (s.as_str(), n)))
    }

    fn add_item(&mut self, item_id: String, count: u32) {
        self.inventory.push((item_id, count));
    }
}

impl Actor for Creature {
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    fn model(&self) -> Option<&str> {
        Some(self.model.as_str())
    }

    fn set_model(&mut self, model: Option<String>) {
        self.model = model.unwrap_or_else(|| String::from(""));
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

impl Form for Creature {
    type Field = Tes3Field;
    type Record = Tes3Record;
    const RECORD_TYPE: &'static [u8; 4] = b"CREA";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Creature::assert(&record)?;

        let mut creature = Creature::default();
        for field in record.iter() {
            match field.name() {
                b"NAME" => creature.id = String::from(field.get_zstring()?),
                b"CNAM" => creature.sound_gen_creature = Some(String::from(field.get_zstring()?)),
                b"NPDT" => creature.data = field.reader().read_le()?,
                b"FLAG" => {
                    creature.flags = CreatureFlags::from_bits(field.get_u32()?)
                        .ok_or_else(|| decode_failed("Invalid creature flags"))?
                }
                b"XSCL" => creature.scale = Some(field.get_f32()?),
                _ => creature.read_actor_field(field)?,
            }
        }

        Ok(creature)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}

impl Creature {
    /// Get this creature's stats
    pub fn data(&self) -> &CreatureData {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    static CREA_RECORD: &[u8] = include_bytes!("test/crea_record.bin");

    #[test]
    fn parse_record() {
        let mut record_ref = CREA_RECORD.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let record = Tes3Record::read(cursor).unwrap();
        let creature = Creature::read(&record).unwrap();
        assert_eq!(creature.id, "atronach_flame");
        assert_eq!(creature.name.unwrap(), "Flame Atronach");
        assert!(creature.sound_gen_creature.is_none());
        assert_eq!(creature.data.creature_type, CreatureType::Daedra);
        assert_eq!(creature.data.fatigue, 600);
        assert!(creature
            .spells
            .iter()
            .any(|s| s == "immune to normal weapons"));
    }
}
