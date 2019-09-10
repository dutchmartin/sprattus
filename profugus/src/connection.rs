use futures::TryStreamExt;
use futures_util::future::FutureExt;
use futures_util::try_future::TryFutureExt;
use futures_util::StreamExt;
use parking_lot::*;
use std::sync::Arc;
use tokio;
use tokio_postgres::types::ToSql;
use tokio_postgres::*;

use crate::*;

pub struct PGConnection {
    client: Arc<Mutex<Client>>,
}

impl PGConnection {
    ///
    /// Creates a new connection to the database.
    ///
    /// Example
    /// ```
    /// use profugus::*;
    ///
    /// let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg");
    /// ```
    pub async fn new(connection_string: &str) -> Result<PGConnection, Error> {
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await?;

        let connection = connection
            .map_err(|e| panic!("connection error: {}", e))
            .map(|conn| conn.unwrap());
        tokio::spawn(connection);
        Ok(PGConnection {
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// Query multiple rows of a table.
    /// Example:
    /// ```
    /// use profugus::PGConnection;
    /// use profugus::FromSql;
    /// use tokio::prelude::*;
    ///
    /// #[derive(FromSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     prod_id: i32,
    ///     title: String
    /// }
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    ///     let product_list : Vec<Product> = conn.query_multiple("SELECT prod_id, title FROM Products LIMIT 3", &[]).await.unwrap();
    ///     assert_eq!(product_list,
    ///         vec!(
    ///         Product {
    ///		        prod_id : 1,
    ///		        title : String::from("ACADEMY ACADEMY")
    ///	        },
    ///	        Product {
    ///	    	    prod_id : 2,
    ///	    	    title : String::from("ACADEMY ACE")
    /// 	    },
    ///	        Product {
    ///	        	prod_id : 3,
    ///	        	title : String::from("ACADEMY ADAPTATION")
    ///	        })
    ///     );
    /// }
    /// ```
    pub async fn query_multiple<T>(self, sql: &str, args: &[&dyn ToSql]) -> Result<Vec<T>, Error>
    where
        T: FromSql,
    {
        let statement = self.client.lock().prepare(sql).await?;
        let result = { self.client.lock().query(&statement, args) };
        result
            .map_ok(|row| T::from_row(&row))
            .try_collect::<Vec<T>>()
            .await
    }

    /// Get a single row of a table.
    /// Example:
    /// ```
    /// use profugus::PGConnection;
    /// use tokio::prelude::*;
    /// use profugus::FromSql;
    ///
    /// #[derive(FromSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    ///     let product : Product = conn.query("SELECT prod_id, title FROM Products LIMIT 1", &[]).await.unwrap();
    ///     assert_eq!(product, Product{ prod_id: 1, title: String::from("ACADEMY ACADEMY")});
    /// }
    /// ```
    #[allow(unused_variables)]
    pub async fn query<T>(self, sql: &str, args: &[&dyn ToSql]) -> Result<T, Error>
    where
        T: FromSql,
    {
        unimplemented!()
    }

    /// Update a single rust value in the database.
    #[allow(unused_variables)]
    pub async fn update<T>(self, item: T) -> Result<(), Error>
    where
        T: Identifiable + ToSql,
    {
        unimplemented!()
    }

    /// Update multiple rust values in the database.
    #[allow(unused_variables)]
    pub async fn update_multiple<T>(self, items: Vec<T>) -> Result<(), Error>
    where
        T: Identifiable + ToSql,
    {
        unimplemented!()
    }

    /// Create a new row in the database.
    #[allow(unused_variables)]
    pub async fn create<T>(self, item: T) -> Result<Vec<T>, Error>
    where
        T: Identifiable + ToSql,
    {
        unimplemented!()
    }

    /// Create new rows in the database.
    #[allow(unused_variables)]
    pub async fn create_multiple<T>(self, items: Vec<T>) -> Result<Vec<T>, Error>
    where
        T: Identifiable + ToSql,
    {
        unimplemented!()
    }

    pub async fn delete<T>(item: T) -> Result<(), Error>
    where
        T: Identifiable,
    {
        unimplemented!()
    }

    pub async fn delete_multiple<T>(item: Vec<T>) -> Result<(), Error>
    where
        T: Identifiable,
    {
        unimplemented!()
    }
}
