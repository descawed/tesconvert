use std::convert::{TryFrom, TryInto};
use std::io::Read;

use crate::tes4::plugin::*;
use crate::tes4::{ActorValue, Skill};
use crate::Form;

use binrw::{BinReaderExt, BinWriterExt};
use bitflags::bitflags;

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
    pub specialization: Specialization,
    major_skills: [Skill; 7],
    pub is_playable: bool,
    pub is_guard: bool,
    services: ServiceFlags,
    pub skill_trained: Skill,
    pub max_training_level: u8,
}

impl Form for Class {
    type Field = Tes4Field;
    type Record = Tes4Record;

    const RECORD_TYPE: &'static [u8; 4] = b"CLAS";

    //noinspection RsTypeCheck
    /// Reads class data from a raw record
    ///
    /// # Errors
    ///
    /// Fails if the provided record is not a `b"CLAS"` record or if the record data is invalid.
    fn read(record: &Tes4Record) -> Result<Class, TesError> {
        Class::assert(record)?;

        let mut class = Class::new(String::new()).unwrap();

        for field in record.iter() {
            match field.name() {
                b"EDID" => class.editor_id = Some(String::from(field.get_zstring()?)),
                b"FULL" => class.name = String::from(field.get_zstring()?),
                b"DESC" => class.description = Some(String::from(field.get_zstring()?)),
                b"ICON" => class.icon = Some(String::from(field.get_zstring()?)),
                b"DATA" => {
                    let mut reader = field.reader();
                    for attr in &mut class.primary_attributes {
                        *attr = ActorValue::try_from(reader.read_le::<u32>()? as u8)
                            .map_err(|e| decode_failed_because("Invalid attribute value", e))?
                            .try_into()?;
                    }
                    class.specialization = Specialization::try_from(reader.read_le::<u32>()? as u8)
                        .map_err(|e| decode_failed_because("Invalid specialization", e))?;
                    for skill in class.major_skills.iter_mut() {
                        *skill = ActorValue::try_from(reader.read_le::<u32>()? as u8)
                            .map_err(|e| decode_failed_because("Invalid skill value", e))?
                            .try_into()?;
                    }
                    let class_flags = ClassFlags::from_bits(reader.read_le()?)
                        .ok_or_else(|| decode_failed("Invalid class flags"))?;
                    class.is_playable = class_flags.contains(ClassFlags::PLAYABLE);
                    class.is_guard = class_flags.contains(ClassFlags::GUARD);
                    class.services = ServiceFlags::from_bits(reader.read_le()?)
                        .ok_or_else(|| decode_failed("Invalid service flags"))?;
                    class.skill_trained = Skill::try_from(reader.read_le::<u8>()?)
                        .map_err(|e| decode_failed_because("Invalid training skill value", e))?;
                    class.max_training_level = reader.read_le()?;
                    // 2 additional unused bytes at the end
                }
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {}",
                        field.name_as_str()
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
    /// Creates a new class with default settings
    pub fn new(name: String) -> Result<Class, TesError> {
        check_size(&name, MAX_BSTRING, "Class name too long")?;

        Ok(Class {
            editor_id: None,
            name,
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
        })
    }

    //noinspection RsTypeCheck
    /// Reads a custom class from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn read_custom<T: Read + Seek>(mut f: T) -> Result<Class, TesError> {
        let mut primary_attributes = [Attribute::Strength; 2];
        for attr in &mut primary_attributes {
            *attr = ActorValue::try_from(f.read_le::<u32>()? as u8)
                .map_err(|e| decode_failed_because("Invalid attribute value", e))?
                .try_into()?;
        }

        let specialization = Specialization::try_from(f.read_le::<u32>()? as u8)
            .map_err(|e| decode_failed_because("Invalid specialization", e))?;

        let mut major_skills = [Skill::Acrobatics; 7];
        for skill in &mut major_skills {
            *skill = ActorValue::try_from(f.read_le::<u32>()? as u8)
                .map_err(|e| decode_failed_because("Invalid skill value", e))?
                .try_into()?;
        }

        let flags = ClassFlags::from_bits(f.read_le()?)
            .ok_or_else(|| decode_failed("Invalid class flags"))?;
        let is_playable = flags.contains(ClassFlags::PLAYABLE);
        let is_guard = flags.contains(ClassFlags::GUARD);

        let services = ServiceFlags::from_bits(f.read_le()?)
            .ok_or_else(|| decode_failed("Invalid service flags"))?;
        let skill_trained = Skill::try_from(f.read_le::<u8>()?)
            .map_err(|e| decode_failed_because("Invalid training skill value", e))?;
        let max_training_level = f.read_le()?;
        f.seek(SeekFrom::Current(2))?; // dummy
        let name = read_bstring(&mut f)?;
        let icon = Some(read_bstring(&mut f)?);

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

    /// Gets the primary attributes of this class
    pub fn primary_attribute(&self) -> &[Attribute; 2] {
        &self.primary_attributes
    }

    /// Sets the primary attributes of this class
    ///
    /// # Errors
    ///
    /// Fails if both provided attributes are the same.
    pub fn set_primary_attributes(&mut self, attributes: &[Attribute]) -> Result<(), TesError> {
        let num_attributes = self.primary_attributes.len() as u32;
        check_range(
            attributes.len() as u32,
            num_attributes,
            num_attributes,
            "Wrong number of class attributes",
        )?;

        if attributes[0] == attributes[1] {
            return Err(TesError::RequirementFailed(String::from(
                "Class primary attributes must be unique",
            )));
        }

        self.primary_attributes.copy_from_slice(attributes);
        Ok(())
    }

    /// Gets the major skills of this class
    pub fn major_skills(&self) -> &[Skill; 7] {
        &self.major_skills
    }

    /// Sets the major skills of this class
    ///
    /// # Errors
    ///
    /// Fails if the same skill appears in the array more than once.
    pub fn set_major_skills(&mut self, skills: &[Skill]) -> Result<(), TesError> {
        let num_skills = self.major_skills.len() as u32;
        check_range(
            skills.len() as u32,
            num_skills,
            num_skills,
            "Wrong number of class major skills",
        )?;

        for (i, skill) in skills.iter().enumerate() {
            if skills.iter().skip(i + 1).any(|s| s == skill) {
                return Err(TesError::RequirementFailed(String::from(
                    "Class major skills must be unique",
                )));
            }
        }

        self.major_skills.copy_from_slice(skills);
        Ok(())
    }

    /// Checks whether a given skill is a major skill for this class
    pub fn is_major_skill(&self, skill: Skill) -> bool {
        self.major_skills.iter().any(|s| *s == skill)
    }

    /// Writes a custom class to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    pub fn write_custom<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        for attribute in self.primary_attributes.iter() {
            f.write_le(&(<Attribute as Enum>::into_usize(*attribute) as u32))?;
        }

        f.write_le(&(<Specialization as Enum>::into_usize(self.specialization) as u32))?;

        for skill in self.major_skills.iter() {
            let av: ActorValue = (*skill).into();
            f.write_le(&(<ActorValue as Enum>::into_usize(av) as u32))?;
        }

        let mut class_flags = ClassFlags::empty();
        if self.is_playable {
            class_flags |= ClassFlags::PLAYABLE;
        }
        if self.is_guard {
            class_flags |= ClassFlags::GUARD;
        }

        f.write_le(&class_flags.bits)?;
        f.write_le(&self.services.bits)?;
        f.write_le(&(self.skill_trained as u8))?;
        f.write_le(&self.max_training_level)?;
        f.write_le(&0u16)?;
        write_bstring(&mut f, &self.name)?;
        write_bstring(
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
    use std::io::Cursor;

    static CLASS_RECORD: &[u8] = include_bytes!("test/clas_record.bin");

    #[test]
    fn test_load() {
        let mut record_ref = CLASS_RECORD.as_ref();
        let cursor = Cursor::new(&mut record_ref);
        let record = Tes4Record::read(cursor).unwrap();
        let class = Class::read(&record).unwrap();
        assert_eq!(class.editor_id.unwrap(), "SE32Smith");
        assert_eq!(class.name, "Vitharn Smith");
        assert_eq!(class.description.unwrap(), "");
        assert!(class.icon.is_none());
    }
}
