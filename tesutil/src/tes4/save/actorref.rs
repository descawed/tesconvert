use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::convert::TryFrom;
use std::io::{Cursor, Seek, SeekFrom};

use crate::tes4::plugin::Class;
use crate::tes4::save::{ChangeRecord, ChangeType, FormChange, FORM_PLAYER_REF};
use crate::tes4::{ActorValues, Skills, SoulType};
use crate::*;

use binrw::{binrw, BinReaderExt, BinWriterExt};
use bitflags::bitflags;

bitflags! {
    struct ActorReferenceChangeFlags: u32 {
        const FORM_FLAGS = 0x00000001;
        const CREATED = 0x00000002;
        const MOVED = 0x00000004;
        const HAVOK_MOVED = 0x00000008;
        const SCALE = 0x00000010;
        const LIFE_STATE = 0x00000040;
        const OWNER_CRIME_GOLD = 0x00000080;
        const INVESTMENT_GOLD = 0x00002000;
        const OBLIVION_ENTRY = 0x00004000;
        const DISP_MODIFIERS = 0x00008000;
        const NON_SAVED_PACKAGE = 0x00010000;
        const INTERRUPT_FOLLOW_DIALOGUE = 0x00020000;
        const TRESPASS = 0x00040000;
        const RUN_ONCE = 0x00080000;
        const MAGIC_MODIFIERS = 0x00100000;
        const SCRIPT_MODIFIERS = 0x00200000;
        const GAME_MODIFIERS = 0x00400000;
        const OBLIVION_FLAG = 0x00800000;
        const MOVEMENT_EXTRA = 0x01000000;
        const ANIMATION = 0x02000000;
        const SCRIPT = 0x04000000;
        const INVENTORY = 0x08000000;
        const LEVELED_CREATURE = 0x10000000;
        const EQUIPMENT = 0x20000000;
        const ENABLED_DISABLED = 0x40000000;
        const CELL_CHANGED = 0x80000000;
    }
}

/// Determines an actor's processing priority
#[derive(Copy, Clone, Debug, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum ActorFlag {
    High = 0,
    MidHigh = 1,
    MidLow = 2,
    Low = 3,
    None = 0xff,
}

/// Value of a script variable
#[binrw]
#[derive(Debug)]
pub enum ScriptVariableValue {
    #[brw(magic = 0u16)]
    Reference(u32),
    #[brw(magic = 0xF000u16)]
    Number(f64),
}

impl ScriptVariableValue {
    /// Reads a script variable value from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O or decoding error occurs
    pub fn read<T: Read + Seek>(mut f: T) -> Result<ScriptVariableValue, TesError> {
        Ok(f.read_le()?)
    }

    /// Writes a script variable value to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_le(&self)?;

        Ok(())
    }
}

/// Variables of a script referenced by a script property
#[binrw]
#[derive(Debug)]
pub struct ScriptVariable {
    index: u16,
    value: ScriptVariableValue,
}

impl ScriptVariable {
    /// Reads a script variable from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O or decoding error occurs
    pub fn read<T: Read + Seek>(mut f: T) -> Result<ScriptVariable, TesError> {
        Ok(f.read_le()?)
    }

    /// Writes a script variable to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_le(&self)?;
        Ok(())
    }
}

/// Miscellaneous properties that appear in a change record's properties section
///
/// Currently, only properties that appear in inventory items are implemented.
#[binrw]
#[derive(Debug)]
pub enum Property {
    #[brw(magic = 0x12u8)]
    Script {
        script: u32,
        #[br(temp)]
        #[bw(calc = variables.len() as u16)]
        num_vars: u16,
        #[br(count = num_vars)]
        variables: Vec<ScriptVariable>,
        unknown: u8,
    },
    #[brw(magic = 0x1bu8)]
    EquippedItem,
    #[brw(magic = 0x1cu8)]
    EquippedAccessory,
    #[brw(magic = 0x22u8)]
    Unknown22(u32),
    #[brw(magic = 0x23u8)]
    Unknown23 {
        #[br(temp)]
        #[bw(calc = items.len() as u16)]
        num_items: u16,
        #[br(count = num_items)]
        items: Vec<u32>,
    },
    #[brw(magic = 0x27u8)]
    Owner(u32),
    #[brw(magic = 0x2au8)]
    AffectedItemCount(u16),
    #[brw(magic = 0x2bu8)]
    ItemHealth(f32),
    #[brw(magic = 0x2du8)]
    Time(f32),
    #[brw(magic = 0x2eu8)]
    EnchantmentPoints(f32),
    #[brw(magic = 0x2fu8)]
    Soul(SoulType),
    #[brw(magic = 0x36u8)]
    LeveledItem([u8; 5]),
    #[brw(magic = 0x37u8)]
    Scale(f32),
    #[brw(magic = 0x3du8)]
    CrimeGold(f32),
    #[brw(magic = 0x3eu8)]
    OblivionEntry { door: u32, x: f32, y: f32, z: f32 },
    #[brw(magic = 0x47u8)]
    CantWear,
    #[brw(magic = 0x48u8)]
    Poison(u32),
    #[brw(magic = 0x4fu8)]
    Unknown4f(u32),
    #[brw(magic = 0x50u8)]
    BoundItem,
    #[brw(magic = 0x55u8)]
    ShortcutKey(u8),
}

impl Property {
    /// Reads a property from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O or decoding error occurs
    pub fn read<T: Read + Seek>(mut f: T) -> Result<Property, TesError> {
        Ok(f.read_le()?)
    }

    /// Writes a property to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_le(&self)?;

        Ok(())
    }
}

/// An item in the player's inventory
#[derive(Debug)]
pub struct InventoryItem {
    pub iref: u32,
    pub stack_count: i32,
    changes: Vec<Vec<Property>>,
}

impl InventoryItem {
    /// Creates a new inventory item
    pub fn new(iref: u32, stack_count: i32) -> InventoryItem {
        InventoryItem {
            iref,
            stack_count,
            changes: vec![],
        }
    }

    /// DOes this item have changes?
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Add a change set to the item stack
    pub fn add_change(&mut self, properties: Vec<Property>) {
        self.changes.push(properties);
    }

    /// Reads an item from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn read<T: Read + Seek>(mut f: T) -> Result<InventoryItem, TesError> {
        let iref = f.read_le()?;
        let stack_count = f.read_le()?;
        let num_changes = f.read_le::<u32>()? as usize;
        let mut changes = Vec::with_capacity(num_changes);
        for _ in 0..num_changes {
            let num_properties = f.read_le::<u16>()? as usize;
            let mut properties = Vec::with_capacity(num_properties);
            for _ in 0..num_properties {
                properties.push(Property::read(&mut f)?);
            }
            changes.push(properties);
        }

        Ok(InventoryItem {
            iref,
            stack_count,
            changes,
        })
    }

    /// Writes an item to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_le(&self.iref)?;
        f.write_le(&self.stack_count)?;
        f.write_le(&(self.changes.len() as u32))?;
        for change in self.changes.iter() {
            f.write_le(&(change.len() as u16))?;
            for property in change.iter() {
                property.write(&mut f)?;
            }
        }

        Ok(())
    }
}

/// An active magical effect being applied to the player
#[binrw]
#[derive(Debug)]
pub struct ActiveEffect {
    #[br(temp)]
    #[bw(calc = details.len() as u16)]
    size: u16,
    spell: u32,
    effect: u8,
    #[br(count = size)]
    details: Vec<u8>,
}

impl ActiveEffect {
    /// Reads an active effect from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn read<T: Read + Seek>(mut f: T) -> Result<ActiveEffect, TesError> {
        Ok(f.read_le()?)
    }

    /// Writes an active effect to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_le(&self)?;

        Ok(())
    }
}

/// Changes to the player
///
/// This is a subset of the functionality for change records detailing changes to a placed instance
/// of an NPC (ACHR) or creature (ACRE). However, these records are quite complex and not fully
/// documented, so for now we're focusing on just the player.
pub struct PlayerReferenceChange {
    flags: u32,
    // location
    cell: u32,
    x: f32,
    y: f32,
    z: f32,
    rx: f32,
    ry: f32,
    rz: f32,
    // temporary attribute changes
    temp_active_effects: ActorValues<f32>,
    tac_unknown: ActorValues<f32>,
    damage: ActorValues<f32>,
    health_delta: f32,
    magicka_delta: f32,
    fatigue_delta: f32,
    // flag
    actor_flag: ActorFlag,
    // inventory
    inventory: Vec<InventoryItem>,
    // properties
    properties: Vec<Property>,
    // TODO: do we need to grab any of the modifier sections?
    raw: Vec<u8>,
    // player stats
    statistics: [u32; 34],
    stat_unknown1: [u8; 118],
    birthsign: u32,
    stat_unknown2: [u32; 13],
    stat_unknown3: [u8; 2],
    stat_unknown4: Vec<u32>,
    stat_unknown5: [u8; 2],
    oblivion_doors: Vec<(u32, u8)>,
    stat_unknown6: [u8; 2],
    stat_active_effects: Vec<ActiveEffect>,
    pub skill_xp: Skills<f32>,
    pub advancements: Vec<Attributes<u8>>,
    pub spec_increases: Specializations<u8>,
    pub skill_usage: Skills<u32>,
    pub major_skill_advancements: u32,
    stat_unknown7: u8,
    active_quest: u32,
    known_topics: Vec<u32>,
    open_quests: Vec<(u32, u8, u8)>,
    known_magic_effects: Vec<[u8; 4]>,
    facegen_symmetric: [u8; 200],
    facegen_asymmetric: [u8; 120],
    facegen_texture: [u8; 200],
    race: u32,
    hair: u32,
    eyes: u32,
    hair_length: f32,
    hair_color: [u8; 3],
    stat_unknown8: u8,
    pub is_female: bool,
    name: String,
    class: u32,
    custom_class: Option<Class>,
    stat_unknown9: u32,
}

impl FormChange for PlayerReferenceChange {
    /// Reads a player reference change from a raw change record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs or if the data is invalid
    fn read(record: &ChangeRecord) -> Result<PlayerReferenceChange, TesError> {
        if record.form_id() != FORM_PLAYER_REF {
            return Err(decode_failed(
                "Only the player's change record may currently be decoded",
            ));
        }

        if record.change_type() != ChangeType::CharacterReference {
            return Err(decode_failed(
                "Only character reference change record may currently be decoded",
            ));
        }

        let mut data = record.data();
        let data_size = data.len();
        let mut reader = Cursor::new(&mut data);

        let flags = record.flags();

        // location
        let cell = reader.read_le()?;
        let x = reader.read_le()?;
        let y = reader.read_le()?;
        let z = reader.read_le()?;
        let rx = reader.read_le()?;
        let ry = reader.read_le()?;
        let rz = reader.read_le()?;

        let mut temp_active_effects = ActorValues::default();
        let mut tac_unknown = ActorValues::default();
        let mut damage = ActorValues::default();

        for effect in temp_active_effects.values_mut() {
            *effect = reader.read_le()?;
        }

        for tac in tac_unknown.values_mut() {
            *tac = reader.read_le()?;
        }

        for dmg in damage.values_mut() {
            *dmg = reader.read_le()?;
        }

        let health_delta = reader.read_le()?;
        let magicka_delta = reader.read_le()?;
        let fatigue_delta = reader.read_le()?;

        let actor_flag = ActorFlag::try_from(reader.read_le::<u8>()?)
            .map_err(|e| decode_failed_because("Invalid actor flags", e))?;

        // inventory might not be present if the save is from the very beginning of the game
        let inventory = if flags & 0x08000000 != 0 {
            let num_items = reader.read_le::<u16>()? as usize;
            let mut inventory = Vec::with_capacity(num_items);
            for _ in 0..num_items {
                inventory.push(InventoryItem::read(&mut reader)?);
            }
            inventory
        } else {
            vec![]
        };

        let num_properties = reader.read_le::<u16>()? as usize;
        let mut properties = Vec::with_capacity(num_properties);
        for _ in 0..num_properties {
            properties.push(Property::read(&mut reader)?);
        }

        // the following data is not fully decoded and/or not relevant to us here, so we just grab
        // it all raw so we can spit it back out later
        let start = reader.seek(SeekFrom::Current(0))?;
        let mut end = data_size as u64;
        for _ in start..end - 1 {
            // scan for certain marker bytes to tell when we've reached the player statistics section
            let landmark: (u8, u8) = (reader.read_le()?, reader.read_le()?);
            if landmark == (0xec, 0x42) {
                // we have a potential match; check for the next pair of landmark bytes
                reader.seek(SeekFrom::Current(19))?;
                let landmark: (u8, u8) = (reader.read_le()?, reader.read_le()?);
                if landmark == (0x96, 0x42) {
                    // we found it!
                    end = reader.seek(SeekFrom::Current(0))? + 28;
                    break;
                }
                reader.seek(SeekFrom::Current(-22))?;
            } else {
                reader.seek(SeekFrom::Current(-1))?;
            }
        }

        let size = (end - start) as usize;
        reader.seek(SeekFrom::Start(start))?;
        let mut raw = vec![0u8; size];
        reader.read_exact(&mut raw[..])?;

        // player statistics
        let mut statistics = [0u32; 34];
        for statistic in &mut statistics {
            *statistic = reader.read_le()?;
        }

        let mut stat_unknown1 = [0u8; 118];
        reader.read_exact(&mut stat_unknown1)?;

        let birthsign = reader.read_le()?;

        let mut stat_unknown2 = [0u32; 13];
        for unk in &mut stat_unknown2 {
            *unk = reader.read_le()?;
        }

        let num2 = reader.read_le::<u16>()? as usize;

        let mut stat_unknown3 = [0u8; 2];
        reader.read_exact(&mut stat_unknown3)?;

        let mut stat_unknown4 = Vec::with_capacity(num2);
        for _ in 0..num2 {
            stat_unknown4.push(reader.read_le()?);
        }

        let mut stat_unknown5 = [0u8; 2];
        reader.read_exact(&mut stat_unknown5)?;

        let num_doors = reader.read_le::<u16>()? as usize;
        let mut oblivion_doors = Vec::with_capacity(num_doors);
        for _ in 0..num_doors {
            oblivion_doors.push((reader.read_le()?, reader.read_le()?));
        }

        let mut stat_unknown6 = [0u8; 2];
        reader.read_exact(&mut stat_unknown6)?;

        let num_active_effects = reader.read_le::<u16>()? as usize;
        let mut stat_active_effects = Vec::with_capacity(num_active_effects);
        for _ in 0..num_active_effects {
            stat_active_effects.push(ActiveEffect::read(&mut reader)?);
        }

        let mut skill_xp = Skills::default();
        for skill in skill_xp.values_mut() {
            *skill = reader.read_le()?;
        }

        let num_advancements = reader.read_le::<u32>()? as usize;
        let mut advancements = Vec::with_capacity(num_advancements);
        for _ in 0..num_advancements {
            let mut attributes = Attributes::default();
            for attribute in attributes.values_mut() {
                *attribute = reader.read_le()?;
            }

            advancements.push(attributes);
        }

        let mut spec_increases = Specializations::default();
        for specialization in spec_increases.values_mut() {
            *specialization = reader.read_le()?;
        }

        let mut skill_usage = Skills::default();
        for skill in skill_usage.values_mut() {
            *skill = reader.read_le()?;
        }

        let major_skill_advancements = reader.read_le()?;
        let stat_unknown7 = reader.read_le()?;
        let active_quest = reader.read_le()?;

        let num_known_topics = reader.read_le::<u16>()? as usize;
        let mut known_topics = Vec::with_capacity(num_known_topics);
        for _ in 0..num_known_topics {
            known_topics.push(reader.read_le()?);
        }

        let num_open_quests = reader.read_le::<u16>()? as usize;
        let mut open_quests = Vec::with_capacity(num_open_quests);
        for _ in 0..num_open_quests {
            open_quests.push((reader.read_le()?, reader.read_le()?, reader.read_le()?));
        }

        let num_magic_effects = reader.read_le::<u32>()? as usize;
        let mut known_magic_effects = Vec::with_capacity(num_magic_effects);
        for _ in 0..num_magic_effects {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            known_magic_effects.push(buf);
        }

        let mut facegen_symmetric = [0u8; 200];
        reader.read_exact(&mut facegen_symmetric)?;

        let mut facegen_asymmetric = [0u8; 120];
        reader.read_exact(&mut facegen_asymmetric)?;

        let mut facegen_texture = [0u8; 200];
        reader.read_exact(&mut facegen_texture)?;

        let race = reader.read_le()?;
        let hair = reader.read_le()?;
        let eyes = reader.read_le()?;
        let hair_length = reader.read_le()?;

        let mut hair_color = [0u8; 3];
        reader.read_exact(&mut hair_color)?;

        let stat_unknown8 = reader.read_le()?;

        let is_female = reader.read_le::<u8>()? != 0;
        let name = read_bzstring(&mut reader)?;
        let class = reader.read_le()?;

        // we could check if the form ID of the class was FORM_PLAYER_CUSTOM_CLASS, but that would
        // require the save to be passed into this function so we could look up the iref. to avoid
        // that, we just check if there are more bytes, and if so, assume there's a class
        let here = reader.seek(SeekFrom::Current(0))?;
        let there = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(here))?;
        let custom_class = if there - here > 4 {
            Some(Class::read_custom(&mut reader)?)
        } else {
            None
        };

        let stat_unknown9 = reader.read_le()?;

        Ok(PlayerReferenceChange {
            flags,
            cell,
            x,
            y,
            z,
            rx,
            ry,
            rz,
            temp_active_effects,
            tac_unknown,
            damage,
            health_delta,
            magicka_delta,
            fatigue_delta,
            actor_flag,
            inventory,
            properties,
            raw,
            statistics,
            stat_unknown1,
            birthsign,
            stat_unknown2,
            stat_unknown3,
            stat_unknown4,
            stat_unknown5,
            oblivion_doors,
            stat_unknown6,
            stat_active_effects,
            skill_xp,
            advancements,
            spec_increases,
            skill_usage,
            major_skill_advancements,
            stat_unknown7,
            active_quest,
            known_topics,
            open_quests,
            known_magic_effects,
            facegen_symmetric,
            facegen_asymmetric,
            facegen_texture,
            race,
            hair,
            eyes,
            hair_length,
            hair_color,
            stat_unknown8,
            is_female,
            name,
            class,
            custom_class,
            stat_unknown9,
        })
    }

    /// Writes a player reference change to a raw change record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    fn write(&self, record: &mut ChangeRecord) -> Result<(), TesError> {
        let mut buf: Vec<u8> = vec![];
        let mut writer = Cursor::new(&mut buf);

        writer.write_le(&self.cell)?;
        writer.write_le(&self.x)?;
        writer.write_le(&self.y)?;
        writer.write_le(&self.z)?;
        writer.write_le(&self.rx)?;
        writer.write_le(&self.ry)?;
        writer.write_le(&self.rz)?;

        for effect in self.temp_active_effects.values() {
            writer.write_le(&effect)?;
        }

        for unknown in self.tac_unknown.values() {
            writer.write_le(&unknown)?;
        }

        for damage in self.damage.values() {
            writer.write_le(&damage)?;
        }

        writer.write_le(&self.health_delta)?;
        writer.write_le(&self.magicka_delta)?;
        writer.write_le(&self.fatigue_delta)?;

        writer.write_le(&Into::<u8>::into(self.actor_flag))?;

        if self.flags & 0x08000000 != 0 {
            writer.write_le(&(self.inventory.len() as u16))?;
            for item in self.inventory.iter() {
                item.write(&mut writer)?;
            }
        }

        writer.write_le(&(self.properties.len() as u16))?;
        for property in self.properties.iter() {
            property.write(&mut writer)?;
        }

        writer.write_all(&self.raw)?;

        for stat in self.statistics.iter() {
            writer.write_le(&stat)?;
        }

        writer.write_all(&self.stat_unknown1)?;
        writer.write_le(&self.birthsign)?;
        for unknown in self.stat_unknown2.iter() {
            writer.write_le(&unknown)?;
        }

        writer.write_le(&(self.stat_unknown4.len() as u16))?;
        writer.write_all(&self.stat_unknown3)?;
        for unknown in self.stat_unknown4.iter() {
            writer.write_le(&unknown)?;
        }
        writer.write_all(&self.stat_unknown5)?;

        writer.write_le(&(self.oblivion_doors.len() as u16))?;
        for (door, flag) in self.oblivion_doors.iter() {
            writer.write_le(&door)?;
            writer.write_le(&flag)?;
        }

        writer.write_all(&self.stat_unknown6)?;

        writer.write_le(&(self.stat_active_effects.len() as u16))?;
        for effect in self.stat_active_effects.iter() {
            effect.write(&mut writer)?;
        }

        for skill in self.skill_xp.values() {
            writer.write_le(&skill)?;
        }

        writer.write_le(&(self.advancements.len() as u32))?;
        for adv in self.advancements.iter().flat_map(|v| v.values()) {
            writer.write_le(&adv)?;
        }

        for spec in self.spec_increases.values() {
            writer.write_le(&spec)?;
        }

        for skill in self.skill_usage.values() {
            writer.write_le(&skill)?;
        }

        writer.write_le(&self.major_skill_advancements)?;
        writer.write_le(&self.stat_unknown7)?;
        writer.write_le(&self.active_quest)?;

        writer.write_le(&(self.known_topics.len() as u16))?;
        for topic in self.known_topics.iter() {
            writer.write_le(&topic)?;
        }

        writer.write_le(&(self.open_quests.len() as u16))?;
        for (quest, stage, log_entry) in self.open_quests.iter() {
            writer.write_le(&quest)?;
            writer.write_le(&stage)?;
            writer.write_le(&log_entry)?;
        }

        writer.write_le(&(self.known_magic_effects.len() as u32))?;
        for effect in self.known_magic_effects.iter() {
            writer.write_all(effect)?;
        }

        writer.write_all(&self.facegen_symmetric)?;
        writer.write_all(&self.facegen_asymmetric)?;
        writer.write_all(&self.facegen_texture)?;

        writer.write_le(&self.race)?;
        writer.write_le(&self.hair)?;
        writer.write_le(&self.eyes)?;
        writer.write_le(&self.hair_length)?;
        writer.write_all(&self.hair_color)?;
        writer.write_le(&self.stat_unknown8)?;
        writer.write_le(&(if self.is_female { 1u8 } else { 0u8 }))?;
        write_bzstring(&mut writer, &self.name)?;
        writer.write_le(&self.class)?;

        if let Some(ref custom_class) = self.custom_class {
            custom_class.write_custom(&mut writer)?;
        }

        writer.write_le(&self.stat_unknown9)?;

        record.set_data(self.flags, buf)?;

        Ok(())
    }
}

impl PlayerReferenceChange {
    /// Gets the player's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the player's name
    ///
    /// # Errors
    ///
    /// Fails if the player's name is longer than [`MAX_BSTRING`]
    ///
    /// [`MAX_BSTRING`]: constant.MAX_BSTRING.html
    pub fn set_name(&mut self, name: String) -> Result<(), TesError> {
        check_size(&name, MAX_BSTRING, "Player name too long")?;
        self.name = name;
        Ok(())
    }

    /// Gets the player's custom class, if any
    pub fn class(&self) -> Option<&Class> {
        self.custom_class.as_ref()
    }

    /// Sets the player's custom class
    pub fn set_class(&mut self, class: Option<Class>, iref: u32) {
        self.class = iref;
        self.custom_class = class;
    }

    /// Gets the player's race as an iref
    pub fn race(&self) -> u32 {
        self.race
    }

    /// Sets the player's race as an iref
    pub fn set_race(&mut self, race: u32) {
        self.race = race;
    }

    /// Gets the player's birthsign as an iref
    pub fn birthsign(&self) -> u32 {
        self.birthsign
    }

    /// Sets the player's birthsign as an iref
    pub fn set_birthsign(&mut self, birthsign: u32) {
        self.birthsign = birthsign;
    }

    /// Iterates through the player's known magic effects
    pub fn known_magic_effects(&self) -> impl Iterator<Item = [u8; 4]> + '_ {
        self.known_magic_effects.iter().copied()
    }

    /// Sets the player's known magic effects
    pub fn set_known_magic_effects(&mut self, effects: Vec<[u8; 4]>) {
        self.known_magic_effects = effects;
    }

    /// Iterates through the player's active magic effects
    pub fn active_magic_effects(&self) -> impl Iterator<Item = &ActiveEffect> + '_ {
        self.stat_active_effects.iter()
    }

    /// Clears the player's active magic effects
    pub fn clear_active_magic_effects(&mut self) {
        self.stat_active_effects.clear();
    }

    /// Gets changes to player actor values from active effects
    pub fn active_effect_modifiers(&self) -> &ActorValues<f32> {
        &self.temp_active_effects
    }

    /// Gets changes to player actor values from active effects, mutably
    pub fn active_effect_modifiers_mut(&mut self) -> &mut ActorValues<f32> {
        &mut self.temp_active_effects
    }

    /// Gets damage to player actor values
    pub fn damage_modifiers(&self) -> &ActorValues<f32> {
        &self.damage
    }

    /// Gets damage to player actor values
    pub fn damage_modifiers_mut(&mut self) -> &mut ActorValues<f32> {
        &mut self.damage
    }

    /// Gets the change in the player's health
    pub fn health_delta(&self) -> f32 {
        self.health_delta
    }

    /// Sets the change in the player's health
    pub fn set_health_delta(&mut self, value: f32) {
        self.health_delta = value;
    }

    /// Gets the change in the player's magicka
    pub fn magicka_delta(&self) -> f32 {
        self.magicka_delta
    }

    /// Sets the change in the player's magicka
    pub fn set_magicka_delta(&mut self, value: f32) {
        self.magicka_delta = value;
    }

    /// Gets the change in the player's fatigue
    pub fn fatigue_delta(&self) -> f32 {
        self.fatigue_delta
    }

    /// Sets the change in the player's fatigue
    pub fn set_fatigue_delta(&mut self, value: f32) {
        self.fatigue_delta = value;
    }

    /// Clears the player's inventory
    ///
    /// Note that this is not the same as making the inventory empty; rather, it reverts the
    /// inventory to its initial state at the start of the game.
    pub fn clear_inventory(&mut self) {
        self.inventory.clear();
    }

    /// Add an item stack to the player's inventory
    pub fn add_item(&mut self, item: InventoryItem) {
        self.inventory.push(item);
    }

    /// Iterate through the player's inventory
    pub fn iter_inventory(&self) -> impl Iterator<Item = &InventoryItem> {
        self.inventory.iter()
    }

    /// Iterate through the player's inventory mutably
    pub fn iter_inventory_mut(&mut self) -> impl Iterator<Item = &mut InventoryItem> {
        self.inventory.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tes4::save::{Save, TEST_SAVE};
    use std::io::Cursor;

    #[test]
    fn read_player_ref_change() {
        let mut record_ref = TEST_SAVE.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let save = Save::read(cursor).unwrap();
        let player = save.get_change_record(FORM_PLAYER_REF).unwrap();
        let player_change = PlayerReferenceChange::read(player).unwrap();
        assert_eq!(player_change.name, "test");
        assert!(!player_change.is_female);
    }

    #[test]
    fn write_player_ref_change() {
        let mut record_ref = TEST_SAVE.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let mut save = Save::read(cursor).unwrap();
        let mut player = save.get_change_record_mut(FORM_PLAYER_REF).unwrap();
        let original = player.data().to_vec();
        let player_change = PlayerReferenceChange::read(player).unwrap();
        player_change.write(&mut player).unwrap();
        assert_eq!(original, player.data());
    }
}
