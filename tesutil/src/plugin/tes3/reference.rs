use crate::plugin::tes3::*;
use crate::plugin::*;
use crate::*;

/// A statistic, such as an attribute, skill, health, etc.
#[derive(Debug, Default)]
pub struct Stat<T> {
    pub current: T,
    pub base: T,
}

macro_rules! extract_stats {
    ($f:ident, $p:ident, $t:ty, $($s:ident),+) => {
        $({
            $p.$s.current = extract!($f as $t).unwrap();
            $p.$s.base = extract!($f as $t).unwrap();
        })*
    }
}

macro_rules! serialize_stats {
    ($f:ident, $p:ident, $($s:ident),+) => {
        $({
            serialize!($p.$s.current => $f).unwrap();
            serialize!($p.$s.base => $f).unwrap();
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
    pub strength: Stat<f32>,
    pub intelligence: Stat<f32>,
    pub willpower: Stat<f32>,
    pub agility: Stat<f32>,
    pub speed: Stat<f32>,
    pub endurance: Stat<f32>,
    pub personality: Stat<f32>,
    pub luck: Stat<f32>,
    magic_effects: [f32; 27],
    unknown4: [u8; 4],
    gold: u32,
    count_down: u8,
    unknown5: [u8; 3],
    pub block: Stat<u32>,
    pub armorer: Stat<u32>,
    pub medium_armor: Stat<u32>,
    pub heavy_armor: Stat<u32>,
    pub blunt: Stat<u32>,
    pub long_blade: Stat<u32>,
    pub axe: Stat<u32>,
    pub spear: Stat<u32>,
    pub athletics: Stat<u32>,
    pub enchant: Stat<u32>,
    pub destruction: Stat<u32>,
    pub alteration: Stat<u32>,
    pub illusion: Stat<u32>,
    pub conjuration: Stat<u32>,
    pub mysticism: Stat<u32>,
    pub restoration: Stat<u32>,
    pub alchemy: Stat<u32>,
    pub unarmored: Stat<u32>,
    pub security: Stat<u32>,
    pub sneak: Stat<u32>,
    pub acrobatics: Stat<u32>,
    pub light_armor: Stat<u32>,
    pub short_blade: Stat<u32>,
    pub marksman: Stat<u32>,
    pub mercantile: Stat<u32>,
    pub speechcraft: Stat<u32>,
    pub hand_to_hand: Stat<u32>,
}

impl PlayerReference {
    /// Reads a player reference change from a raw record
    /// 
    /// # Errors
    /// 
    /// Fails if an I/O error occurs or if the data is invalid
    pub fn read(record: &Record) -> Result<PlayerReference, TesError> {
        if record.name() != b"REFR" {
            return Err(TesError::DecodeFailed { description: String::from("Record was not a REFR record"), source: None });
        }

        let mut player = PlayerReference {
            unknown1: [0; 12],
            flags: 0,
            breath_meter: 0.,
            unknown2: [0; 20],
            health: Stat::default(),
            fatigue: Stat::default(),
            magicka: Stat::default(),
            unknown3: [0; 16],
            strength: Stat::default(),
            intelligence: Stat::default(),
            willpower: Stat::default(),
            agility: Stat::default(),
            speed: Stat::default(),
            endurance: Stat::default(),
            personality: Stat::default(),
            luck: Stat::default(),
            magic_effects: [0.; 27],
            unknown4: [0; 4],
            gold: 0,
            count_down: 0,
            unknown5: [0; 3],
            block: Stat::default(),
            armorer: Stat::default(),
            medium_armor: Stat::default(),
            heavy_armor: Stat::default(),
            blunt: Stat::default(),
            long_blade: Stat::default(),
            axe: Stat::default(),
            spear: Stat::default(),
            athletics: Stat::default(),
            enchant: Stat::default(),
            destruction: Stat::default(),
            alteration: Stat::default(),
            illusion: Stat::default(),
            conjuration: Stat::default(),
            mysticism: Stat::default(),
            restoration: Stat::default(),
            alchemy: Stat::default(),
            unarmored: Stat::default(),
            security: Stat::default(),
            sneak: Stat::default(),
            acrobatics: Stat::default(),
            light_armor: Stat::default(),
            short_blade: Stat::default(),
            marksman: Stat::default(),
            mercantile: Stat::default(),
            speechcraft: Stat::default(),
            hand_to_hand: Stat::default(),
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
                    extract_stats!(reader, player, f32, strength, intelligence, willpower, agility, speed, endurance, personality, luck);
                    for i in 0..player.magic_effects.len() {
                        player.magic_effects[i] = extract!(reader as f32)?;
                    }
                    reader.read_exact(&mut player.unknown4)?;
                    player.gold = extract!(reader as u32)?;
                    player.count_down = extract!(reader as u8)?;
                    reader.read_exact(&mut player.unknown5)?;
                },
                b"CHRD" => {
                    let mut buf_ref = field.get();
                    let reader = &mut buf_ref;

                    extract_stats!(reader, player, u32, block, armorer, medium_armor, heavy_armor,
                        blunt, long_blade, axe, spear, athletics, enchant, destruction, alteration,
                        illusion, conjuration, mysticism, restoration, alchemy, unarmored, security,
                        sneak, acrobatics, light_armor, short_blade, marksman, mercantile, speechcraft, hand_to_hand
                    );
                },
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
    pub fn write(&self, record: &mut Record) -> Result<(), TesError> {
        for field in record.iter_mut() {
            match field.name() {
                b"ACDT" => {
                    let mut buf: Vec<u8> = Vec::new();
                    let writer = &mut buf;
                    
                    // write operations on Vec<u8> are infallible
                    // TODO: is it safe to rely on this?
                    writer.write_exact(&self.unknown1).unwrap();
                    serialize!(self.flags => writer).unwrap();
                    serialize!(self.breath_meter => writer).unwrap();
                    writer.write_exact(&self.unknown2).unwrap();
                    serialize_stats!(writer, self, health, fatigue, magicka);
                    writer.write_exact(&self.unknown3).unwrap();
                    serialize_stats!(writer, self, strength, intelligence, willpower, agility, speed, endurance, personality, luck);
                    for effect in self.magic_effects.iter() {
                        serialize!(effect => writer).unwrap();
                    }
                    writer.write_exact(&self.unknown4).unwrap();
                    serialize!(self.gold => writer).unwrap();
                    serialize!(self.count_down => writer).unwrap();
                    writer.write_exact(&self.unknown5).unwrap();

                    field.set(buf)?;
                },
                b"CHRD" => {
                    let mut buf: Vec<u8> = Vec::new();
                    let writer = &mut buf;
                    
                    serialize_stats!(writer, self, block, armorer, medium_armor, heavy_armor,
                        blunt, long_blade, axe, spear, athletics, enchant, destruction, alteration,
                        illusion, conjuration, mysticism, restoration, alchemy, unarmored, security,
                        sneak, acrobatics, light_armor, short_blade, marksman, mercantile, speechcraft, hand_to_hand
                    );

                    field.set(buf)?;
                },
                _ => (),
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static REFR_RECORD: &[u8] = include_bytes!("test/refr_record.bin");

    #[test]
    fn read() {
        let record = Record::read(&mut REFR_RECORD.as_ref()).unwrap().unwrap();
        let player = PlayerReference::read(&record).unwrap();
        assert_eq!(player.strength.base, 38.);
        assert_eq!(player.destruction.base, 91);
    }
}