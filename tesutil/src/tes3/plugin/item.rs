pub trait Item {
    /// Get this item's ID
    fn id(&self) -> &str;

    /// Set this item's ID
    fn set_id(&mut self, id: String);

    /// Get this item's NIF model, if any
    fn model(&self) -> Option<&str>;

    /// Set this item's NIF model
    fn set_model(&mut self, model: Option<String>);

    /// Get this item's name, if any
    fn name(&self) -> Option<&str>;

    /// Set this item's name
    fn set_name(&mut self, name: Option<String>);

    /// Get this item's weight
    fn weight(&self) -> f32;

    /// Set this item's weight
    fn set_weight(&mut self, weight: f32);

    /// Get this item's value
    fn value(&self) -> u32;

    /// Set this item's value
    fn set_value(&mut self, value: u32);

    /// Get this item's script, if any
    fn script(&self) -> Option<&str>;

    /// Set this item's script
    fn set_script(&mut self, script: Option<String>);

    /// Get this item's icon texture, if any
    fn icon(&self) -> Option<&str>;

    /// Set this item's icon texture
    fn set_icon(&mut self, icon: Option<String>);
}
