use crate::{
    definitions::types::ByteVec,
    index::{iter::KeyIteratorOptions, traits::KeyIndex},
    records::log_record::LogRecordPtr,
};
use crossbeam_skiplist::SkipMap;
use std::sync::Arc;

pub struct SkiplistIndex {
    list: Arc<SkipMap<ByteVec, LogRecordPtr>>,
}

impl SkiplistIndex {
    pub fn new() -> Self {
        Self {
            list: Arc::new(SkipMap::new()),
        }
    }

    pub fn list_cloned(&self) -> SkipMap<ByteVec, LogRecordPtr> {
        self.list
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

impl KeyIndex for SkiplistIndex {
    fn put(&self, key: ByteVec, ptr: LogRecordPtr) -> Option<LogRecordPtr> {
        let prev_value = self.list.get(&key).map(|item| item.value().clone());
        self.list.insert(key, ptr);
        prev_value
    }

    fn delete(&self, key: ByteVec) -> Option<LogRecordPtr> {
        self.list.remove(&key).map(|item| item.value().clone())
    }

    fn get(&self, key: ByteVec) -> Option<LogRecordPtr> {
        self.list.get(&key).map(|item| item.value().clone())
    }

    fn iter_snapshot(&self) -> KeyIteratorOptions {
        // An ugly workaround
        let keys: Vec<_> = self.list_cloned().into_iter().collect();
        let items_iter = Box::new(keys.into_iter());
        KeyIteratorOptions::begin(items_iter)
    }

    fn deepcopy(&self) -> Box<dyn KeyIndex> {
        // Why the heck can a collection not support `clone`?
        // An ugly workaround
        let list: SkipMap<ByteVec, LogRecordPtr> = self.list_cloned();

        Box::new(Self {
            list: Arc::new(list),
        })
    }
}
