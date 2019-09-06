use futures_util::try_future::TryFutureExt;
use futures_util::future::FutureExt;
use futures::TryStreamExt;
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
    pub async fn new(connection_string: &str) -> Result<PGConnection,Error> {
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await?;

        let connection = connection
            .map_err(|e| panic!("connection error: {}", e))
            .map(|conn|conn.unwrap());
        tokio::spawn(connection);
        Ok(PGConnection{
            client: Arc::new(Mutex::new(client))
        })
    }

    pub async fn query<T>(self, sql: &str) -> Result<Vec<T>, Error>
    where T: FromSql
    {
        let statement = self.client
            .lock()
            .prepare(sql)
            .await?;
        self.client
            .lock()
            .query(&statement, &[])
            .map_ok(|row|{
                T::from_row(&row)
            })
            .try_collect::<Vec<T>>().await
    }
}

pub use profugus_derive::FromSql;
pub use tokio_postgres::{ Row, Error };

pub trait FromSql {
    fn from_row(row: &Row) -> Self;
}