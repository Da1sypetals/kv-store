use crate::{
    config::config::BatchedConfig,
    definitions::types::ByteVec,
    errors::{Errors, Result},
    records::log_record::LogRecord,
    store::store::Store,
};
use bytes::Bytes;
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};

use super::{batched_index::BatchedIndex, log_record::BatchedLogRecord};

pub struct BatchedWrite {
    /// Maps `key` to the **last** **meaningful** write operation related to itself,
    ///     omitting intermediate operations.
    pending: Arc<Mutex<HashMap<ByteVec, BatchedLogRecord>>>,
    store: Arc<Store>,
    config: BatchedConfig,
}

pub trait CreateBatch {
    fn new_batched(&self) -> BatchedWrite;
}

impl CreateBatch for Arc<Store> {
    fn new_batched(&self) -> BatchedWrite {
        BatchedWrite {
            pending: Arc::new(Mutex::new(HashMap::new())),
            store: Arc::clone(self),
            config: self.batched_config,
        }
    }
}

impl BatchedWrite {
    pub fn put(&self, key: Bytes, value: Bytes) -> Result<()> {
        if key.is_empty() {
            return Err(Errors::KeyIsEmpty);
        }

        let mut pending = self.pending.lock();
        if pending.len() >= self.config.max_batch_size {
            return Err(Errors::BatchOverflow);
        }
        let record = BatchedLogRecord::Data {
            key: key.to_vec(),
            value: value.to_vec(),
        };
        pending.insert(key.to_vec(), record);

        Ok(())
    }

    pub fn delete(&self, key: Bytes) -> Result<()> {
        if key.is_empty() {
            return Err(Errors::KeyIsEmpty);
        }
        // Do not check key existense here
        //      as store index may vary with time.
        // Check key existense at commit.
        let mut pending = self.pending.lock();
        if pending.len() >= self.config.max_batch_size {
            return Err(Errors::BatchOverflow);
        }
        let record = BatchedLogRecord::Tomb { key: key.to_vec() };
        pending.insert(key.to_vec(), record);

        Ok(())
    }

    pub fn commit(&self) -> Result<()> {
        // Since every item in `pending` hashmap
        //      refers to different key (whose order need not to be maintained)
        // we simply read from hashmap without enforcing order.
        let mut pending = self.pending.lock();

        let _commit_lock = self.store.batch_commit_lock.lock();
        let batch_id = self
            .store
            .batch_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // no iter/adapter here since errors need to be propagated
        let mut record_ptrs = Vec::new();
        for record in pending.values_mut() {
            let ptr = self.store.log(&mut record.into_batched(batch_id))?;
            record_ptrs.push(ptr);
        }

        // println!("batch {} commit", batch_id);
        self.store.log(&mut LogRecord::BatchDone { batch_id })?;

        if self.config.sync_every_write {
            self.store.sync()?;
        }

        // if all write succeeded, we should reach here
        for (record, ptr) in pending.values_mut().zip(record_ptrs) {
            match record {
                BatchedLogRecord::Data { key, value: _ } => {
                    self.store.index.put(key.to_vec(), ptr);
                }
                BatchedLogRecord::Tomb { key } => {
                    self.store.index.delete(key.to_vec());
                }
            }
        }

        pending.clear();

        Ok(())
    }
}
