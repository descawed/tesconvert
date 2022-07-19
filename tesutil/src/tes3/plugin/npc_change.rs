use super::field::Tes3Field;
use super::record::Tes3Record;
use crate::plugin::Field;
use crate::*;

use crate::tes3::Package;
use binrw::{binrw, BinReaderExt};

/// NPC disposition and reputation
#[binrw]
#[derive(Debug, Default)]
struct Disposition {
    disposition: u8,
    unknown1: u8,
    reputation: u8,
    unknown2: u8,
    index: i32,
}

/// Script state attached to an item
#[derive(Debug, Default, Clone)]
pub struct Script {
    pub name: String,
    pub shorts: Vec<i16>,
    pub longs: Vec<i32>,
    pub floats: Vec<f32>,
}

/// An item in the NPC's inventory
#[derive(Debug, Default, Clone)]
pub struct InventoryItem {
    pub id: String,
    pub count: u32,
    pub is_equipped: bool,
    pub soul: Option<String>,
    pub enchantment_charge: Option<f32>,
    pub remaining_durability: Option<u32>,
    pub script: Option<Script>,
}

impl InventoryItem {
    pub fn is_pristine(&self) -> bool {
        !self.is_equipped
            && self.soul.is_none()
            && self.enchantment_charge.is_none()
            && self.remaining_durability.is_none()
            && self.script.is_none()
    }
}

/// Changes to an NPC (or the PC) in a save game
#[derive(Debug, Default)]
pub struct NpcChange {
    id: String,
    disposition: Disposition,
    inventory: Vec<InventoryItem>,
    packages: Vec<Package>,
}

impl NpcChange {
    pub fn iter_inventory(&self) -> impl Iterator<Item = &InventoryItem> {
        self.inventory.iter()
    }
}

impl Form for NpcChange {
    type Field = Tes3Field;
    type Record = Tes3Record;

    const RECORD_TYPE: &'static [u8; 4] = b"NPCC";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        NpcChange::assert(&record)?;

        let mut npc_change = NpcChange::default();

        let mut stack_start = 0;
        let mut num_shorts = 0;
        let mut num_longs = 0;
        let mut num_floats = 0;
        let mut base_indexes = vec![];
        for field in record.iter() {
            match field.name() {
                b"NAME" => npc_change.id = String::from(field.get_zstring()?),
                b"NPDT" => npc_change.disposition = field.reader().read_le()?,
                b"NPCO" => {
                    let mut reader = field.reader();
                    let item = InventoryItem {
                        count: reader.read_le()?,
                        id: read_string::<32, _>(reader)?,
                        ..InventoryItem::default()
                    };
                    stack_start = npc_change.inventory.len();
                    base_indexes.push(stack_start);
                    npc_change.inventory.push(item);
                }
                b"XIDX" => {
                    // start a new stack of a single item. the number of "pristine" items is the count
                    // from the NPCO record minus the number of XIDXs, so we decrease the original count
                    // each time we see one.
                    let first = &mut npc_change.inventory[stack_start];
                    first.count -= 1;
                    let mut new = first.clone();
                    new.count = 1;
                    npc_change.inventory.push(new);
                }
                b"SCRI" => {
                    let script = Script {
                        name: String::from(field.get_zstring()?),
                        shorts: vec![],
                        longs: vec![],
                        floats: vec![],
                    };
                    npc_change
                        .inventory
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned SCRI field"))?
                        .script = Some(script);
                    num_shorts = 0;
                    num_longs = 0;
                    num_floats = 0;
                }
                b"SLCS" => {
                    let mut reader = field.reader();
                    num_shorts = reader.read_le()?;
                    num_longs = reader.read_le()?;
                    num_floats = reader.read_le()?;
                }
                b"SLSD" => {
                    let script = npc_change
                        .inventory
                        .last_mut()
                        .and_then(|i| i.script.as_mut())
                        .ok_or_else(|| decode_failed("Orphaned SLSD field"))?;
                    let mut reader = field.reader();
                    for _ in 0..num_shorts {
                        script.shorts.push(reader.read_le()?);
                    }
                }
                b"SLLD" => {
                    let script = npc_change
                        .inventory
                        .last_mut()
                        .and_then(|i| i.script.as_mut())
                        .ok_or_else(|| decode_failed("Orphaned SLLD field"))?;
                    let mut reader = field.reader();
                    for _ in 0..num_longs {
                        script.longs.push(reader.read_le()?);
                    }
                }
                b"SLFD" => {
                    let script = npc_change
                        .inventory
                        .last_mut()
                        .and_then(|i| i.script.as_mut())
                        .ok_or_else(|| decode_failed("Orphaned SLFD field"))?;
                    let mut reader = field.reader();
                    for _ in 0..num_floats {
                        script.floats.push(reader.read_le()?);
                    }
                }
                b"XSOL" => {
                    let item = npc_change
                        .inventory
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned XSOL field"))?;
                    item.soul = Some(String::from(field.get_zstring()?));
                }
                b"XCHG" => {
                    let item = npc_change
                        .inventory
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned XCHG field"))?;
                    item.enchantment_charge = Some(field.get_f32()?);
                }
                b"XHLT" => {
                    let item = npc_change
                        .inventory
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned XHLT field"))?;
                    item.remaining_durability = Some(field.get_u32()?);
                }
                b"AI_A" | b"AI_E" | b"AI_F" | b"AI_T" | b"AI_W" => {
                    npc_change.packages.push(Package::read(&field)?)
                }
                b"CNDT" => Package::read_cell_name(npc_change.packages.last_mut(), &field)?,
                b"WIDX" => {
                    // index is NPCO index, slot is XIDX index
                    let mut reader = field.reader();
                    let index: u32 = reader.read_le()?;
                    let slot: u32 = reader.read_le()?;
                    let final_index = base_indexes[index as usize] + (slot as usize) + 1;
                    npc_change.inventory[final_index].is_equipped = true;
                }
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.name_as_str()
                    )))
                }
            }
        }

        // remove any zero-count entries from stacks which have no pristine copies
        npc_change.inventory.retain(|i| i.count > 0);

        Ok(npc_change)
    }

    fn write(&self, record: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    static NPCC_RECORD: &[u8] = include_bytes!("test/npcc_record.bin");

    #[test]
    fn parse_record() {
        let mut record_ref = NPCC_RECORD.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let record = Tes3Record::read(cursor).unwrap();
        let npc_change = NpcChange::read(&record).unwrap();

        assert_eq!(npc_change.id, "PlayerSaveGame");
        // check inventory contents
        let inventory_ids = npc_change
            .inventory
            .iter()
            .map(|i| i.id.as_ref())
            .collect::<Vec<&str>>();
        assert!(inventory_ids.contains(&"bk_a1_1_directionscaiuscosades"));
        assert!(inventory_ids.contains(&"exquisite_ring_processus"));
        // check equipped items
        let mut equipped_ids = npc_change
            .inventory
            .iter()
            .filter_map(|i| {
                if i.is_equipped {
                    Some(i.id.as_ref())
                } else {
                    None
                }
            })
            .collect::<Vec<&str>>();
        equipped_ids.sort();
        assert_eq!(
            equipped_ids,
            [
                "common_pants_01",
                "common_shirt_01",
                "common_shoes_01",
                "exquisite_ring_processus",
                "iron club",
                "ring_keley"
            ]
        );
    }
}
