use std::io;
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
    fn read<T: Read>(mut f: T) -> Result<GroupKind, TesError> {
        let mut label = [0u8; 4];
        f.read_exact(&mut label)?;
        match extract!(f as u32)? {
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

    fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
        match *self {
            GroupKind::Top(label) => {
                f.write_exact(&label)?;
                serialize!(0u32 => f)?;
            }
            GroupKind::WorldChildren(id) => {
                serialize!(id => f)?;
                serialize!(1u32 => f)?;
            }
            GroupKind::InteriorCellBlock(num) => {
                serialize!(num => f)?;
                serialize!(2u32 => f)?;
            }
            GroupKind::InteriorCellSubBlock(num) => {
                serialize!(num => f)?;
                serialize!(3u32 => f)?;
            }
            GroupKind::ExteriorCellBlock(y, x) => {
                serialize!(y => f)?;
                serialize!(x => f)?;
                serialize!(4u32 => f)?;
            }
            GroupKind::ExteriorCellSubBlock(y, x) => {
                serialize!(y => f)?;
                serialize!(x => f)?;
                serialize!(5u32 => f)?;
            }
            GroupKind::CellChildren(id) => {
                serialize!(id => f)?;
                serialize!(6u32 => f)?;
            }
            GroupKind::TopicChildren(id) => {
                serialize!(id => f)?;
                serialize!(7u32 => f)?;
            }
            GroupKind::CellPersistentChildren(id) => {
                serialize!(id => f)?;
                serialize!(8u32 => f)?;
            }
            GroupKind::CellTemporaryChildren(id) => {
                serialize!(id => f)?;
                serialize!(9u32 => f)?;
            }
            GroupKind::CellVisibleDistantChildren(id) => {
                serialize!(id => f)?;
                serialize!(10u32 => f)?;
            }
        }
        Ok(())
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
        let full_size = extract!(f as u32)? as usize;
        // size includes this header, so subtract that
        let mut size = full_size - 20;
        let kind = GroupKind::read(&mut f)?;
        let stamp = extract!(f as u32)?;

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
        f.write_exact(b"GRUP")?;

        let len_offset = f.seek(SeekFrom::Current(0))?;
        serialize!(0u32 => f)?;
        self.kind.write(&mut *f)?;
        serialize!(self.stamp => f)?;

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
        serialize!(total_size as u32 => f)?;
        f.seek(SeekFrom::Start(end_offset))?; // return to where we were

        Ok(())
    }
}
