use crate::{
    definitions::types::{ByteVec, KvSizeType},
    errors::{Errors, Result},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogRecordPtr {
    pub(crate) file_id: u32,
    pub(crate) offset: u64,
}

impl From<LogRecordPtr> for ByteVec {
    /// used for storing it in disk
    fn from(value: LogRecordPtr) -> Self {
        let mut res = value.file_id.to_be_bytes().to_vec();
        res.append(&mut value.offset.to_be_bytes().to_vec());
        res
    }
}

impl From<ByteVec> for LogRecordPtr {
    fn from(value: Vec<u8>) -> Self {
        let file_id = u32::from_be_bytes([value[0], value[1], value[2], value[3]]);
        let offset = u64::from_be_bytes([
            value[4], value[5], value[6], value[7], value[8], value[9], value[10], value[11],
        ]);

        Self { file_id, offset }
    }
}

#[derive(Debug, PartialEq)]
pub enum LogRecord {
    Data {
        key: ByteVec,
        value: ByteVec,
    },
    Tomb {
        key: ByteVec,
    },
    DataInBatch {
        batch_id: usize,
        key: ByteVec,
        value: ByteVec,
    },
    TombInBatch {
        batch_id: usize,
        key: ByteVec,
    },
    BatchDone {
        batch_id: usize,
    },
}

// metadata
impl LogRecord {
    pub fn key_is_empty(&self) -> bool {
        match self {
            LogRecord::Data { key, value: _ } => key.is_empty(),
            LogRecord::Tomb { key } => key.is_empty(),
            LogRecord::DataInBatch {
                batch_id: _,
                key,
                value: _,
            } => key.is_empty(),
            LogRecord::TombInBatch { batch_id: _, key } => key.is_empty(),
            LogRecord::BatchDone { batch_id: _ } => false,
        }
    }

    pub fn type_id(&self) -> u8 {
        match self {
            LogRecord::Data { key: _, value: _ } => 0,
            LogRecord::Tomb { key: _ } => 1,
            LogRecord::DataInBatch {
                batch_id: _,
                key: _,
                value: _,
            } => 2,
            LogRecord::TombInBatch {
                batch_id: _,
                key: _,
            } => 3,
            LogRecord::BatchDone { batch_id: _ } => 4,
        }
    }

    pub fn encoded_len(&self) -> usize {
        match self {
            LogRecord::Data { key, value } => {
                1 /* type */ + 8 /* sizes */
                    + key.len() + value.len() + 4 /* crc */
            }
            LogRecord::Tomb { key } => {
                1 /* type */ + 4 /* size */ + key.len() + 4 /* crc */
            }
            LogRecord::DataInBatch {
                batch_id: _,
                key,
                value,
            } => {
                1 /* type */ + 8 /* sizes */ + key.len() + value.len() +
                    4 /* crc */ + 8 /* batch id */
            }
            LogRecord::TombInBatch { batch_id: _, key } => {
                1 /* type */ + 4 /* size */ + key.len() +
                    4 /* crc */ + 8 /* batch id */
            }
            LogRecord::BatchDone { batch_id: _ } => {
                1 /* type */ + 8 /* batch id */
            }
        }
    }
}

// ser/de metadata
impl LogRecord {
    pub fn type_length() -> usize {
        1
    }

    pub fn header_length_data() -> usize {
        size_of::<KvSizeType>() * 2 /* keysize + valuesize */
    }

    pub fn header_length_tomb() -> usize {
        size_of::<KvSizeType>() /* keysize */
    }

    pub fn header_length_data_in_batch() -> usize {
        size_of::<KvSizeType>() * 2 /* keysize + valuesize */ + 8 /* batch id */
    }

    pub fn header_length_tomb_in_batch() -> usize {
        size_of::<KvSizeType>() /* keysize */ + 8 /* batch id */
    }

    pub fn header_length_batch_done() -> usize {
        size_of::<usize>()
    }

    pub fn tail_length() -> usize {
        4 /* crc */
    }
}
