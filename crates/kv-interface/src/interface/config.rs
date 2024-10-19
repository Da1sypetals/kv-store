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
    pub fn from_toml(path: PathBuf) -> anyhow::Result<Self> {
        let toml_str = fs::read_to_string(path)?;
        let config: Self = toml::from_str(toml_str.as_str())?;

        Ok(config)
    }
}
