use std::{
    fs,
    path::{Path, PathBuf},
};

use super::store::Store;

impl Store {
    pub fn blocking_copy_to(&self, dest_dir: PathBuf) -> anyhow::Result<()> {
        // lock all writes
        let _lock1 = self.active_file.write();
        let _lock2 = self.batch_commit_lock.lock();
        let _lock3 = self.merge_lock.lock();

        // backup
        let dir = self.store_config.dir.clone();

        copy_dir_contents(dir, dest_dir)?;

        Ok(())
    }
}

fn copy_dir_contents<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> std::io::Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();

    if !to.exists() {
        fs::create_dir_all(&to)?;
    }

    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = to.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_contents(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use bytes::Bytes;

    use crate::{
        config::config::Config,
        store::{store::Store, utils::TempStore},
    };

    #[test]
    fn test_backup() {
        let test_id = 27;
        let backup_name = "store/store_backup";
        {
            let (_raii, store) = TempStore::init(test_id);

            let batch1 = store.new_batched();
            for i in [0, 1, 2] {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                batch1.put(key.into(), val.into()).unwrap();
            }
            let batch2 = store.new_batched();
            for i in [10, 11, 12] {
                let key = format!("{}", i);
                let val = english_numbers::convert_all_fmt(i);
                batch2.put(key.into(), val.into()).unwrap();
            }

            batch1.commit().unwrap();
            batch2.commit().unwrap();

            store.blocking_copy_to(backup_name.into()).unwrap();
        }

        {
            // Open existing instance but not create new instance here.
            let (_raii, store) = {
                let dir = backup_name.to_string();
                let (mut store_config, file_config, batched_config) =
                    Config::from_toml("config.toml".into());
                store_config.dir = dir.clone().into();
                let store = Store::open(store_config, file_config, batched_config).unwrap();
                (TempStore { dir }, store)
            };

            store
                .put(b"114514".to_vec().into(), b"Bad Number".to_vec().into())
                .unwrap();
            store
                .put(b"1048576".to_vec().into(), b"Bad Number".to_vec().into())
                .unwrap();

            assert_eq!(
                store.list_keys(),
                [
                    Bytes::from(b"0".to_vec()),
                    Bytes::from(b"1".to_vec()),
                    Bytes::from(b"10".to_vec()),
                    Bytes::from(b"1048576".to_vec()),
                    Bytes::from(b"11".to_vec()),
                    Bytes::from(b"114514".to_vec()),
                    Bytes::from(b"12".to_vec()),
                    Bytes::from(b"2".to_vec()),
                ]
            );

            fs::remove_dir_all(backup_name).unwrap();
        }
    }
}
