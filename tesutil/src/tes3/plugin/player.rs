use super::*;
use crate::plugin::Field;
use crate::tes3::Skills;
use crate::Form;

/// Information about the state of a resting player
#[derive(Debug)]
pub struct RestState {
    hours_left: i32,
    x: f32,
    y: f32,
    z: f32,
}

/// Player exterior location?
#[derive(Debug)]
pub struct ExteriorLocation {
    x: i32,
    y: i32,
}

/// Honestly don't know what this is
#[derive(Debug)]
pub struct Lnam {
    // in my test, these had the exact same values as ENAM (above)
    unknown1: i32,
    unknown2: i32,
}

/// Player faction membership
#[derive(Debug)]
pub struct Faction {
    rank: u32,
    reputation: i32,
    flags: u32,
    name: String,
}

/// Player animation data
#[derive(Debug)]
pub struct AnimationData {
    anim_group_index: i32,
    unknown: Vec<u8>, // always 40
}

/// Quick key assignment
#[derive(Debug)]
pub struct QuickKey {
    bind_type: u8, // 0 = not set, 1 = item, 2 = spell?
    bound_form: String,
    unknown: i32,
}

impl Default for QuickKey {
    fn default() -> Self {
        QuickKey {
            bind_type: 0,
            bound_form: String::from(""),
            unknown: -1,
        }
    }
}

/// Player-specific data
#[derive(Debug, Default)]
pub struct PlayerData {
    // DNAM
    known_topics: Vec<String>,
    // MNAM
    mark_cell: Option<String>,
    // PNAM
    player_flags: u32,
    pub level_progress: u32,
    pub skill_progress: Skills<f32>,
    pub attribute_progress: Attributes<u8>,
    telekinesis_range_bonus: i32,
    vision_bonus: f32,
    detect_key_magnitude: i32,
    detect_enchantment_magnitude: i32,
    detect_animal_magnitude: i32,
    mark_x: f32,
    mark_y: f32,
    mark_z: f32,
    mark_rot: f32,
    mark_grid_x: i32,
    mark_grid_y: i32,
    unknown1: Vec<u8>, // always 40
    pub spec_increases: Specializations<u8>,
    unknown2: u8,
    // SNAM
    snam: Vec<u8>,
    // NAM9
    nam9: Option<u32>,
    // RNAM
    rest_state: Option<RestState>,
    // CNAM
    bounty: Option<i32>,
    // BNAM
    birthsign: Option<String>,
    // NAM0-NAM3
    alchemy_equipment: [Option<String>; 4],
    // ENAM
    exterior: Option<ExteriorLocation>,
    // LNAM
    lnam: Option<Lnam>,
    // FNAM
    factions: Vec<Faction>,
    // AADT
    animation_data: Option<AnimationData>,
    // KNAM
    quick_keys: Vec<QuickKey>,
    // ANIS
    anis: Option<[u8; 16]>,
    // WERE
    werewolf_data: Vec<u8>,
}

impl PlayerData {
    pub fn birthsign(&self) -> Option<&str> {
        self.birthsign.as_deref()
    }
}

impl Form for PlayerData {
    type Field = Tes3Field;
    type Record = Tes3Record;

    fn record_type() -> &'static [u8; 4] {
        b"PCDT"
    }

    /// Read player data from a raw record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs or the data is not valid
    fn read(record: &Tes3Record) -> Result<PlayerData, TesError> {
        PlayerData::assert(record)?;

        let mut player_data = PlayerData::default();
        for field in record.iter() {
            match field.name() {
                b"DNAM" => player_data
                    .known_topics
                    .push(String::from(field.get_zstring()?)),
                b"MNAM" => player_data.mark_cell = Some(String::from(field.get_zstring()?)),
                b"PNAM" => {
                    let mut reader = field.reader();

                    player_data.player_flags = extract!(reader as u32)?;
                    player_data.level_progress = extract!(reader as u32)?;

                    for skill in player_data.skill_progress.values_mut() {
                        *skill = extract!(reader as f32)?;
                    }

                    for attribute in player_data.attribute_progress.values_mut() {
                        *attribute = extract!(reader as u8)?;
                    }

                    player_data.telekinesis_range_bonus = extract!(reader as i32)?;
                    player_data.vision_bonus = extract!(reader as f32)?;
                    player_data.detect_key_magnitude = extract!(reader as i32)?;
                    player_data.detect_enchantment_magnitude = extract!(reader as i32)?;
                    player_data.detect_animal_magnitude = extract!(reader as i32)?;

                    player_data.mark_x = extract!(reader as f32)?;
                    player_data.mark_y = extract!(reader as f32)?;
                    player_data.mark_z = extract!(reader as f32)?;
                    player_data.mark_rot = extract!(reader as f32)?;
                    player_data.mark_grid_x = extract!(reader as i32)?;
                    player_data.mark_grid_y = extract!(reader as i32)?;

                    player_data.unknown1 = vec![0u8; 40];
                    reader.read_exact(player_data.unknown1.as_mut())?;

                    for specialization in player_data.spec_increases.values_mut() {
                        *specialization = extract!(reader as u8)?;
                    }

                    player_data.unknown2 = extract!(reader as u8)?;
                }
                b"SNAM" => player_data.snam = field.get().to_vec(),
                b"NAM9" => player_data.nam9 = Some(field.get_u32()?),
                b"RNAM" => {
                    let mut reader = field.reader();
                    player_data.rest_state = Some(RestState {
                        hours_left: extract!(reader as i32)?,
                        x: extract!(reader as f32)?,
                        y: extract!(reader as f32)?,
                        z: extract!(reader as f32)?,
                    });
                }
                b"CNAM" => player_data.bounty = Some(field.get_i32()?),
                b"BNAM" => player_data.birthsign = Some(String::from(field.get_zstring()?)),
                b"NAM0" => {
                    player_data.alchemy_equipment[0] = Some(String::from(field.get_zstring()?))
                }
                b"NAM1" => {
                    player_data.alchemy_equipment[1] = Some(String::from(field.get_zstring()?))
                }
                b"NAM2" => {
                    player_data.alchemy_equipment[2] = Some(String::from(field.get_zstring()?))
                }
                b"NAM3" => {
                    player_data.alchemy_equipment[3] = Some(String::from(field.get_zstring()?))
                }
                b"ENAM" => {
                    let mut reader = field.reader();
                    player_data.exterior = Some(ExteriorLocation {
                        x: extract!(reader as i32)?,
                        y: extract!(reader as i32)?,
                    });
                }
                b"LNAM" => {
                    let mut reader = field.reader();
                    player_data.lnam = Some(Lnam {
                        unknown1: extract!(reader as i32)?,
                        unknown2: extract!(reader as i32)?,
                    });
                }
                b"FNAM" => {
                    let mut reader = field.reader();
                    player_data.factions.push(Faction {
                        rank: extract!(reader as u32)?,
                        reputation: extract!(reader as i32)?,
                        flags: extract!(reader as u32)?,
                        name: extract_string(32, &mut reader)?,
                    });
                }
                b"AADT" => {
                    let mut reader = field.reader();
                    let anim_group_index = extract!(reader as i32)?;
                    let mut buf = vec![0u8; 40];
                    reader.read_exact(buf.as_mut())?;
                    player_data.animation_data = Some(AnimationData {
                        anim_group_index,
                        unknown: buf,
                    });
                }
                b"KNAM" => {
                    let mut reader = field.reader();
                    player_data.quick_keys.reserve(10);
                    for _ in 0..10 {
                        player_data.quick_keys.push(QuickKey {
                            bind_type: extract!(reader as u8)?,
                            bound_form: extract_string(35, &mut reader)?,
                            unknown: extract!(reader as i32)?,
                        });
                    }
                }
                b"ANIS" => {
                    let mut reader = field.reader();
                    let mut buf = [0u8; 16];
                    reader.read_exact(&mut buf)?;
                    player_data.anis = Some(buf);
                }
                b"WERE" => player_data.werewolf_data = field.get().to_vec(),
                _ => {
                    return Err(TesError::DecodeFailed {
                        description: format!("Unexpected field {}", field.name_as_str()),
                        source: None,
                    })
                }
            }
        }

        Ok(player_data)
    }

    fn write(&self, _: &mut Tes3Record) -> Result<(), TesError> {
        unimplemented!()
    }
}
