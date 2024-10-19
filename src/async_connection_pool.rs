use tokio::sync::Mutex;

use crate::{
    connection::Connection,
    domain::base_params::SqlParam,
    errors::InformixError,
};
use std::collections::HashMap;
use std::fmt::format;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, sender: Sender<Result<Option<Vec<Vec<String>>>, InformixError>>, receiver: Receiver<(String, Vec<SqlParam>)>, connection: Connection) -> Worker {
        let thread = thread::spawn(move || loop {
            let (query, parameters) = receiver.recv().unwrap();
            println!("Worker {} got a job; executing.", id);
            sender.send(connection.query_with_parameters(query, parameters));
        });
        
        Worker { id, thread }
    }
}

#[derive(Clone)]
pub struct AsyncConnectionPool {
    workers: Arc<Mutex<HashMap<usize, (Sender<(String,Vec<SqlParam>)>, Receiver<Result<Option<Vec<Vec<String>>>, InformixError>>, Worker)>>>,
    index: Arc<Mutex<u16>>
}
impl AsyncConnectionPool {
    pub fn new(conn_string: &str, size: usize) -> Result<Self, InformixError> {
        let mut workers = HashMap::with_capacity(size);
        for id in 0..size {
            let (sender, receiver) = channel();
            let (thread_sender, thread_receiver) = channel();
            let connection = Connection::new()?;
            connection.connect_with_string(conn_string)?;

            workers.insert(id, (sender, thread_receiver, Worker::new(id, thread_sender, receiver, connection)));
        }
        Ok(AsyncConnectionPool {
            workers: Arc::new(Mutex::new(workers)),
            index: Arc::new(Mutex::new(0))
        })
    }

    pub async fn query(&mut self, query: String, parameters: Vec<SqlParam>) -> Result<Option<Vec<Vec<String>>>, InformixError> {
        let workers = Arc::clone(&self.workers);
        let mut index = self.index.lock().await;
        let guard = workers.lock().await;
        let current = *index;
        *index = if (*index + 1) as usize >= guard.len() { 0 } else { *index + 1 }; 
        drop(index);
        let worker = guard.get(&(current as usize));
        if let Some((sender, receiver, _)) = worker {
            if let Ok(_) = sender.send((query, parameters)){
                if let Ok(promise) = receiver.recv() {
                    return promise;
                };
            }
            return Err(InformixError::ConnectionError("Error in query".into()));
        } else {
            return Err(InformixError::ConnectionError(format!("Could not get worker: {current}").into()));
        }
        
    }
}
