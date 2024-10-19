/*
    Abstraction:
    read/write operations <-> memory as storage cache + pointer
    implements en/decode of LogRecord into raw bytes
    abstract overmultiple storage backend
*/

use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicU64,
};

use bytes::{Buf, BytesMut};
use log::{error, info};

use crate::{
    config::config::FileConfig,
    definitions::types::ByteVec,
    errors::{Errors, Result},
    io::{
        file::FileIo,
        traits::{IoLayer, IoType},
    },
    records::log_record::LogRecord,
    store::utils::format_filename,
};

pub struct FileHandle {
    pub(crate) write_offset: AtomicU64,
    io: Box<dyn IoLayer>,
    file_config: FileConfig,
}

impl FileHandle {
    /// panic at file not found
    pub fn open(
        dir: PathBuf,
        file_id: u32,
        file_config: FileConfig,
        io_type: IoType,
    ) -> Result<Self> {
        /*
           1. find file with formatted name; (success)
           2. if cannot find, fail, panic.
        */
        let filename = format_filename(dir, file_id);
        let io = io_type.make(filename);
        // let io = Box::new(
        //     FileIo::open(filename.clone()).expect(
        //         format!(
        //             "File {:?} not found! Storage directory is corrupted.",
        //             filename
        //         )
        //         .as_str(),
        //     ),
        // );

        Ok(Self {
            write_offset: AtomicU64::new(0),
            io,
            file_config,
        })
    }

    pub fn create(dir: PathBuf, file_id: u32, file_config: FileConfig) -> Result<Self> {
        /*
           1. if found, panic
           2. else, create (success)
        */
        let filename = format_filename(dir, file_id);
        if Path::exists(&filename) {
            panic!("File found while should to be created! Storage directory is corrupted.")
        }
        let io = Box::new(FileIo::create(filename)?);
        Ok(Self {
            write_offset: AtomicU64::new(0),
            io,
            file_config,
        })
    }

    pub fn get_write_offset(&self) -> u64 {
        self.write_offset.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_write_offset(&self, offset: u64) {
        self.write_offset
            .store(offset, std::sync::atomic::Ordering::Relaxed)
    }

    // returns the current record and its size in bytes
    pub fn read_at_offset(&self, offset: u64) -> Result<(LogRecord, u64)> {
        let mut all_buf = Vec::new();
        let mut type_buf = BytesMut::zeroed(LogRecord::type_length());
        self.io.read(&mut type_buf, offset)?;
        all_buf.extend(type_buf.to_vec());
        let record_type = type_buf.get_u8();
        let mut offset_delta = LogRecord::type_length() as u64;

        // decode & verify logic:
        //      this is even more cumbersome to abstract away
        //      so I simply leave it here...
        /*
           Please do note:
           1. crc checks everything <before> it, excluding itself;
           2. remember to use all_buf to read the content <before> crc
                when creating new record types.
        */
        match record_type {
            // Data
            0 => {
                // read ksize, vsize
                let mut header_buf = BytesMut::zeroed(LogRecord::header_length_data());
                self.io.read(&mut header_buf, offset + offset_delta)?;
                all_buf.extend(header_buf.to_vec());
                let key_size = header_buf.get_u32();
                let value_size = header_buf.get_u32();
                if key_size == 0 && value_size == 0 {
                    return Err(Errors::Eof);
                }
                offset_delta += LogRecord::header_length_data() as u64;

                // read k, v
                let mut kv_buf =
                    BytesMut::zeroed((key_size + value_size) as usize + LogRecord::tail_length());
                self.io.read(&mut kv_buf, offset + offset_delta)?;

                let key = kv_buf
                    .get(0..key_size as usize)
                    .expect("read key failed")
                    .to_vec();
                all_buf.extend_from_slice(&key);
                kv_buf.advance(key_size as usize);

                let value = kv_buf
                    .get(0..value_size as usize)
                    .expect("read value failed")
                    .to_vec();
                all_buf.extend_from_slice(&value);
                kv_buf.advance(value_size as usize);
                // read crc
                let crc = kv_buf.get_u32();
                offset_delta +=
                    ((key_size + value_size) as usize + LogRecord::tail_length()) as u64;

                Self::verify_crc(&all_buf, crc)?;

                let record = LogRecord::Data { key, value };

                Ok((record, offset_delta))
            }
            // Tombstone
            1 => {
                // read ksize, vsize
                let mut header_buf = BytesMut::zeroed(LogRecord::header_length_tomb());
                self.io.read(&mut header_buf, offset + offset_delta)?;
                all_buf.extend(header_buf.to_vec());
                let key_size = header_buf.get_u32();
                if key_size == 0 {
                    return Err(Errors::Eof);
                }
                offset_delta += LogRecord::header_length_tomb() as u64;

                // read key
                let mut kv_buf = BytesMut::zeroed(key_size as usize + LogRecord::tail_length());
                self.io.read(&mut kv_buf, offset + offset_delta)?;

                let key = kv_buf
                    .get(0..key_size as usize)
                    .expect("read key failed")
                    .to_vec();
                all_buf.extend_from_slice(&key);
                kv_buf.advance(key_size as usize);
                // read crc
                let crc = kv_buf.get_u32();
                offset_delta += (key_size as usize + LogRecord::tail_length()) as u64;

                Self::verify_crc(&all_buf, crc)?;

                let record = LogRecord::Tomb { key };

                Ok((record, offset_delta))
            }
            // Data in batch
            2 => {
                let mut header_buf = BytesMut::zeroed(LogRecord::header_length_data_in_batch());
                self.io.read(&mut header_buf, offset + offset_delta)?;
                all_buf.extend(header_buf.to_vec());
                // read batch id
                let batch_id = header_buf.get_u64();
                // read ksize, vsize
                let key_size = header_buf.get_u32();
                let value_size = header_buf.get_u32();
                if key_size == 0 && value_size == 0 {
                    return Err(Errors::Eof);
                }
                offset_delta += LogRecord::header_length_data_in_batch() as u64;

                // read k, v
                let mut kv_buf =
                    BytesMut::zeroed((key_size + value_size) as usize + LogRecord::tail_length());
                self.io.read(&mut kv_buf, offset + offset_delta)?;

                let key = kv_buf
                    .get(0..key_size as usize)
                    .expect("read key failed")
                    .to_vec();
                all_buf.extend_from_slice(&key);
                kv_buf.advance(key_size as usize);

                let value = kv_buf
                    .get(0..value_size as usize)
                    .expect("read value failed")
                    .to_vec();
                all_buf.extend_from_slice(&value);
                kv_buf.advance(value_size as usize);
                // read crc
                let crc = kv_buf.get_u32();
                offset_delta +=
                    ((key_size + value_size) as usize + LogRecord::tail_length()) as u64;

                Self::verify_crc(&all_buf, crc)?;

                let record = LogRecord::DataInBatch {
                    batch_id: batch_id as usize,
                    key,
                    value,
                };

                Ok((record, offset_delta))
            }
            // Tomb in batch
            3 => {
                let mut header_buf = BytesMut::zeroed(LogRecord::header_length_tomb_in_batch());
                self.io.read(&mut header_buf, offset + offset_delta)?;
                all_buf.extend(header_buf.to_vec());
                // read batch id
                let batch_id = header_buf.get_u64();
                // read ksize, vsize
                let key_size = header_buf.get_u32();
                if key_size == 0 {
                    return Err(Errors::Eof);
                }
                offset_delta += LogRecord::header_length_tomb_in_batch() as u64;

                // read key
                let mut kv_buf = BytesMut::zeroed(key_size as usize + LogRecord::tail_length());
                self.io.read(&mut kv_buf, offset + offset_delta)?;

                let key = kv_buf
                    .get(0..key_size as usize)
                    .expect("read key failed")
                    .to_vec();
                all_buf.extend_from_slice(&key);
                kv_buf.advance(key_size as usize);
                // read crc
                let crc = kv_buf.get_u32();
                offset_delta += (key_size as usize + LogRecord::tail_length()) as u64;

                Self::verify_crc(&all_buf, crc)?;

                let record = LogRecord::TombInBatch {
                    batch_id: batch_id as usize,
                    key,
                };

                Ok((record, offset_delta))
            }
            // BatchDone
            4 => {
                let mut header_buf = BytesMut::zeroed(LogRecord::header_length_batch_done());
                self.io.read(&mut header_buf, offset + offset_delta)?;
                all_buf.extend(header_buf.to_vec());
                let batch_id = header_buf.get_u64();
                offset_delta += LogRecord::header_length_batch_done() as u64;

                let mut crc_buf = BytesMut::zeroed(LogRecord::tail_length());
                self.io.read(&mut crc_buf, offset + offset_delta)?;
                // read crc
                let crc = crc_buf.get_u32();
                offset_delta += LogRecord::tail_length() as u64;

                Self::verify_crc(&all_buf, crc)?;

                let record = LogRecord::BatchDone {
                    batch_id: batch_id as usize,
                };

                Ok((record, offset_delta))
            }
            _ => {
                panic!(
                    "Abort: invalid record type: code {}, expected (0..5)",
                    record_type
                );
            }
        }

        // todo!()
    }

    pub fn sync(&self) -> Result<()> {
        self.io.sync()
    }

    /// returns bytes written
    pub fn try_append(&self, record: &mut LogRecord) -> Result<usize> {
        if record.key_is_empty() {
            panic!("LogRecord has empty key! Internal invariant broken.");
        }
        let bin = FileHandle::encode_record(record);
        if self.write_offset.load(std::sync::atomic::Ordering::Relaxed) + bin.len() as u64
            >= self.file_config.max_file_size
        {
            return Err(Errors::BufferOverflow);
        }
        self.append(bin.as_slice())
    }
}

/// private
impl FileHandle {
    fn encode_record(record: &LogRecord) -> ByteVec {
        match record {
            LogRecord::Data { key, value } => {
                let mut res = Vec::new();
                // make sure here key_size and value_size are 32-bit!!!!!
                let key_size = key.len() as u32;
                let value_size = value.len() as u32;
                res.push(record.type_id());
                res.extend_from_slice(&key_size.to_be_bytes());
                res.extend_from_slice(&value_size.to_be_bytes());
                res.extend_from_slice(&key.as_slice());
                res.extend_from_slice(&value.as_slice());

                let crc = Self::crc(res.as_slice());
                res.extend_from_slice(&crc.to_be_bytes());

                res
            }
            LogRecord::Tomb { key } => {
                let mut res = Vec::new();
                let key_size = key.len() as u32;
                res.push(record.type_id());
                res.extend_from_slice(&key_size.to_be_bytes());
                res.extend_from_slice(&key.as_slice());

                let crc = Self::crc(res.as_slice());
                res.extend_from_slice(&crc.to_be_bytes());

                res
            }
            LogRecord::DataInBatch {
                batch_id,
                key,
                value,
            } => {
                // store batch_id as usize
                // |type|batch_id|ksz|vsz|k|v|crc|
                let mut res = Vec::new();
                // make sure here key_size and value_size are 32-bit!!!!!
                let batch_id = *batch_id as u64;
                let key_size = key.len() as u32;
                let value_size = value.len() as u32;
                res.push(record.type_id());
                res.extend_from_slice(&batch_id.to_be_bytes());
                res.extend_from_slice(&key_size.to_be_bytes());
                res.extend_from_slice(&value_size.to_be_bytes());
                res.extend_from_slice(&key.as_slice());
                res.extend_from_slice(&value.as_slice());

                let crc = Self::crc(res.as_slice());
                res.extend_from_slice(&crc.to_be_bytes());

                res
            }
            LogRecord::TombInBatch { batch_id, key } => {
                let mut res = Vec::new();
                let batch_id = *batch_id as u64;
                let key_size = key.len() as u32;
                res.push(record.type_id());
                res.extend_from_slice(&batch_id.to_be_bytes());
                res.extend_from_slice(&key_size.to_be_bytes());
                res.extend_from_slice(&key.as_slice());

                let crc = Self::crc(res.as_slice());
                res.extend_from_slice(&crc.to_be_bytes());

                res
            }
            LogRecord::BatchDone { batch_id } => {
                let mut res = Vec::new();
                res.push(record.type_id());
                let batch_id = *batch_id as u64;
                res.extend_from_slice(&batch_id.to_be_bytes());

                let crc = Self::crc(res.as_slice());
                res.extend_from_slice(&crc.to_be_bytes());

                res
            }
        }
    }

    /// returns bytes written
    fn append(&self, buf: &[u8]) -> Result<usize> {
        let n_bytes = self.io.write(buf)?;
        self.write_offset
            .fetch_add(n_bytes as u64, std::sync::atomic::Ordering::Relaxed);
        Ok(n_bytes)
    }

    pub fn crc(buf: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(buf);
        // finalize()
        let crc = hasher.finalize();
        // println!("crc: {}", crc);
        crc
        // todo!()
    }

    pub fn verify_crc(buf: &[u8], crc: u32) -> Result<()> {
        // CrcMismatch
        // todo!()
        let expected = Self::crc(buf);
        if crc == expected {
            Ok(())
        } else {
            error!("Crc mismatch: expected {}, got {}", expected, crc);
            Err(Errors::CrcMismatch { expected, got: crc })
        }
    }

    pub fn size(&self) -> u64 {
        self.io.size()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::config::FileConfig;

    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn test_file_handle_open() {
        let dir = PathBuf::from("./test_data");
        let file_id = 1;
        let file_config = FileConfig {
            max_file_size: 1024,
        };

        // Create a file for testing
        let filename = format_filename(dir.clone(), file_id);
        fs::create_dir_all(&dir).unwrap();
        let mut file = fs::File::create(&filename).unwrap();
        file.write_all(b"test data").unwrap();

        let file_handle = FileHandle::open(dir, file_id, file_config, IoType::File).unwrap();
        assert_eq!(file_handle.get_write_offset(), 0);
    }

    #[test]
    fn test_file_handle_create() {
        let dir = PathBuf::from("./test_data");
        let file_id = 2;
        let file_config = FileConfig {
            max_file_size: 1024,
        };

        // Ensure the file does not exist
        let filename = format_filename(dir.clone(), file_id);
        if fs::metadata(&filename).is_ok() {
            fs::remove_file(&filename).unwrap();
        }

        let file_handle = FileHandle::create(dir, file_id, file_config).unwrap();
        assert_eq!(file_handle.get_write_offset(), 0);
    }

    #[test]
    fn test_write_offset() {
        let dir = PathBuf::from("./test_data");
        let file_id = 3;
        let file_config = FileConfig {
            max_file_size: 1024,
        };

        let file_handle = FileHandle::create(dir.clone(), file_id, file_config).unwrap();
        assert_eq!(file_handle.get_write_offset(), 0);

        file_handle.set_write_offset(42);
        assert_eq!(file_handle.get_write_offset(), 42);
        fs::remove_file(format_filename(dir, file_id)).unwrap();
    }

    #[test]
    fn test_read_at_offset() {
        let dir = PathBuf::from("./test_data");
        let file_id = 4;
        let file_config = FileConfig {
            max_file_size: 1024,
        };

        // remove if exist
        fs::remove_file(format_filename(dir.clone(), file_id));
        let file_handle = FileHandle::create(dir.clone(), file_id, file_config).unwrap();

        // Write a test record
        let mut record = LogRecord::Data {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        };
        file_handle.try_append(&mut record).unwrap();

        // Read the record back
        let (read_record, _) = file_handle.read_at_offset(0).unwrap();
        match read_record {
            LogRecord::Data { key, value } => {
                assert_eq!(key, b"key");
                assert_eq!(value, b"value");
                // dbg!(String::from_utf8_lossy(&key));
                // dbg!(String::from_utf8_lossy(&value));
            }
            _ => panic!("Unexpected record type"),
        }
        fs::remove_file(format_filename(dir, file_id)).unwrap();
    }

    #[test]
    fn test_sync() {
        let dir = PathBuf::from("./test_data");
        let file_id = 5;
        let file_config = FileConfig {
            max_file_size: 1024,
        };

        fs::remove_file(format_filename(dir.clone(), file_id));
        let file_handle = FileHandle::create(dir, file_id, file_config).unwrap();
        file_handle.sync().unwrap();
    }

    #[test]
    fn test_try_append() {
        let dir = PathBuf::from("./test_data");
        let file_id = 6;
        fs::remove_file(format_filename(dir.clone(), file_id));
        let file_config = FileConfig {
            max_file_size: 1024,
        };

        let file_handle = FileHandle::create(dir, file_id, file_config).unwrap();

        let mut record = LogRecord::Data {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        };
        let bytes_written = file_handle.try_append(&mut record).unwrap();
        assert_eq!(bytes_written, record.encoded_len());
    }

    #[test]
    fn test_multi() {
        let dir = PathBuf::from("./test_data");
        let file_id = 7;
        let file_config = FileConfig {
            max_file_size: 1024,
        };

        // remove if exist
        fs::remove_file(format_filename(dir.clone(), file_id));
        let file_handle = FileHandle::create(dir.clone(), file_id, file_config).unwrap();

        // Write a test record
        let mut record1 = LogRecord::Data {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        };
        let mut record2 = LogRecord::Data {
            key: b"somebody".to_vec(),
            value: b"something".to_vec(),
        };
        let mut offset = 0;
        offset += file_handle.try_append(&mut record1).unwrap();
        let off1 = offset;
        offset += file_handle.try_append(&mut record2).unwrap();

        // Read the record back
        let (read_record, _) = file_handle.read_at_offset(off1 as u64).unwrap();
        match read_record {
            LogRecord::Data { key, value } => {
                assert_eq!(key, b"somebody");
                assert_eq!(value, b"something");
                // dbg!(String::from_utf8_lossy(&key));
                // dbg!(String::from_utf8_lossy(&value));
            }
            _ => panic!("Unexpected record type"),
        }

        let mut record3 = LogRecord::Tomb {
            key: b"somebody".to_vec(),
        };
        let mut record4 = LogRecord::Tomb {
            key: b"key".to_vec(),
        };
        let off2 = offset;
        offset += file_handle.try_append(&mut record3).unwrap();
        let off3 = offset;
        offset += file_handle.try_append(&mut record4).unwrap();
        let (read_record, _) = file_handle.read_at_offset(off2 as u64).unwrap();
        match read_record {
            LogRecord::Tomb { key } => {
                assert_eq!(key, b"somebody");
                // dbg!(String::from_utf8_lossy(&key));
            }
            _ => panic!("Unexpected record type"),
        }

        let (read_record, _) = file_handle.read_at_offset(off3 as u64).unwrap();
        match read_record {
            LogRecord::Tomb { key } => {
                assert_eq!(key, b"key");
                // dbg!(String::from_utf8_lossy(&key));
            }
            _ => panic!("Unexpected record type"),
        }

        // fs::remove_file(format_filename(dir, file_id)).unwrap();
    }

    #[test]
    #[should_panic(expected = "LogRecord has empty key! Internal invariant broken.")]
    fn test_key_empty() {
        let dir = PathBuf::from("./test_data");
        let file_id = 8;
        fs::remove_file(format_filename(dir.clone(), file_id));
        let file_config = FileConfig {
            max_file_size: 1024,
        };

        let file_handle = FileHandle::create(dir, file_id, file_config).unwrap();

        let mut record = LogRecord::Data {
            key: Vec::new(),
            value: b"value".to_vec(),
        };
        let bytes_written = file_handle.try_append(&mut record).unwrap();
        assert_eq!(bytes_written, record.encoded_len());

        let (read_record, _) = file_handle.read_at_offset(0).unwrap();
        match read_record {
            LogRecord::Data { key, value } => {
                assert_eq!(key, b"");
                assert_eq!(value, b"value");
                // dbg!(String::from_utf8_lossy(&key));
            }
            _ => panic!("Unexpected record type"),
        }
    }
}
