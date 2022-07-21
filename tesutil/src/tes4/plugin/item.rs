use crate::tes4::{FormId, Tes4Field, Tes4Record};
use crate::{decode_failed, Field, TesError};

pub trait Item {
    /// Get this item's editor ID
    fn editor_id(&self) -> &str;

    /// Set this item's editor ID
    fn set_editor_id(&mut self, id: String);

    /// Get this item's display name
    fn name(&self) -> &str;

    /// Set this item's display name
    fn set_name(&mut self, name: String);

    /// Get this item's gold value
    fn value(&self) -> u32;

    /// Set this item's gold value
    fn set_value(&mut self, value: u32);

    /// Get this item's weight
    fn weight(&self) -> f32;

    /// Set this item's weight
    fn set_weight(&mut self, weight: f32);

    /// Get the script attached to this item, if any
    fn script(&self) -> Option<FormId>;

    /// Set the script attached to this item
    fn set_script(&mut self, script: Option<FormId>);

    /// Get the path to the NIF model used by this item, if any
    fn model(&self) -> Option<&str>;

    /// Set the path to the NIF model used by this item
    fn set_model(&mut self, model: Option<String>);

    /// Get the path to the icon texture used by this item, if any
    fn icon(&self) -> Option<&str>;

    /// Set the path to the icon texture used by this item
    fn set_icon(&mut self, icon: Option<String>);

    /// Get the bound radius of the model used by this item, if any
    fn bound_radius(&self) -> Option<f32>;

    /// Set the bound radius of the model used by this item
    fn set_bound_radius(&mut self, bound_radius: Option<f32>);

    /// Get the texture hashes for the textures used by this item, if any
    fn texture_hash(&self) -> Option<&[u8]>;

    /// Set the texture hashes for the textures used by this item, if any
    fn set_texture_hash(&mut self, texture_hash: Option<Vec<u8>>);

    /// Read data from a field in a record describing this item
    fn read_item_field(&mut self, field: &Tes4Field) -> Result<(), TesError> {
        match field.name() {
            b"EDID" => self.set_editor_id(String::from(field.get_zstring()?)),
            b"FULL" => self.set_name(String::from(field.get_zstring()?)),
            b"MODL" => self.set_model(Some(String::from(field.get_zstring()?))),
            b"MODB" => self.set_bound_radius(Some(field.get_f32()?)),
            b"MODT" => self.set_texture_hash(Some(field.get().to_vec())),
            b"ICON" => self.set_icon(Some(String::from(field.get_zstring()?))),
            b"SCRI" => self.set_script(Some(FormId(field.get_u32()?))),
            _ => {
                return Err(decode_failed(format!(
                    "Unexpected field {} in item record",
                    field.name_as_str()
                )))
            }
        }

        Ok(())
    }

    /// Write zero or more item data fields to the provided record
    fn write_item_fields(
        &self,
        record: &mut Tes4Record,
        fields: &[&[u8; 4]],
    ) -> Result<(), TesError> {
        for field_name in fields.iter().copied() {
            let field = match field_name {
                b"EDID" => Tes4Field::new_zstring(field_name, String::from(self.editor_id()))?,
                b"FULL" => Tes4Field::new_zstring(field_name, String::from(self.name()))?,
                b"MODL" => {
                    if let Some(model) = self.model() {
                        Tes4Field::new_zstring(field_name, String::from(model))?
                    } else {
                        continue;
                    }
                }
                b"MODB" => {
                    if let Some(bound_radius) = self.bound_radius() {
                        Tes4Field::new_f32(field_name, bound_radius)
                    } else {
                        continue;
                    }
                }
                b"MODT" => {
                    if let Some(texture_hash) = self.texture_hash() {
                        Tes4Field::new(field_name, texture_hash.to_vec())?
                    } else {
                        continue;
                    }
                }
                b"ICON" => {
                    if let Some(icon) = self.icon() {
                        Tes4Field::new_zstring(field_name, String::from(icon))?
                    } else {
                        continue;
                    }
                }
                b"SCRI" => {
                    if let Some(script) = self.script() {
                        Tes4Field::new_u32(field_name, script.0)
                    } else {
                        continue;
                    }
                }
                _ => {
                    return Err(TesError::RequirementFailed(format!(
                        "Unexpected item field {:?}",
                        field_name
                    )))
                }
            };

            record.add_field(field);
        }

        Ok(())
    }
}
