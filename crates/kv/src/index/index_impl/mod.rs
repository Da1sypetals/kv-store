pub mod btree;
pub mod disktree;
pub mod skiplist;

use std::path::PathBuf;

use super::traits::KeyIndex;
use btree::BTreeIndex;
use disktree::DiskTreeIndex;
use serde::{Deserialize, Serialize};
use skiplist::SkiplistIndex;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum IndexType {
    BTree,
    Skiplist,
    DiskTree,
}

impl IndexType {
    pub fn create_index(&self, dir: PathBuf) -> Box<dyn KeyIndex> {
        match self {
            IndexType::BTree => Box::new(BTreeIndex::new()),
            IndexType::Skiplist => Box::new(SkiplistIndex::new()),
            IndexType::DiskTree => Box::new(DiskTreeIndex::new(dir)),
        }
    }
}
