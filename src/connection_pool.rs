use crate::{connection::Connection, errors::InformixError};

pub struct ConnectionPool {
    connections: Vec<Connection>,
}
impl ConnectionPool {
    pub fn new(size: usize) -> Result<Self, InformixError> {
        let mut connections = Vec::with_capacity(size);
        for _ in 0..size {
            let connection = Connection::new()?;
            connections.push(connection);
        }
        Ok(ConnectionPool { connections })
    }

    pub fn get_connection(&mut self) -> Option<Connection> {
        self.connections.pop()
    }
    pub fn free_connection(&mut self, conn: Connection) {
        self.connections.push(conn);
    }
}
