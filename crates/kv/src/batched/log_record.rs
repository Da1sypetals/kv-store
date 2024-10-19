use crate::{
    definitions::types::ByteVec,
    errors::Errors,
    records::log_record::{LogRecord, LogRecordPtr},
};

pub enum BatchedLogRecord {
    Data { key: ByteVec, value: ByteVec },
    Tomb { key: ByteVec },
}

impl TryFrom<LogRecord> for BatchedLogRecord {
    type Error = Errors;

    fn try_from(value: LogRecord) -> Result<Self, Self::Error> {
        match value {
            LogRecord::DataInBatch {
                batch_id: _,
                key,
                value,
            } => Ok(Self::Data { key, value }),
            LogRecord::TombInBatch { batch_id: _, key } => Ok(Self::Tomb { key }),
            _ => Err(Errors::InvalidBatchedRecordType { record: value }),
        }
    }
}

impl BatchedLogRecord {
    pub fn into_batched(&self, batch_id: usize) -> LogRecord {
        match self {
            BatchedLogRecord::Data { key, value } => LogRecord::DataInBatch {
                batch_id,
                key: key.clone(),
                value: value.clone(),
            },
            BatchedLogRecord::Tomb { key } => LogRecord::TombInBatch {
                batch_id,
                key: key.clone(),
            },
        }
    }
}
