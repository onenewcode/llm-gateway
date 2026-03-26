pub mod request;
pub mod response;
pub mod streaming;

use std::{cmp, error, fmt};

#[derive(Debug)]
pub enum ProtocolError {
    InvalidJson(serde_json::Error),
    MissingRequiredField(String),
    ConversionError(String),
    InvalidRequest(String),
    InvalidStreamEvent(String),
}

impl cmp::PartialEq for ProtocolError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::InvalidJson(_), Self::InvalidJson(_)) => true,
            (Self::MissingRequiredField(l0), Self::MissingRequiredField(r0)) => l0 == r0,
            (Self::ConversionError(l0), Self::ConversionError(r0)) => l0 == r0,
            (Self::InvalidRequest(l0), Self::InvalidRequest(r0)) => l0 == r0,
            (Self::InvalidStreamEvent(l0), Self::InvalidStreamEvent(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(e) => write!(f, "Invalid json in message: {e}"),
            Self::MissingRequiredField(field) => write!(f, "Missing required field: {field}"),
            Self::ConversionError(msg) => write!(f, "Conversion error: {msg}"),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Self::InvalidStreamEvent(msg) => write!(f, "Invalid stream event: {msg}"),
        }
    }
}

impl From<serde_json::Error> for ProtocolError {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidJson(value)
    }
}

impl error::Error for ProtocolError {}

pub type ProtocolResult<T> = Result<T, ProtocolError>;
