use crate::plugin::tes3::*;
use crate::plugin::*;
use crate::*;

/// A statistic, such as an attribute, skill, health, etc.
#[derive(Debug)]
pub struct Stat {
    pub current: f32,
    pub base: f32,
}

impl Stat {
    fn new() -> Stat {
        Stat {
            current: 0.,
            base: 0.,
        }
    }
}

macro_rules! extract_stats {
    ($f:ident, $p:ident, $($s:ident),+) => {
        $({
            $p.$s.current = extract!($f as f32).unwrap();
            $p.$s.base = extract!($f as f32).unwrap();
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
    health: Stat,
    fatigue: Stat,
    magicka: Stat,
    unknown3: [u8; 16],
    strength: Stat,
    intelligence: Stat,
    willpower: Stat,
    agility: Stat,
    speed: Stat,
    endurance: Stat,
    personality: Stat,
    luck: Stat,
    magic_effects: [f32; 27],
    unknown4: [u8; 4],
    gold: u32,
    count_down: u8,
    unknown5: [u8; 3],
    block: Stat,
    armorer: Stat,
    medium_armor: Stat,
    heavy_armor: Stat,
    blunt: Stat,
    long_blade: Stat,
    axe: Stat,
    spear: Stat,
    athletics: Stat,
    enchant: Stat,
    destruction: Stat,
    alteration: Stat,
    illusion: Stat,
    conjuration: Stat,
    mysticism: Stat,
    restoration: Stat,
    alchemy: Stat,
    unarmored: Stat,
    security: Stat,
    sneak: Stat,
    acrobatics: Stat,
    light_armor: Stat,
    short_blade: Stat,
    marksman: Stat,
    mercantile: Stat,
    speechcraft: Stat,
    hand_to_hand: Stat,
}

impl PlayerReference {
    /// Reads a player reference change from a raw record
    /// 
    /// # Errors
    /// 
    /// Fails if an I/O error occurs or if the data is invalid
    pub fn read(record: &Record) -> Result<PlayerReference, TesError> {
        let mut player = PlayerReference {
            unknown1: [0; 12],
            flags: 0,
            breath_meter: 0.,
            unknown2: [0; 20],
            health: Stat::new(),
            fatigue: Stat::new(),
            magicka: Stat::new(),
            unknown3: [0; 16],
            strength: Stat::new(),
            intelligence: Stat::new(),
            willpower: Stat::new(),
            agility: Stat::new(),
            speed: Stat::new(),
            endurance: Stat::new(),
            personality: Stat::new(),
            luck: Stat::new(),
            magic_effects: [0.; 27],
            unknown4: [0; 4],
            gold: 0,
            count_down: 0,
            unknown5: [0; 3],
            block: Stat::new(),
            armorer: Stat::new(),
            medium_armor: Stat::new(),
            heavy_armor: Stat::new(),
            blunt: Stat::new(),
            long_blade: Stat::new(),
            axe: Stat::new(),
            spear: Stat::new(),
            athletics: Stat::new(),
            enchant: Stat::new(),
            destruction: Stat::new(),
            alteration: Stat::new(),
            illusion: Stat::new(),
            conjuration: Stat::new(),
            mysticism: Stat::new(),
            restoration: Stat::new(),
            alchemy: Stat::new(),
            unarmored: Stat::new(),
            security: Stat::new(),
            sneak: Stat::new(),
            acrobatics: Stat::new(),
            light_armor: Stat::new(),
            short_blade: Stat::new(),
            marksman: Stat::new(),
            mercantile: Stat::new(),
            speechcraft: Stat::new(),
            hand_to_hand: Stat::new(),
        };
        
        for field in record.iter() {
            match field.name() {
                b"ACDT" => {
                    let mut buf_ref = field.get();
                    let reader = &mut buf_ref;

                    wrap_decode("Failed to decode ACDT field", || {
                        reader.read_exact(&mut player.unknown1)?;
                        player.flags = extract!(reader as u32)?;
                        player.breath_meter = extract!(reader as f32)?;
                        reader.read_exact(&mut player.unknown2)?;
                        extract_stats!(reader, player, health, fatigue, magicka);
                        reader.read_exact(&mut player.unknown3)?;
                        extract_stats!(reader, player, strength, intelligence, willpower, agility, speed, personality, luck);
                        for i in 0..player.magic_effects.len() {
                            player.magic_effects[i] = extract!(reader as f32)?;
                        }
                        reader.read_exact(&mut player.unknown4)?;
                        player.gold = extract!(reader as u32)?;
                        player.count_down = extract!(reader as u8)?;
                        reader.read_exact(&mut player.unknown5)?;
                        Ok(())
                    })?;
                },
                b"CHRD" => {
                    let mut buf_ref = field.get();
                    let reader = &mut buf_ref;

                    wrap_decode("Failed to decode CHRD field", || {
                        extract_stats!(reader, player, block, armorer, medium_armor, heavy_armor,
                            blunt, long_blade, axe, spear, athletics, enchant, destruction, alteration,
                            illusion, conjuration, mysticism, restoration, alchemy, unarmored, security,
                            sneak, acrobatics, light_armor, short_blade, marksman, mercantile, speechcraft, hand_to_hand
                        );
                        Ok(())
                    })?;
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
                    serialize_stats!(writer, self, strength, intelligence, willpower, agility, speed, personality, luck);
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