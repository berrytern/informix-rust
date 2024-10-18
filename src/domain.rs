use std::os::raw::{c_short, c_ushort};

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
