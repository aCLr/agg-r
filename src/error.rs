use std::fmt;

// TODO: enum
#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<tokio_diesel::AsyncError> for Error {
    fn from(err: tokio_diesel::AsyncError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

impl From<&tokio_diesel::AsyncError> for Error {
    fn from(err: &tokio_diesel::AsyncError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

impl From<http_collector::error::Error> for Error {
    fn from(err: http_collector::error::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
