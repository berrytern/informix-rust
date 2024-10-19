use super::c_binds::{
    SQLBindParameter, SQL_C_CHAR, SQL_C_LONG, SQL_INTEGER, SQL_PARAM_INPUT, SQL_SUCCESS,
    SQL_SUCCESS_WITH_INFO, SQL_TYPE_DATE, SQL_VARCHAR,
};
use crate::{connection::SendPtr, errors};
use chrono::Datelike;
use chrono::NaiveDate;
use errors::InformixError;
use std::borrow::Cow;
use std::mem;
use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_long, c_short, c_uchar, c_ulong, c_ushort, c_void},
};

#[derive(Debug, Clone)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: i16,
    pub column_size: u32,
    pub decimal_digits: i16,
    pub nullable: bool,
}

#[repr(C)]
pub struct SQL_DATE_STRUCT {
    pub year: c_short,
    pub month: c_ushort,
    pub day: c_ushort,
}

#[derive(Debug, Clone)]
pub enum SqlParam {
    Integer(i32),
    Str(Cow<'static, str>),
    Date(NaiveDate),
}

impl ToSql for &SqlParam {
    fn bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Result<(), InformixError> {
        match self {
            SqlParam::Integer(value) => value.bind_parameter(stmt, param_num),
            SqlParam::Str(value) => value.bind_parameter(stmt, param_num),
            SqlParam::Date(value) => value.bind_parameter(stmt, param_num),
        }
    }
}

pub trait ToSql {
    fn bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Result<(), InformixError>;
}

impl ToSql for i32 {
    fn bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Result<(), InformixError> {
        let result = unsafe {
            SQLBindParameter(
                stmt.as_ptr(),
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
            Err(InformixError::ParameterBindingError(format!(
                "Failed to bind i32 parameter: {}",
                result
            )))
        }
    }
}
impl ToSql for &str {
    fn bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Result<(), InformixError> {
        let c_str = CString::new(*self).map_err(|e| {
            InformixError::ParameterBindingError(format!("Failed to create CString: {}", e))
        })?;

        let result = unsafe {
            SQLBindParameter(
                stmt.as_ptr(),
                param_num as c_ushort,
                SQL_PARAM_INPUT,
                SQL_C_CHAR,
                SQL_VARCHAR,
                self.len() as c_ulong,
                0, // decimal digits
                c_str.as_ptr() as *const c_void,
                self.len() as c_long,
                &(self.len() as c_long) as *const c_long,
            )
        };

        if result == SQL_SUCCESS || result == SQL_SUCCESS_WITH_INFO {
            Ok(())
        } else {
            Err(InformixError::ParameterBindingError(format!(
                "Failed to bind string parameter: SQLBindParameter returned {}",
                result
            )))
        }
    }
}
impl ToSql for str {
    fn bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Result<(), InformixError> {
        let c_str = CString::new(self).map_err(|e| {
            InformixError::ParameterBindingError(format!("Failed to create CString: {}", e))
        })?;
        let result = unsafe {
            SQLBindParameter(
                stmt.as_ptr(),
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
            Err(InformixError::ParameterBindingError(format!(
                "Failed to bind string parameter: {}",
                result
            )))
        }
    }
}

impl ToSql for String {
    fn bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Result<(), InformixError> {
        self.as_str().bind_parameter(stmt, param_num)
    }
}

impl ToSql for NaiveDate {
    fn bind_parameter(&self, stmt: SendPtr<c_void>, param_num: u16) -> Result<(), InformixError> {
        // Create a SQL_DATE_STRUCT
        let date_struct = SQL_DATE_STRUCT {
            year: self.year() as c_short,
            month: self.month() as c_ushort,
            day: self.day() as c_ushort,
        };

        let result = unsafe {
            SQLBindParameter(
                stmt.as_ptr(),
                param_num,
                SQL_PARAM_INPUT,
                SQL_TYPE_DATE,
                SQL_TYPE_DATE,
                10, // size of YYYY-MM-DD
                0,  // decimal digits
                &date_struct as *const SQL_DATE_STRUCT as *const c_void,
                mem::size_of::<SQL_DATE_STRUCT>() as c_long,
                std::ptr::null(),
            )
        };

        if result == SQL_SUCCESS as c_short || result == SQL_SUCCESS_WITH_INFO as c_short {
            Ok(())
        } else {
            Err(InformixError::ParameterBindingError(format!(
                "Failed to bind date parameter: SQLBindParameter returned {}",
                result
            )))
        }
    }
}
