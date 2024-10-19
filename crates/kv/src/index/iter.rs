use std::sync::Arc;

use crate::{
    definitions::types::{ByteVec, KvBytes},
    records::log_record::LogRecordPtr,
    store::store::Store,
};
use either::Either;
use parking_lot::RwLock;

/*

    Note:
    If unordered index is to be further supported,
    `KeyIterator` should then be an `enum`,
    where method `find` should implement
    - linear search (unordered), or
    - binary search (ordered)
    on demand.

*/

pub struct KeyIteratorOptions {
    begin: Box<dyn DoubleEndedIterator<Item = (ByteVec, LogRecordPtr)>>,

    /// options: reversed
    reversed: bool,
    /// options: prefix
    prefix: Option<ByteVec>,
}

impl KeyIteratorOptions {
    pub fn begin(begin: Box<dyn DoubleEndedIterator<Item = (ByteVec, LogRecordPtr)>>) -> Self {
        Self {
            begin,
            reversed: false,
            prefix: None,
        }
    }

    pub fn rev(mut self) -> Self {
        self.reversed = !self.reversed;
        self
    }

    pub fn with_prefix(mut self, prefix: ByteVec) -> Self {
        match self.prefix {
            Some(_) => {
                panic!("Prefix already exist! Should not set prefix multiple times.");
            }
            None => {
                self.prefix = Some(prefix);
            }
        }
        self
    }

    pub(crate) fn make(mut self) -> KeyIterator {
        if let Some(prefix) = self.prefix {
            self.begin = Box::new(self.begin.filter(move |(key, _)| {
                prefix.len() <= key.len() && prefix.as_slice() == &key[0..prefix.len()]
            }));
        }
        let items = if self.reversed {
            self.begin.rev().collect()
        } else {
            self.begin.collect()
        };

        KeyIterator {
            items,
            cur_index: 0,
        }
    }
}

pub struct KvIteratorOptions<'a> {
    key_options: KeyIteratorOptions,
    store: &'a Store,
}

impl<'a> KvIteratorOptions<'a> {
    pub fn begin(key_options: KeyIteratorOptions, store: &'a Store) -> Self {
        Self { key_options, store }
    }

    pub fn rev(self) -> Self {
        Self {
            key_options: self.key_options.rev(),
            store: self.store,
        }
    }

    pub fn with_key_prefix(self, prefix: ByteVec) -> Self {
        Self {
            key_options: self.key_options.with_prefix(prefix),
            store: self.store,
        }
    }

    pub fn make(self) -> KvIterator<'a> {
        KvIterator {
            key_iter: Arc::new(RwLock::new(self.key_options.make())),
            store: &self.store,
        }
    }
}

/*
    Iterators
*/

#[derive(Debug)]
pub struct KeyIterator {
    /// sorted: (key, value_ptr)
    pub(crate) items: Vec<(ByteVec, LogRecordPtr)>,
    pub(crate) cur_index: usize,
}

impl KeyIterator {
    pub fn rewind(&mut self) {
        self.cur_index = 0
    }

    pub fn find(&mut self, key: ByteVec) {
        let cur_index: Either<usize, usize> =
            self.items.binary_search_by(|(k, _)| k.cmp(&key)).into();

        self.cur_index = cur_index.into_inner();
    }
}

impl Iterator for KeyIterator {
    type Item = (ByteVec, LogRecordPtr);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_index < self.items.len() {
            let res = self.items[self.cur_index].clone();
            self.cur_index += 1;
            Some(res)
        } else {
            None
        }
    }
}

pub struct KvIterator<'a> {
    key_iter: Arc<RwLock<KeyIterator>>,
    store: &'a Store,
}

impl KvIterator<'_> {
    pub fn rewind(&self) {
        self.key_iter.write().rewind();
    }

    pub fn find(&self, key: ByteVec) {
        self.key_iter.write().find(key);
    }
}

impl Iterator for KvIterator<'_> {
    type Item = KvBytes;

    fn next(&mut self) -> Option<Self::Item> {
        let mut key_iter = self.key_iter.write();
        let res = key_iter.next();
        res.map(|(key, _)| {
            let value = self
                .store
                .get(key.clone().into())
                // internal error, panic
                .expect("Key not found while iterating index! Internal invariant broken.");

            KvBytes {
                key: key.into(),
                value,
            }
        })
    }
}

impl Store {
    pub fn iter_options<'a>(&'a self) -> KvIteratorOptions<'a> {
        KvIteratorOptions {
            key_options: self.index.iter_snapshot(),
            store: self,
        }
    }
}

/*
    If test fails: First check if test_id are different?
*/
#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::{definitions::types::KvBytes, store::utils::TempStore};

    #[test]
    fn kv_iter_test() {
        let (_raii, store) = TempStore::init(0);
        for i in 100..108 {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            store.put(key.into(), val.into()).unwrap();
            if (i + 1) % 100 == 0 {
                dbg!("Inserting i = {}", i);
            }
        }

        for KvBytes { key, value } in store.iter_options().make() {
            info!("key = {}", String::from_utf8_lossy(key.to_vec().as_slice()));
            info!(
                "value = {}",
                String::from_utf8_lossy(value.to_vec().as_slice())
            );
        }
    }

    #[test]
    fn kv_iter_find_test() {
        let (_raii, store) = TempStore::init(1);
        let mut iter = store.iter_options().make();
        iter.find("205".into());
        assert!(iter.next().is_none());
        // even numbers
        for i in (0..500).filter(|x| x % 2 == 0) {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            store.put(key.into(), val.into()).unwrap();
        }

        let mut iter = store.iter_options().make();
        iter.find("205".into());
        assert_eq!(iter.next().unwrap().key.to_vec(), b"206");
        assert_eq!(iter.next().unwrap().key.to_vec(), b"208");
        for _ in 0..10 {
            dbg!(String::from_utf8_lossy(
                iter.next().unwrap().key.to_vec().as_slice()
            ));
        }
    }

    #[test]
    fn kv_iter_prefix_test() {
        let (_raii, store) = TempStore::init(2);
        let mut iter = store.iter_options().make();
        assert!(iter.next().is_none());

        for i in 0..500 {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            store.put(key.into(), val.into()).unwrap();
        }

        let mut iter = store.iter_options().with_key_prefix(b"20".to_vec()).make();
        assert_eq!(iter.next().unwrap().key, Bytes::from(b"20".as_slice()));
        assert_eq!(iter.next().unwrap().key, Bytes::from(b"200".as_slice()));
        assert_eq!(iter.next().unwrap().key, Bytes::from(b"201".as_slice()));
        assert_eq!(iter.next().unwrap().key, Bytes::from(b"202".as_slice()));

        let mut iter = store
            .iter_options()
            .with_key_prefix(b"30".to_vec())
            .rev()
            .make();
        assert_eq!(iter.next().unwrap().key, Bytes::from(b"309".as_slice()));
        assert_eq!(iter.next().unwrap().key, Bytes::from(b"308".as_slice()));
        assert_eq!(iter.next().unwrap().key, Bytes::from(b"307".as_slice()));
        assert_eq!(iter.next().unwrap().key, Bytes::from(b"306".as_slice()));
    }
}
