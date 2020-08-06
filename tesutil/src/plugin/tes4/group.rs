use std::io;
use std::io::{Read, Write};

use crate::*;
use super::record::Record;

/// Indicates the type of group a group is
#[derive(Debug)]
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
    fn read<T: Read>(mut f: T) -> io::Result<GroupKind> {
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
                Ok(GroupKind::ExteriorCellBlock(i16::from_le_bytes(y), i16::from_le_bytes(x)))
            },
            5 => {
                let mut y = [0u8; 2];
                let mut x = [0u8; 2];
                y.copy_from_slice(&label[..2]);
                x.copy_from_slice(&label[2..]);
                Ok(GroupKind::ExteriorCellSubBlock(i16::from_le_bytes(y), i16::from_le_bytes(x)))
            },
            6 => Ok(GroupKind::CellChildren(u32::from_le_bytes(label))),
            7 => Ok(GroupKind::TopicChildren(u32::from_le_bytes(label))),
            8 => Ok(GroupKind::CellPersistentChildren(u32::from_le_bytes(label))),
            9 => Ok(GroupKind::CellTemporaryChildren(u32::from_le_bytes(label))),
            10 => Ok(GroupKind::CellVisibleDistantChildren(u32::from_le_bytes(label))),
            kind @ _ => Err(io_error(format!("Unexpected group type {}", kind))),
        }
    }

    fn write<T: Write>(&self, mut f: T) -> io::Result<()> {
        match *self {
            GroupKind::Top(label) => {
                f.write_exact(&label)?;
                serialize!(0u32 => f)?;
            },
            GroupKind::WorldChildren(id) => {
                serialize!(id => f)?;
                serialize!(1u32 => f)?;
            },
            GroupKind::InteriorCellBlock(num) => {
                serialize!(num => f)?;
                serialize!(2u32 => f)?;
            },
            GroupKind::InteriorCellSubBlock(num) => {
                serialize!(num => f)?;
                serialize!(3u32 => f)?;
            },
            GroupKind::ExteriorCellBlock(y, x) => {
                serialize!(y => f)?;
                serialize!(x => f)?;
                serialize!(4u32 => f)?;
            },
            GroupKind::ExteriorCellSubBlock(y, x) => {
                serialize!(y => f)?;
                serialize!(x => f)?;
                serialize!(5u32 => f)?;
            },
            GroupKind::CellChildren(id) => {
                serialize!(id => f)?;
                serialize!(6u32 => f)?;
            },
            GroupKind::TopicChildren(id) => {
                serialize!(id => f)?;
                serialize!(7u32 => f)?;
            },
            GroupKind::CellPersistentChildren(id) => {
                serialize!(id => f)?;
                serialize!(8u32 => f)?;
            },
            GroupKind::CellTemporaryChildren(id) => {
                serialize!(id => f)?;
                serialize!(9u32 => f)?;
            },
            GroupKind::CellVisibleDistantChildren(id) => {
                serialize!(id => f)?;
                serialize!(10u32 => f)?;
            },
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
    records: Vec<Record>,
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
    pub fn read_without_name<T: Read>(mut f: T) -> io::Result<(Group, usize)> {
        let full_size = extract!(f as u32)? as usize;
        // size includes this header, so subtract that
        let mut size = full_size - 20;
        let kind = GroupKind::read(&mut f)?;
        let stamp = extract!(f as u32)?;

        let mut groups = vec![];
        let mut records: Vec<Record> = vec![];
        while size > 0 {
            let mut name = [0u8; 4];
            f.read_exact(&mut name)?;
            if name == *b"GRUP" {
                let (group, group_size) = Group::read_without_name(&mut f)?;
                if group_size > size {
                    return Err(io_error("Sub-group size exceeds group size"));
                }
                size -= group_size;

                if let Some(last_record) = records.last_mut() {
                    last_record.add_group(group);
                } else {
                    groups.push(group);
                }
            } else {
                let (record, record_size) = Record::read_with_name(&mut f, name)?;
                if record_size > size {
                    return Err(io_error("Record size exceeds group size"));
                }
                size -= record_size;
                records.push(record);
            }
        }

        Ok((Group {
            kind,
            stamp,
            groups,
            records,
        }, full_size))
    }

    /// Reads a group from a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O operation fails or if the group structure is not valid.
    pub fn read<T: Read>(mut f: T) -> io::Result<Option<(Group, usize)>> {
        let mut name = [0u8; 4];
        if !f.read_all_or_none(&mut name)? {
            return Ok(None);
        }

        if name != *b"GRUP" {
            return Err(io_error(format!("Expected GRUP, found {:?}", name)));
        }

        Ok(Some(Group::read_without_name(f)?))
    }

    /// Writes a group to a binary stream
    ///
    /// # Errors
    ///
    /// Fails if an I/O operation fails
    pub fn write<T: Write>(&self, mut f: T) -> io::Result<usize> {
        f.write_exact(b"GRUP")?;

        // FIXME: currently, to calculate the group size, we have to write the whole group to a
        //  buffer, which sucks
        let mut buf = vec![];
        for record in self.records.iter() {
            record.write(&mut &mut buf)?;
        }

        for group in self.groups.iter() {
            group.write(&mut &mut buf)?;
        }

        // 20 = GRUP + size + label + type + stamp
        let total_size = buf.len() + 20;
        serialize!(total_size => f)?;
        self.kind.write(&mut f)?;
        serialize!(self.stamp => f)?;
        f.write_exact(&buf)?;

        Ok(total_size)
    }
}