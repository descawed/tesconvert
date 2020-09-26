use std::convert::TryFrom;
use std::io::Read;

use crate::tes3::plugin::*;
use crate::tes3::{Skill, SkillType};

use bitflags::bitflags;

bitflags! {
    struct AutoCalcFlags: u32 {
        const WEAPON = 0x0001;
        const ARMOR = 0x0002;
        const CLOTHING = 0x0004;
        const BOOKS = 0x0008;
        const INGREDIENTS = 0x0010;
        const PICKS = 0x0020;
        const PROBES = 0x0040;
        const LIGHTS = 0x0080;
        const APPARATUS = 0x0100;
        const REPAIR_ITEMS = 0x0200;
        const MISC = 0x0400;
        const SPELLS = 0x0800;
        const MAGIC_ITEMS = 0x1000;
        const POTIONS = 0x2000;
        const TRAINING = 0x4000;
        const SPELLMAKING = 0x8000;
        const ENCHANTING = 0x10000;
        const REPAIR = 0x20000;
    }
}

/// A character class
#[derive(Debug)]
pub struct Class {
    id: String,
    name: String,
    description: Option<String>,
    primary_attributes: [Attribute; 2],
    pub specialization: Specialization,
    major_skills: [Skill; 5],
    minor_skills: [Skill; 5],
    pub is_playable: bool,
    auto_calc_flags: AutoCalcFlags,
}

impl Form for Class {
    type Field = Tes3Field;
    type Record = Tes3Record;

    fn record_type() -> &'static [u8; 4] {
        b"CLAS"
    }

    /// Reads class data from a raw record
    ///
    /// # Errors
    ///
    /// Fails if the provided record is not a `b"CLAS"` record or if the record data is invalid.
    fn read(record: &Tes3Record) -> Result<Class, TesError> {
        Class::assert(record)?;

        let mut class = Class {
            id: String::new(),
            name: String::new(),
            description: None,
            primary_attributes: [Attribute::Strength, Attribute::Strength],
            specialization: Specialization::Combat,
            major_skills: [
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
            ],
            minor_skills: [
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
                Skill::Armorer,
            ],
            is_playable: false,
            auto_calc_flags: AutoCalcFlags::empty(),
        };

        for field in record.iter() {
            match field.name() {
                b"NAME" => class.id = String::from(field.get_zstring()?),
                b"FNAM" => class.name = String::from(field.get_zstring()?),
                b"DESC" => class.description = Some(String::from(field.get_string()?)),
                b"CLDT" => {
                    let reader = &mut field.get();
                    for attr in &mut class.primary_attributes {
                        *attr = Attribute::try_from(extract!(reader as u32)? as u8)
                            .map_err(|e| decode_failed_because("Invalid attribute value", e))?;
                    }
                    class.specialization = Specialization::try_from(extract!(reader as u32)? as u8)
                        .map_err(|e| decode_failed_because("Invalid specialization", e))?;
                    for (major, minor) in class
                        .major_skills
                        .iter_mut()
                        .zip(class.minor_skills.iter_mut())
                    {
                        *minor = Skill::try_from(extract!(reader as u32)? as u8)
                            .map_err(|e| decode_failed_because("Invalid skill value", e))?;
                        *major = Skill::try_from(extract!(reader as u32)? as u8)
                            .map_err(|e| decode_failed_because("Invalid skill value", e))?;
                    }
                    class.is_playable = extract!(reader as u32)? != 0;
                    class.auto_calc_flags = AutoCalcFlags::from_bits(extract!(reader as u32)?)
                        .ok_or_else(|| decode_failed("Invalid auto-calc flags"))?;
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

    fn write(&self, _: &mut Tes3Record) -> Result<(), TesError> {
        unimplemented!()
    }
}

impl Class {
    /// Returns whether a given skill is a major, minor, or misc skill for this class
    pub fn get_skill_type(&self, skill: Skill) -> SkillType {
        if self.major_skills.iter().any(|s| *s == skill) {
            SkillType::Major
        } else if self.minor_skills.iter().any(|s| *s == skill) {
            SkillType::Minor
        } else {
            SkillType::Miscellaneous
        }
    }

    /// Returns this class's ID
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Returns this class's name
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Gets this class's primary attributes
    pub fn primary_attribute(&self) -> &[Attribute; 2] {
        &self.primary_attributes
    }

    /// Gets this class's major skills
    pub fn major_skills(&self) -> &[Skill; 5] {
        &self.major_skills
    }

    /// Gets this class's minor skills
    pub fn minor_skills(&self) -> &[Skill; 5] {
        &self.minor_skills
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static CLASS_RECORD: &[u8] = include_bytes!("test/clas_record.bin");

    #[test]
    fn test_load() {
        let record = Tes3Record::read(&mut CLASS_RECORD.as_ref()).unwrap();
        let class = Class::read(&record).unwrap();
        assert_eq!(class.id, "Warrior");
        assert_eq!(class.name, "Warrior");
        assert!(class.description.is_some());
        assert_eq!(class.specialization, Specialization::Combat);
        assert!(class.is_playable);
    }
}
