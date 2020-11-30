use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::{decode_failed, extract, serialize, TesError, WriteExact};

mod obconvert;
pub use obconvert::*;

pub const OPCODE_BASE: u32 = 0x4000;
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug)]
pub struct Chunk {
    pub tag: [u8; 4],
    pub version: u32,
    pub data: Vec<u8>,
}

impl Chunk {
    pub fn read<T: Read>(mut f: T) -> Result<Chunk, TesError> {
        let mut tag = [0u8; 4];
        f.read_exact(&mut tag)?;
        let version = extract!(f as u32)?;
        let size = extract!(f as u32)? as usize;
        let mut data = vec![0u8; size];
        f.read_exact(&mut data)?;

        Ok(Chunk { tag, version, data })
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    pub fn write<T: Write>(&self, mut f: T) -> Result<(), TesError> {
        f.write_exact(&self.tag)?;
        serialize!(self.version => f)?;
        serialize!(self.data.len() as u32 => f)?;
        f.write_exact(&self.data)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Plugin {
    opcode_base: u32,
    chunks: Vec<Chunk>,
}

impl Plugin {
    pub fn read<T: Read>(mut f: T) -> Result<Plugin, TesError> {
        let opcode_base = extract!(f as u32)?;
        let num_chunks = extract!(f as u32)? as usize;
        extract!(f as u32)?; // length in bytes; we'll go by the count above instead

        let mut chunks = Vec::with_capacity(num_chunks);
        for _ in 0..num_chunks {
            chunks.push(Chunk::read(&mut f)?);
        }

        Ok(Plugin {
            opcode_base,
            chunks,
        })
    }

    pub fn opcode_base(&self) -> u32 {
        self.opcode_base
    }

    pub fn iter(&self) -> impl Iterator<Item = &Chunk> + '_ {
        self.chunks.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Chunk> + '_ {
        self.chunks.iter_mut()
    }

    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        serialize!(self.opcode_base => f)?;
        serialize!(self.chunks.len() as u32 => f)?;
        let len_pos = f.seek(SeekFrom::Current(0))?;
        serialize!(0u32 => f)?;

        let data_start = len_pos + 4;
        for chunk in &self.chunks {
            chunk.write(&mut f)?;
        }
        let data_end = f.seek(SeekFrom::Current(0))?;
        let data_size = data_end - data_start;
        f.seek(SeekFrom::Start(len_pos))?;
        serialize!(data_size as u32 => f)?;
        f.seek(SeekFrom::Start(data_end))?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct CoSave {
    format_version: u32,
    obse_version: (u16, u16),
    oblivion_version: u32,
    plugins: Vec<Plugin>,
}

impl CoSave {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<CoSave, TesError> {
        CoSave::read(BufReader::new(File::open(path)?))
    }

    pub fn read<T: Read>(mut f: T) -> Result<CoSave, TesError> {
        let mut tag = [0u8; 4];
        f.read_exact(&mut tag)?;
        if tag != *b"OBSE" {
            return Err(decode_failed("Invalid co-save"));
        }

        let format_version = extract!(f as u32)?;
        let obse_version = (extract!(f as u16)?, extract!(f as u16)?);
        let oblivion_version = extract!(f as u32)?;
        let num_plugins = extract!(f as u32)? as usize;

        if format_version != FORMAT_VERSION {
            return Err(decode_failed(format!(
                "Unexpected co-save format version {}",
                format_version
            )));
        }

        let mut plugins = Vec::with_capacity(num_plugins);
        for _ in 0..num_plugins {
            plugins.push(Plugin::read(&mut f)?);
        }

        Ok(CoSave {
            format_version,
            obse_version,
            oblivion_version,
            plugins,
        })
    }

    pub fn get_plugin_by_opcode(&self, opcode_base: u32) -> Option<&Plugin> {
        self.plugins
            .iter()
            .filter(|p| p.opcode_base() == opcode_base)
            .last()
    }

    pub fn get_plugin_by_opcode_mut(&mut self, opcode_base: u32) -> Option<&mut Plugin> {
        self.plugins
            .iter_mut()
            .filter(|p| p.opcode_base() == opcode_base)
            .last()
    }

    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<(), TesError> {
        self.write(BufWriter::new(File::create(path)?))
    }

    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_exact(b"OBSE")?;
        serialize!(self.format_version => f)?;
        serialize!(self.obse_version.0 => f)?;
        serialize!(self.obse_version.1 => f)?;
        serialize!(self.oblivion_version => f)?;
        serialize!(self.plugins.len() as u32 => f)?;

        for plugin in &self.plugins {
            plugin.write(&mut f)?;
        }

        Ok(())
    }
}
