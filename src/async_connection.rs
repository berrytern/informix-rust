use crate::{
    connection::{Connection, SendPtr, ToSql},
    errors,
};
use chrono::NaiveDate;
use errors::InformixError;
use std::borrow::Cow;
use std::os::raw::c_void;
use std::sync::{Arc, Mutex};

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

pub struct AsyncConnection {
    connection: Arc<Mutex<Connection>>,
}
impl AsyncConnection {
    pub fn new(conn_string: &str) -> Result<Self, InformixError> {
        let connection = Connection::new()?;
        connection.connect_with_string(conn_string)?;
        Ok(AsyncConnection {
            connection: Arc::new(Mutex::new(connection)),
        })
    }
    pub async fn query_with_parameters(
        &self,
        query: String,
        parameters: Vec<SqlParam>,
    ) -> Result<Option<Vec<Vec<String>>>, InformixError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let connection = self.connection.clone();
        std::thread::spawn(move || {
            if let Ok(conn) = connection.lock() {
                let result_statement = conn.prepare(&query);
                if let Ok(statement) = result_statement {
                    for (index, param) in parameters.iter().enumerate() {
                        statement.bind_parameter(index as u16 + 1, &param).unwrap();
                    }
                    if let Err(err) = statement.execute() {
                        let _ = tx.send(Err(err));
                    } else {
                        let mut result: Vec<Vec<String>> = Vec::new();
                        while let Some(row) = statement.fetch().unwrap() {
                            result.push(row);
                        }
                        if result.is_empty() {
                            let _ = tx.send(Ok(None));
                        } else {
                            let _ = tx.send(Ok(Some(result)));
                        }
                    }
                } else {
                    let _ = tx.send(Err(result_statement.err().unwrap()));
                }
            } else {
                let _ = tx.send(Err(InformixError::ConnectionError(
                    "Error getting connection lock".into(),
                )));
            }
        });
        match rx.await {
            Ok(result) => result,
            Err(e) => Err(InformixError::ConnectionError(format!(
                "Error executing query: {}",
                e
            ))),
        }
    }
}
