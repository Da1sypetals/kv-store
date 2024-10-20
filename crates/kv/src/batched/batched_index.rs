use crate::{definitions::types::ByteVec, records::log_record::LogRecordPtr, store::store::Store};
use std::{collections::HashMap, sync::Arc};

pub enum BatchedIndexPtr {
    Put(LogRecordPtr),
    Delete,
}

pub struct BatchedIndex {
    /// Maps `key` to the **last** **meaningful** write operation related to itself,
    ///     omitting intermediate operations.
    pub ptr: HashMap<ByteVec, BatchedIndexPtr>,
}

impl BatchedIndex {
    pub fn new() -> Self {
        BatchedIndex {
            ptr: HashMap::new(),
        }
    }

    pub fn mark_put(&mut self, key: ByteVec, value: LogRecordPtr) {
        self.ptr.insert(key, BatchedIndexPtr::Put(value));
    }

    pub fn mark_delete(&mut self, key: ByteVec) {
        self.ptr.insert(key, BatchedIndexPtr::Delete);
    }

    pub fn reset(&mut self) {
        self.ptr.clear();
    }
}

impl BatchedIndex {
    pub(crate) fn commit(&self, store: &Store) {
        for (key, val) in self.ptr.iter() {
            match val {
                BatchedIndexPtr::Put(ptr) => {
                    store.index.put(key.clone(), ptr.clone());
                }
                BatchedIndexPtr::Delete => {
                    store.index.delete(key.clone());
                }
            }
        }
    }
}
