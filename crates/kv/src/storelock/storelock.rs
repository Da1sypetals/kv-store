use std::{
    fs::{self, File},
    path::PathBuf,
};

use fslock::LockFile;

use crate::{
    definitions::constants::LOCK_FILE_NAME,
    errors::{Errors, Result},
    propagate_err,
};

pub struct StoreExclusiveLock {
    lock: LockFile,
}

impl StoreExclusiveLock {
    pub fn lock_at(dir: PathBuf) -> Result<Self> {
        let path = dir.join(LOCK_FILE_NAME);
        let mut lock = LockFile::open(&path).expect("Failed to open lock file");
        let locked = lock
            .try_lock()
            .expect("Failed to acquire lock status (different from failed to lock)");

        if !locked {
            Err(Errors::ExclusiveStartFailure { dir })
        } else {
            Ok(Self { lock })
        }
    }
}

// impl Drop for StoreExclusiveLock {
//     fn drop(&mut self) {
//         self.lock
//             .unlock()
//             .map_err(propagate_err!(Errors::UnlockFailure))
//             .unwrap()
//     }
// }

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File, OpenOptions},
        path::PathBuf,
        rc::Rc,
    };

    use fslock::LockFile;

    use crate::{
        config::config::Config,
        errors::{Errors, Result},
        store::store::Store,
    };

    /// Temporary RAII store for tests.
    /// Please first create corresponding directory
    #[derive(Debug)]
    pub struct NonClearTempStore {
        dir: String,
    }

    impl NonClearTempStore {
        pub fn init(test_id: usize) -> Result<(Self, Store)> {
            let dir = format!("store/test_{}", test_id);
            let (mut store_config, file_config, batched_config) =
                Config::from_toml("config.toml".into());
            store_config.dir = dir.clone().into();
            let store = Store::open(store_config, file_config, batched_config)?;
            Ok((Self { dir }, store))
        }
    }

    #[test]
    fn test_unique() {
        let test_id = 11;
        let (_raii1, _store1) = NonClearTempStore::init(test_id).unwrap();
        let res = NonClearTempStore::init(test_id);

        match res {
            Err(Errors::ExclusiveStartFailure { dir }) => {
                assert_eq!(dir, PathBuf::from("store/test_11"))
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_unique_thread() {
        let test_id = 12;
        {
            let (_raii, _store1) = NonClearTempStore::init(test_id).unwrap();
            {
                let handle = std::thread::spawn(move || NonClearTempStore::init(test_id));

                match handle.join().unwrap() {
                    Err(Errors::ExclusiveStartFailure { dir }) => {
                        assert_eq!(dir, PathBuf::from("store/test_12"))
                    }
                    _ => panic!(),
                }
            }
        }

        // make sure _store1 is released
        let handle = std::thread::spawn(move || NonClearTempStore::init(test_id));
        handle.join().unwrap().expect("Unexpected panic!");
    }
}
