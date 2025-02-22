use crate::interface::data_structure::value::Value;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    //
    #[error("Invalid syntax: {}", msg)]
    InvalidSyntax { msg: String },

    #[error(
        "Invalid identifier: expected upper/lower English character and numbers, found {}",
        ident
    )]
    InvalidIdentifier { ident: String },

    #[error("Incompatible argument count: expected {}, found {}", expected, found)]
    IncompatibleArgCount { expected: usize, found: usize },

    #[error("Invalid directory: {}", src)]
    InvalidDirectory { src: String },

    #[error("Unsupported command: {}", cmd)]
    UnsupportedCommand { cmd: String },

    #[error("Unsupported value type: expected (s, i, r, z), found {}", value_type)]
    UnsupportedValueType { value_type: String },

    #[error("Invalid value for type [{}]: {}", value_type, value)]
    InvalidValue { value_type: String, value: String },
}

pub type ParseResult<T> = Result<T, ParseError>;
