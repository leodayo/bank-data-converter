use std::{error, fmt, io};

#[derive(Debug)]
pub enum ParserError {
    InvalidField { field: &'static str, reason: String },
    EmptyFile,
    InvalidHeader { expected: &'static str, got: String },
    InvalidFormat { reason: String },
    MissingField(&'static str),
    UnknownField(String),
    DuplicateField(&'static str),

    Io(io::Error),
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::InvalidField { field, reason } => {
                write!(f, "Invalid field '{}': {}", field, reason)
            }
            ParserError::EmptyFile => {
                write!(f, "The file is empty")
            }
            ParserError::InvalidHeader { expected, got } => {
                write!(f, "Invalid header: expected '{}', got '{}'", expected, got)
            }
            ParserError::Io(err) => {
                write!(f, "I/O error: {}", err)
            }
            ParserError::InvalidFormat { reason } => {
                write!(f, "Invalid format: {}", reason)
            }
            ParserError::MissingField(field) => {
                write!(f, "Missing field: {}", field)
            }
            ParserError::UnknownField(field) => {
                write!(f, "Unknown field: {}", field)
            }
            ParserError::DuplicateField(field) => {
                write!(f, "Duplicate field: {}", field)
            }
        }
    }
}

impl error::Error for ParserError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for ParserError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
