use std::path::PathBuf;

use crate::errors::Result;

use super::{file::FileIo, memmap::MemMappedIo};

pub trait IoLayer: Sync + Send {
    fn write(&self, buf: &[u8]) -> Result<usize>;
    fn read(&self, buf: &mut [u8], offset: u64) -> Result<usize>;
    fn sync(&self) -> Result<()>;
    /// maintain the size to get write offset at open.
    fn size(&self) -> u64;
}

pub enum IoType {
    File,
    MemMapped,
}

impl IoType {
    pub fn make(self, filename: PathBuf) -> Box<dyn IoLayer> {
        match self {
            IoType::File => Box::new(
                FileIo::open(filename.clone()).expect(
                    format!(
                        "File {:?} not found! Storage directory is corrupted.",
                        filename
                    )
                    .as_str(),
                ),
            ),
            IoType::MemMapped => Box::new(
                MemMappedIo::open(filename.clone()).expect(
                    format!(
                        "File {:?} not found! Storage directory is corrupted.",
                        filename
                    )
                    .as_str(),
                ),
            ),
        }
    }
}
