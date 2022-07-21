use crate::tes3::{Enchantable, Item, Tes3Field, Tes3Record};
use crate::{decode_failed, Field, Form, Record, TesError};
use binrw::{binrw, BinReaderExt};
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct WeaponFlags: u32 {
        const IGNORE_NORMAL_WEAPON_RESISTANCE = 0x01;
        const SILVER = 0x02;
    }
}

#[binrw]
#[derive(Debug, Default, Eq, PartialEq)]
#[repr(u16)]
#[brw(repr = u16)]
pub enum WeaponType {
    #[default]
    ShortBladeOneHand,
    LongBladeOneHand,
    LongBladeTwoClose,
    BluntOneHand,
    BluntTwoClose,
    BluntTwoWide,
    SpearTwoWide,
    AxeOneHand,
    AxeTwoHand,
    MarksmanBow,
    MarksmanCrossbow,
    MarksmanThrown,
    Arrow,
    Bolt,
}

#[binrw]
#[derive(Debug, Default)]
pub struct WeaponData {
    pub weight: f32,
    pub value: u32,
    pub weapon_type: WeaponType,
    pub health: u16,
    pub speed: f32,
    pub reach: f32,
    pub enchantment_points: u16,
    pub min_chop: u8,
    pub max_chop: u8,
    pub min_slash: u8,
    pub max_slash: u8,
    pub min_thrust: u8,
    pub max_thrust: u8,
    #[br(try_map = |f| WeaponFlags::from_bits(f).ok_or("Invalid weapon flags"))]
    #[bw(map = |f| f.bits)]
    flags: WeaponFlags,
}

#[derive(Debug, Default)]
pub struct Weapon {
    id: String,
    model: String,
    name: Option<String>,
    pub data: WeaponData,
    icon: Option<String>,
    enchantment: Option<String>,
    script: Option<String>,
}

impl Item for Weapon {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }

    fn model(&self) -> Option<&str> {
        Some(self.model.as_str())
    }

    fn set_model(&mut self, model: Option<String>) {
        self.model = model.unwrap_or_else(String::new);
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    fn weight(&self) -> f32 {
        self.data.weight
    }

    fn set_weight(&mut self, weight: f32) {
        self.data.weight = weight;
    }

    fn value(&self) -> u32 {
        self.data.value
    }

    fn set_value(&mut self, value: u32) {
        self.data.value = value;
    }

    fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    fn set_script(&mut self, script: Option<String>) {
        self.script = script;
    }

    fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    fn set_icon(&mut self, icon: Option<String>) {
        self.icon = icon;
    }
}

impl Form for Weapon {
    type Field = Tes3Field;
    type Record = Tes3Record;
    const RECORD_TYPE: &'static [u8; 4] = b"WEAP";

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Weapon::assert(&record)?;

        let mut weapon = Weapon::default();
        for field in record.iter() {
            match field.name() {
                b"NAME" => weapon.id = String::from(field.get_zstring()?),
                b"MODL" => weapon.model = String::from(field.get_zstring()?),
                b"FNAM" => weapon.name = Some(String::from(field.get_zstring()?)),
                b"WPDT" => weapon.data = field.reader().read_le()?,
                b"ITEX" => weapon.icon = Some(String::from(field.get_zstring()?)),
                b"ENAM" => weapon.enchantment = Some(String::from(field.get_zstring()?)),
                b"SCRI" => weapon.script = Some(String::from(field.get_zstring()?)),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected {} field in WEAP record",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(weapon)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        todo!()
    }
}

impl Enchantable for Weapon {
    fn enchantment(&self) -> Option<&str> {
        self.enchantment.as_deref()
    }

    fn set_enchantment(&mut self, enchantment: Option<String>) {
        self.enchantment = enchantment;
    }

    fn enchantment_points(&self) -> u32 {
        self.data.enchantment_points as u32
    }

    fn set_enchantment_points(&mut self, enchantment_points: u32) {
        self.data.enchantment_points = enchantment_points as u16;
    }
}

impl Weapon {
    pub fn ignores_normal_weapon_resistance(&self) -> bool {
        self.data
            .flags
            .contains(WeaponFlags::IGNORE_NORMAL_WEAPON_RESISTANCE)
    }

    pub fn set_ignores_normal_weapon_resistance(&mut self, ignore: bool) {
        self.data
            .flags
            .set(WeaponFlags::IGNORE_NORMAL_WEAPON_RESISTANCE, ignore);
    }

    pub fn is_silver(&self) -> bool {
        self.data.flags.contains(WeaponFlags::SILVER)
    }

    pub fn set_is_silver(&mut self, is_silver: bool) {
        self.data.flags.set(WeaponFlags::SILVER, is_silver);
    }
}
