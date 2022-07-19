use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;

use crate::TesError;

use binrw::{binrw, BinReaderExt, BinWriterExt};

mod obconvert;
pub use obconvert::*;

pub const OPCODE_BASE: u32 = 0x4000;
pub const FORMAT_VERSION: u32 = 1;

#[binrw]
#[derive(Debug, Default)]
pub struct Chunk {
    pub tag: [u8; 4],
    pub version: u32,
    #[br(temp)]
    #[bw(calc = data.len() as u32)]
    size: u32,
    #[br(count = size)]
    pub data: Vec<u8>,
}

impl Chunk {
    pub fn new(tag: [u8; 4]) -> Chunk {
        Chunk {
            tag,
            ..Chunk::default()
        }
    }

    pub fn read<T: Read + Seek>(mut f: T) -> Result<Chunk, TesError> {
        Ok(f.read_le()?)
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    pub fn size(&self) -> usize {
        // tag + version + size = 12
        self.data.len() + 12
    }

    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_le(&self)?;

        Ok(())
    }
}

#[binrw]
#[derive(Debug)]
pub struct Plugin {
    opcode_base: u32,
    #[br(temp)]
    #[bw(calc = chunks.len() as u32)]
    num_chunks: u32,
    #[br(temp)]
    #[bw(calc = chunks.iter().map(|c| c.size()).sum::<usize>() as u32)]
    data_len: u32,
    #[br(count = num_chunks)]
    chunks: Vec<Chunk>,
}

impl Plugin {
    pub fn read<T: Read + Seek>(mut f: T) -> Result<Plugin, TesError> {
        Ok(f.read_le()?)
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

    pub fn add_chunk(&mut self, chunk: Chunk) {
        self.chunks.push(chunk);
    }

    pub fn write<T: Write + Seek>(&self, mut f: T) -> Result<(), TesError> {
        f.write_le(&self)?;

        Ok(())
    }
}

#[binrw]
#[derive(Debug)]
#[brw(magic = b"OBSE")]
pub struct CoSave {
    #[br(assert(format_version == FORMAT_VERSION))]
    format_version: u32,
    obse_version: (u16, u16),
    oblivion_version: u32,
    #[br(temp)]
    #[bw(calc = plugins.len() as u32)]
    num_plugins: u32,
    #[br(count = num_plugins)]
    plugins: Vec<Plugin>,
}

impl CoSave {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<CoSave, TesError> {
        CoSave::read(BufReader::new(File::open(path)?))
    }

    pub fn read<T: Read + Seek>(mut f: T) -> Result<CoSave, TesError> {
        Ok(f.read_le()?)
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
        f.write_le(&self)?;

        Ok(())
    }
}
