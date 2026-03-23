#![allow(unused)]

mod request;
mod response;
mod streaming;

use std::{error, fmt};
pub use {request::*, response::*, streaming::*};

#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolError {
    MissingRequiredField(String),
    ConversionError(String),
    InvalidRequest(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(f, "Missing required field: {field}")
            }
            Self::ConversionError(msg) => write!(f, "Conversion error: {msg}"),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
        }
    }
}

impl error::Error for ProtocolError {}

pub type ProtocolResult<T> = Result<T, ProtocolError>;
