use std::convert::TryFrom;
use std::io::Read;

use crate::tes3::{Skill, Tes3Field, Tes3Record};
use crate::{
    decode_failed, decode_failed_because, extract, extract_string, Attribute, Attributes, Field,
    Form, Record, TesError,
};

const ID_LENGTH: usize = 32;

#[derive(Debug)]
pub struct Race {
    id: String,
    name: String,
    skills: [Option<(Skill, u32)>; 7],
    attributes: Attributes<(u32, u32)>,
    height: (f32, f32),
    weight: (f32, f32),
    is_playable: bool,
    is_beast_race: bool,
    specials: Vec<String>,
    description: Option<String>,
}

impl Race {
    /// Gets the starting attributes for a male character of this race
    pub fn attributes_male(&self) -> impl Iterator<Item = (Attribute, u32)> + '_ {
        self.attributes.iter().map(|(a, (v, _))| (a, *v))
    }

    /// Gets a starting attribute for a male character of this race
    pub fn attribute_male(&self, attribute: Attribute) -> u32 {
        self.attributes[attribute].0
    }

    /// Gets the starting attributes for a female character of this race
    pub fn attributes_female(&self) -> impl Iterator<Item = (Attribute, u32)> + '_ {
        self.attributes.iter().map(|(a, (_, v))| (a, *v))
    }

    /// Gets a starting attribute for a female character of this race
    pub fn attribute_female(&self, attribute: Attribute) -> u32 {
        self.attributes[attribute].1
    }

    /// Gets the specials (abilities and powers) for a character of this race
    pub fn specials(&self) -> impl Iterator<Item = &str> + '_ {
        self.specials.iter().map(String::as_str)
    }
}

impl Form for Race {
    type Field = Tes3Field;
    type Record = Tes3Record;

    fn record_type() -> &'static [u8; 4] {
        b"RACE"
    }

    fn read(record: &Self::Record) -> Result<Self, TesError> {
        Race::assert(&record)?;

        let mut race = Race {
            id: String::new(),
            name: String::new(),
            skills: [None, None, None, None, None, None, None],
            attributes: Attributes::new(),
            height: (0., 0.),
            weight: (0., 0.),
            is_playable: false,
            is_beast_race: false,
            specials: vec![],
            description: None,
        };

        for field in record.iter() {
            match field.name() {
                b"NAME" => race.id = String::from(field.get_zstring()?),
                b"FNAM" => race.name = String::from(field.get_zstring()?),
                b"RADT" => {
                    let mut reader = field.reader();
                    for skill in &mut race.skills {
                        let skill_id = extract!(reader as i32)?;
                        let bonus = extract!(reader as u32)?;
                        if skill_id != -1 {
                            let enum_skill = Skill::try_from(skill_id as u8)
                                .map_err(|e| decode_failed_because("Invalid skill value", e))?;
                            *skill = Some((enum_skill, bonus));
                        }
                    }

                    for (_, (male_value, female_value)) in race.attributes.iter_mut() {
                        *male_value = extract!(reader as u32)?;
                        *female_value = extract!(reader as u32)?;
                    }

                    race.height = (extract!(reader as f32)?, extract!(reader as f32)?);
                    race.weight = (extract!(reader as f32)?, extract!(reader as f32)?);

                    let flags = extract!(reader as u32)?;
                    race.is_playable = flags & 1 != 0;
                    race.is_beast_race = flags & 2 != 0;
                }
                b"NPCS" => race
                    .specials
                    .push(extract_string(ID_LENGTH, &mut field.get())?),
                b"DESC" => race.description = Some(String::from(field.get_string()?)),
                _ => {
                    return Err(decode_failed(format!(
                        "Unexpected field {} in RACE",
                        field.name_as_str()
                    )))
                }
            }
        }

        Ok(race)
    }

    fn write(&self, _record: &mut Self::Record) -> Result<(), TesError> {
        unimplemented!()
    }
}
