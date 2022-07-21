use crate::tes4::FormId;

pub trait Enchantable<T: Copy> {
    fn enchantment(&self) -> Option<FormId>;

    fn set_enchantment(&mut self, enchantment: Option<FormId>);

    fn enchantment_points(&self) -> Option<T>;

    fn set_enchantment_points(&mut self, enchantment_points: Option<T>);
}
