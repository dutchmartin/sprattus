use futures::{
    future::Future,
    stream::Stream
};
use tokio_postgres::*;
use parking_lot::*;
use std::sync::Arc;
use tokio;

pub struct PGConnection {
    client: Arc<Mutex<Client>>
}

impl PGConnection {
    ///
    /// Creates a new connection to the database.
    ///
    /// example
    /// ```
    /// use profugus::*;
    ///
    /// let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg");
    /// ```
    pub fn new(connection_string: &str) -> Box<dyn Future<Item = PGConnection, Error = tokio_postgres::Error>> {
            let fut = tokio_postgres::connect(connection_string, NoTls)
            .map(|(mut client, connection)| {
                let connection = connection.map_err(|e| eprintln!("connection error: {}", e));
                tokio::spawn(connection);
                PGConnection{
                    client: Arc::new(Mutex::new(client))
                }
            });
        Box::new(fut)
    }

    pub fn query(self, sql: &str) {
        unimplemented!()
    }
}
