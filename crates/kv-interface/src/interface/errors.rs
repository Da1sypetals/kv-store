use thiserror::Error;

use super::data_structure::{directory::Directory, value::Value};

#[derive(Error, Debug)]
pub enum ExecError {
    //
    #[error("Depth exceeded: max depth = {}, given depth = {}", max, given)]
    DepthExceeded { max: usize, given: usize },

    #[error("Key not found: {}", key)]
    KeyNotFound { key: String },

    #[error("Batch not found: {}", batchname)]
    BatchNotFound { batchname: String },

    #[error("Internal error: {}", emsg)]
    Internal { emsg: String },
}

pub enum ExecOutput {
    Kv(String, Box<Value>),
    Value(Value),
    List(Vec<ExecOutput>),
    Info(String),
}

impl ExecOutput {
    pub fn ok() -> Self {
        Self::Info("Ok".into())
    }
}

impl ToString for ExecOutput {
    fn to_string(&self) -> String {
        match self {
            ExecOutput::Kv(key, value) => format!("[k-v] {} => {}", key, value.to_string()),
            ExecOutput::Value(value) => format!("[value] {}", value.to_string()),
            ExecOutput::List(vec) => {
                //
                let len = vec.len();
                let content = vec
                    .iter()
                    .map(|x| format!("  {},\n", x.to_string()))
                    .collect::<Vec<_>>()
                    .join("");

                format!("[list({})]\n[\n{}]", len, content)
            }
            ExecOutput::Info(msg) => format!("[info] {}", msg),
        }
    }
}

pub type ExecReturn = Result<ExecOutput, ExecError>;
pub type ExecResult<T> = Result<T, ExecError>;
