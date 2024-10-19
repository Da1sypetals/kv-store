use super::traits::IoLayer;
use crate::{
    errors::{Errors, Result},
    propagate_err,
};
use log::error;
use parking_lot::RwLock;
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::FileExt,
    path::PathBuf,
    sync::Arc,
};

pub struct FileIo {
    file: Arc<RwLock<File>>,
}

impl FileIo {
    pub fn create(path: PathBuf) -> Result<Self> {
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .write(true)
            .open(path)
            .map_err(propagate_err!(Errors::FileInitError))?;

        Ok(Self {
            file: Arc::new(RwLock::new(file)),
        })
    }

    // panic if not exist
    pub fn open(path: PathBuf) -> Result<Self> {
        let file = fs::OpenOptions::new()
            .append(true)
            .read(true)
            .write(true)
            .open(path)
            .map_err(propagate_err!(Errors::FileInitError))?;

        Ok(Self {
            file: Arc::new(RwLock::new(file)),
        })
    }
}

impl IoLayer for FileIo {
    fn write(&self, buf: &[u8]) -> Result<usize> {
        let mut file = self.file.write();
        file.write(buf)
            .map_err(propagate_err!(Errors::FileIoWriteError))
    }

    fn read(&self, buf: &mut [u8], offset: u64) -> Result<usize> {
        let file = self.file.read();
        file.read_at(buf, offset)
            .map_err(propagate_err!(Errors::FileIoReadError))
    }

    /// ensure that all in-memory data reaches the filesystem before returning
    fn sync(&self) -> Result<()> {
        let file = self.file.read();
        file.sync_all()
            .map_err(propagate_err!(Errors::FileIoSyncError))
    }

    fn size(&self) -> u64 {
        self.file.read().metadata().unwrap().len()
    }
}

mod tests {
    use super::{FileIo, IoLayer};
    use crate::errors::{Errors, Result};
    use std::{
        fs::{self, File},
        io::{Read, Write},
        path::{Path, PathBuf},
        sync::Arc,
    };

    #[test]
    fn test_create_file() -> Result<()> {
        let path = "test_create_file.txt";

        // 创建 FileIo 实例
        let file_io = FileIo::create(PathBuf::from(path)).unwrap();

        // 检查文件是否存在
        assert!(Path::new(path).exists());

        // 删除文件
        fs::remove_file(path).unwrap();

        Ok(())
    }

    #[test]
    fn test_write_and_read() {
        let path = PathBuf::from("test_write_and_read.txt");

        // 创建 FileIo 实例
        let file_io = FileIo::create(path.clone()).unwrap();

        // 写入数据
        let data = b"Hello, World!";
        let bytes_written = file_io.write(data).unwrap();
        assert_eq!(bytes_written, data.len());

        // 读取数据
        let mut buffer = vec![0; data.len()];
        let bytes_read = file_io.read(&mut buffer, 0).unwrap();
        assert_eq!(bytes_read, data.len());
        assert_eq!(buffer, data);

        // 删除文件
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_sync() {
        let path = PathBuf::from("test_sync.txt");

        // 创建 FileIo 实例
        let file_io = FileIo::create(path.clone()).unwrap();

        // 写入数据
        let data = b"Hello, World!";
        file_io.write(data).unwrap();

        // 同步数据
        file_io.sync().unwrap();

        // 手动打开文件并读取数据
        let mut file = File::open(path.clone()).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        assert_eq!(buffer, data);

        // 删除文件
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_read_offset() {
        let path = PathBuf::from("test_read_offset.txt");

        // 创建 FileIo 实例
        let file_io = FileIo::create(path.clone()).unwrap();

        // 写入数据
        let data = b"Hello, World!";
        file_io.write(data).unwrap();

        // 读取部分数据
        let mut buffer = vec![0; 5];
        let bytes_read = file_io.read(&mut buffer, 7).unwrap();
        assert_eq!(bytes_read, 5);
        assert_eq!(buffer, b"World");

        // 删除文件
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_error_handling() {
        // 尝试创建一个不存在的文件
        let result = FileIo::create(PathBuf::from("/non/existent/path"));
        assert!(result.is_err());
    }
}
