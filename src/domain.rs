pub mod base_params;
pub mod c_binds;
use crate::{connection::SendPtr, errors};
use chrono::NaiveDate;
use errors::InformixError;
use std::borrow::Cow;
use std::mem;
use std::os::raw::c_void;
use std::os::raw::{c_short, c_ushort};

pub const SQL_PARAM_INPUT: c_short = 1;
