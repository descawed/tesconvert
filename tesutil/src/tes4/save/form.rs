use super::change::ChangeRecord;
use crate::TesError;

/// A record of changes to a game object
pub trait FormChange: Sized {
    fn read(record: &ChangeRecord) -> Result<Self, TesError>;
    fn write(&self, record: &mut ChangeRecord) -> Result<(), TesError>;
}
