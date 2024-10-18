// File: src/lib.rs
use chrono::NaiveDate;
use chrono::Datelike;
use tokio::sync::oneshot;
use std::mem;
use std::ops::Deref;
use std::os::raw::{c_char, c_uchar, c_int, c_void, c_short, c_ushort, c_long, c_ulong};
use std::ffi::{CStr, CString};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll};
use crate::errors;
use errors::{InformixError, Result};


use std::marker::PhantomData;

#[derive(Debug)]
struct SendPtr<T>(*mut T, PhantomData<T>);

impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self {
        SendPtr(self.0, PhantomData)
    }
}


unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

impl<T> SendPtr<T> {
    pub fn new(ptr: *mut T) -> Self {
        SendPtr(ptr, PhantomData)
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0
    }
}
impl<T> Deref for SendPtr<T> {
    type Target = *mut T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


#[link(name = "ifcli")]
extern "C" {
    fn SQLAllocHandle(HandleType: c_int, InputHandle: *mut c_void, OutputHandle: *mut *mut c_void) -> c_int;
    fn SQLConnect(ConnectionHandle: *mut c_void, ServerName: *const c_char, NameLength1: c_int,
                  UserName: *const c_char, NameLength2: c_int,
                  Authentication: *const c_char, NameLength3: c_int) -> c_int;
    fn SQLPrepare(StatementHandle: *mut c_void, 
        StatementText: *const c_uchar, 
        TextLength: c_int) -> c_short;
    fn SQLBindParameter(StatementHandle: *mut c_void, ParameterNumber: c_ushort, InputOutputType: c_short,
        ValueType: c_short, ParameterType: c_short, ColumnSize: c_ulong,
        DecimalDigits: c_short, ParameterValuePtr: *const c_void,
        BufferLength: c_long, StrLen_or_IndPtr: *const c_long) -> c_short;
    fn SQLExecute(StatementHandle: *mut c_void) -> c_short;
    fn SQLExecDirect(StatementHandle: *mut c_void, StatementText: *const c_char, TextLength: c_int) -> c_int;
    fn SQLFetch(StatementHandle: *mut c_void) -> c_int;
    fn SQLGetData(StatementHandle: *mut c_void, ColumnNumber: c_ushort, TargetType: c_short,
        TargetValue: *mut c_void, BufferLength: c_long, StrLen_or_Ind: *mut c_long) -> c_short;
    fn SQLGetDiagRec(HandleType: c_short, Handle: *mut c_void, RecNumber: c_short,
        SQLState: *mut c_char, NativeErrorPtr: *mut c_int,
        MessageText: *mut c_char, BufferLength: c_short,
        TextLengthPtr: *mut c_short) -> c_short;
    fn SQLDriverConnect(
        ConnectionHandle: *mut c_void,
        WindowHandle: *mut c_void,
        InConnectionString: *const c_char,
        StringLength1: c_short,
        OutConnectionString: *mut c_char,
        BufferLength: c_short,
        StringLength2Ptr: *mut c_short,
        DriverCompletion: c_ushort) -> c_short;
    fn SQLDisconnect(ConnectionHandle: *mut c_void) -> c_int;
    fn SQLFreeHandle(HandleType: c_short, Handle: *mut c_void) -> c_int;
}

/*  FFI declarations
#[link(name = "ifxa")]
extern "C" {
    fn SQLAllocHandle(HandleType: c_int, InputHandle: *mut c_void, OutputHandle: *mut *mut c_void) -> c_int;
    fn SQLConnect(ConnectionHandle: *mut c_void, ServerName: *const c_char, NameLength1: c_int,
                  UserName: *const c_char, NameLength2: c_int,
                  Authentication: *const c_char, NameLength3: c_int) -> c_int;
    fn SQLExecDirect(StatementHandle: *mut c_void, StatementText: *const c_char, TextLength: c_int) -> c_int;
    fn SQLFetch(StatementHandle: *mut c_void) -> c_int;
    fn SQLGetData(StatementHandle: *mut c_void, ColumnNumber: c_int, TargetType: c_int,
                  TargetValue: *mut c_void, BufferLength: c_int, StrLen_or_Ind: *mut c_int) -> c_int;
    fn SQLDisconnect(ConnectionHandle: *mut c_void) -> c_int;
    fn SQLFreeHandle(HandleType: c_int, Handle: *mut c_void) -> c_int;
}*/
// Constants
// SQL type constants
// SQL return codes
pub const SQL_SUCCESS: c_short = 0;
pub const SQL_SUCCESS_WITH_INFO: c_short = 1;
pub const SQL_NO_DATA: c_short = 100;

// SQL data type constants
pub const SQL_C_CHAR: c_short = 1;
pub const SQL_VARCHAR: c_short = 12;
pub const SQL_TYPE_DATE: c_short = 91;
pub const SQL_C_LONG: c_short = 4;
pub const SQL_INTEGER: c_short = 4;


// You may also want to add these related constants for completeness:
pub const SQL_C_SHORT: c_short = 5;
pub const SQL_SMALLINT: c_short = 5;

// SQL special values
pub const SQL_NULL_DATA: c_long = -1;

// SQL handle types
pub const SQL_HANDLE_ENV: c_short = 1;
pub const SQL_HANDLE_DBC: c_short = 2;
pub const SQL_HANDLE_STMT: c_short = 3;

pub const SQL_DRIVER_NOPROMPT: c_ushort = 0;
// Other SQL constants
pub const SQL_PARAM_INPUT: c_short = 1;
pub const SQL_NTS: c_long = -3;
// FFI declarations remain the same

// Constants remain the same

// Define a custom Future for async FFI calls
struct AsyncFFIFuture {
    result: Option<c_int>,
}
impl Future for AsyncFFIFuture {
    type Output = c_int;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = self.result.take() {
            Poll::Ready(result)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// Implement missing async wrappers for FFI functions
async fn async_sql_driver_connect(handle: SendPtr<c_void>, conn_string: &str) -> Result<()> {
    let conn_string = CString::new(conn_string)?;
    let mut out_conn_string = [0u8; 1024];
    let mut out_conn_string_len: c_short = 0;
    
    let result = tokio::task::spawn_blocking(move || {
        unsafe {
            SQLDriverConnect(
                handle.as_ptr(),
                std::ptr::null_mut(),
                conn_string.as_ptr(),
                conn_string.as_bytes().len() as c_short,
                out_conn_string.as_mut_ptr() as *mut c_char,
                out_conn_string.len() as c_short,
                &mut out_conn_string_len,
                SQL_DRIVER_NOPROMPT
            )
        }
    }).await.unwrap();

    if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
        Ok(())
    } else {
        Err(InformixError::ConnectionError(format!("SQLDriverConnect failed with code {}", result)))
    }
}

async fn async_sql_prepare(handle: SendPtr<c_void>, sql: &str) -> Result<()> {
    let sql_cstring = CString::new(sql)?;
    let result = tokio::task::spawn_blocking(move || {
        unsafe {
            SQLPrepare(handle.as_ptr(), sql_cstring.as_ptr() as *const c_uchar, sql_cstring.as_bytes().len() as c_int)
        }
    }).await.unwrap();

    if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
        Ok(())
    } else {
        Err(InformixError::PrepareStatementError(format!("SQLPrepare failed with code {}", result)))
    }
}

async fn async_sql_execute(handle: SendPtr<c_void>) -> Result<()> {
    let result = tokio::task::spawn_blocking(move || {
        unsafe { SQLExecute(handle.as_ptr()) }
    }).await.unwrap();

    if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
        Ok(())
    } else {
        Err(InformixError::SQLExecutionError(format!("SQLExecute failed with code {}", result)))
    }
}

async fn async_sql_fetch(handle: SendPtr<c_void>) -> Result<bool> {
    let result = tokio::task::spawn_blocking(move || {
        unsafe { SQLFetch(handle.as_ptr()) }
    }).await.unwrap();

    match result as c_short {
        SQL_SUCCESS | SQL_SUCCESS_WITH_INFO => Ok(true),
        SQL_NO_DATA => Ok(false),
        _ => Err(InformixError::FetchError(format!("SQLFetch failed with code {}", result)))
    }
}

async fn async_sql_get_data(handle: SendPtr<c_void>, column: c_ushort, target_type: c_short) -> Result<Option<String>> {
    let mut buffer = [0u8; 1024];
    let mut indicator: c_long = 0;

    let result = tokio::task::spawn_blocking(move || {
        unsafe {
            SQLGetData(
                handle.as_ptr(),
                column,
                target_type,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as c_long,
                &mut indicator
            )
        }
    }).await.unwrap();

    if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
        if indicator == SQL_NULL_DATA {
            Ok(None)
        } else {
            let data = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) }.to_string_lossy().into_owned();
            Ok(Some(data))
        }
    } else {
        Err(InformixError::GetDataError(format!("SQLGetData failed with code {}", result)))
    }
}

// Async wrapper for SQLAllocHandle
async fn async_sql_alloc_handle(handle_type: c_int, input_handle: Option<SendPtr<c_void>>) -> Result<SendPtr<c_void>> {
    let input_ptr = input_handle.as_ref().map_or(std::ptr::null_mut(), |h| h.as_ptr());
    
    let (sender, receiver) = oneshot::channel();

    tokio::task::spawn_blocking(move || {
        let mut output_handle: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            SQLAllocHandle(handle_type, input_ptr, output_handle)
        };
        let _ = sender.send((result, SendPtr::new(output_handle)));
    });

    let (result, output_handle) = receiver.await.map_err(|_| InformixError::HandleAllocationError(-1))?;

    if result == 0 {
        Ok(output_handle.clone())
    } else {
        Err(InformixError::HandleAllocationError(result))
    }
}


async fn async_sql_free_handle(handle_type: c_short, handle: SendPtr<c_void>) -> Result<()> {
    let result = tokio::task::spawn_blocking(move || {
        unsafe { SQLFreeHandle(handle_type, handle.as_ptr()) }
    }).await.unwrap();

    if result == 0 {
        Ok(())
    } else {
        Err(InformixError::HandleFreeError(result))
    }
}

async fn async_sql_disconnect(handle: SendPtr<c_void>) -> Result<()> {
    let result = tokio::task::spawn_blocking(move || {
        unsafe { SQLDisconnect(handle.as_ptr()) }
    }).await.unwrap();

    if result == 0 {
        Ok(())
    } else {
        Err(InformixError::DisconnectError(result))
    }
}

// Similar async wrappers for other FFI functions...
pub struct AsyncConnection {
    handle: SendPtr<c_void>,
}

impl AsyncConnection {
    pub async fn new() -> Result<Self> {
        let env_handle = async_sql_alloc_handle(SQL_HANDLE_ENV.into(), None).await?;
        let conn_handle = async_sql_alloc_handle(SQL_HANDLE_DBC.into(), Some(env_handle)).await?;
        Ok(AsyncConnection { handle: conn_handle })
    }

    pub async fn connect_with_string(&self, conn_string: &str) -> Result<()> {
        println!("Attempting to connect with string: {}", conn_string);
        async_sql_driver_connect(self.handle.clone(), conn_string).await
    }

    pub async fn prepare(&self, sql: &str) -> Result<AsyncStatement> {
        let stmt_handle = async_sql_alloc_handle(SQL_HANDLE_STMT.into(), Some(self.handle.clone())).await?;
        async_sql_prepare(stmt_handle.clone(), sql).await?;
        Ok(AsyncStatement::new(stmt_handle.clone(), sql))
    }

    pub async fn execute(&self, sql: &str) -> Result<AsyncStatement> {
        let stmt = self.prepare(sql).await?;
        stmt.execute().await?;
        Ok(stmt)
    }

    // Other methods (connect, execute, etc.) should be implemented similarly...
    async fn get_error_message(&self) -> String {
        let mut state = [0i8; 6];
        let mut native_error = 0i32;
        let mut message = [0i8; 1024];
        let mut out_len = 0i16;
        let handle = self.handle.clone();
        
        let result = tokio::task::spawn_blocking(move || {
            unsafe {
                SQLGetDiagRec(
                    SQL_HANDLE_DBC,
                    handle.as_ptr(),
                    1,
                    state.as_mut_ptr() as *mut c_char,
                    &mut native_error,
                    message.as_mut_ptr() as *mut c_char,
                    message.len() as c_short,
                    &mut out_len,
                )
            }
        }).await.unwrap();

        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            let state = unsafe { CStr::from_ptr(state.as_ptr() as *const c_char) }.to_string_lossy();
            let message = unsafe { CStr::from_ptr(message.as_ptr() as *const c_char) }.to_string_lossy();
            format!("SQLSTATE = {}, Native Error = {}, Message = {}", state, native_error, message)
        } else {
            format!("Failed to retrieve error message, SQLGetDiagRec returned {}", result)
        }
    }
}

impl Drop for AsyncConnection {
    fn drop(&mut self) {
        let handle = self.handle.clone();
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let _ = async_sql_disconnect(handle.clone()).await;
                let _ = async_sql_free_handle(SQL_HANDLE_DBC, handle).await;
            });
        });
    }
}

pub struct AsyncStatement {
    pub handle: SendPtr<c_void>,
    query: String,
}

impl AsyncStatement {
    pub fn new(handle: SendPtr<c_void>, query: &str) -> Self {
        AsyncStatement {
            handle,
            query: query.into(),
        }
    }

    pub async fn bind_parameter<T: AsyncToSql>(&self, param_num: u16, value: &T) -> Result<()> {
        value.async_bind_parameter(self.handle.clone(), param_num).await
    }

    pub async fn execute(&self) -> Result<()> {
        let handle = self.handle.clone();
        let result = tokio::task::spawn_blocking(move || {
            unsafe { SQLExecute(handle.as_ptr()) }
        }).await.unwrap();

        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            Ok(())
        } else {
            // We need to get the error message before we return the error
            let error_message = self.get_error_message().await;
            Err(InformixError::SQLExecutionError(error_message))
        }
    }

    pub async fn fetch(&self) -> Result<Option<Vec<String>>> {
        if !async_sql_fetch(self.handle.clone()).await? {
            return Ok(None);
        }

        let mut row = Vec::new();
        for i in 1..=1000 { // Assuming max 1000 columns, adjust as needed
            match async_sql_get_data(self.handle.clone(), i, SQL_C_CHAR).await? {
                Some(value) => row.push(value),
                None => break,
            }
        }
        Ok(Some(row))
    }

    async fn get_error_message(&self) -> String {
        let mut state = [0i8; 6];
        let mut native_error = 0i32;
        let mut message = [0i8; 1024];
        let mut out_len = 0i16;
        let handle = self.handle.clone();

        let result = tokio::task::spawn_blocking(move || {
            unsafe {
                SQLGetDiagRec(
                    SQL_HANDLE_STMT,
                    handle.as_ptr(),
                    1,
                    state.as_mut_ptr() as *mut c_char,
                    &mut native_error,
                    message.as_mut_ptr() as *mut c_char,
                    message.len() as c_short,
                    &mut out_len,
                )
            }
        }).await.unwrap();

        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            let state = unsafe { CStr::from_ptr(state.as_ptr() as *const c_char) }.to_string_lossy();
            let message = unsafe { CStr::from_ptr(message.as_ptr() as *const c_char) }.to_string_lossy();
            format!("SQLSTATE = {}, Native Error = {}, Message = {}", state, native_error, message)
        } else {
            format!("Failed to retrieve error message, SQLGetDiagRec returned {}", result)
        }
    }
}

impl Drop for AsyncStatement {
    fn drop(&mut self) {
        let handle = self.handle.clone();
        tokio::spawn(async move {
            let _ = async_sql_free_handle(SQL_HANDLE_STMT, handle).await;
        });
    }
}

pub trait AsyncToSql: Send + Sync {
    fn async_bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}
impl AsyncToSql for i32 {
    fn async_bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            let value = *self;
            let result = tokio::task::spawn_blocking(move || {
                unsafe {
                    SQLBindParameter(
                        stmt.as_ptr(),
                        param_num,
                        SQL_PARAM_INPUT,
                        SQL_C_LONG,
                        SQL_INTEGER,
                        0,
                        0,
                        &value as *const i32 as *const c_void,
                        0,
                        std::ptr::null(),
                    )
                }
            }).await.unwrap();

            if result == SQL_SUCCESS as i16 || result == SQL_SUCCESS_WITH_INFO as i16 {
                Ok(())
            } else {
                Err(InformixError::ParameterBindingError(format!("Failed to bind i32 parameter: {}", result)))
            }
        })
    }
}

impl AsyncToSql for String {
    fn async_bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let value = self.clone();
        Box::pin(async move {
            let c_str = CString::new(value.clone())?;
            let result = tokio::task::spawn_blocking(move || {
                unsafe {
                    SQLBindParameter(
                        stmt.as_ptr(),
                        param_num,
                        SQL_PARAM_INPUT,
                        SQL_C_CHAR,
                        SQL_VARCHAR,
                        value.len() as c_ulong,
                        0,
                        c_str.as_ptr() as *const c_void,
                        value.len() as c_long,
                        &(value.len() as c_long) as *const c_long,
                    )
                }
            }).await.unwrap();

            if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
                Ok(())
            } else {
                Err(InformixError::ParameterBindingError(format!("Failed to bind String parameter: {}", result)))
            }
        })
    }
}

// Implement AsyncToSql for various types (i32, &str, String, NaiveDate)...

// Higher-level abstractions
pub struct AsyncCursor<'a> {
    pub stmt: AsyncStatement,
    pub conn: &'a AsyncConnection,
}

impl<'a> AsyncCursor<'a> {
    pub async fn execute(&mut self, sql: &str) -> Result<()> {
        self.stmt = self.conn.execute(sql).await?;
        Ok(())
    }

    pub async fn execute_with_params(&mut self, sql: &str, params: &[&(dyn AsyncToSql + Sync)]) -> Result<()> {
        self.stmt = self.conn.prepare(sql).await?;
        for (i, param) in params.iter().enumerate() {
            param.async_bind_parameter(self.stmt.handle.clone(), (i + 1) as u16).await?;
        }
        self.stmt.execute().await
    }


    pub async fn fetchone(&self) -> Result<Option<Vec<String>>> {
        self.stmt.fetch().await
    }

    pub async fn fetchall(&self) -> Result<Vec<Vec<String>>> {
        let mut results = Vec::new();
        while let Some(row) = self.stmt.fetch().await? {
            results.push(row);
        }
        Ok(results)
    }
}
