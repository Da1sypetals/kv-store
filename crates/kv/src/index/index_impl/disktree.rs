use bytes::Bytes;
use jammdb::{OpenOptions, DB};
use uuid::Uuid;

use crate::{
    definitions::{
        constants::{DISK_TREE_BUCKET_NAME, DISK_TREE_INDEX_FLIE_NAME},
        types::ByteVec,
    },
    index::{iter::KeyIteratorOptions, traits::KeyIndex},
    records::log_record::LogRecordPtr,
};
use std::{fs, path::PathBuf, sync::Arc};

pub struct DiskTreeIndex {
    path: PathBuf,
    tree: Arc<DB>,
}

impl DiskTreeIndex {
    pub fn new(dir: PathBuf) -> Self {
        let path = dir.join(DISK_TREE_INDEX_FLIE_NAME);

        // remove if exist:
        fs::remove_file(path.clone());

        let db = DB::open(path.clone())
            .expect("Internal error: failed to initialize on-disk B+ tree index!");
        let tx = db
            .tx(true)
            .expect("Internal error: failed to initialize on-disk B+ tree index!");
        tx.create_bucket(DISK_TREE_BUCKET_NAME)
            .expect("Internal error: failed to initialize on-disk B+ tree index!");
        tx.commit()
            .expect("Internal error: failed to initialize on-disk B+ tree index!");

        Self {
            path,
            tree: Arc::new(db),
        }
    }

    pub fn copy_to(filename: PathBuf) -> Self {
        // remove if exist:
        fs::remove_file(filename.clone());

        let db = DB::open(filename.clone())
            .expect("Internal error: failed to initialize on-disk B+ tree index!");
        let tx = db
            .tx(true)
            .expect("Internal error: failed to initialize on-disk B+ tree index!");
        tx.create_bucket(DISK_TREE_BUCKET_NAME)
            .expect("Internal error: failed to initialize on-disk B+ tree index!");
        tx.commit()
            .expect("Internal error: failed to initialize on-disk B+ tree index!");

        Self {
            path: filename,
            tree: Arc::new(db),
        }
    }
}

impl Drop for DiskTreeIndex {
    fn drop(&mut self) {
        // remove if exist:
        fs::remove_file(self.path.clone());
    }
}

impl KeyIndex for DiskTreeIndex {
    fn put(&self, key: ByteVec, ptr: LogRecordPtr) -> Option<LogRecordPtr> {
        let tx = self
            .tree
            .tx(true)
            .expect("Internal error: failed to update on-disk index!");
        let index_bucket = tx
            .get_bucket(DISK_TREE_BUCKET_NAME)
            .expect("Internal error: failed to update on-disk index!");

        // get return value
        let prev_val = index_bucket
            .get(key.clone())
            .map(|bin| ByteVec::from(bin.kv().value()).into());

        let bin_ptr: ByteVec = ptr.into();
        index_bucket
            .put(key, bin_ptr)
            .expect("Internal error: failed to update on-disk index!");

        tx.commit()
            .expect("Internal error: failed to update on-disk index!");

        prev_val
    }

    fn delete(&self, key: ByteVec) -> Option<LogRecordPtr> {
        let tx = self
            .tree
            .tx(true)
            .expect("Internal error: failed to update on-disk index!");
        let index_bucket = tx
            .get_bucket(DISK_TREE_BUCKET_NAME)
            .expect("Internal error: failed to update on-disk index!");

        // get return value
        let prev_val = index_bucket
            .get(key.clone())
            .map(|bin| ByteVec::from(bin.kv().value()).into());

        index_bucket
            .delete(key)
            .expect("Internal error: failed to update on-disk index!");

        tx.commit()
            .expect("Internal error: failed to update on-disk index!");

        prev_val
    }

    fn get(&self, key: ByteVec) -> Option<LogRecordPtr> {
        let tx = self
            .tree
            .tx(true)
            .expect("Internal error: failed to update on-disk index!");
        let index_bucket = tx
            .get_bucket(DISK_TREE_BUCKET_NAME)
            .expect("Internal error: failed to update on-disk index!");

        // get return value
        let val = index_bucket
            .get(key.clone())
            .map(|bin| ByteVec::from(bin.kv().value()).into());

        tx.commit()
            .expect("Internal error: failed to update on-disk index!");

        val
    }

    /// temporarily we use in-memory vector,
    /// but further a <disk-io-per-next-call> iterator can be used.
    fn iter_snapshot(&self) -> KeyIteratorOptions {
        let tx = self
            .tree
            .tx(true)
            .expect("Internal error: failed to update on-disk index!");
        let index_bucket = tx
            .get_bucket(DISK_TREE_BUCKET_NAME)
            .expect("Internal error: failed to update on-disk index!");

        let items: Vec<_> = index_bucket
            .cursor()
            .map(|kv| {
                let key = kv.key().to_vec();
                let value = kv.kv().value().to_vec().into();
                (key, value)
            })
            .collect();

        KeyIteratorOptions::begin(Box::new(items.into_iter()))
    }

    fn deepcopy(&self) -> Box<dyn KeyIndex> {
        let id = Uuid::new_v4();
        let filename = format!("{}.store", id);
        let copied = Self::copy_to(filename.into());

        {
            let tx = self
                .tree
                .tx(true)
                .expect("Internal error: failed to update on-disk index!");
            let index_bucket = tx
                .get_bucket(DISK_TREE_BUCKET_NAME)
                .expect("Internal error: failed to update on-disk index!");
            let copied_tx = copied
                .tree
                .tx(true)
                .expect("Internal error: failed to update on-disk index!");
            let copied_index_bucket = copied_tx
                .get_bucket(DISK_TREE_BUCKET_NAME)
                .expect("Internal error: failed to update on-disk index!");

            let _: Vec<_> = index_bucket
                .cursor()
                .map(|kv| {
                    let key = kv.key().to_vec();
                    let value = kv.kv().value().to_vec();
                    copied_index_bucket
                        .put(key, value)
                        .expect("Internal error: failed to update on-disk index!");
                })
                .collect();

            tx.commit()
                .expect("Internal error: failed to update on-disk index!");
            copied_tx
                .commit()
                .expect("Internal error: failed to update on-disk index!");
        }

        Box::new(copied)
    }
}
