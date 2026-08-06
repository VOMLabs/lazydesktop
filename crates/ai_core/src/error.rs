use std::io;

/// C-visible status codes returned by the FFI boundary. Mirrored in `ai_core.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum AiStatus {
    Ok = 0,
    InvalidArgument = 1,
    NotFound = 2,
    Io = 3,
    Parse = 4,
    Cancelled = 5,
    Busy = 6,
    Other = -1,
}

impl AiStatus {
    pub fn code(self) -> i32 {
        self as i32
    }

    pub fn from_code(code: i32) -> AiStatus {
        match code {
            0 => AiStatus::Ok,
            1 => AiStatus::InvalidArgument,
            2 => AiStatus::NotFound,
            3 => AiStatus::Io,
            4 => AiStatus::Parse,
            5 => AiStatus::Cancelled,
            6 => AiStatus::Busy,
            _ => AiStatus::Other,
        }
    }
}

/// Internal error type used across crate boundaries.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("operation cancelled")]
    Cancelled,
    #[error("operation is busy")]
    Busy,
    #[error("{0}")]
    Other(String),
}

impl AiError {
    pub fn status(&self) -> AiStatus {
        match self {
            AiError::InvalidArgument(_) => AiStatus::InvalidArgument,
            AiError::NotFound(_) => AiStatus::NotFound,
            AiError::Io(_) => AiStatus::Io,
            AiError::Parse(_) => AiStatus::Parse,
            AiError::Cancelled => AiStatus::Cancelled,
            AiError::Busy => AiStatus::Busy,
            AiError::Other(_) => AiStatus::Other,
        }
    }
}

impl From<serde_json::Error> for AiError {
    fn from(e: serde_json::Error) -> Self {
        AiError::Parse(e.to_string())
    }
}

impl From<reqwest::Error> for AiError {
    fn from(e: reqwest::Error) -> Self {
        AiError::Other(format!("http error: {e}"))
    }
}
