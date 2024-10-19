use std::{fs, path::PathBuf};

use kv::config::config::{BatchedConfig, Config, FileConfig, StoreConfig};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DirectoryConfig {
    pub(crate) depth: usize,
}

#[derive(Serialize, Deserialize)]
pub struct DirStoreConfig {
    pub(crate) directory: DirectoryConfig,
    pub(crate) backend: Config,
}

impl DirStoreConfig {
    pub fn from_toml(path: PathBuf) -> Self {
        let var_name = toml::from_str(
            fs::read_to_string(path)
                .expect("File does not exist")
                .as_str(),
        );
        let config: Self = var_name.expect("Failed to deserialize from configuration file!");

        config
    }
}
