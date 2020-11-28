use crate::tes3::plugin::*;
use crate::tes3::Skills;
use crate::Form;

/// A statistic, such as an attribute, skill, health, etc.
#[derive(Debug, Default)]
pub struct Stat<T> {
    pub current: T,
    pub base: T,
}

macro_rules! extract_stats {
    ($f:ident, $p:ident, $t:ty, $($s:ident),+) => {
        $({
            $p.$s.current = extract!($f as $t)?;
            $p.$s.base = extract!($f as $t)?;
        })*
    }
}

macro_rules! serialize_stats {
    ($f:ident, $p:ident, $($s:ident),+) => {
        $({
            serialize!($p.$s.current => $f)?;
            serialize!($p.$s.base => $f)?;
        })*
    }
}

/// Changes to the reference to the player
///
/// The full format of REFR records is not fully documented, and is also not currently necessary for
/// our purposes, so this type only has the ability to read and edit the reference to the player.
#[derive(Debug)]
pub struct PlayerReference {
    unknown1: [u8; 12],
    flags: u32,
    breath_meter: f32,
    unknown2: [u8; 20],
    pub health: Stat<f32>,
    pub fatigue: Stat<f32>,
    pub magicka: Stat<f32>,
    unknown3: [u8; 16],
    pub attributes: Attributes<Stat<f32>>,
    magic_effects: [f32; 27],
    unknown4: [u8; 4],
    gold: u32,
    count_down: u8,
    unknown5: [u8; 3],
    pub skills: Skills<Stat<i32>>,
}

impl Form for PlayerReference {
    type Field = Tes3Field;
    type Record = Tes3Record;

    fn record_type() -> &'static [u8; 4] {
        b"REFR"
    }

    /// Reads a player reference change from a raw record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs or if the data is invalid
    fn read(record: &Tes3Record) -> Result<PlayerReference, TesError> {
        PlayerReference::assert(record)?;

        let mut player = PlayerReference {
            unknown1: [0; 12],
            flags: 0,
            breath_meter: 0.,
            unknown2: [0; 20],
            health: Stat::default(),
            fatigue: Stat::default(),
            magicka: Stat::default(),
            unknown3: [0; 16],
            attributes: Attributes::new(),
            magic_effects: [0.; 27],
            unknown4: [0; 4],
            gold: 0,
            count_down: 0,
            unknown5: [0; 3],
            skills: Skills::new(),
        };

        for field in record.iter() {
            match field.name() {
                b"ACDT" => {
                    let mut buf_ref = field.get();
                    let reader = &mut buf_ref;

                    reader.read_exact(&mut player.unknown1)?;
                    player.flags = extract!(reader as u32)?;
                    player.breath_meter = extract!(reader as f32)?;
                    reader.read_exact(&mut player.unknown2)?;
                    extract_stats!(reader, player, f32, health, fatigue, magicka);
                    reader.read_exact(&mut player.unknown3)?;

                    for attribute in player.attributes.values_mut() {
                        (*attribute).current = extract!(reader as f32)?;
                        (*attribute).base = extract!(reader as f32)?;
                    }

                    for magic_effect in &mut player.magic_effects {
                        *magic_effect = extract!(reader as f32)?;
                    }

                    reader.read_exact(&mut player.unknown4)?;
                    player.gold = extract!(reader as u32)?;
                    player.count_down = extract!(reader as u8)?;
                    reader.read_exact(&mut player.unknown5)?;
                }
                b"CHRD" => {
                    let mut buf_ref = field.get();
                    let reader = &mut buf_ref;

                    for skill in player.skills.values_mut() {
                        (*skill).current = extract!(reader as i32)?;
                        (*skill).base = extract!(reader as i32)?;
                    }
                }
                _ => (),
            }
        }

        Ok(player)
    }

    /// Updates a player reference change record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs
    fn write(&self, record: &mut Tes3Record) -> Result<(), TesError> {
        for field in record.iter_mut() {
            match field.name() {
                b"ACDT" => {
                    let mut buf: Vec<u8> = Vec::new();
                    let writer = &mut buf;

                    // write operations on Vec<u8> are infallible
                    writer.write_exact(&self.unknown1)?;
                    serialize!(self.flags => writer)?;
                    serialize!(self.breath_meter => writer)?;
                    writer.write_exact(&self.unknown2)?;
                    serialize_stats!(writer, self, health, fatigue, magicka);
                    writer.write_exact(&self.unknown3)?;
                    for attribute in self.attributes.values() {
                        serialize!(attribute.current => writer)?;
                        serialize!(attribute.base => writer)?;
                    }
                    for effect in &self.magic_effects {
                        serialize!(effect => writer)?;
                    }
                    writer.write_exact(&self.unknown4)?;
                    serialize!(self.gold => writer)?;
                    serialize!(self.count_down => writer)?;
                    writer.write_exact(&self.unknown5)?;

                    field.set(buf)?;
                }
                b"CHRD" => {
                    let mut buf: Vec<u8> = Vec::new();
                    let writer = &mut buf;

                    for skill in self.skills.values() {
                        serialize!(skill.current => writer)?;
                        serialize!(skill.base => writer)?;
                    }

                    field.set(buf)?;
                }
                _ => (),
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tes3::Skill;

    static REFR_RECORD: &[u8] = include_bytes!("test/refr_record.bin");

    #[test]
    fn read() {
        let cursor = io::Cursor::new(REFR_RECORD);
        let record = Tes3Record::read(cursor).unwrap();
        let player = PlayerReference::read(&record).unwrap();
        assert_eq!(player.attributes[Attribute::Strength].base, 38.);
        assert_eq!(player.skills[Skill::Destruction].base, 91);
    }
}
