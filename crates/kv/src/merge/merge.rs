use std::{
    fs::{self, File},
    io::{self, Seek, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{
    definitions::constants::{get_max_prefix_number, MERGE_OK_FILE_NAME, MERGE_STORE_PATH},
    errors::{Errors, MergePhase, Result},
    propagate_err,
    records::log_record::LogRecord,
    store::{self, store::Store, utils::format_filename},
};

#[derive(Serialize, Deserialize)]
pub struct MergeMetadata {
    pub(crate) cur_active_file_id: u32,
    pub(crate) cur_write_offset: u64,
}

impl MergeMetadata {
    pub fn save(&self, dir: PathBuf) -> Result<()> {
        let toml = toml::to_string(self).expect("Failed to parse merge metadata to TOML format!");
        fs::write(dir, toml).map_err(propagate_err!(Errors::FileInitError))
    }
}

impl Store {
    pub fn merge(&self) -> Result<()> {
        self.merge_compact()?;
        Ok(())
    }

    pub(crate) fn merge_finalize(store_dir: PathBuf) -> Result<()> {
        Self::merge_combine(store_dir.clone())?;
        Self::merge_clean(store_dir)?;
        Ok(())
    }

    pub(crate) fn merge_temp_store(&self) -> Result<Self> {
        let mut store_config = self.store_config.clone();
        store_config.dir = store_config.dir.join(MERGE_STORE_PATH);
        // remove if exists
        fs::remove_dir_all(store_config.dir.clone());
        Store::open(store_config, self.file_config, self.batched_config)
    }

    /// Merge store is a minimal instance
    ///     with only 1 record associated to 1 key.
    /// Therefore, no hint file is needed
    ///     since the merge_store gives similar performance.
    pub(crate) fn merge_compact(&self) -> Result<()> {
        let merge_lock = self.merge_lock.try_lock().ok_or(Errors::MergeInProgress)?;

        // create merge store in new directory
        let merge_store = self.merge_temp_store()?;
        let cur_active_file_id = self
            .active_file_id
            .load(std::sync::atomic::Ordering::Relaxed);
        let cur_write_offset = self
            .active_file
            .read()
            .write_offset
            .load(std::sync::atomic::Ordering::Relaxed);
        let index = self.index.deepcopy();
        let keys = index.iter_snapshot().make();

        for (key, _) in keys {
            let ptr = index
                .get(key)
                .expect("Internal error: key not found while merging.");
            let record = self
                .get_at(ptr)
                .expect("Internal error: log record not found while merging.");

            // process the record associated with the original index
            match record {
                LogRecord::Data { key, value } => {
                    merge_store
                        .put(key.into(), value.into())
                        .expect("Internal error: original record invalid while merging");
                }
                LogRecord::Tomb { key } => {
                    merge_store
                        .delete(key.into())
                        .expect("Internal error: original record invalid while merging");
                }
                LogRecord::DataInBatch {
                    batch_id: _,
                    key,
                    value,
                } => {
                    merge_store
                        .put(key.into(), value.into())
                        .expect("Internal error: original record invalid while merging");
                }
                LogRecord::TombInBatch { batch_id: _, key } => {
                    merge_store
                        .delete(key.into())
                        .expect("Internal error: original record invalid while merging");
                }
                LogRecord::BatchDone { batch_id: _ } => {
                    // do nothing
                }
            }
        }

        let meta = MergeMetadata {
            cur_active_file_id,
            cur_write_offset,
        };
        meta.save(merge_store.store_config.dir.join(MERGE_OK_FILE_NAME))?;

        Ok(())
    }

    pub fn merge_validate(store_dir: PathBuf) -> Result<MergeMetadata> {
        let merge_meta_path = store_dir
            .clone()
            .join(MERGE_STORE_PATH)
            .join(MERGE_OK_FILE_NAME);

        let meta_string =
            fs::read_to_string(merge_meta_path).map_err(propagate_err!(Errors::MergeFailure {
                phase: MergePhase::Validate
            }))?;

        let meta: MergeMetadata =
            toml::from_str(&meta_string).map_err(propagate_err!(Errors::MergeFailure {
                phase: MergePhase::Validate
            }))?;

        Ok(meta)
    }

    pub fn merge_combine(store_dir: PathBuf) -> Result<()> {
        let merge_store_dir = store_dir.clone().join(MERGE_STORE_PATH);

        if merge_store_dir.is_dir() {
            let meta = Self::merge_validate(store_dir.clone())?;

            // update merge directory
            let newest_file_id = get_max_prefix_number(store_dir.clone())?
                .expect("Should not call merge on empty store!");
            let mut merge_file_id = get_max_prefix_number(merge_store_dir.clone())?
                .expect("Should not call merge on empty store!");

            for file_id in meta.cur_active_file_id..=newest_file_id {
                merge_file_id += 1;
                let merge_filename = format_filename(merge_store_dir.clone(), merge_file_id);
                let orig_filename = format_filename(store_dir.clone(), file_id);
                let offset = if file_id == meta.cur_active_file_id {
                    meta.cur_write_offset
                } else {
                    0
                };
                // copy to merge file
                let mut orig_file = File::open(orig_filename.clone()).expect(
                    format!("Internal error: file not found: {:?}", orig_filename).as_str(),
                );
                orig_file
                    .seek(std::io::SeekFrom::Start(offset))
                    .expect(format!("Offset = {} is greater than file size!", offset).as_str());
                let mut merge_file = File::create(merge_filename)
                    .expect(format!("Offset = {} is greater than file size!", offset).as_str());

                io::copy(&mut orig_file, &mut merge_file)
                    .expect("Failed to copy from original file to merge directory!");
            }

            // delete all .store files in original directory
            if let Ok(entries) = fs::read_dir(store_dir.clone()) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        // 检查文件名是否符合条件
                        if let Some(file_name) = path.clone().file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                // 检查文件名是否以 ".store" 结尾，并且前面的部分是纯数字
                                if file_name_str.ends_with(".store")
                                    && file_name_str[..file_name_str.len() - 6]
                                        .chars()
                                        .all(|c| c.is_digit(10))
                                {
                                    // 删除文件
                                    fs::remove_file(path.clone()).expect(
                                        format!("Failed to remove file: {:?}", path).as_str(),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // copy from merge directory to orginal directory
            if let Ok(entries) = fs::read_dir(merge_store_dir.clone()) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        // 检查文件名是否符合条件
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                // 检查文件名是否以 ".store" 结尾，并且前面的部分是纯数字
                                if file_name_str.ends_with(".store")
                                    && file_name_str[..file_name_str.len() - 6]
                                        .chars()
                                        .all(|c| c.is_digit(10))
                                {
                                    // 构建目标文件路径
                                    let dest_path = store_dir.join(file_name);
                                    // 复制文件
                                    fs::copy(&path, &dest_path).expect(
                                        format!(
                                            "Failed to copy from file: {:?} to {:?}",
                                            path, dest_path
                                        )
                                        .as_str(),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // set write offset to size of active file

            Ok(())
        } else {
            Err(Errors::MergeNotFound)
        }
    }

    pub(crate) fn merge_clean(store_dir: PathBuf) -> Result<()> {
        let merge_store_dir = store_dir.join(MERGE_STORE_PATH);
        fs::remove_dir_all(merge_store_dir).map_err(propagate_err!(Errors::MergeFailure {
            phase: MergePhase::Clean
        }))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{config::config::Config, store::store::Store};

    #[test]
    fn test_merge() {
        let test_id = 88;
        let dir = format!("store/test_{}", test_id);
        {
            // remove if exist
            fs::remove_dir_all(dir.clone());
            let (mut store_config, file_config, batched_config) =
                Config::from_toml("config.toml".into());
            store_config.dir = dir.clone().into();
            let store = Store::open(store_config, file_config, batched_config).unwrap();

            for i in 0..1000 {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                store.put(key.into(), val.into()).unwrap();
            }

            for i in 100..900 {
                let key = format!("{}", i);
                store.delete(key.into()).unwrap();
            }

            store.merge().unwrap();

            for i in 666..700 {
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

            // dbg!(store.active_file.read().get_write_offset());
            // dbg!(store
            //     .active_file_id
            //     .load(std::sync::atomic::Ordering::Relaxed));

            // 1. do some insert
            for i in 766..800 {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                store.put(key.into(), val.into()).unwrap();
            }

            // 2. do batched insert
            let batch = store.new_batched();
            for i in 2400..2450 {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                batch.put(key.into(), val.into()).unwrap();
            }

            assert_eq!(
                store.get("44".into()).unwrap(),
                english_numbers::convert_all_fmt(44)
            );
            assert_eq!(
                store.get("75".into()).unwrap(),
                english_numbers::convert_all_fmt(75)
            );

            assert_eq!(
                store.get("666".into()).unwrap(),
                english_numbers::convert_all_fmt(666)
            );
            assert_eq!(
                store.get("692".into()).unwrap(),
                english_numbers::convert_all_fmt(692)
            );
            assert_eq!(
                store.get("777".into()).unwrap(),
                english_numbers::convert_all_fmt(777)
            );
            assert_eq!(
                store.get("784".into()).unwrap(),
                english_numbers::convert_all_fmt(784)
            );
            assert_eq!(
                store.get("922".into()).unwrap(),
                english_numbers::convert_all_fmt(922)
            );
            assert_eq!(
                store.get("986".into()).unwrap(),
                english_numbers::convert_all_fmt(986)
            );
            assert!(store.get("2425".into()).is_err());
            assert!(store.get("2435".into()).is_err());

            assert_eq!(store.list_keys().len(), 100 + 100 + 34 + 34);

            batch.commit().unwrap();
            assert_eq!(
                store.get("2425".into()).unwrap(),
                english_numbers::convert_all_fmt(2425)
            );
            assert_eq!(
                store.get("2435".into()).unwrap(),
                english_numbers::convert_all_fmt(2435)
            );
            assert_eq!(
                store.get("999".into()).unwrap(),
                english_numbers::convert_all_fmt(999)
            );
            assert_eq!(
                store.get("699".into()).unwrap(),
                english_numbers::convert_all_fmt(699)
            );
            assert_eq!(
                store.get("799".into()).unwrap(),
                english_numbers::convert_all_fmt(799)
            );
            assert_eq!(
                store.get("2449".into()).unwrap(),
                english_numbers::convert_all_fmt(2449)
            );
            assert_eq!(store.list_keys().len(), 100 + 100 + 34 + 34 + 50);
        }
    }
}
