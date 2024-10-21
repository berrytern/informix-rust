use tokio::sync::Mutex;

use crate::{connection::Connection, domain::base_params::SqlParam, errors::InformixError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

fn create_worker(
    id: usize,
    sender: Sender<Result<Option<Vec<Vec<String>>>, InformixError>>,
    mut receiver: Receiver<(String, Vec<SqlParam>)>,
    conn_string: &str,
) -> Result<(), InformixError> {
    let connection = Connection::new()?;
    connection.connect_with_string(conn_string)?;
    tokio::spawn(async move {
        while let Some((query, parameters)) = receiver.recv().await {
            if let Err(e) = sender
                .send(connection.query_with_parameters(query, parameters))
                .await
            {
                println!("Failed to send query result: {:?}", e);
            }
        }
        println!("Worker thread {} shutting down. Reason: channel closed", id);
    });
    Ok(())
}

#[derive(Clone)]
pub struct AsyncConnectionPool {
    workers: Arc<
        HashMap<
            usize,
            Arc<
                Mutex<(
                    Sender<(String, Vec<SqlParam>)>,
                    Receiver<Result<Option<Vec<Vec<String>>>, InformixError>>,
                )>,
            >,
        >,
    >,
    index: Arc<Mutex<u16>>,
}
impl AsyncConnectionPool {
    pub async fn new(conn_string: &str, size: usize) -> Result<Self, InformixError> {
        let mut workers = HashMap::with_capacity(size);

        for id in 0..size {
            let (sender, receiver) = channel(1);
            let (thread_sender, thread_receiver) = channel(1);

            workers.insert(id, Arc::new(Mutex::new((sender, thread_receiver))));
            create_worker(id, thread_sender, receiver, conn_string)?;
        }
        Ok(AsyncConnectionPool {
            workers: Arc::new(workers),
            index: Arc::new(Mutex::new(0)),
        })
    }

    pub async fn query(
        &mut self,
        query: String,
        parameters: Vec<SqlParam>,
    ) -> Result<Option<Vec<Vec<String>>>, InformixError> {
        let workers = Arc::clone(&self.workers);
        let mut index = self.index.lock().await;
        let current = *index;
        *index = if (*index + 1) as usize >= workers.len() {
            0
        } else {
            *index + 1
        };
        match workers.get(&(current as usize)) {
            Some(item) => {
                drop(index);
                let mut guard = item.lock().await;
                match guard.0.send((query, parameters)).await {
                    Ok(_) => match guard.1.recv().await {
                        Some(result) => result,
                        None => Err(InformixError::ConnectionError("Channel closed".into())),
                    },
                    Err(e) => Err(InformixError::ConnectionError(format!(
                        "Receiver dropped: {:?}",
                        e
                    ))),
                }
            }
            None => Err(InformixError::ConnectionError(format!(
                "Could not get worker: {current}"
            ))),
        }
    }
}
