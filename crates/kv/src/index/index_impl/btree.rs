use crate::{
    definitions::types::ByteVec,
    index::{iter::KeyIteratorOptions, traits::KeyIndex},
    records::log_record::LogRecordPtr,
};
use parking_lot::RwLock;
use std::{collections::BTreeMap, sync::Arc};

pub struct BTreeIndex {
    tree: Arc<RwLock<BTreeMap<ByteVec, LogRecordPtr>>>,
}

impl BTreeIndex {
    pub fn new() -> Self {
        Self {
            tree: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

impl KeyIndex for BTreeIndex {
    /// Impl-defined behavior: deep copy
    fn deepcopy(&self) -> Box<dyn KeyIndex> {
        let tree = self.tree.read();
        Box::new(Self {
            tree: Arc::new(RwLock::new(tree.clone())),
        })
    }

    /// returns the *original* value if exists
    fn put(&self, key: ByteVec, pos: LogRecordPtr) -> Option<LogRecordPtr> {
        let mut tree = self.tree.write();
        tree.insert(key, pos)
    }

    /// returns the *original* value if exists
    fn delete(&self, key: ByteVec) -> Option<LogRecordPtr> {
        let mut tree = self.tree.write();
        tree.remove(&key)
    }

    /// returns `None` if not exist
    fn get(&self, key: ByteVec) -> Option<LogRecordPtr> {
        let tree = self.tree.read();
        tree.get(&key).copied()
    }

    fn iter_snapshot(&self) -> KeyIteratorOptions {
        let tree = self.tree.read();
        let items_iter = Box::new(tree.clone().into_iter());
        KeyIteratorOptions::begin(items_iter)
    }
}

mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn test_iter() {
        let index = BTreeIndex::new();
        let mut iter = index.iter_snapshot().make();

        iter.find("aa".into());
        assert_eq!(iter.next(), None);

        index.put(
            "key".into(),
            LogRecordPtr {
                file_id: 0,
                offset: 0,
            },
        );
        let mut iter = index.iter_snapshot().make();
        let next = iter.next();
        dbg!(&next);
        assert!(next.is_some());

        index.put(
            "vkey".into(),
            LogRecordPtr {
                file_id: 0,
                offset: 0,
            },
        );
        let mut iter = index.iter_snapshot().make();
        dbg!(&iter);

        iter.find("nobody".into()); // between key and vkey
        let next = iter.next();
        dbg!(&next);
        assert_eq!(next.unwrap().0, b"vkey".to_vec());

        iter.find("key".into());
        let next = iter.next();
        dbg!(&next);
        assert_eq!(next.unwrap().0, b"key".to_vec());

        iter.find("alpha".into()); // before key
        let next = iter.next();
        dbg!(&next);
        assert_eq!(next.unwrap().0, b"key".to_vec());

        iter.find("zeno".into()); // after vkey
        let next = iter.next();
        dbg!(&next);
        assert!(next.is_none());
    }

    #[test]
    fn test_rev_iter() {
        let index = BTreeIndex::new();

        for i in 0..5 {
            let key = format!("{}key{}", if i % 2 == 0 { "even" } else { "odd" }, i).into();
            index.put(
                key,
                LogRecordPtr {
                    file_id: 0,
                    offset: 0,
                },
            );
        }
        let mut iter = index.iter_snapshot().rev().make();
        assert_eq!(iter.next().unwrap().0, b"oddkey3".to_vec());
        assert_eq!(iter.next().unwrap().0, b"oddkey1".to_vec());
        assert_eq!(iter.next().unwrap().0, b"evenkey4".to_vec());
        assert_eq!(iter.next().unwrap().0, b"evenkey2".to_vec());
        assert_eq!(iter.next().unwrap().0, b"evenkey0".to_vec());
        assert!(iter.next().is_none());

        let mut iter = index.iter_snapshot().with_prefix(b"evenkey".into()).make();
        assert_eq!(iter.next().unwrap().0, b"evenkey0".to_vec());
        assert_eq!(iter.next().unwrap().0, b"evenkey2".to_vec());
        assert_eq!(iter.next().unwrap().0, b"evenkey4".to_vec());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_concurrent_put() {
        let index = Arc::new(BTreeIndex::new());
        let num_threads = 10;
        let num_keys_per_thread = 100;

        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                let index = Arc::clone(&index);
                thread::spawn(move || {
                    for j in 0..num_keys_per_thread {
                        let key = ByteVec::from(format!("key_{}_{}", i, j));
                        let pos = LogRecordPtr {
                            file_id: i as u32,
                            offset: j as u64,
                        };
                        index.put(key, pos);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify that all keys are present
        for i in 0..num_threads {
            for j in 0..num_keys_per_thread {
                let key = ByteVec::from(format!("key_{}_{}", i, j));
                let pos = index.get(key);
                assert!(
                    pos.is_some(),
                    "Key {} not found",
                    format!("key_{}_{}", i, j)
                );
                let pos = pos.unwrap();
                assert_eq!(
                    pos.file_id,
                    i as u32,
                    "File ID mismatch for key {}",
                    format!("key_{}_{}", i, j)
                );
                assert_eq!(
                    pos.offset,
                    j as u64,
                    "Offset mismatch for key {}",
                    format!("key_{}_{}", i, j)
                );
            }
        }
    }

    #[test]
    fn test_concurrent_delete() {
        let index = Arc::new(BTreeIndex::new());
        let num_threads = 10;
        let num_keys_per_thread = 100;

        // First, populate the index with keys
        for i in 0..num_threads {
            for j in 0..num_keys_per_thread {
                let key = ByteVec::from(format!("key_{}_{}", i, j));
                let pos = LogRecordPtr {
                    file_id: i as u32,
                    offset: j as u64,
                };
                index.put(key, pos);
            }
        }

        // Now, delete keys concurrently
        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                let index = Arc::clone(&index);
                thread::spawn(move || {
                    for j in 0..num_keys_per_thread {
                        let key = ByteVec::from(format!("key_{}_{}", i, j));
                        index.delete(key);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify that all keys are deleted
        for i in 0..num_threads {
            for j in 0..num_keys_per_thread {
                let key = ByteVec::from(format!("key_{}_{}", i, j));
                let pos = index.get(key);
                assert!(
                    pos.is_none(),
                    "Key {} not deleted",
                    format!("key_{}_{}", i, j)
                );
            }
        }
    }

    #[test]
    fn test_concurrent_put_and_get() {
        let index = Arc::new(BTreeIndex::new());
        let num_threads = 10;
        let num_keys_per_thread = 100;

        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                let index = Arc::clone(&index);
                thread::spawn(move || {
                    for j in 0..num_keys_per_thread {
                        let key = ByteVec::from(format!("key_{}_{}", i, j));
                        let pos = LogRecordPtr {
                            file_id: i as u32,
                            offset: j as u64,
                        };
                        index.put(key.clone(), pos);

                        // Immediately get the key and verify
                        let retrieved_pos = index.get(key);
                        assert!(
                            retrieved_pos.is_some(),
                            "Key {} not found after put",
                            format!("key_{}_{}", i, j)
                        );
                        let retrieved_pos = retrieved_pos.unwrap();
                        assert_eq!(
                            retrieved_pos.file_id,
                            i as u32,
                            "File ID mismatch for key {}",
                            format!("key_{}_{}", i, j)
                        );
                        assert_eq!(
                            retrieved_pos.offset,
                            j as u64,
                            "Offset mismatch for key {}",
                            format!("key_{}_{}", i, j)
                        );
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_put_and_delete() {
        let index = Arc::new(BTreeIndex::new());
        let num_threads = 10;
        let num_keys_per_thread = 100;

        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                let index = Arc::clone(&index);
                thread::spawn(move || {
                    for j in 0..num_keys_per_thread {
                        let key = ByteVec::from(format!("key_{}_{}", i, j));
                        let pos = LogRecordPtr {
                            file_id: i as u32,
                            offset: j as u64,
                        };
                        index.put(key.clone(), pos);

                        // Immediately delete the key
                        index.delete(key.clone());

                        // Verify that the key is deleted
                        let retrieved_pos = index.get(key);
                        assert!(
                            retrieved_pos.is_none(),
                            "Key {} not deleted after put and delete",
                            format!("key_{}_{}", i, j)
                        );
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
