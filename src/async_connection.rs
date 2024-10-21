use crate::{connection::Connection, domain::base_params::SqlParam, errors};
use errors::InformixError;
use std::sync::{Arc, Mutex};

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
        std::thread::spawn(move || match connection.lock() {
            Ok(conn) => {
                let result_statement = conn.prepare(&query);
                match result_statement {
                    Ok(statement) => {
                        for (index, param) in parameters.iter().enumerate() {
                            if let Err(err) = statement.bind_parameter(index as u16 + 1, &param) {
                                let _ = tx.send(Err(err));
                                return;
                            };
                        }
                        if let Err(err) = statement.execute() {
                            let _ = tx.send(Err(err));
                        } else {
                            let mut result: Vec<Vec<String>> = Vec::new();
                            loop {
                                match statement.fetch() {
                                    Ok(Some(row)) => result.push(row),
                                    Ok(None) => {
                                        if result.is_empty() {
                                            let _ = tx.send(Ok(None));
                                        } else {
                                            let _ = tx.send(Ok(Some(result)));
                                        }
                                        break;
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Err(e));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(Err(err));
                    }
                }
            }
            Err(err) => {
                let _ = tx.send(Err(InformixError::ConnectionError(format!(
                    "Error getting connection lock {err}"
                ))));
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
