
use std::os::raw::{c_char, c_int, c_long, c_short, c_uchar, c_ulong, c_ushort, c_void};


#[link(name = "ifcli")]
extern "C" {
    pub fn SQLAllocHandle(
        HandleType: c_int,
        InputHandle: *mut c_void,
        OutputHandle: *mut *mut c_void,
    ) -> c_int;
    pub fn SQLConnect(
        ConnectionHandle: *mut c_void,
        ServerName: *const c_char,
        NameLength1: c_int,
        UserName: *const c_char,
        NameLength2: c_int,
        Authentication: *const c_char,
        NameLength3: c_int,
    ) -> c_int;
    pub fn SQLPrepare(
        StatementHandle: *mut c_void,
        StatementText: *const c_uchar,
        TextLength: c_int,
    ) -> c_short;
    pub fn SQLBindParameter(
        StatementHandle: *mut c_void,
        ParameterNumber: c_ushort,
        InputOutputType: c_short,
        ValueType: c_short,
        ParameterType: c_short,
        ColumnSize: c_ulong,
        DecimalDigits: c_short,
        ParameterValuePtr: *const c_void,
        BufferLength: c_long,
        StrLen_or_IndPtr: *const c_long,
    ) -> c_short;
    pub fn SQLExecute(StatementHandle: *mut c_void) -> c_short;
    pub fn SQLExecDirect(
        StatementHandle: *mut c_void,
        StatementText: *const c_char,
        TextLength: c_int,
    ) -> c_int;
    pub fn SQLFetch(StatementHandle: *mut c_void) -> c_int;
    pub fn SQLGetData(
        StatementHandle: *mut c_void,
        ColumnNumber: c_ushort,
        TargetType: c_short,
        TargetValue: *mut c_void,
        BufferLength: c_long,
        StrLen_or_Ind: *mut c_long,
    ) -> c_short;
    pub fn SQLGetDiagRec(
        HandleType: c_short,
        Handle: *mut c_void,
        RecNumber: c_short,
        SQLState: *mut c_char,
        NativeErrorPtr: *mut c_int,
        MessageText: *mut c_char,
        BufferLength: c_short,
        TextLengthPtr: *mut c_short,
    ) -> c_short;
    pub fn SQLDriverConnect(
        ConnectionHandle: *mut c_void,
        WindowHandle: *mut c_void,
        InConnectionString: *const c_char,
        StringLength1: c_short,
        OutConnectionString: *mut c_char,
        BufferLength: c_short,
        StringLength2Ptr: *mut c_short,
        DriverCompletion: c_ushort,
    ) -> c_short;
    pub fn SQLDisconnect(ConnectionHandle: *mut c_void) -> c_int;
    pub fn SQLFreeHandle(HandleType: c_short, Handle: *mut c_void) -> c_int;
    pub fn SQLNumResultCols(StatementHandle: *mut c_void, ColumnCountPtr: *mut c_short) -> c_short;
    pub fn SQLDescribeCol(
        StatementHandle: *mut c_void,
        ColumnNumber: c_ushort,
        ColumnName: *mut c_char,
        BufferLength: c_short,
        NameLengthPtr: *mut c_short,
        DataTypePtr: *mut c_short,
        ColumnSizePtr: *mut c_ulong,
        DecimalDigitsPtr: *mut c_short,
        NullablePtr: *mut c_short,
    ) -> c_short;
}


pub const SQL_PARAM_INPUT: c_short = 1;

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
pub const SQL_NTS: c_long = -3;