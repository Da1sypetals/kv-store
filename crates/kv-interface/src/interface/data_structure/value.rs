use bytes::Bytes;
use num::complex::Complex64;

use crate::ksis::parse::errors::{ParseError, ParseResult};

pub type Complex = Complex64;

#[derive(Debug)]
pub enum Value {
    Str(String),
    Int(i64),
    Real(f64),
    Complex(Complex),
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Str(s) => format!("Str({})", s),
            Value::Int(val) => format!("Int({})", val),
            Value::Real(val) => format!("Real({})", val),
            Value::Complex(complex) => format!("Complex({} + {}i)", complex.re, complex.im),
        }
    }
}

impl Value {
    pub fn type_id(&self) -> u8 {
        match self {
            Value::Str(_) => 0,
            Value::Int(_) => 1,
            Value::Real(_) => 2,
            Value::Complex(_) => 3,
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut vec = vec![self.type_id()];

        match self {
            Value::Str(s) => {
                vec.extend_from_slice(s.as_bytes());
            }
            Value::Int(val) => {
                vec.extend(val.to_be_bytes());
            }
            Value::Real(val) => {
                vec.extend(val.to_be_bytes());
            }
            Value::Complex(complex) => {
                vec.extend(complex.re.to_be_bytes());
                vec.extend(complex.im.to_be_bytes());
            }
        }

        vec.into()
    }

    // Direct panic if invalid type couind
    pub fn decode(bin: &Bytes) -> Self {
        let value_type = bin[0];
        let content = &bin[1..];

        match value_type {
            // str
            0 => {
                //
                Self::Str(String::from_utf8_lossy(content).to_string())
            }
            // int
            1 => {
                //
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(content);
                Self::Int(i64::from_be_bytes(bytes))
            }
            // Real
            2 => {
                //
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&content[..8]);
                Self::Real(f64::from_be_bytes(bytes))
            }
            // Complex
            3 => {
                //
                let mut bytes_re = [0u8; 8];
                let mut bytes_im = [0u8; 8];
                bytes_re.copy_from_slice(&content[..8]);
                bytes_im.copy_from_slice(&content[8..16]);
                let z = Complex {
                    re: f64::from_be_bytes(bytes_re),
                    im: f64::from_be_bytes(bytes_im),
                };
                Self::Complex(z)
            }
            x => {
                panic!("Internal irrecoverable error: invalid value type: {x}")
            }
        }
    }

    pub(crate) fn parse(value_type: String, val_str: String) -> ParseResult<Self> {
        if value_type.chars().nth(0) != Some('-') {
            return Err(ParseError::InvalidSyntax {
                msg: "Specify a type by `-<Type>` or its abbreviation `-<t>`.".into(),
            });
        }
        let value_type = &value_type[1..];
        match value_type {
            "s" | "Str" => {
                //
                Ok(Self::Str(val_str))
            }
            "i" | "Int" => {
                //
                let value: i64 = val_str.parse().map_err(|_| ParseError::InvalidValue {
                    value_type: value_type.into(),
                    value: val_str,
                })?;
                Ok(Self::Int(value))
            }
            "r" | "Real" => {
                //
                let value: f64 = val_str.parse().map_err(|_| ParseError::InvalidValue {
                    value_type: value_type.into(),
                    value: val_str,
                })?;
                Ok(Self::Real(value))
            }
            "z" | "Complex" => {
                //
                let value: Complex = val_str.parse().map_err(|_| ParseError::InvalidValue {
                    value_type: value_type.into(),
                    value: val_str,
                })?;
                Ok(Self::Complex(value))
            }
            value_type => {
                //
                Err(ParseError::UnsupportedValueType {
                    value_type: value_type.into(),
                })
            }
        }
    }
}
