use std::{collections::HashMap, hash::Hash, sync::Arc};

use kv::{batched::batched_write::BatchedWrite, definitions::types::KvBytes, store::store::Store};
use parking_lot::Mutex;

use crate::ksis::parse::commands::Command;

use super::{
    config::DirStoreConfig,
    data_structure::{directory::Directory, key_type::KeyType, value::Value},
    errors::{ExecError, ExecOutput, ExecResult, ExecReturn},
};

pub struct DirStore<'a> {
    pub(crate) store: Store,
    pub(crate) depth: usize,
    pub(crate) batches: Arc<Mutex<HashMap<String, BatchedWrite<'a>>>>,
}

impl DirStore<'_> {
    pub fn open(config: DirStoreConfig) -> anyhow::Result<Self> {
        let store = Store::open(
            config.backend.store,
            config.backend.file,
            config.backend.batched,
        )?;

        let depth = config.directory.depth;

        Ok(Self {
            store,
            depth,
            batches: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

impl<'a> DirStore<'a> {
    // command -> output, error
    pub fn exec_command(&'a self, cmd: Command) -> ExecReturn {
        match cmd {
            Command::Get { key } => {
                self.validate_depth(&key)?;
                self.get(key)
            }
            Command::Put { key, value } => {
                self.validate_depth(&key)?;
                self.put(key, value)
            }
            Command::Delete { key } => {
                self.validate_depth(&key)?;
                self.delete(key)
            }
            Command::List { key } => {
                self.validate_depth(&key)?;
                self.list(key)
            }
            Command::MakeBatch { batchname } => {
                //
                self.make_batch(batchname)
            }
            Command::BatchedPut {
                key,
                batchname,
                value,
            } => {
                self.validate_depth(&key)?;
                self.batched_put(batchname, key, value)
            }
            Command::BatchedDelete { batchname, key } => {
                self.validate_depth(&key)?;
                self.batched_delete(batchname, key)
            }
            Command::BatchCommit { batchname } => {
                //
                self.batch_commit(batchname)
            }
        }
    }
}

impl DirStore<'_> {
    fn validate_depth(&self, key: &Directory) -> ExecResult<()> {
        if key.level() > self.depth {
            Err(ExecError::DepthExceeded {
                max: self.depth,
                given: key.level(),
            })
        } else {
            Ok(())
        }
    }
}

impl<'a> DirStore<'a> {
    pub fn get(&self, key: Directory) -> ExecReturn {
        let bin_key = key.encode_wrapped();
        match self.store.get(bin_key) {
            Ok(value) => {
                // decode value
                Ok(ExecOutput::Value(Value::decode(&value)))
            }
            Err(kv::errors::Errors::KeyNotFound) => {
                //
                Err(ExecError::KeyNotFound {
                    key: key.to_string(),
                })
            }
            Err(e) => {
                //
                Err(ExecError::Internal {
                    emsg: e.to_string(),
                })
            }
        }
    }

    pub fn put(&self, key: Directory, value: Value) -> ExecReturn {
        let bin_key = key.encode_wrapped();
        let bin_value = value.encode();
        self.store
            .put(bin_key, bin_value)
            .map(|_| ExecOutput::ok())
            .map_err(|e| ExecError::Internal {
                emsg: e.to_string(),
            })
    }

    pub fn delete(&self, key: Directory) -> ExecReturn {
        let bin_key = key.encode_wrapped();
        match self.store.delete(bin_key) {
            Ok(_) => {
                // decode value
                Ok(ExecOutput::ok())
            }
            Err(kv::errors::Errors::KeyNotFound) => {
                //
                Err(ExecError::KeyNotFound {
                    key: key.to_string(),
                })
            }
            Err(e) => {
                //
                Err(ExecError::Internal {
                    emsg: e.to_string(),
                })
            }
        }
    }

    pub fn list(&self, key: Directory) -> ExecReturn {
        let bin_key = key.encode_wrapped();

        let list = self
            .store
            .iter_options()
            .with_key_prefix(bin_key.into())
            .make()
            .map(|KvBytes { key, value }| {
                ExecOutput::Kv(
                    Directory::decode(&key).to_string(),
                    Box::new(Value::decode(&value)),
                )
            })
            .collect();

        Ok(ExecOutput::List(list))
    }

    pub fn make_batch(&'a self, batchname: String) -> ExecReturn {
        self.batches
            .lock()
            .insert(batchname, self.store.new_batched());
        Ok(ExecOutput::ok())
    }

    pub fn batched_put(&'a self, batchname: String, key: Directory, value: Value) -> ExecReturn {
        self.try_find_batch(&batchname)?;

        let batches = self.batches.lock();
        let batch = batches
            .get(&batchname)
            .expect("Expected this batch to exist");

        let bin_key = key.encode_wrapped();
        let bin_value = value.encode();
        batch
            .put(bin_key, bin_value)
            .map(|_| ExecOutput::ok())
            .map_err(|e| ExecError::Internal {
                emsg: e.to_string(),
            })
    }

    pub fn batched_delete(&'a self, batchname: String, key: Directory) -> ExecReturn {
        self.try_find_batch(&batchname)?;

        let batches = self.batches.lock();
        let batch = batches
            .get(&batchname)
            .expect("Expected this batch to exist");

        let bin_key = key.encode_wrapped();
        batch
            .delete(bin_key)
            .map(|_| ExecOutput::ok())
            .map_err(|e| ExecError::Internal {
                emsg: e.to_string(),
            })
    }

    pub fn batch_commit(&'a self, batchname: String) -> ExecReturn {
        self.try_find_batch(&batchname)?;

        let mut batches = self.batches.lock();
        let batch = batches
            .get(&batchname)
            .expect("Expected this batch to exist");

        batch
            .commit()
            .map(|_| ExecOutput::ok())
            .map_err(|e| ExecError::Internal {
                emsg: e.to_string(),
            })?;

        batches.remove(&batchname);

        Ok(ExecOutput::ok())
    }
}

impl DirStore<'_> {
    fn try_find_batch(&self, batch: &str) -> ExecResult<()> {
        if self.batches.lock().contains_key(batch) {
            Ok(())
        } else {
            Err(ExecError::BatchNotFound {
                batchname: batch.into(),
            })
        }
    }
}
