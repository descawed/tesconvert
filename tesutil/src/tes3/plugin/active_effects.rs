use std::convert::TryFrom;

use super::{Tes3Field, Tes3Record};
use crate::{decode_failed, decode_failed_because, read_string, Field, Form, Record, TesError};

use binrw::BinReaderExt;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::io::{Read, Seek, SeekFrom};

/// Type of magical spell/item an effect originated from
#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum MagicType {
    Spell = 1,
    Enchantment,
    Potion,
}

/// An associated item of an active magical effect
#[derive(Debug)]
pub struct EffectAssociatedItem {
    unknown1: i32,
    unknown2: u8,
    id: String,
}

/// An active effect of a magical spell
#[derive(Debug)]
pub struct ActiveEffect {
    // NPDT
    affected_actor: String,
    index: i32,
    unknown1: [u8; 4],
    magnitude: i32,
    seconds_active: f32,
    unknown2: [u8; 8],
    // INAM
    // TODO: can there actually be more than one of these? OpenMW says it's used for both bound item and item to re-equip
    associated_items: Vec<EffectAssociatedItem>,
    // CNAM
    summon: Option<String>,
    // VNAM
    // TODO: figure out what's in here
    vampirism: Vec<u8>,
}

impl ActiveEffect {
    pub fn affected_actor(&self) -> &str {
        self.affected_actor.as_str()
    }

    pub fn index(&self) -> i32 {
        self.index
    }

    pub fn magnitude(&self) -> i32 {
        self.magnitude
    }

    pub fn seconds_active(&self) -> f32 {
        self.seconds_active
    }
}

/// An active magical spell
#[derive(Debug)]
pub struct ActiveSpell {
    // NAME
    index: i32,
    // SPDT
    magic_type: MagicType,
    id: String,
    unknown1: [u8; 16],
    caster: String,
    source: String,
    unknown2: [u8; 44],
    // TNAM
    target: Option<String>,
    effects: Vec<ActiveEffect>,
}

impl ActiveSpell {
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn effects(&self) -> impl Iterator<Item = &ActiveEffect> + '_ {
        self.effects.iter()
    }
}

/// Maximum length of an ID on an ActiveEffect
pub const ID_LENGTH: usize = 32;

/// Maximum length of an associated item ID on an ActiveEffect
pub const ASSOCIATED_ID_LENGTH: usize = 35;

/// All active magical spells in the save game
#[derive(Debug)]
pub struct ActiveSpellList(Vec<ActiveSpell>);

impl Form for ActiveSpellList {
    type Field = Tes3Field;
    type Record = Tes3Record;

    fn record_type() -> &'static [u8; 4] {
        b"SPLM"
    }

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        ActiveSpellList::assert(record)?;

        let mut list = ActiveSpellList(vec![]);

        for field in record.iter() {
            match field.name() {
                b"NAME" => list.0.push(ActiveSpell {
                    index: field.get_i32()?,
                    magic_type: MagicType::Spell,
                    id: String::new(),
                    unknown1: [0; 16],
                    caster: String::new(),
                    source: String::new(),
                    unknown2: [0; 44],
                    target: None,
                    effects: vec![],
                }),
                b"SPDT" => {
                    let spell = list
                        .0
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned SPDT in SPLM"))?;
                    let mut reader = field.reader();
                    spell.magic_type = MagicType::try_from(reader.read_le::<u32>()? as u8)
                        .map_err(|e| decode_failed_because("Invalid magic type in SPDT", e))?;
                    spell.id = read_string::<ID_LENGTH, _>(&mut reader)?;
                    reader.read_exact(&mut spell.unknown1)?;
                    spell.caster = read_string::<ID_LENGTH, _>(&mut reader)?;
                    spell.source = read_string::<ID_LENGTH, _>(&mut reader)?;
                    reader.read_exact(&mut spell.unknown2)?;
                }
                // TODO: I'm actually not sure if this field is a string or a zstring. OpenMW seems to parse both the same.
                b"TNAM" => {
                    list.0
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned TNAM in SPLM"))?
                        .target = Some(String::from(field.get_string()?))
                }
                b"NPDT" => {
                    let mut reader = field.reader();
                    let spell = list
                        .0
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned NPDT in SPLM"))?;
                    let effect = ActiveEffect {
                        affected_actor: read_string::<ID_LENGTH, _>(&mut reader)?,
                        index: reader.read_le()?,
                        unknown1: {
                            let mut buf = [0; 4];
                            reader.read_exact(&mut buf)?;
                            buf
                        },
                        magnitude: reader.read_le()?,
                        seconds_active: reader.read_le()?,
                        unknown2: {
                            let mut buf = [0; 8];
                            reader.read_exact(&mut buf)?;
                            buf
                        },
                        associated_items: vec![],
                        summon: None,
                        vampirism: vec![],
                    };

                    spell.effects.push(effect);
                }
                b"INAM" => {
                    let mut reader = field.reader();
                    let spell = list
                        .0
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned INAM in SPLM"))?;
                    let effect = spell
                        .effects
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned INAM in SPLM"))?;
                    let associated_item = EffectAssociatedItem {
                        unknown1: reader.read_le()?,
                        unknown2: reader.read_le()?,
                        id: read_string::<ASSOCIATED_ID_LENGTH, _>(&mut reader)?,
                    };
                    effect.associated_items.push(associated_item);
                }
                b"CNAM" => {
                    let mut reader = field.reader();
                    let spell = list
                        .0
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned CNAM in SPLM"))?;
                    let effect = spell
                        .effects
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned CNAM in SPLM"))?;
                    reader.seek(SeekFrom::Current(4))?; // always 0 according to OpenMW
                    effect.summon = Some(read_string::<ID_LENGTH, _>(&mut reader)?);
                }
                b"VNAM" => {
                    let spell = list
                        .0
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned VNAM in SPLM"))?;
                    let effect = spell
                        .effects
                        .last_mut()
                        .ok_or_else(|| decode_failed("Orphaned VNAM in SPLM"))?;
                    effect.vampirism = field.get().to_vec();
                }
                b"NAM0" => (), // end of effect
                b"XNAM" => (), // end of spell
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(list)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        unimplemented!()
    }
}

impl IntoIterator for ActiveSpellList {
    type Item = ActiveSpell;
    type IntoIter = <Vec<ActiveSpell> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ActiveSpellList {
    pub fn iter(&self) -> impl Iterator<Item = &ActiveSpell> + '_ {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ActiveSpell> + '_ {
        self.0.iter_mut()
    }
}
