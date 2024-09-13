// File: src/lib.rs
use chrono::NaiveDate;
use chrono::Datelike;
use std::mem;
use std::os::raw::{c_char, c_uchar, c_int, c_void, c_short, c_ushort, c_long, c_ulong};
use std::ffi::{CStr, CString};



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

// Safe Rust wrappers
pub struct Connection {
    handle: *mut c_void,
}


impl Connection {
    pub fn new() -> Result<Self, String> {
        let mut handle: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            SQLAllocHandle(SQL_HANDLE_ENV.into(), std::ptr::null_mut(), &mut handle)
        };
        if result == 0 {
            let mut conn_handle: *mut c_void = std::ptr::null_mut();
            let conn_result = unsafe {
                SQLAllocHandle(SQL_HANDLE_DBC.into(), handle, &mut conn_handle)
            };
            if conn_result == 0 {
                Ok(Connection { handle: conn_handle })
            } else {
                Err(format!("Failed to allocate connection handle: {}", conn_result))
            }
        } else {
            Err(format!("Failed to allocate environment handle: {}", result))
        }
    }

    pub fn connect_with_string(&self, conn_string: &str) -> Result<(), String> {
        println!("Attempting to connect with string: {}", conn_string);
        
        let conn_string = CString::new(conn_string).map_err(|e| format!("Invalid connection string: {}", e))?;
        
        let mut out_conn_string = [0u8; 1024];
        let mut out_conn_string_len: c_short = 0;
        let result = unsafe {
            SQLDriverConnect(
                self.handle,
                std::ptr::null_mut(),
                conn_string.as_ptr() as *const c_char,
                conn_string.as_bytes().len() as c_short,
                out_conn_string.as_mut_ptr() as *mut c_char,
                out_conn_string.len() as c_short,
                &mut out_conn_string_len,
                SQL_DRIVER_NOPROMPT
            )
        };
        
        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            println!("Successfully connected to the database");
            Ok(())
        } else {
            let error_message = self.get_error_message();
            Err(format!("Failed to connect: result = {}, {}", result, error_message))
        }
    }

    pub fn prepare(&self, sql: &str) -> Result<Statement, String> {
        let mut stmt_handle: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            SQLAllocHandle(SQL_HANDLE_STMT.into(), self.handle, &mut stmt_handle)
        };
        if result != 0 {
            return Err(format!("Failed to allocate statement handle: {}", result));
        }

        let sql_cstring = CString::new(sql).map_err(|e| format!("Invalid SQL string: {}", e))?;
        let result = unsafe {
            SQLPrepare(stmt_handle, sql_cstring.as_ptr() as *const c_uchar, sql.len() as c_int)
        };
        if result != 0 {
            unsafe { SQLFreeHandle(SQL_HANDLE_STMT, stmt_handle) };
            return Err(format!("Failed to prepare SQL: {}", result));
        }

        Ok(Statement::new(stmt_handle , ""))
    }

    pub fn connect(&self, server: &str, user: &str, password: &str) -> Result<(), String> {
        let server = CString::new(server).unwrap();
        let user = CString::new(user).unwrap();
        let password = CString::new(password).unwrap();
        let result = unsafe {
            SQLConnect(self.handle, 
                       server.as_ptr(), server.as_bytes().len() as c_int,
                       user.as_ptr(), user.as_bytes().len() as c_int,
                       password.as_ptr(), password.as_bytes().len() as c_int)
        };
        if result == 0 {
            Ok(())
        } else {
            let error_message = self.get_error_message();
            Err(format!("Failed to connect: {}", error_message))
        }
    }

    fn get_error_message(&self) -> String {
        let mut state = [0i8; 6];
        let mut native_error = 0i32;
        let mut message = [0i8; 1024];
        let mut out_len = 0i16;
        
        unsafe {
            SQLGetDiagRec(SQL_HANDLE_DBC, self.handle, 1, 
                          state.as_mut_ptr() as *mut c_char, 
                          &mut native_error, 
                          message.as_mut_ptr() as *mut c_char, 
                          message.len() as c_short, 
                          &mut out_len);
        }
        
        let state = unsafe { CStr::from_ptr(state.as_ptr() as *const c_char) }.to_string_lossy();
        let message = unsafe { CStr::from_ptr(message.as_ptr() as *const c_char) }.to_string_lossy();
        
        format!("SQLSTATE = {}, Native Error = {}, Message = {}", state, native_error, message)
    }

    pub fn execute(&self, sql: &str) -> Result<Statement, String> {
        let mut stmt_handle: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            SQLAllocHandle(3, self.handle, &mut stmt_handle)
        };
        if result != 0 {
            return Err(format!("Failed to allocate statement handle: {}", result));
        }

        let sql = CString::new(sql).unwrap();
        let result = unsafe {
            SQLExecDirect(stmt_handle, sql.as_ptr(), sql.as_bytes().len() as c_int)
        };
        if result == 0 {
            Ok(Statement::new(stmt_handle , ""))
        } else {
            unsafe { SQLFreeHandle(3, stmt_handle) };
            Err(format!("Failed to execute SQL: {}", result))
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            SQLDisconnect(self.handle);
            SQLFreeHandle(1, self.handle);
        }
    }
}

pub struct Statement {
    pub handle: *mut c_void,
    query: String,
}

impl Statement {
    pub fn new(handle: *mut c_void, query: &str) -> Self {
        Statement {
            handle,
            query: query.into(),
        }
    }

    pub fn bind_parameter<T: ToSql>(&self, param_num: u16, value: &T) -> Result<(), String> {
        value.bind_parameter(self.handle, param_num)
    }

    pub fn execute(&self) -> Result<(), String> {
        let result = unsafe { SQLExecute(self.handle) };
        if result != SQL_SUCCESS as c_short && result != SQL_SUCCESS_WITH_INFO as c_short {
            Err(self.get_error_message())
        } else {
            Ok(())
        }
    }

    pub fn fetch(&self) -> Result<Option<Vec<String>>, String> {
        let result = unsafe { SQLFetch(self.handle) };
        if result == SQL_NO_DATA.into() {
            return Ok(None);
        } else if result != SQL_SUCCESS.into() && result != SQL_SUCCESS_WITH_INFO.into() {
            return Err(self.get_error_message());
        }
    
        let mut row = Vec::new();
        for i in 1..=1000 { // We'll still try up to 1000 columns, but we'll break when we hit the end
            let mut buffer = [0u8; 2048];
            let mut indicator: c_long = 0;
            let result = unsafe {
                SQLGetData(
                    self.handle,
                    i as c_ushort,
                    SQL_C_CHAR,
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as c_long,
                    &mut indicator,
                )
            };
            if result == SQL_NO_DATA as c_short {
                break; // We've reached the end of the columns
            } else if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
                if indicator == SQL_NULL_DATA {
                    row.push(String::from("NULL"));
                } else {
                    let s = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) }
                        .to_string_lossy()
                        .into_owned();
                    row.push(s);
                }
            } else {
                // If we get an error other than "Invalid descriptor index", return it
                let error_message = self.get_error_message();
                if !error_message.contains("Invalid descriptor index") {
                    return Err(format!("GetData failed for column {}: {}", i, error_message));
                }
                // If we get "Invalid descriptor index", we've reached the end of the columns
                break;
            }
        }
        Ok(Some(row))
    }

    fn get_error_message(&self) -> String {
        let mut state = [0i8; 6];
        let mut native_error = 0i32;
        let mut message = [0i8; 1024];
        let mut out_len = 0i16;
        
        unsafe {
            SQLGetDiagRec(
                SQL_HANDLE_STMT,
                self.handle,
                1,
                state.as_mut_ptr() as *mut c_char,
                &mut native_error,
                message.as_mut_ptr() as *mut c_char,
                message.len() as c_short,
                &mut out_len,
            );
        }
        
        let state = unsafe { CStr::from_ptr(state.as_ptr() as *const c_char) }.to_string_lossy();
        let message = unsafe { CStr::from_ptr(message.as_ptr() as *const c_char) }.to_string_lossy();
        
        format!("SQLSTATE = {}, Native Error = {}, Message = {}", state, native_error, message)
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        unsafe {
            SQLFreeHandle(3, self.handle);
        }
    }
}

pub trait ToSql {
    fn bind_parameter(&self, stmt: *mut c_void, param_num: u16) -> Result<(), String>;
}

impl ToSql for i32 {
    fn bind_parameter(&self, stmt: *mut c_void, param_num: u16) -> Result<(), String> {
        let result = unsafe {
            SQLBindParameter(
                stmt,
                param_num,
                SQL_PARAM_INPUT,
                SQL_C_LONG,
                SQL_INTEGER,
                0,
                0,
                self as *const i32 as *const c_void,
                0,
                std::ptr::null(),
            )
        };
        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            Ok(())
        } else {
            Err(format!("Failed to bind i32 parameter: {}", result))
        }
    }
}
impl ToSql for &str {
    fn bind_parameter(&self, stmt: *mut c_void, param_num: u16) -> Result<(), String> {
        let c_str = CString::new(*self).map_err(|e| format!("Failed to create CString: {}", e))?;
        
        let result = unsafe {
            SQLBindParameter(
                stmt,
                param_num as c_ushort,
                SQL_PARAM_INPUT,
                SQL_C_CHAR,
                SQL_VARCHAR,
                self.len() as c_ulong,
                0,  // decimal digits
                c_str.as_ptr() as *const c_void,
                self.len() as c_long,
                &(self.len() as c_long) as *const c_long
            )
        };

        if result == SQL_SUCCESS || result == SQL_SUCCESS_WITH_INFO {
            Ok(())
        } else {
            Err(format!("Failed to bind string parameter: SQLBindParameter returned {}", result))
        }
    }
}
impl ToSql for str {
    fn bind_parameter(&self, stmt: *mut c_void, param_num: u16) -> Result<(), String> {
        let c_str = CString::new(self).map_err(|e| format!("Failed to create CString: {}", e))?;
        let result = unsafe {
            SQLBindParameter(
                stmt,
                param_num,
                SQL_PARAM_INPUT,
                SQL_C_CHAR,
                SQL_VARCHAR,
                self.len() as c_ulong,
                0,
                c_str.as_ptr() as *const c_void,
                0,
                &(self.len() as c_long) as *const c_long,
            )
        };
        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            Ok(())
        } else {
            Err(format!("Failed to bind string parameter: {}", result))
        }
    }
}

impl ToSql for String {
    fn bind_parameter(&self, stmt: *mut c_void, param_num: u16) -> Result<(), String> {
        self.as_str().bind_parameter(stmt, param_num)
    }
}

#[repr(C)]
struct SQL_DATE_STRUCT {
    year: c_short,
    month: c_ushort,
    day: c_ushort,
}

impl ToSql for NaiveDate {
    fn bind_parameter(&self, stmt: *mut c_void, param_num: u16) -> Result<(), String> {
        // Create a SQL_DATE_STRUCT
        let date_struct = SQL_DATE_STRUCT {
            year: self.year() as c_short,
            month: self.month() as c_ushort,
            day: self.day() as c_ushort,
        };

        let result = unsafe {
            SQLBindParameter(
                stmt,
                param_num,
                SQL_PARAM_INPUT,
                SQL_TYPE_DATE,
                SQL_TYPE_DATE,
                10,  // size of YYYY-MM-DD
                0,   // decimal digits
                &date_struct as *const SQL_DATE_STRUCT as *const c_void,
                mem::size_of::<SQL_DATE_STRUCT>() as c_long,
                std::ptr::null(),
            )
        };

        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            Ok(())
        } else {
            Err(format!("Failed to bind date parameter: SQLBindParameter returned {}", result))
        }
    }
}

// Higher-level abstractions
pub struct Cursor<'a> {
    pub stmt: Statement,
    pub conn: &'a Connection,
}

impl<'a> Cursor<'a> {
    pub fn execute(&mut self, sql: &str) -> Result<(), String> {
        self.stmt = self.conn.execute(sql)?;
        Ok(())
    }

    pub fn execute_with_params(&mut self, sql: &str, params: &[&dyn ToSql]) -> Result<(), String> {
        self.stmt = self.conn.prepare(sql)?;
        for (i, param) in params.iter().enumerate() {
            param.bind_parameter(self.stmt.handle, (i + 1) as u16)?;
        }
        self.stmt.execute()
    }

    pub fn fetchone(&self) -> Result<Option<Vec<String>>, String> {
        self.stmt.fetch()
    }

    pub fn fetchall(&self) -> Result<Vec<Vec<String>>, String> {
        let mut results = Vec::new();
        while let Some(row) = self.stmt.fetch()? {
            results.push(row);
        }
        Ok(results)
    }
}

// Example usage
pub fn example() -> Result<(), String> {
    let conn = Connection::new()?;
    conn.connect("REDACTED_HOST", "REDACTED_USER", "REDACTED_PASSWORD")?;

    let mut cursor = Cursor { stmt: Statement::new(std::ptr::null_mut() , ""), conn: &conn };
    cursor.execute("SELECT * FROM your_table")?;

    while let Some(row) = cursor.fetchone()? {
        println!("{:?}", row);
    }

    Ok(())
}
