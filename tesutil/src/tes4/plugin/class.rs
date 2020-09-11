#![allow(clippy::single_component_path_imports)]

use std::convert::{TryFrom, TryInto};
use std::io::Read;

use crate::tes4::plugin::*;
use crate::tes4::{ActorValue, Skill};
use crate::Form;

// this line is only to help the IDE
use bitflags;

bitflags! {
    struct ClassFlags: u32 {
        const PLAYABLE = 0x01;
        const GUARD = 0x02;
    }
}

bitflags! {
    struct ServiceFlags: u32 {
        const WEAPONS = 0x00001;
        const ARMOR = 0x00002;
        const CLOTHING = 0x00004;
        const BOOKS = 0x00008;
        const INGREDIENTS = 0x00010;
        const LIGHTS = 0x00080;
        const APPARATUS = 0x00100;
        const MISCELLANEOUS = 0x00400;
        const SPELLS = 0x00800;
        const MAGIC_ITEMS = 0x01000;
        const POTIONS = 0x02000;
        const TRAINING = 0x04000;
        const RECHARGE = 0x10000;
        const REPAIR = 0x20000;
    }
}

/// A character class
pub struct Class {
    editor_id: String,
    name: String,
    description: String,
    icon: Option<String>,
    primary_attributes: [Attribute; 2],
    specialization: Specialization,
    major_skills: [Skill; 7],
    pub is_playable: bool,
    pub is_guard: bool,
    services: ServiceFlags,
    skill_trained: Skill,
    max_training_level: u8,
}

impl Form for Class {
    type Field = Tes4Field;
    type Record = Tes4Record;

    fn record_type() -> &'static [u8; 4] {
        b"CLAS"
    }

    //noinspection RsTypeCheck
    /// Reads class data from a raw record
    ///
    /// # Errors
    ///
    /// Fails if the provided record is not a `b"CLAS"` record or if the record data is invalid.
    fn read(record: &Tes4Record) -> Result<Class, TesError> {
        Class::assert(record)?;

        let mut class = Class {
            editor_id: String::new(),
            name: String::new(),
            description: String::new(),
            icon: None,
            primary_attributes: [Attribute::Strength, Attribute::Strength],
            specialization: Specialization::Combat,
            major_skills: [
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
            ],
            is_playable: false,
            is_guard: false,
            services: ServiceFlags::empty(),
            skill_trained: Skill::Armorer,
            max_training_level: 0,
        };

        for field in record.iter() {
            match field.name() {
                b"EDID" => class.editor_id = String::from(field.get_zstring()?),
                b"FULL" => class.name = String::from(field.get_zstring()?),
                b"DESC" => class.description = String::from(field.get_zstring()?),
                b"ICON" => class.icon = Some(String::from(field.get_zstring()?)),
                b"DATA" => {
                    let reader = &mut field.get();
                    for attr in &mut class.primary_attributes {
                        *attr = ActorValue::try_from(extract!(reader as u32)? as u8)
                            .map_err(|e| decode_failed_because("Invalid attribute value", e))?
                            .try_into()?;
                    }
                    class.specialization = Specialization::try_from(extract!(reader as u32)? as u8)
                        .map_err(|e| decode_failed_because("Invalid specialization", e))?;
                    for skill in class.major_skills.iter_mut() {
                        *skill = ActorValue::try_from(extract!(reader as u32)? as u8)
                            .map_err(|e| decode_failed_because("Invalid skill value", e))?
                            .try_into()?;
                    }
                    let class_flags = ClassFlags::from_bits(extract!(reader as u32)?)
                        .ok_or_else(|| decode_failed("Invalid class flags"))?;
                    class.is_playable = class_flags.contains(ClassFlags::PLAYABLE);
                    class.is_guard = class_flags.contains(ClassFlags::GUARD);
                    class.services = ServiceFlags::from_bits(extract!(reader as u32)?)
                        .ok_or_else(|| decode_failed("Invalid service flags"))?;
                    class.skill_trained = Skill::try_from(extract!(reader as u8)?)
                        .map_err(|e| decode_failed_because("Invalid training skill value", e))?;
                    class.max_training_level = extract!(reader as u8)?;
                }
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.display_name()
                    )))
                }
            }
        }

        Ok(class)
    }

    fn write(&self, _: &mut Self::Record) -> Result<(), TesError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static CLASS_RECORD: &[u8] = include_bytes!("test/clas_record.bin");

    #[test]
    fn test_load() {
        let record = Tes4Record::read(&mut CLASS_RECORD.as_ref()).unwrap();
        let class = Class::read(&record).unwrap();
        assert_eq!(class.editor_id, "SE32Smith");
        assert_eq!(class.name, "Vitharn Smith");
        assert_eq!(class.description, "");
        assert!(class.icon.is_none());
    }
}
