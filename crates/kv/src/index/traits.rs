use crate::{definitions::types::ByteVec, records::log_record::LogRecordPtr};

use super::iter::{KeyIterator, KeyIteratorOptions};

pub trait KeyIndex: Sync + Send {
    fn put(&self, key: ByteVec, ptr: LogRecordPtr) -> Option<LogRecordPtr>;
    fn delete(&self, key: ByteVec) -> Option<LogRecordPtr>;
    fn get(&self, key: ByteVec) -> Option<LogRecordPtr>;
    fn iter_snapshot(&self) -> KeyIteratorOptions;
    // object safe: size of everything in parameter/return value should be known at compile time
    // do not use `Self` here
    fn deepcopy(&self) -> Box<dyn KeyIndex>;
}

pub trait KvIterator: Sync + Send {}
