use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::index::index_impl::IndexType;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct BatchedConfig {
    pub(crate) max_batch_size: usize,
    pub(crate) sync_every_write: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoreConfig {
    pub(crate) dir: PathBuf,
    pub(crate) sync_every_write: bool,
    pub(crate) index_type: IndexType,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct FileConfig {
    pub(crate) max_file_size: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub store: StoreConfig,
    pub file: FileConfig,
    pub batched: BatchedConfig,
}

impl Config {
    pub fn from_toml(path: PathBuf) -> (StoreConfig, FileConfig, BatchedConfig) {
        let config: Self = toml::from_str(
            fs::read_to_string(path)
                .expect("File does not exist")
                .as_str(),
        )
        .expect("Deserialize configuration file failed!");

        (config.store, config.file, config.batched)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Config;

    #[test]
    fn serde_test() {
        let config: Config =
            toml::from_str(fs::read_to_string("config.toml").unwrap().as_str()).unwrap();

        dbg!(config);
    }
}
