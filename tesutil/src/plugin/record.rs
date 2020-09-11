use std::io::{Read, Seek, Write};
use std::str;

use super::Field;
use crate::TesError;

/// A record
///
/// This trait is a general interface to the record types of different games.
pub trait Record<F: Field>: IntoIterator<Item = F> + Sized {
    fn read<T: Read>(f: T) -> Result<Self, TesError>;

    fn name(&self) -> &[u8; 4];

    /// Returns the record name as a string
    ///
    /// If the record name cannot be decoded as UTF-8 (which will never happen in a valid plugin
    /// file), the string `"<invalid>"` will be returned.
    fn display_name(&self) -> &str {
        str::from_utf8(self.name()).unwrap_or("<invalid>")
    }

    // returning a boxed trait object from these methods avoids lifetimes metastasizing throughout
    // all the other related traits
    fn iter(&self) -> Box<dyn Iterator<Item = &F> + '_>;

    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut F> + '_>;

    fn write<T: Write + Seek>(&self, f: &mut T) -> Result<(), TesError>;
}
