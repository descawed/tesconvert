use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, RwLock};

use super::record::Tes4Record;
use crate::*;

/// Indicates the type of group a group is
#[derive(Clone, Copy, Debug)]
pub enum GroupKind {
    Top([u8; 4]),
    WorldChildren(u32),
    InteriorCellBlock(u32),
    InteriorCellSubBlock(u32),
    ExteriorCellBlock(i16, i16),
    ExteriorCellSubBlock(i16, i16),
    CellChildren(u32),
    TopicChildren(u32),
    CellPersistentChildren(u32),
    CellTemporaryChildren(u32),
    CellVisibleDistantChildren(u32),
}

impl GroupKind {
    fn read<T: Read + Seek>(mut f: T) -> Result<GroupKind, TesError> {
        let mut label = [0u8; 4];
        f.read_exact(&mut label)?;
        match f.read_le::<u32>()? {
            0 => Ok(GroupKind::Top(label)),
            1 => Ok(GroupKind::WorldChildren(u32::from_le_bytes(label))),
            2 => Ok(GroupKind::InteriorCellBlock(u32::from_le_bytes(label))),
            3 => Ok(GroupKind::InteriorCellSubBlock(u32::from_le_bytes(label))),
            4 => {
                let mut y = [0u8; 2];
                let mut x = [0u8; 2];
                y.copy_from_slice(&label[..2]);
                x.copy_from_slice(&label[2..]);
                Ok(GroupKind::ExteriorCellBlock(
                    i16::from_le_bytes(y),
                    i16::from_le_bytes(x),
                ))
            }
            5 => {
                let mut y = [0u8; 2];
                let mut x = [0u8; 2];
                y.copy_from_slice(&label[..2]);
                x.copy_from_slice(&label[2..]);
                Ok(GroupKind::ExteriorCellSubBlock(
                    i16::from_le_bytes(y),
                    i16::from_le_bytes(x),
                ))
            }
            6 => Ok(GroupKind::CellChildren(u32::from_le_bytes(label))),
            7 => Ok(GroupKind::TopicChildren(u32::from_le_bytes(label))),
            8 => Ok(GroupKind::CellPersistentChildren(u32::from_le_bytes(label))),
            9 => Ok(GroupKind::CellTemporaryChildren(u32::from_le_bytes(label))),
            10 => Ok(GroupKind::CellVisibleDistantChildren(u32::from_le_bytes(
                label,
            ))),
            kind => Err(TesError::DecodeFailed {
                description: format!("Unexpected group type {}", kind),
                source: None,
            }),
        }
    }

    fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        match *self {
            GroupKind::Top(label) => {
                f.write_all(&label)?;
                f.write_le(&0u32)?;
            }
            GroupKind::WorldChildren(id) => {
                f.write_le(&id)?;
                f.write_le(&1u32)?;
            }
            GroupKind::InteriorCellBlock(num) => {
                f.write_le(&num)?;
                f.write_le(&2u32)?;
            }
            GroupKind::InteriorCellSubBlock(num) => {
                f.write_le(&num)?;
                f.write_le(&3u32)?;
            }
            GroupKind::ExteriorCellBlock(y, x) => {
                f.write_le(&y)?;
                f.write_le(&x)?;
                f.write_le(&4u32)?;
            }
            GroupKind::ExteriorCellSubBlock(y, x) => {
                f.write_le(&y)?;
                f.write_le(&x)?;
                f.write_le(&5u32)?;
            }
            GroupKind::CellChildren(id) => {
                f.write_le(&id)?;
                f.write_le(&6u32)?;
            }
            GroupKind::TopicChildren(id) => {
                f.write_le(&id)?;
                f.write_le(&7u32)?;
            }
            GroupKind::CellPersistentChildren(id) => {
                f.write_le(&id)?;
                f.write_le(&8u32)?;
            }
            GroupKind::CellTemporaryChildren(id) => {
                f.write_le(&id)?;
                f.write_le(&9u32)?;
            }
            GroupKind::CellVisibleDistantChildren(id) => {
                f.write_le(&id)?;
                f.write_le(&10u32)?;
            }
        }
        Ok(())
    }

    fn acceptable_records(&self) -> Vec<&[u8; 4]> {
        match self {
            GroupKind::Top(ref tag) => vec![tag],
            GroupKind::WorldChildren(_) => vec![b"ROAD", b"CELL"],
            GroupKind::InteriorCellSubBlock(_) | GroupKind::ExteriorCellSubBlock(_, _) => {
                vec![b"CELL"]
            }
            GroupKind::TopicChildren(_) => vec![b"INFO"],
            GroupKind::CellPersistentChildren(_) | GroupKind::CellVisibleDistantChildren(_) => {
                vec![b"REFR", b"ACHR", b"ACRE"]
            }
            GroupKind::CellTemporaryChildren(_) => {
                vec![b"LAND", b"PGRD", b"REFR", b"ACHR", b"ACRE"]
            }
            _ => vec![], // remaining group kinds can only contain other groups
        }
    }
}

/// A group of records
///
/// Oblivion organizes records into groups. A plugin consists of a series of top-level groups, each
/// containing one record type. Certain record types, such as cells and worldspaces, also contain
/// sub-groups with their children.
#[derive(Debug)]
pub struct Group {
    kind: GroupKind,
    stamp: u32,
    groups: Vec<Group>,
    records: Vec<Arc<RwLock<Tes4Record>>>,
}

impl Group {
    /// Creates a new group of the specified kind
    pub fn new(kind: GroupKind) -> Group {
        Group {
            kind,
            stamp: 0,
            groups: vec![],
            records: vec![],
        }
    }

    /// Read a group without reading the GRUP name
    ///
    /// This assumes you've already read the name and verified that this is a GRUP.
    ///
    /// # Errors
    ///
    /// Fails if an I/O operation fails or if the group structure is not valid.
    pub fn read_without_name<T: Read + Seek>(mut f: &mut T) -> Result<Group, TesError> {
        let full_size = f.read_le::<u32>()? as usize;
        // size includes this header, so subtract that
        let mut size = full_size - 20;
        let kind = GroupKind::read(&mut f)?;
        let stamp: u32 = f.read_le()?;

        let mut groups = vec![];
        let mut records: Vec<Arc<RwLock<Tes4Record>>> = vec![];
        let mut start = f.seek(SeekFrom::Current(0))?;
        while size > 0 {
            let mut name = [0u8; 4];
            f.read_exact(&mut name)?;
            if name == *b"GRUP" {
                let group = Group::read_without_name(&mut *f)?;

                if let Some(last_record) = records.last_mut() {
                    last_record.write().unwrap().add_group(group);
                } else {
                    groups.push(group);
                }
            } else {
                f.seek(SeekFrom::Current(-4))?;
                let record = Tes4Record::read_lazy(&mut f)?;
                records.push(Arc::new(RwLock::new(record)));
            }
            let end = f.seek(SeekFrom::Current(0))?;
            let bytes_read = (end - start) as usize;
            if bytes_read > size {
                return Err(decode_failed("Data size exceeds group size"));
            }
            size -= bytes_read;
            start = end;
        }

        Ok(Group {
            kind,
            stamp,
            groups,
            records,
        })
    }

    /// Reads a group from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O operation fails or if the group structure is not valid.
    pub fn read<T: Read + Seek>(mut f: T) -> Result<Group, TesError> {
        let mut name = [0u8; 4];
        f.read_exact(&mut name)?;

        if name != *b"GRUP" {
            return Err(decode_failed(format!("Expected GRUP, found {:?}", name)));
        }

        Group::read_without_name(&mut f)
    }

    /// Add a record to this group
    pub fn add_record(&mut self, record: Tes4Record) -> Result<Arc<RwLock<Tes4Record>>, TesError> {
        // ensure the record is appropriate for this group type
        if !self.kind.acceptable_records().contains(&record.name()) {
            Err(TesError::RequirementFailed(format!(
                "Group type {:?} cannot contain record of type {:?}",
                self.kind,
                record.name()
            )))
        } else {
            self.records.push(Arc::new(RwLock::new(record)));
            Ok(Arc::clone(self.records.last().unwrap()))
        }
    }

    /// Returns an iterator over Rc smart pointers to this group's records
    // need Box because the iterator is recursive
    pub fn iter_rc(&self) -> Box<dyn Iterator<Item = Arc<RwLock<Tes4Record>>> + '_> {
        Box::new(
            self.records
                .iter()
                .map(Arc::clone)
                .chain(self.groups.iter().flat_map(|g| g.iter_rc())),
        )
    }

    /// Gets this group's [`GroupKind`]
    ///
    /// [`GroupKind`]: enum.GroupKind.html
    pub fn kind(&self) -> GroupKind {
        self.kind
    }

    /// Writes a group to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O operation fails
    // FIXME: this interface violates Rust API guideline C-RW-VALUE, but if we accept `mut f: T`
    //  instead of `f: &mut T`, we have to make our recursive call as group.write(&mut f), which
    //  creates an infinite number of versions of this method each with an extra `&mut` at the
    //  beginning, which fails to compile. With `f: &mut T`, we can pass group.write(&mut *f), which
    //  ensures each recursive call sees the same type for T. Is there a better way?
    pub fn write<T: Write + Seek>(&self, f: &mut T) -> Result<(), TesError> {
        f.write_all(b"GRUP")?;

        let len_offset = f.seek(SeekFrom::Current(0))?;
        f.write_le(&0u32)?;
        self.kind.write(&mut *f)?;
        f.write_le(&self.stamp)?;

        for record in &self.records {
            record.read().unwrap().write(&mut *f)?;
        }

        for group in &self.groups {
            group.write(&mut *f)?;
        }

        let end_offset = f.seek(SeekFrom::Current(0))?;
        // write the group size now that we know how much we wrote
        f.seek(SeekFrom::Start(len_offset))?;
        // 4 = GRUP characters
        let total_size = (end_offset - len_offset) + 4;
        f.write_le(&(total_size as u32))?;
        f.seek(SeekFrom::Start(end_offset))?; // return to where we were

        Ok(())
    }

    /// Number of records in this group, including the group record itself
    pub fn len(&self) -> usize {
        1 + self.records.len() + self.groups.iter().map(|g| g.len()).sum::<usize>()
    }
}
