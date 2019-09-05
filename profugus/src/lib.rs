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
    pub fn new(connection_string: &str) -> Box<dyn Future<Item = PGConnection, Error = tokio_postgres::Error> + Send> {
            let fut = tokio_postgres::connect(connection_string, NoTls)
            .map(|(client, connection)| {
                let connection = connection.map_err(|e| eprintln!("connection error: {}", e));
                tokio::spawn(connection);
                PGConnection{
                    client: Arc::new(Mutex::new(client))
                }
            });
        Box::new(fut)
    }


    pub fn query<T>(self, sql: &str) -> Box<dyn Future<Item=Vec<T>, Error=error::Error> + Send>
    where T: FromSql
    {
        let prepared_client = self.client
            .lock()
            //.unwrap()
            .prepare(sql);
        let returned_future = prepared_client
            .and_then(move |statement|{
                self.client
                    .lock()
                    //.unwrap()
                    .query(&statement, &[]).collect()
            })
            .map(move |rows|{
                rows.iter().map(move |row|{
                    T::from_row(row)
                }).collect()
            });
        Box::new(returned_future)
    }
}

pub use profugus_derive::FromSql;
pub use tokio_postgres::{ Row, Error };

pub trait FromSql {
    fn from_row(row: &Row) -> Self;
}