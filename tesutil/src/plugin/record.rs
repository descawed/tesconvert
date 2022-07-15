use std::io::{Read, Seek, Write};
use std::str;

use super::Field;
use crate::TesError;

/// Initialization status of a lazy-loaded record
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RecordStatus {
    Initialized,
    Finalized,
    Failed,
}

/// A record
///
/// This trait is a general interface to the record types of different games.
pub trait Record<F: Field>: IntoIterator<Item = F> + Sized {
    /// Records a record from a binary stream with deferred parsing
    ///
    /// This method will read a record from a stream but not immediately parse its contents. This
    /// can improve performance when reading a large number of records but only doing processing on
    /// some of them. To finish loading the record, call its `finalize` method. Some record methods
    /// will panic if called on a lazy-loaded record that has not been finalized. Refer to
    /// individual method documentation for details.
    fn read_lazy<T: Read + Seek>(f: T) -> Result<Self, TesError>;

    fn read<T: Read + Seek>(f: T) -> Result<Self, TesError> {
        let mut record = Self::read_lazy(f)?;
        record.finalize()?;
        Ok(record)
    }

    fn name(&self) -> &[u8; 4];

    /// Returns the record name as a string
    ///
    /// If the record name cannot be decoded as UTF-8 (which will never happen in a valid plugin
    /// file), the string `"<invalid>"` will be returned.
    fn display_name(&self) -> &str {
        str::from_utf8(self.name()).unwrap_or("<invalid>")
    }

    fn status(&self) -> RecordStatus;

    fn finalize(&mut self) -> Result<(), TesError>;

    // returning a boxed trait object from these methods avoids lifetimes metastasizing throughout
    // all the other related traits
    fn iter(&self) -> Box<dyn Iterator<Item = &F> + '_>;

    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut F> + '_>;

    fn write<T: Write + Seek>(&self, f: &mut T) -> Result<(), TesError>;
}
