use std::{fs, path::PathBuf};

use crate::config::config::Config;

use super::store::Store;

pub fn format_filename(dir: PathBuf, file_id: u32) -> PathBuf {
    // 创建文件名
    let filename = format!("{}.store", file_id);

    // 将目录路径和文件名拼接
    let full_path = dir.join(filename);

    // 将路径转换为字符串并返回
    full_path.into()
}

pub fn legacy_files(dir: PathBuf, active_file_id: u32) -> impl IntoIterator<Item = PathBuf> {
    (0..active_file_id - 1).map(move |i| format_filename(dir.clone(), i))
}

/// Temporary RAII store for tests.
pub struct TempStore {
    pub dir: String,
}

impl TempStore {
    pub fn init(test_id: usize) -> (Self, Store) {
        let dir = format!("store/test_{}", test_id);
        // remove if exist
        fs::remove_dir_all(dir.clone());
        let (mut store_config, file_config, batched_config) =
            Config::from_toml("config.toml".into());
        store_config.dir = dir.clone().into();
        let store = Store::open(store_config, file_config, batched_config).unwrap();
        (Self { dir }, store)
    }
}

impl Drop for TempStore {
    fn drop(&mut self) {
        // remove if exist
        fs::remove_dir_all(self.dir.clone());
    }
}
