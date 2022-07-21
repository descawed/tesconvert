pub trait Enchantable {
    fn enchantment(&self) -> Option<&str>;

    fn set_enchantment(&mut self, enchantment: Option<String>);

    fn enchantment_points(&self) -> u32;

    fn set_enchantment_points(&mut self, enchantment_points: u32);
}
