use super::file_handle::FileHandle;
use crate::{
    batched::batched_index::BatchedIndex,
    config::config::{BatchedConfig, FileConfig, StoreConfig},
    definitions::{
        constants::{get_max_prefix_number, MERGE_OK_FILE_NAME, MERGE_STORE_PATH},
        types::KvBytes,
    },
    errors::{Errors, MergePhase, Result},
    index::traits::KeyIndex,
    io::traits::IoType,
    merge::{self, merge::MergeMetadata},
    propagate_err,
    records::{
        self,
        log_record::{LogRecord, LogRecordPtr},
    },
    store::utils::format_filename,
    storelock::storelock::StoreExclusiveLock,
};
use bytes::Bytes;
use log::error;
use parking_lot::{Mutex, RwLock};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Seek},
    ops::Deref,
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, AtomicUsize},
        Arc,
    },
};

pub struct Store {
    /// readonly
    pub(crate) store_config: StoreConfig,
    /// readonly
    pub(crate) file_config: FileConfig,
    /// readonly
    pub(crate) batched_config: BatchedConfig,

    /// k-vptr in memory
    pub(crate) index: Box<dyn KeyIndex>,

    /// files
    pub(crate) active_file: Arc<RwLock<FileHandle>>,
    pub(crate) active_file_id: AtomicU32,
    /// file id -> file handle
    pub(crate) legacy_files: Arc<RwLock<HashMap<u32, FileHandle>>>,

    /// ############ batch related ############
    pub(crate) batch_commit_lock: Mutex<()>,
    pub(crate) batch_id: AtomicUsize,

    // merge
    pub(crate) merge_lock: Mutex<()>,

    // unique ownership of directory
    /// Used for RAII management ot file lock, not explicitly
    pub(crate) store_lock: StoreExclusiveLock,
}

impl Drop for Store {
    fn drop(&mut self) {
        self.active_file
            .write()
            .sync()
            .expect("Disk synchronization failed: Data failed to write to disk!")
    }
}

impl Store {
    pub fn open(
        store_config: StoreConfig,
        file_config: FileConfig,
        batched_config: BatchedConfig,
    ) -> Result<Self> {
        // create dir if exist
        let dir = store_config.dir.clone();
        fs::create_dir_all(dir.clone()).map_err(propagate_err!(Errors::CreateDirFailure {
            dir: dir.clone()
        }))?;
        // acquire unique lock on dir
        let store_lock = match StoreExclusiveLock::lock_at(store_config.dir.clone()) {
            Ok(lock) => Ok(lock),
            Err(Errors::ExclusiveStartFailure { dir }) => {
                Err(Errors::ExclusiveStartFailure { dir })
            }
            Err(e) => panic!("Failure while acquiring store lock: {:?}", e),
        }?;

        // init
        let merge_finalize = Self::merge_finalize(store_config.dir.clone());
        match merge_finalize {
            Ok(_) => {}
            Err(Errors::MergeNotFound) => {}
            Err(e) => return Err(e),
        }

        // 1. get all .store files, check biggest, check if corrupted;
        let active_file_id = get_max_prefix_number(dir.clone())?;

        match active_file_id {
            // new instance
            None => {
                let active_file_id = 0;
                let legacy_files = Arc::new(RwLock::new(HashMap::new()));
                let active_file = Arc::new(RwLock::new(FileHandle::create(
                    dir.clone(),
                    active_file_id,
                    file_config,
                )?));

                let active_file_id = AtomicU32::new(active_file_id);
                let store = Self {
                    index: store_config
                        .index_type
                        .create_index(store_config.dir.clone()),
                    store_config,
                    file_config,
                    batched_config,
                    active_file,
                    active_file_id,
                    legacy_files,
                    batch_commit_lock: Mutex::new(()),
                    batch_id: 0.into(),
                    merge_lock: Mutex::new(()),
                    store_lock,
                };
                // does not need to build index

                Ok(store)
            }
            // existing instance
            Some(_) => {
                // 2. create a file handle for each file;
                //      first ones -> files
                //      last one -> active_file
                // load mem-mapped file for now
                let active_file_id = active_file_id.unwrap();
                // let (legacy_files, active_file) =
                //     Self::fetch_files(dir.clone(), active_file_id, file_config)?;
                let (legacy_files, active_file) =
                    Self::fetch_files_mem_mapped(dir.clone(), active_file_id, file_config)?;

                // todo!("Given all files, build index")

                // then, build instance
                let active_file_id_atomic = AtomicU32::new(active_file_id);
                let mut store = Self {
                    index: store_config
                        .index_type
                        .create_index(store_config.dir.clone()),
                    store_config,
                    file_config,
                    batched_config,
                    active_file,
                    active_file_id: active_file_id_atomic,
                    legacy_files,
                    batch_commit_lock: Mutex::new(()),
                    batch_id: 0.into(),
                    merge_lock: Mutex::new(()),
                    store_lock,
                };

                // 3. load index.
                store.build_index()?;

                // 4. load file-based storage
                (store.legacy_files, store.active_file) =
                    Self::fetch_files(dir.clone(), active_file_id, file_config)?;

                // return
                Ok(store)
            }
        }
    }

    pub fn sync(&self) -> Result<()> {
        self.active_file.write().sync()
    }
}

// basic operations
impl Store {
    pub fn delete(&self, key: Bytes) -> Result<LogRecordPtr> {
        if key.is_empty() {
            return Err(Errors::KeyIsEmpty);
        }
        if self.index.get(key.to_vec()).is_none() {
            return Err(Errors::KeyNotFound);
        }

        let mut record = LogRecord::Tomb { key: key.to_vec() };
        let record_ptr = self.log(&mut record)?;
        self.index.delete(key.to_vec());

        Ok(record_ptr)
    }

    pub fn put(&self, key: Bytes, value: Bytes) -> Result<LogRecordPtr> {
        if key.is_empty() {
            return Err(Errors::KeyIsEmpty);
        }

        let mut record = LogRecord::Data {
            key: key.to_vec(),
            value: value.to_vec(),
        };

        let record_ptr = self.log(&mut record)?;

        // leave option as it is; it just returns old value.
        self.index.put(key.to_vec(), record_ptr);

        Ok(record_ptr)
    }

    /*
    /// Handle: `KeyNotFound` error, let others panic
    pub fn get(&self, key: Bytes) -> Result<Bytes> {
        if key.is_empty() {
            return Err(Errors::KeyIsEmpty);
        }

        // get log record from files
        let rec_ptr = self.index.get(key.to_vec()).ok_or(Errors::KeyNotFound)?;
        let (record, _) = if rec_ptr.file_id
            == self
                .active_file_id
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            let file = self.active_file.read();
            file.read_at_offset(rec_ptr.offset)?
        } else {
            let files = self.legacy_files.read();
            let file = files
                .get(&rec_ptr.file_id)
                .ok_or(Errors::StoreFileNotFound {
                    file_id: rec_ptr.file_id,
                })?;
            file.read_at_offset(rec_ptr.offset)?
        };

        // verify log record
        match record {
            LogRecord::Data { key: _, value } => Ok(Bytes::from(value)),
            LogRecord::Tomb { key: _ } => Err(Errors::KeyNotFound),
            LogRecord::DataInBatch {
                batch_id: _,
                key: _,
                value,
            } => Ok(Bytes::from(value)),
            LogRecord::TombInBatch {
                batch_id: _,
                key: _,
            } => Err(Errors::KeyNotFound),
            LogRecord::BatchDone { batch_id: _ } => {
                panic!("BatchDone variant is not a data record!")
            }
        }
    } */

    /// Handle: `KeyNotFound` error, let others panic
    pub fn get(&self, key: Bytes) -> Result<Bytes> {
        if key.is_empty() {
            return Err(Errors::KeyIsEmpty);
        }

        // get log record from files
        let rec_ptr = self.index.get(key.to_vec()).ok_or(Errors::KeyNotFound)?;
        let record = self.get_at(rec_ptr)?;

        // verify log record
        match record {
            LogRecord::Data { key: _, value } => Ok(Bytes::from(value)),
            LogRecord::Tomb { key: _ } => Err(Errors::KeyNotFound),
            LogRecord::DataInBatch {
                batch_id: _,
                key: _,
                value,
            } => Ok(Bytes::from(value)),
            LogRecord::TombInBatch {
                batch_id: _,
                key: _,
            } => Err(Errors::KeyNotFound),
            LogRecord::BatchDone { batch_id: _ } => {
                panic!("BatchDone variant is not a data record!")
            }
        }
    }
}

// advanced operations
impl Store {
    pub fn list_keys(&self) -> Vec<Bytes> {
        self.index
            .iter_snapshot()
            .make()
            .map(|(key, _)| key.into())
            .collect()
    }

    /// Execute a function which: **has no side effect** on every k-v pair until `keep` returns false.
    ///
    /// For example, print key and value.
    pub fn fold(&self, keep: impl Fn(KvBytes) -> bool) {
        for kvbytes in self.iter_options().make() {
            if !keep(kvbytes) {
                break;
            }
        }
    }
}

// private: op utils
impl Store {
    pub(crate) fn log(&self, record: &mut LogRecord) -> Result<LogRecordPtr> {
        let mut active_file = self.active_file.write();
        // track offset before write
        let mut offset = active_file.get_write_offset();
        while let Err(Errors::BufferOverflow) = active_file.try_append(record) {
            active_file.sync()?;

            // move current file to older file hashmap
            let active_file_id = self
                .active_file_id
                .load(std::sync::atomic::Ordering::Relaxed);
            let cur_handle = FileHandle::open(
                self.store_config.dir.clone(),
                active_file_id,
                self.file_config,
                IoType::File,
            )?;
            self.legacy_files.write().insert(active_file_id, cur_handle);

            // create new file
            let new_file = self.new_file()?;
            // this line REPLACES the content in `self.active_file` with the newly created one
            *active_file = new_file;
            offset = active_file.get_write_offset();
        }

        if self.store_config.sync_every_write {
            active_file.sync()?;
        }

        Ok(LogRecordPtr {
            file_id: self
                .active_file_id
                .load(std::sync::atomic::Ordering::Relaxed),
            offset,
        })
    }

    pub(crate) fn get_at(&self, rec_ptr: LogRecordPtr) -> Result<LogRecord> {
        if rec_ptr.file_id
            == self
                .active_file_id
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            let file = self.active_file.read();
            Ok(file.read_at_offset(rec_ptr.offset)?.0)
        } else {
            let files = self.legacy_files.read();
            let file = files
                .get(&rec_ptr.file_id)
                .ok_or(Errors::StoreFileNotFound {
                    file_id: rec_ptr.file_id,
                })?;
            Ok(file.read_at_offset(rec_ptr.offset)?.0)
        }
    }

    fn new_file(&self) -> Result<FileHandle> {
        self.active_file_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(FileHandle::create(
            self.store_config.dir.clone(),
            self.active_file_id
                .load(std::sync::atomic::Ordering::Relaxed),
            self.file_config,
        )?)
    }
}

// private: init utils
impl Store {
    fn build_index(&mut self) -> Result<()> {
        // for use of the store's batch id
        let mut newest_batch_id = 0;
        let mut batch_id: Option<usize> = None;
        let mut batched_index = self.new_batched_index();
        // let mut batched_write = self.new_batched(self.batched_config);

        // build on legacy files
        let legacy_files = self.legacy_files.read();
        let active_file_id = self
            .active_file_id
            .load(std::sync::atomic::Ordering::Relaxed);

        /* build index start */
        // iterate thru legacy files
        for file_id in 0..active_file_id {
            let file = legacy_files
                .get(&file_id)
                .expect(format!("File {} not found, corrupted!", file_id).as_str());

            let _offset = self.update_index_on_file(
                &file,
                file_id,
                &mut batch_id,
                &mut newest_batch_id,
                &mut batched_index,
            )?;
        }

        // build on active file
        let active_file = self.active_file.write();
        let offset = self.update_index_on_file(
            &active_file,
            active_file_id,
            &mut batch_id,
            &mut newest_batch_id,
            &mut batched_index,
        )?;
        /* build index end */

        self.batch_id = (newest_batch_id + 1).into();
        Ok(())
    }

    /*
    fn build_index(&mut self) -> Result<()> {
        // build on legacy files
        let legacy_files = self.legacy_files.read();
        let active_file_id = self
            .active_file_id
            .load(std::sync::atomic::Ordering::Relaxed);
        // iterate thru legacy files
        for file_id in 0..active_file_id {
            let file = legacy_files
                .get(&file_id)
                .expect(format!("File {} not found, corrupted!", file_id).as_str());
            let mut offset = 0;
            loop {
                // read a record
                let record_result = file.read_at_offset(offset);
                let (record, size) = match record_result {
                    Ok(record) => record,
                    Err(e) => {
                        if let Errors::Eof = e {
                            break;
                        } else {
                            return Err(e);
                        }
                    }
                };
                // data or delete
                match record {
                    LogRecord::Data { key, value: _ } => {
                        let ptr = LogRecordPtr { file_id, offset };
                        self.index.put(key, ptr);
                    }
                    LogRecord::Tomb { key } => {
                        self.index.delete(key);
                    }
                }
                offset += size;
            }
        }

        // build on active file
        let active_file = self.active_file.write();
        let mut offset = 0;
        loop {
            let record_result = active_file.read_at_offset(offset);
            let (record, size) = match record_result {
                Ok(record) => record,
                Err(e) => {
                    if let Errors::Eof = e {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            };
            match record {
                LogRecord::Data { key, value: _ } => {
                    let ptr = LogRecordPtr {
                        file_id: active_file_id,
                        offset,
                    };
                    self.index.put(key, ptr);
                }
                LogRecord::Tomb { key } => {
                    self.index.delete(key);
                }
            }
            offset += size;
        }

        active_file.set_write_offset(offset);
        Ok(())
    }

     */

    fn fetch_files(
        dir: PathBuf,
        active_file_id: u32,
        file_config: FileConfig,
    ) -> Result<(
        Arc<RwLock<HashMap<u32, FileHandle>>>,
        Arc<RwLock<FileHandle>>,
    )> {
        let legacy_files = Arc::new(RwLock::new(HashMap::new()));
        {
            let mut legacy_files = legacy_files.write();
            for file_id in 0..active_file_id {
                let legacy_file =
                    FileHandle::open(dir.clone(), file_id, file_config, IoType::File)?;
                legacy_files.insert(file_id, legacy_file);
            }
        }
        let active_file = Arc::new(RwLock::new(FileHandle::open(
            dir.clone(),
            active_file_id,
            file_config,
            IoType::File,
        )?));

        // scope to avoid borrow check
        {
            let active_file_write = active_file.write();
            let active_file_size = active_file_write.size();
            active_file_write.set_write_offset(active_file_size);
        }

        Ok((legacy_files, active_file))
    }

    fn fetch_files_mem_mapped(
        dir: PathBuf,
        active_file_id: u32,
        file_config: FileConfig,
    ) -> Result<(
        Arc<RwLock<HashMap<u32, FileHandle>>>,
        Arc<RwLock<FileHandle>>,
    )> {
        let legacy_files = Arc::new(RwLock::new(HashMap::new()));
        {
            let mut legacy_files = legacy_files.write();
            for file_id in 0..active_file_id {
                let legacy_file =
                    FileHandle::open(dir.clone(), file_id, file_config, IoType::MemMapped)?;
                legacy_files.insert(file_id, legacy_file);
            }
        }
        let active_file = Arc::new(RwLock::new(FileHandle::open(
            dir.clone(),
            active_file_id,
            file_config,
            IoType::MemMapped,
        )?));

        // scope to avoid borrow check
        {
            let active_file_write = active_file.write();
            let active_file_size = active_file_write.size();
            active_file_write.set_write_offset(active_file_size);
        }

        Ok((legacy_files, active_file))
    }

    /// Returns write offset on this file
    pub fn update_index_on_file(
        &self,
        file: &impl Deref<Target = FileHandle>,
        file_id: u32,
        cur_batch_id: &mut Option<usize>,
        newest_batch_id: &mut usize, // this tracks the store's batch id
        batched_index: &mut BatchedIndex<'_>, // this maintains the ongoing batch
    ) -> Result<u64> {
        let mut offset = 0;
        loop {
            // read a record
            let record_result = file.read_at_offset(offset);
            let (record, size) = match record_result {
                Ok(record) => record,
                Err(e) => {
                    if let Errors::Eof = e {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            };
            // state machine of optional batch
            match record {
                LogRecord::Data { key, value: _ } => {
                    let ptr = LogRecordPtr { file_id, offset };
                    self.index.put(key, ptr);

                    // clear batch since we are out of batch
                    *cur_batch_id = None;
                    batched_index.reset();
                }
                LogRecord::Tomb { key } => {
                    self.index.delete(key);

                    // clear batch since we are out of batch
                    *cur_batch_id = None;
                    batched_index.reset();
                }
                // start batched
                LogRecord::DataInBatch {
                    batch_id,
                    key,
                    value: _,
                } => {
                    *newest_batch_id = batch_id;
                    // Data variant points to existing record
                    if let Some(cur_bid) = cur_batch_id {
                        if batch_id == *cur_bid {
                            // case 1: in same batch
                            let ptr = LogRecordPtr { file_id, offset };
                            batched_index.mark_put(key.into(), ptr);
                        } else {
                            // case3: in new batch
                            // give up previous batch id, create new batch
                            *cur_batch_id = Some(batch_id);
                            batched_index.reset();
                            // add index
                            let ptr = LogRecordPtr { file_id, offset };
                            batched_index.mark_put(key.into(), ptr);
                        }
                    } else {
                        // case 2: start new batch from no batch
                        *cur_batch_id = Some(batch_id);
                        batched_index.reset();
                        // add index
                        let ptr = LogRecordPtr { file_id, offset };
                        batched_index.mark_put(key.into(), ptr);
                    }
                }
                LogRecord::TombInBatch { batch_id, key } => {
                    *newest_batch_id = batch_id;
                    // Tomb variant deletes corresponding index
                    if let Some(cur_bid) = cur_batch_id {
                        if batch_id == *cur_bid {
                            // case 1: in same batch
                            batched_index.mark_delete(key.into());
                        } else {
                            // case3: in new batch
                            // give up previous batch id, create new batch
                            *cur_batch_id = Some(batch_id);
                            batched_index.reset();
                            // delete index
                            batched_index.mark_delete(key.into());
                        }
                    } else {
                        // case 2: start new batch
                        *cur_batch_id = Some(batch_id);
                        batched_index.reset();
                        // delete index
                        batched_index.mark_delete(key.into());
                    }
                }
                // end batch
                LogRecord::BatchDone { batch_id } => {
                    println!("batch {} done", batch_id);
                    *newest_batch_id = batch_id;
                    if let Some(cur_bid) = cur_batch_id {
                        // the same batch
                        if batch_id == *cur_bid {
                            // end this batch, commit changes
                            batched_index.commit();
                        }
                        // else: got another batch
                    }
                    // else: an empty batch
                    *cur_batch_id = None;
                    batched_index.reset();
                }
            }
            offset += size;
        }
        Ok(offset)
    }
}

/*
    Warning:
        - Remember to configure different test ids to different tests functions
            since tests are running concurrently!
*/
#[cfg(test)]
mod tests {

    use std::fs;

    use bytes::Bytes;

    use crate::{config::config::Config, errors::Errors, store::utils::TempStore};

    use super::Store;

    #[test]
    fn test_put() {
        let (_raii, store) = TempStore::init(0);

        store.put("1".into(), "One".into()).unwrap();
        store.put("2".into(), "Two".into()).unwrap();
        store.put("1".into(), "Uno".into()).unwrap();

        let uno = store.get("1".into()).unwrap();
        assert_eq!(uno.to_vec().as_slice(), b"Uno");
        let two = store.get("2".into()).unwrap();
        assert_eq!(two.to_vec().as_slice(), b"Two");

        store.put("2".into(), "Er".into()).unwrap();
        let two = store.get("2".into()).unwrap();
        assert_eq!(two.to_vec().as_slice(), b"Er");
    }

    #[test]
    fn test_putput() {
        let test_id = 19;
        let dir = format!("store/test_{}", test_id);
        {
            // remove if exist
            fs::remove_dir_all(dir.clone());
            let (mut store_config, file_config, batched_config) =
                Config::from_toml("config.toml".into());
            store_config.dir = dir.clone().into();
            let store = Store::open(store_config, file_config, batched_config).unwrap();

            for i in 0..500 {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                store.put(key.into(), val.into()).unwrap();
            }
        }
        {
            let (mut store_config, file_config, batched_config) =
                Config::from_toml("config.toml".into());
            store_config.dir = dir.clone().into();
            let store = Store::open(store_config, file_config, batched_config).unwrap();

            dbg!(store
                .active_file
                .read()
                .write_offset
                .load(std::sync::atomic::Ordering::Relaxed));

            for i in 1000..1300 {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                store.put(key.into(), val.into()).unwrap();
            }

            dbg!(store.get("496".into()).unwrap());
            dbg!(store.get("497".into()).unwrap());
            dbg!(store.get("498".into()).unwrap());
            dbg!(store.get("499".into()).unwrap());
        }
    }

    #[test]
    fn test_getput() {
        std::env::set_var("RUST_LOG", "trace");
        pretty_env_logger::init();

        let (_raii, store) = TempStore::init(2);

        for i in 0..1000 {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            store.put(key.into(), val.into()).unwrap();
            if (i + 1) % 500 == 0 {
                info!("Inserting i = {}", i);
            }
        }

        let val = store.get("123".into()).unwrap();
        assert_eq!(val.to_vec().as_slice(), b"One Hundred and Twenty-Three");
        let val = store.get("256".into()).unwrap();
        assert_eq!(val.to_vec().as_slice(), b"Two Hundred and Fifty-Six");
        let val = store.get("999".into()).unwrap();
        assert_eq!(val.to_vec().as_slice(), b"Nine Hundred and Ninety-Nine");
        let val = store.get("1024".into()).unwrap_err();
        assert_eq!(val, Errors::KeyNotFound);

        for i in 0..1000 {
            if i % 2 == 0 {
                let key = format!("{}", i);
                // delete if exist
                store.delete(key.into()).unwrap();
            }
        }

        for i in 0..10 {
            let key = format!("{}", i);
            let val = store.get(key.clone().into());
            if i % 2 == 0 {
                assert!(val.is_err());
            } else {
                assert!(val.is_ok());
            }
        }
    }

    #[test]
    fn test_empty_key() {
        let (_raii, store) = TempStore::init(3);

        let res = store.put("".into(), "One".into());
        assert_eq!(res.unwrap_err(), Errors::KeyIsEmpty);

        let res = store.put("111".into(), "Oneoneone".into());
        assert!(res.is_ok());
        let res = store.get("111".into());
        assert_eq!(res.unwrap().to_vec().as_slice(), b"Oneoneone");

        let res = store.get("".into());
        assert_eq!(res.unwrap_err(), Errors::KeyIsEmpty);
        let res = store.delete("".into());
        assert_eq!(res.unwrap_err(), Errors::KeyIsEmpty);
    }

    #[test]
    fn test_del_put() {
        let (_raii, store) = TempStore::init(4);

        let _ = store.put("111".into(), "Oneoneone".into());

        let res = store.get("111".into());
        assert_eq!(res.unwrap().to_vec().as_slice(), b"Oneoneone");

        let res = store.delete("111".into());
        assert!(res.is_ok());
        let res = store.get("111".into());
        assert_eq!(res.unwrap_err(), Errors::KeyNotFound);

        let res = store.put("111".into(), "Yiyiyi".into());
        assert!(res.is_ok());
        let res = store.get("111".into());
        assert_eq!(res.unwrap().to_vec().as_slice(), b"Yiyiyi");
    }

    #[test]
    fn test_list_key() {
        let (_raii, store) = TempStore::init(5);

        for i in 0..15 {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            store.put(key.into(), val.into()).unwrap();
        }

        assert_eq!(
            store.list_keys(),
            vec![
                Bytes::from(b"0".to_vec()),
                Bytes::from(b"1".to_vec()),
                Bytes::from(b"10".to_vec()),
                Bytes::from(b"11".to_vec()),
                Bytes::from(b"12".to_vec()),
                Bytes::from(b"13".to_vec()),
                Bytes::from(b"14".to_vec()),
                Bytes::from(b"2".to_vec()),
                Bytes::from(b"3".to_vec()),
                Bytes::from(b"4".to_vec()),
                Bytes::from(b"5".to_vec()),
                Bytes::from(b"6".to_vec()),
                Bytes::from(b"7".to_vec()),
                Bytes::from(b"8".to_vec()),
                Bytes::from(b"9".to_vec()),
            ]
        )
    }

    #[test]
    fn test_fold() {
        let (_raii, store) = TempStore::init(6);

        for i in 0..100 {
            let key = format!("{}", i);
            let val = english_numbers::convert_all_fmt(i);
            store.put(key.into(), val.into()).unwrap();
        }

        store.fold(|kv| {
            dbg!(kv.key);
            kv.value != "Fifty"
        });
    }
}
