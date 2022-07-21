use crate::tes4::{EnchantmentType, FormId};

pub trait Enchantable {
    fn enchantment(&self) -> Option<FormId>;

    fn set_enchantment(&mut self, enchantment: Option<FormId>);

    fn enchantment_points(&self) -> Option<u32>;

    fn set_enchantment_points(&mut self, enchantment_points: Option<u32>);

    fn enchantment_type(&self) -> EnchantmentType;
}
