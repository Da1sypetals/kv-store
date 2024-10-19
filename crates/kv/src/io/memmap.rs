use std::{fs, path::PathBuf, sync::Arc};

use memmap2::Mmap;
use parking_lot::Mutex;

use crate::{
    errors::{Errors, Result},
    propagate_err,
};

use super::traits::IoLayer;

pub struct MemMappedIo {
    map: Arc<Mutex<Mmap>>,
}

impl MemMappedIo {
    pub fn open(path: PathBuf) -> Result<Self> {
        let file = fs::OpenOptions::new()
            .append(true)
            .read(true)
            .open(path)
            .map_err(propagate_err!(Errors::FileInitError))?;

        let map = unsafe { Mmap::map(&file).expect("Internal error: failed to map file") };

        Ok(Self {
            map: Arc::new(Mutex::new(map)),
        })
    }
}

impl IoLayer for MemMappedIo {
    fn write(&self, buf: &[u8]) -> crate::errors::Result<usize> {
        unimplemented!()
    }

    fn read(&self, buf: &mut [u8], offset: u64) -> crate::errors::Result<usize> {
        let map = self.map.lock();
        let offset = offset as usize;
        let end = offset + buf.len();
        // end can be EOF, so here a strict inequality is used
        if end > map.len() {
            Err(Errors::Eof)
        } else {
            buf.copy_from_slice(&map[offset..end]);
            Ok(buf.len())
        }
    }

    fn sync(&self) -> crate::errors::Result<()> {
        unimplemented!()
    }

    fn size(&self) -> u64 {
        self.map.lock().len() as u64
    }
}
