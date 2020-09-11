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
    editor_id: Option<String>, // not present if this is a custom class
    name: String,
    description: Option<String>,
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
            editor_id: None,
            name: String::new(),
            description: None,
            icon: None,
            primary_attributes: [Attribute::Strength; 2],
            specialization: Specialization::Combat,
            major_skills: [Skill::Armorer; 7],
            is_playable: false,
            is_guard: false,
            services: ServiceFlags::empty(),
            skill_trained: Skill::Armorer,
            max_training_level: 0,
        };

        for field in record.iter() {
            match field.name() {
                b"EDID" => class.editor_id = Some(String::from(field.get_zstring()?)),
                b"FULL" => class.name = String::from(field.get_zstring()?),
                b"DESC" => class.description = Some(String::from(field.get_zstring()?)),
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
                    // 2 additional unused bytes at the end
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

impl Class {
    //noinspection RsTypeCheck
    /// Reads a custom class from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn read_custom<T: Read>(mut f: T) -> Result<Class, TesError> {
        let mut primary_attributes = [Attribute::Strength; 2];
        for attr in &mut primary_attributes {
            *attr = ActorValue::try_from(extract!(f as u32)? as u8)
                .map_err(|e| decode_failed_because("Invalid attribute value", e))?
                .try_into()?;
        }

        let specialization = Specialization::try_from(extract!(f as u32)? as u8)
            .map_err(|e| decode_failed_because("Invalid specialization", e))?;

        let mut major_skills = [Skill::Acrobatics; 7];
        for skill in &mut major_skills {
            *skill = ActorValue::try_from(extract!(f as u32)? as u8)
                .map_err(|e| decode_failed_because("Invalid skill value", e))?
                .try_into()?;
        }

        let flags = ClassFlags::from_bits(extract!(f as u32)?)
            .ok_or_else(|| decode_failed("Invalid class flags"))?;
        let is_playable = flags.contains(ClassFlags::PLAYABLE);
        let is_guard = flags.contains(ClassFlags::GUARD);

        let services = ServiceFlags::from_bits(extract!(f as u32)?)
            .ok_or_else(|| decode_failed("Invalid service flags"))?;
        let skill_trained = Skill::try_from(extract!(f as u8)?)
            .map_err(|e| decode_failed_because("Invalid training skill value", e))?;
        let max_training_level = extract!(f as u8)?;
        extract!(f as u16)?; // dummy
        let name = extract_bstring(&mut f)?;
        let icon = Some(extract_bstring(&mut f)?);

        Ok(Class {
            editor_id: None,
            name,
            description: None,
            icon,
            primary_attributes,
            specialization,
            major_skills,
            is_playable,
            is_guard,
            services,
            skill_trained,
            max_training_level,
        })
    }

    /// Writes a custom class to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write_custom<T: Write>(&self, mut f: T) -> Result<(), TesError> {
        for attribute in self.primary_attributes.iter() {
            serialize!(<Attribute as Enum<()>>::to_usize(*attribute) as u32 => f)?;
        }

        serialize!(<Specialization as Enum<()>>::to_usize(self.specialization) as u32 => f)?;

        for skill in self.major_skills.iter() {
            let av: ActorValue = (*skill).into();
            serialize!(<ActorValue as Enum<()>>::to_usize(av) as u32 => f)?;
        }

        let mut class_flags = ClassFlags::empty();
        if self.is_playable {
            class_flags |= ClassFlags::PLAYABLE;
        }
        if self.is_guard {
            class_flags |= ClassFlags::GUARD;
        }

        serialize!(class_flags.bits => f)?;
        serialize!(self.services.bits => f)?;
        serialize!(self.skill_trained as u8 => f)?;
        serialize!(self.max_training_level => f)?;
        serialize!(0u16 => f)?;
        serialize_bstring(&mut f, &self.name)?;
        serialize_bstring(
            &mut f,
            match self.icon.as_ref() {
                Some(icon) => &icon[..],
                None => "",
            },
        )?;

        Ok(())
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
        assert_eq!(class.editor_id.unwrap(), "SE32Smith");
        assert_eq!(class.name, "Vitharn Smith");
        assert_eq!(class.description.unwrap(), "");
        assert!(class.icon.is_none());
    }
}
