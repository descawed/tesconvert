use crate::*;
use super::*;
use crate::plugin::FieldInterface;

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
    pub block_progress: f32,
    pub armorer_progress: f32,
    pub medium_armor_progress: f32,
    pub heavy_armor_progress: f32,
    pub blunt_progress: f32,
    pub long_blade_progress: f32,
    pub axe_progress: f32,
    pub spear_progress: f32,
    pub athletics_progress: f32,
    pub enchant_progress: f32,
    pub destruction_progress: f32,
    pub alteration_progress: f32,
    pub illusion_progress: f32,
    pub conjuration_progress: f32,
    pub mysticism_progress: f32,
    pub restoration_progress: f32,
    pub alchemy_progress: f32,
    pub unarmored_progress: f32,
    pub security_progress: f32,
    pub sneak_progress: f32,
    pub acrobatics_progress: f32,
    pub light_armor_progress: f32,
    pub short_blade_progress: f32,
    pub marksman_progress: f32,
    pub mercantile_progress: f32,
    pub speechcraft_progress: f32,
    pub hand_to_hand_progress: f32,
    pub strength_progress: u8,
    pub intelligence_progress: u8,
    pub willpower_progress: u8,
    pub agility_progress: u8,
    pub speed_progress: u8,
    pub endurance_progress: u8,
    pub personality_progress: u8,
    pub luck_progress: u8,
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
    pub combat_increases: u8,
    pub magic_increases: u8,
    pub stealth_increases: u8,
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
    /// Read player data from a raw record
    ///
    /// # Errors
    ///
    /// Fails if an I/O error occurs or the data is not valid
    pub fn read(record: &Record) -> Result<PlayerData, TesError> {
        if record.name() != b"PCDT" {
            return Err(TesError::DecodeFailed { description: String::from("Record was not a PCDT record"), cause: None });
        }

        let mut player_data = PlayerData::default();
        for field in record.iter() {
            match field.name() {
                b"DNAM" => player_data.known_topics.push(String::from(field.get_zstring()?)),
                b"MNAM" => player_data.mark_cell = Some(String::from(field.get_zstring()?)),
                b"PNAM" => {
                    wrap_decode("Failed to decode PNAM", ||{
                        let mut reader = field.reader();

                        player_data.player_flags = extract!(reader as u32)?;
                        player_data.level_progress = extract!(reader as u32)?;

                        player_data.block_progress = extract!(reader as f32)?;
                        player_data.armorer_progress = extract!(reader as f32)?;
                        player_data.medium_armor_progress = extract!(reader as f32)?;
                        player_data.heavy_armor_progress = extract!(reader as f32)?;
                        player_data.blunt_progress = extract!(reader as f32)?;
                        player_data.long_blade_progress = extract!(reader as f32)?;
                        player_data.axe_progress = extract!(reader as f32)?;
                        player_data.spear_progress = extract!(reader as f32)?;
                        player_data.athletics_progress = extract!(reader as f32)?;
                        player_data.enchant_progress = extract!(reader as f32)?;
                        player_data.destruction_progress = extract!(reader as f32)?;
                        player_data.alteration_progress = extract!(reader as f32)?;
                        player_data.illusion_progress = extract!(reader as f32)?;
                        player_data.conjuration_progress = extract!(reader as f32)?;
                        player_data.mysticism_progress = extract!(reader as f32)?;
                        player_data.restoration_progress = extract!(reader as f32)?;
                        player_data.alchemy_progress = extract!(reader as f32)?;
                        player_data.unarmored_progress = extract!(reader as f32)?;
                        player_data.security_progress = extract!(reader as f32)?;
                        player_data.sneak_progress = extract!(reader as f32)?;
                        player_data.acrobatics_progress = extract!(reader as f32)?;
                        player_data.light_armor_progress = extract!(reader as f32)?;
                        player_data.short_blade_progress = extract!(reader as f32)?;
                        player_data.marksman_progress = extract!(reader as f32)?;
                        player_data.mercantile_progress = extract!(reader as f32)?;
                        player_data.speechcraft_progress = extract!(reader as f32)?;
                        player_data.hand_to_hand_progress = extract!(reader as f32)?;

                        player_data.strength_progress = extract!(reader as u8)?;
                        player_data.intelligence_progress = extract!(reader as u8)?;
                        player_data.willpower_progress = extract!(reader as u8)?;
                        player_data.agility_progress = extract!(reader as u8)?;
                        player_data.speed_progress = extract!(reader as u8)?;
                        player_data.endurance_progress = extract!(reader as u8)?;
                        player_data.personality_progress = extract!(reader as u8)?;
                        player_data.luck_progress = extract!(reader as u8)?;

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

                        player_data.combat_increases = extract!(reader as u8)?;
                        player_data.magic_increases = extract!(reader as u8)?;
                        player_data.stealth_increases = extract!(reader as u8)?;
                        player_data.unknown2 = extract!(reader as u8)?;

                        Ok(())
                    })?;
                },
                b"SNAM" => player_data.snam = field.get().to_vec(),
                b"NAM9" => player_data.nam9 = Some(field.get_u32()?),
                b"RNAM" => {
                    wrap_decode("Failed to decode RNAM", ||{
                        let mut reader = field.reader();
                        player_data.rest_state = Some(RestState {
                            hours_left: extract!(reader as i32)?,
                            x: extract!(reader as f32)?,
                            y: extract!(reader as f32)?,
                            z: extract!(reader as f32)?,
                        });
                        Ok(())
                    })?;
                },
                b"CNAM" => player_data.bounty = Some(field.get_i32()?),
                b"BNAM" => player_data.birthsign = Some(String::from(field.get_zstring()?)),
                b"NAM0" => player_data.alchemy_equipment[0] = Some(String::from(field.get_zstring()?)),
                b"NAM1" => player_data.alchemy_equipment[1] = Some(String::from(field.get_zstring()?)),
                b"NAM2" => player_data.alchemy_equipment[2] = Some(String::from(field.get_zstring()?)),
                b"NAM3" => player_data.alchemy_equipment[3] = Some(String::from(field.get_zstring()?)),
                b"ENAM" => {
                    wrap_decode("Failed to decode ENAM", ||{
                        let mut reader = field.reader();
                        player_data.exterior = Some(ExteriorLocation {
                            x: extract!(reader as i32)?,
                            y: extract!(reader as i32)?,
                        });
                        Ok(())
                    })?;
                },
                b"LNAM" => {
                    wrap_decode("Failed to decode LNAM", ||{
                        let mut reader = field.reader();
                        player_data.lnam = Some(Lnam {
                            unknown1: extract!(reader as i32)?,
                            unknown2: extract!(reader as i32)?,
                        });
                        Ok(())
                    })?;
                },
                b"FNAM" => {
                    wrap_decode("Failed to decode FNAM", ||{
                        let mut reader = field.reader();
                        player_data.factions.push(Faction {
                            rank: extract!(reader as u32)?,
                            reputation: extract!(reader as i32)?,
                            flags: extract!(reader as u32)?,
                            name: String::from(extract_string(32, &mut reader)?),
                        });
                        Ok(())
                    })?;
                },
                b"AADT" => {
                    wrap_decode("Failed to decode AADT", ||{
                        let mut reader = field.reader();
                        let anim_group_index = extract!(reader as i32)?;
                        let mut buf = vec![0u8; 40];
                        reader.read_exact(buf.as_mut())?;
                        player_data.animation_data = Some(AnimationData {
                            anim_group_index,
                            unknown: buf,
                        });
                        Ok(())
                    })?;
                },
                b"KNAM" => {
                    wrap_decode("Failed to decode KNAM", ||{
                        let mut reader = field.reader();
                        player_data.quick_keys.reserve(10);
                        for _ in 0..10 {
                            player_data.quick_keys.push(QuickKey {
                                bind_type: extract!(reader as u8)?,
                                bound_form: String::from(extract_string(35, &mut reader)?),
                                unknown: extract!(reader as i32)?,
                            });
                        }
                        Ok(())
                    })?;
                },
                b"ANIS" => {
                    wrap_decode("Failed to decode ANIS", ||{
                        let mut reader = field.reader();
                        let mut buf = [0u8; 16];
                        reader.read_exact(&mut buf)?;
                        player_data.anis = Some(buf);

                        Ok(())
                    })?;
                },
                b"WERE" => player_data.werewolf_data = field.get().to_vec(),
                _ => return Err(TesError::DecodeFailed { description: format!("Unexpected field {}", field.display_name()), cause: None }),
            }
        }

        Ok(player_data)
    }
}