use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum InformixError {
    HandleAllocationError(i32),
    ConnectionError(String),
    SQLExecutionError(String),
    PrepareStatementError(String),
    ParameterBindingError(String),
    DataFetchError(String),
    DescribeColumnsError(String),
}

impl fmt::Display for InformixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InformixError::HandleAllocationError(code) => write!(f, "Failed to allocate handle: {}", code),
            InformixError::ConnectionError(msg) => write!(f, "Failed to connect: {}", msg),
            InformixError::SQLExecutionError(msg) => write!(f, "SQL execution failed: {}", msg),
            InformixError::PrepareStatementError(msg) => write!(f, "Failed to prepare statement: {}", msg),
            InformixError::ParameterBindingError(msg) => write!(f, "Failed to bind parameter: {}", msg),
            InformixError::DataFetchError(msg) => write!(f, "Failed to fetch data: {}", msg),
            InformixError::DescribeColumnsError(msg) => write!(f, "Failed to describe columns: {}", msg),
        }
    }
}

impl Error for InformixError {}

pub type Result<T> = std::result::Result<T, InformixError>;