use std::{fs, path::PathBuf};

use kv::config::config::{BatchedConfig, Config, FileConfig, StoreConfig};
use serde::{Deserialize, Serialize};

use super::dirstore::DirStore;

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

pub fn start_dir_store(config_path: &str) -> DirStore<'_> {
    let config = DirStoreConfig::from_toml(config_path.into());
    let config = match config {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to start kv store: {}", e.to_string());
            std::process::exit(0);
        }
    };

    let ds = match DirStore::open(config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("Directory storage initialization failed: {}", e.to_string());
            std::process::exit(1);
        }
    };

    ds
}
