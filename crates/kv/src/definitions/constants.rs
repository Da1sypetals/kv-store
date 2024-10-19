use std::{fs, path::PathBuf};

use crate::{
    errors::{Errors, Result},
    propagate_err,
};

pub const MERGE_STORE_PATH: &str = "merge";
pub const MERGE_OK_FILE_NAME: &str = "ok.toml";
pub const DISK_TREE_INDEX_FLIE_NAME: &str = "disk_tree_index.store";
pub const DISK_TREE_BUCKET_NAME: &str = "index";
pub const LOCK_FILE_NAME: &str = "exclusive.lock";
pub const SCRIPT_EXTENSION: &str = ".ksis.toml";
pub const SCRIPT_RESULTS_EXTENSION: &str = ".ksis.results.toml";

/// get max prefix number of
pub fn get_max_prefix_number(dir: PathBuf) -> Result<Option<u32>> {
    Ok(fs::read_dir(dir.clone())
        .map_err(propagate_err!(Errors::DirNotFound { dir: dir.clone() }))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "store"))
        .filter_map(|entry| {
            entry.path().file_name().and_then(|name| {
                name.to_str()
                    .map(|s| s.trim_end_matches(".store").to_string())
            })
        })
        .filter_map(|file_name| file_name.parse::<u32>().ok())
        .max())
}
