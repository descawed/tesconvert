use crate::tes4::{ActorFlags, FormId, Skill, Skills, Tes4Field, Tes4Record};
use crate::{decode_failed, Attribute, Attributes, Field, Form, Record, TesError};

use binrw::{binrw, BinReaderExt, BinWriterExt};
use bitflags::bitflags;
use enum_map::Enum;

bitflags! {
    #[derive(Default)]
    struct AiFlags: u32 {
        const WEAPONS = 0x00000001;
        const ARMOR = 0x00000002;
        const CLOTHING = 0x00000004;
        const BOOKS = 0x00000008;
        const INGREDIENTS = 0x00000010;
        const LIGHTS = 0x00000080;
        const APPARATUS = 0x00000100;
        const MISCELLANEOUS = 0x00000400;
        const SPELLS = 0x00000800;
        const MAGIC_ITEMS = 0x00001000;
        const POTIONS = 0x00002000;
        const TRAINING = 0x00004000;
        const RECHARGE = 0x00010000;
        const REPAIR = 0x00020000;
    }
}

#[binrw]
#[derive(Debug, Default)]
pub struct ActorSettings {
    #[br(try_map = |f| ActorFlags::from_bits(f).ok_or("Invalid actor flags"))]
    #[bw(map = |f| f.bits)]
    flags: ActorFlags,
    base_spell: u16,
    fatigue: u16,
    barter_gold: u16,
    level: i16,
    calc_min: u16,
    calc_max: u16,
}

#[binrw]
#[derive(Debug, Default)]
pub struct FactionRank {
    id: FormId,
    rank: u8,
    unknown: [u8; 3],
}

#[binrw]
#[derive(Debug, Default)]
pub struct AiSettings {
    aggression: u8,
    confidence: u8,
    energy_level: u8,
    responsibility: u8,
    #[br(try_map = |f| AiFlags::from_bits(f).ok_or("Invalid AI flags"))]
    #[bw(map = |f| f.bits)]
    flags: AiFlags,
    train_skill: u8,
    train_level: u8,
    unknown: u16,
}

#[binrw]
#[derive(Debug, Default)]
pub struct ActorStats {
    #[br(map = |s: [u8; Skill::LENGTH]| Skills::from_array(s))]
    #[bw(map = |s| s.as_slice())]
    skills: Skills<u8>,
    health: u32,
    #[br(map = |a: [u8; Attribute::LENGTH]| Attributes::from_array(a))]
    #[bw(map = |a| a.as_slice())]
    attributes: Attributes<u8>,
}

#[derive(Debug)]
pub struct Npc {
    editor_id: String,
    name: String,
    model: String,
    modb: f32,
    actor_settings: ActorSettings,
    factions: Vec<FactionRank>,
    death_item: Option<FormId>,
    race: FormId,
    spells: Vec<FormId>,
    script: Option<FormId>,
    inventory: Vec<(FormId, i32)>,
    ai_settings: AiSettings,
    packages: Vec<FormId>,
    class: FormId,
    stats: ActorStats,
    hair: FormId,
    hair_length: Option<f32>,
    eyes: Option<FormId>,
    hair_color: (u8, u8, u8, u8),
    combat_style: Option<FormId>,
    fg_geo_sym: [u8; 200],
    fg_geo_asym: [u8; 120],
    fg_tex_sym: [u8; 200],
    face_race: u16,
}

impl Default for Npc {
    fn default() -> Self {
        Npc {
            editor_id: String::new(),
            name: String::new(),
            model: String::new(),
            modb: 0.,
            actor_settings: ActorSettings::default(),
            factions: vec![],
            death_item: None,
            race: FormId::default(),
            spells: vec![],
            script: None,
            inventory: vec![],
            ai_settings: AiSettings::default(),
            packages: vec![],
            class: FormId::default(),
            stats: ActorStats::default(),
            hair: FormId::default(),
            hair_length: None,
            eyes: None,
            hair_color: (0, 0, 0, 0),
            combat_style: None,
            fg_geo_sym: [0; 200],
            fg_geo_asym: [0; 120],
            fg_tex_sym: [0; 200],
            face_race: 0,
        }
    }
}

impl Npc {
    /// Iterate through the contents of the NPC's inventory
    pub fn iter_inventory(&self) -> impl Iterator<Item = (FormId, i32)> + '_ {
        self.inventory.iter().copied()
    }
}

impl Form for Npc {
    type Field = Tes4Field;
    type Record = Tes4Record;

    fn record_type() -> &'static [u8; 4] {
        b"NPC_"
    }

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Npc::assert(&record)?;

        let mut npc = Npc::default();

        for field in record.iter() {
            match field.name() {
                b"EDID" => npc.editor_id = String::from(field.get_zstring()?),
                b"FULL" => npc.name = String::from(field.get_zstring()?),
                b"MODL" => npc.model = String::from(field.get_zstring()?),
                b"MODB" => npc.modb = field.get_f32()?,
                b"ACBS" => npc.actor_settings = field.reader().read_le()?,
                b"SNAM" => npc.factions.push(field.reader().read_le()?),
                b"INAM" => npc.death_item = Some(FormId(field.get_u32()?)),
                b"RNAM" => npc.race = FormId(field.get_u32()?),
                b"SPLO" => npc.spells.push(FormId(field.get_u32()?)),
                b"SCRI" => npc.script = Some(FormId(field.get_u32()?)),
                b"CNTO" => {
                    let mut reader = field.reader();
                    npc.inventory
                        .push((FormId(reader.read_le()?), reader.read_le()?));
                }
                b"AIDT" => npc.ai_settings = field.reader().read_le()?,
                b"PKID" => npc.packages.push(FormId(field.get_u32()?)),
                b"CNAM" => npc.class = FormId(field.get_u32()?),
                b"DATA" => npc.stats = field.reader().read_le()?,
                b"HNAM" => npc.hair = FormId(field.get_u32()?),
                b"LNAM" => npc.hair_length = Some(field.get_f32()?),
                b"ENAM" => npc.eyes = Some(FormId(field.get_u32()?)),
                b"HCLR" => {
                    let mut reader = field.reader();
                    npc.hair_color = (
                        reader.read_le()?,
                        reader.read_le()?,
                        reader.read_le()?,
                        reader.read_le()?,
                    );
                }
                b"ZNAM" => npc.combat_style = Some(FormId(field.get_u32()?)),
                b"FGGS" => npc.fg_geo_sym.copy_from_slice(field.get()),
                b"FGGA" => npc.fg_geo_asym.copy_from_slice(field.get()),
                b"FGTS" => npc.fg_tex_sym.copy_from_slice(field.get()),
                b"FNAM" => npc.face_race = field.get_u16()?,
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected {} field in NPC_ record",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(npc)
    }

    fn write(&self, record: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}
