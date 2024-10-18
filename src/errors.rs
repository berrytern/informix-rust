use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum InformixError {
    HandleAllocationError(i32),
    ConnectionError(String),
    SQLExecutionError(String),
    PrepareStatementError(String),
    ParameterBindingError(String),
    HandleFreeError(i32),
    DisconnectError(i32),
    FetchError(String),
    GetDataError(String),
    DataFetchError(String),
    DescribeColumnsError(String),
    NulError(std::ffi::NulError),
}

impl fmt::Display for InformixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InformixError::HandleAllocationError(code) => write!(f, "Failed to allocate handle: {}", code),
            InformixError::ConnectionError(msg) => write!(f, "Failed to connect: {}", msg),
            InformixError::SQLExecutionError(msg) => write!(f, "SQL execution failed: {}", msg),
            InformixError::PrepareStatementError(msg) => write!(f, "Failed to prepare statement: {}", msg),
            InformixError::ParameterBindingError(msg) => write!(f, "Failed to bind parameter: {}", msg),
            InformixError::HandleFreeError(code) => write!(f, "Handle free error: {}", code),
            InformixError::DisconnectError(code) => write!(f, "Disconnect error: {}", code),
            InformixError::FetchError(msg) => write!(f, "Fetch error: {}", msg),
            InformixError::GetDataError(msg) => write!(f, "Get data error: {}", msg),
            InformixError::DataFetchError(msg) => write!(f, "Failed to fetch data: {}", msg),
            InformixError::DescribeColumnsError(msg) => write!(f, "Failed to describe columns: {}", msg),
            InformixError::NulError(e) => write!(f, "Null error: {}", e),
        }
    }
}

impl Error for InformixError {}

impl From<std::ffi::NulError> for InformixError {
    fn from(error: std::ffi::NulError) -> Self {
        InformixError::NulError(error)
    }
}

pub type Result<T> = std::result::Result<T, InformixError>;