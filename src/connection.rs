use crate::{
    domain::{
        base_params::{ColumnMetadata, SqlParam, ToSql},
        c_binds::{
            SQLAllocHandle, SQLBindParameter, SQLConnect, SQLDescribeCol, SQLDisconnect,
            SQLDriverConnect, SQLExecDirect, SQLExecute, SQLFetch, SQLFreeHandle, SQLGetData,
            SQLGetDiagRec, SQLNumResultCols, SQLPrepare, SQL_HANDLE_DBC, SQL_HANDLE_ENV,
            SQL_HANDLE_STMT,
        },
    },
    errors::{InformixError, Result},
};
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw::{c_char, c_int, c_long, c_short, c_uchar, c_ulong, c_ushort, c_void};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

#[derive(Debug)]
pub struct SendPtr<T>(*mut T, PhantomData<T>);

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

pub const SQL_DRIVER_NOPROMPT: c_ushort = 0;
// Other SQL constants
pub const SQL_NTS: c_long = -3;

// Safe Rust wrappers
pub struct Connection {
    handle: SendPtr<c_void>,
    pub is_connected: AtomicBool,
}

impl Connection {
    pub fn new() -> Result<Self> {
        let mut handle: *mut c_void = std::ptr::null_mut();
        let result =
            unsafe { SQLAllocHandle(SQL_HANDLE_ENV.into(), std::ptr::null_mut(), &mut handle) };
        if result == 0 {
            let mut conn_handle: *mut c_void = std::ptr::null_mut();
            let conn_result =
                unsafe { SQLAllocHandle(SQL_HANDLE_DBC.into(), handle, &mut conn_handle) };
            if conn_result == 0 {
                Ok(Connection {
                    handle: SendPtr::new(conn_handle),
                    is_connected: AtomicBool::new(false),
                })
            } else {
                Err(InformixError::HandleAllocationError(conn_result))
            }
        } else {
            Err(InformixError::HandleAllocationError(result))
        }
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    pub fn connect_with_string(&self, conn_string: &str) -> Result<()> {
        if self.is_connected() {
            return Err(InformixError::ConnectionError(
                "Already connected".to_string(),
            ));
        }

        let conn_string = CString::new(conn_string).map_err(|e| {
            InformixError::ConnectionError(format!("Invalid connection string: {}", e))
        })?;

        let mut out_conn_string = [0u8; 1024];
        let mut out_conn_string_len: c_short = 0;
        let result = unsafe {
            SQLDriverConnect(
                self.handle.as_ptr(),
                std::ptr::null_mut(),
                conn_string.as_ptr() as *const c_char,
                conn_string.as_bytes().len() as c_short,
                out_conn_string.as_mut_ptr() as *mut c_char,
                out_conn_string.len() as c_short,
                &mut out_conn_string_len,
                SQL_DRIVER_NOPROMPT,
            )
        };

        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            println!("Successfully connected to the database");
            self.is_connected.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            let error_message = self.get_error_message();
            println!("Failed to connect: result = {}, {}", result, error_message);
            Err(InformixError::ConnectionError(format!(
                "Failed to connect: result = {}, {}",
                result, error_message
            )))
        }
    }

    pub fn prepare(&self, sql: &str) -> Result<Statement> {
        let mut stmt_handle: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            SQLAllocHandle(
                SQL_HANDLE_STMT.into(),
                self.handle.as_ptr(),
                &mut stmt_handle,
            )
        };
        if result != 0 {
            return Err(InformixError::HandleAllocationError(result));
        }

        let sql_cstring = CString::new(sql).map_err(|e| {
            InformixError::PrepareStatementError(format!("Invalid SQL string: {}", e))
        })?;
        let result = unsafe {
            SQLPrepare(
                stmt_handle,
                sql_cstring.as_ptr() as *const c_uchar,
                sql.len() as c_int,
            )
        };
        if result != 0 {
            unsafe { SQLFreeHandle(SQL_HANDLE_STMT, stmt_handle) };
            return Err(InformixError::PrepareStatementError(format!(
                "Failed to prepare SQL: {}",
                result
            )));
        }

        Ok(Statement::new(SendPtr::new(stmt_handle), ""))
    }

    pub fn connect(&self, server: &str, user: &str, password: &str) -> Result<()> {
        let server = CString::new(server).unwrap();
        let user = CString::new(user).unwrap();
        let password = CString::new(password).unwrap();
        let result = unsafe {
            SQLConnect(
                self.handle.as_ptr(),
                server.as_ptr(),
                server.as_bytes().len() as c_int,
                user.as_ptr(),
                user.as_bytes().len() as c_int,
                password.as_ptr(),
                password.as_bytes().len() as c_int,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            let error_message = self.get_error_message();
            Err(InformixError::ConnectionError(error_message))
        }
    }

    fn get_error_message(&self) -> String {
        let mut state = [0i8; 6];
        let mut native_error = 0i32;
        let mut message = [0i8; 1024];
        let mut out_len = 0i16;

        unsafe {
            SQLGetDiagRec(
                SQL_HANDLE_DBC,
                self.handle.as_ptr(),
                1,
                state.as_mut_ptr() as *mut c_char,
                &mut native_error,
                message.as_mut_ptr() as *mut c_char,
                message.len() as c_short,
                &mut out_len,
            );
        }

        let state = unsafe { CStr::from_ptr(state.as_ptr() as *const c_char) }.to_string_lossy();
        let message =
            unsafe { CStr::from_ptr(message.as_ptr() as *const c_char) }.to_string_lossy();

        format!(
            "SQLSTATE = {}, Native Error = {}, Message = {}",
            state, native_error, message
        )
    }

    pub fn execute(&self, sql: &str) -> Result<Statement> {
        let mut stmt_handle: *mut c_void = std::ptr::null_mut();
        let result = unsafe { SQLAllocHandle(3, self.handle.as_ptr(), &mut stmt_handle) };
        if result != 0 {
            return Err(InformixError::HandleAllocationError(result));
        }

        let sql = CString::new(sql).unwrap();
        let result =
            unsafe { SQLExecDirect(stmt_handle, sql.as_ptr(), sql.as_bytes().len() as c_int) };
        if result == 0 {
            Ok(Statement::new(SendPtr::new(stmt_handle), ""))
        } else {
            unsafe { SQLFreeHandle(3, stmt_handle) };
            Err(InformixError::SQLExecutionError(format!(
                "Failed to execute SQL: {}",
                result
            )))
        }
    }

    pub fn query_with_parameters(
        &self,
        query: String,
        parameters: Vec<SqlParam>,
    ) -> Result<Option<Vec<Vec<String>>>> {
        let statement = self.prepare(&query)?;
        for (index, param) in parameters.iter().enumerate() {
            statement.bind_parameter(index as u16 + 1, &param)?;
        }
        statement.execute()?;
        let mut result: Vec<Vec<String>> = Vec::new();
        while let Some(row) = statement.fetch()? {
            result.push(row);
        }
        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            SQLDisconnect(self.handle.as_ptr());
            SQLFreeHandle(1, self.handle.as_ptr());
        }
    }
}

pub struct Statement {
    pub handle: SendPtr<c_void>,
    query: String,
}

impl Statement {
    pub fn new(handle: SendPtr<c_void>, query: &str) -> Self {
        Statement {
            handle,
            query: query.into(),
        }
    }

    pub fn bind_parameter<T: ToSql>(&self, param_num: u16, value: &T) -> Result<()> {
        value.bind_parameter(self.handle.clone(), param_num)
    }

    pub fn execute(&self) -> Result<()> {
        let result = unsafe { SQLExecute(self.handle.as_ptr()) };
        if result != SQL_SUCCESS as c_short && result != SQL_SUCCESS_WITH_INFO as c_short {
            Err(InformixError::SQLExecutionError(self.get_error_message()))
        } else {
            Ok(())
        }
    }

    pub fn fetch(&self) -> Result<Option<Vec<String>>> {
        let result = unsafe { SQLFetch(self.handle.as_ptr()) };
        if result == SQL_NO_DATA.into() {
            return Ok(None);
        } else if result != SQL_SUCCESS.into() && result != SQL_SUCCESS_WITH_INFO.into() {
            println!("Fetch failed: {result}");
            return Err(InformixError::DataFetchError(self.get_error_message()));
        }

        let mut row = Vec::new();
        for i in 1..=1000 {
            // We'll still try up to 1000 columns, but we'll break when we hit the end
            let mut buffer = [0u8; 2048];
            let mut indicator: c_long = 0;
            let result = unsafe {
                SQLGetData(
                    self.handle.as_ptr(),
                    i as c_ushort,
                    SQL_C_CHAR,
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as c_long,
                    &mut indicator,
                )
            };
            if result == SQL_NO_DATA as c_short {
                break; // We've reached the end of the columns
            } else if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short
            {
                if indicator == SQL_NULL_DATA {
                    row.push(String::from("NULL"));
                } else {
                    row.push(unsafe {
                        CStr::from_ptr(buffer.as_ptr() as *const c_char)
                            .to_bytes()
                            .iter()
                            .map(|&c| c as char)
                            .collect::<String>()
                    });
                }
            } else {
                // If we get an error other than "Invalid descriptor index", return it
                let error_message = self.get_error_message();
                if !error_message.contains("-11103") {
                    return Err(InformixError::DataFetchError(format!(
                        "GetData failed for column {}: {}",
                        i, error_message
                    )));
                }
                // If we get "Invalid descriptor index", we've reached the end of the columns
                break;
            }
        }
        Ok(Some(row))
    }

    pub fn describe_columns(&self) -> Result<Vec<ColumnMetadata>> {
        let mut column_count: c_short = 0;
        unsafe {
            let result = SQLNumResultCols(self.handle.as_ptr(), &mut column_count);
            if result != SQL_SUCCESS as c_short && result != SQL_SUCCESS_WITH_INFO as c_short {
                return Err(InformixError::DescribeColumnsError(
                    "Failed to get column count".to_string(),
                ));
            }
        }

        let mut columns = Vec::new();

        for i in 1..=column_count {
            let mut name = [0 as c_char; 256];
            let mut name_length: c_short = 0;
            let mut data_type: c_short = 0;
            let mut column_size: c_ulong = 0;
            let mut decimal_digits: c_short = 0;
            let mut nullable: c_short = 0;

            unsafe {
                let result = SQLDescribeCol(
                    self.handle.as_ptr(),
                    i as c_ushort,
                    name.as_mut_ptr(),
                    name.len() as c_short,
                    &mut name_length,
                    &mut data_type,
                    &mut column_size,
                    &mut decimal_digits,
                    &mut nullable,
                );

                if result != SQL_SUCCESS as c_short && result != SQL_SUCCESS_WITH_INFO as c_short {
                    return Err(InformixError::DescribeColumnsError(format!(
                        "Failed to describe column {}",
                        i
                    )));
                }

                let column_name = CStr::from_ptr(name.as_ptr()).to_string_lossy().into_owned();

                columns.push(ColumnMetadata {
                    name: column_name,
                    data_type,
                    column_size: column_size as u32,
                    decimal_digits,
                    nullable: nullable != 0,
                });
            }
        }

        Ok(columns)
    }

    fn get_error_message(&self) -> String {
        let mut state = [0i8; 6];
        let mut native_error = 0i32;
        let mut message = [0i8; 1024];
        let mut out_len = 0i16;

        unsafe {
            SQLGetDiagRec(
                SQL_HANDLE_STMT,
                self.handle.as_ptr(),
                1,
                state.as_mut_ptr() as *mut c_char,
                &mut native_error,
                message.as_mut_ptr() as *mut c_char,
                message.len() as c_short,
                &mut out_len,
            );
        }

        let state = unsafe { CStr::from_ptr(state.as_ptr() as *const c_char) }.to_string_lossy();
        let message =
            unsafe { CStr::from_ptr(message.as_ptr() as *const c_char) }.to_string_lossy();

        format!(
            "SQLSTATE = {}, Native Error = {}, Message = {}",
            state, native_error, message
        )
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        unsafe {
            SQLFreeHandle(3, self.handle.as_ptr());
        }
    }
}

// Higher-level abstractions
pub struct Cursor<'a> {
    pub stmt: Statement,
    pub conn: &'a Connection,
}

impl<'a> Cursor<'a> {
    pub fn execute(&mut self, sql: &str) -> Result<()> {
        self.stmt = self.conn.execute(sql)?;
        Ok(())
    }

    pub fn execute_with_params(&mut self, sql: &str, params: &[&dyn ToSql]) -> Result<()> {
        self.stmt = self.conn.prepare(sql)?;
        for (i, param) in params.iter().enumerate() {
            param.bind_parameter(self.stmt.handle.clone(), (i + 1) as u16)?;
        }
        self.stmt.execute()
    }

    pub fn fetchone(&self) -> Result<Option<Vec<String>>> {
        self.stmt.fetch()
    }

    pub fn fetchall(&self) -> Result<Vec<Vec<String>>> {
        let mut results = Vec::new();
        while let Some(row) = self.stmt.fetch()? {
            results.push(row);
        }
        Ok(results)
    }
}
