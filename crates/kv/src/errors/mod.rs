use std::path::PathBuf;

use log::error;
use thiserror::Error;

use crate::records::log_record::LogRecord;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MergePhase {
    Compact,
    Validate,
    Combine,
    Clean,
}

// Error
#[derive(Error, Debug, PartialEq)]
pub enum Errors {
    #[error("A create directory failure occured while creating {:?}", dir)]
    CreateDirFailure { dir: PathBuf },
    #[error("A file IO read failure occured while reading {:?}", dir)]
    DirNotFound { dir: PathBuf },
    #[error("A directory not found failure occured!")]
    FileIoReadError,
    #[error("A file IO write failure occured!")]
    FileIoWriteError,
    #[error("A file IO sync failure occured!")]
    FileIoSyncError,
    #[error("A file initialization failure occured!")]
    FileInitError,
    #[error("A buffer overflow failure occured!")]
    BufferOverflow,
    #[error("An index update failure occured!")]
    IndexUpdateError,
    #[error("A key is empty failure occured!")]
    KeyIsEmpty,
    #[error("A key not found failure occured!")]
    KeyNotFound,
    #[error("A file not found failure occured! File id: {}", file_id)]
    StoreFileNotFound { file_id: u32 },
    #[error("Reached an end of file (EOF)!")]
    Eof,
    #[error(
        "A CRC verification failure occured! expected {}, got {} ",
        expected,
        got
    )]
    CrcMismatch { expected: u32, got: u32 },
    #[error("A batched write overflow failure occured!")]
    BatchOverflow,
    #[error("An invalid batched record type failure occured!")]
    InvalidBatchedRecordType { record: LogRecord },
    #[error("A merge in process failure occured!")]
    MergeInProgress,
    #[error("A merge not found failure occured!")]
    MergeNotFound,
    #[error("A merge failure occured at phase: {:?}", phase)]
    MergeFailure { phase: MergePhase },
    #[error("An exclusive start of store has failed at directory: {:?}!", dir)]
    ExclusiveStartFailure { dir: PathBuf },
    #[error("Failed to unlock on shutting instance!")]
    UnlockFailure,
    #[error("Binary size of record mismatch! expected {}, got {} ", expected, got)]
    BinarySizeMismatch { expected: u32, got: u32 },
    #[error("A list empty failure occured!")]
    ListIsEmpty,
}

/// use `ok_or` for `Option<T>`
#[macro_export]
macro_rules! propagate_err {
    ($new_e: expr) => {
        |e| {
            error!("Error occurred: {}", e);
            let new_e = $new_e;
            new_e
        }
    };
}

// Result
pub type Result<T> = std::result::Result<T, Errors>;
