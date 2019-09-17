use futures::TryStreamExt;
use futures_util::future::FutureExt;
use futures_util::try_future::TryFutureExt;
use futures_util::StreamExt;
use parking_lot::*;
use std::sync::Arc;
use tokio;
use tokio_postgres::*;

use crate::*;
#[derive(Clone)]
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

    ///
    /// Query multiple rows of a table.
    ///
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
    pub async fn query_multiple<T>(
        self,
        sql: &str,
        args: &[&dyn ToSqlItem],
    ) -> Result<Vec<T>, Error>
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

    ///
    /// Get a single row of a table.
    ///
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
    pub async fn query<T>(self, sql: &str, args: &[&dyn ToSqlItem]) -> Result<T, Error>
    where
        T: FromSql,
    {
        // TODO: Figure out a way to do this more efficiently without panic on fail.
        let mut results: Vec<T> = self.query_multiple(&sql, args).await?;
        let result = results.pop().expect(
            format!(
                "The result of the query `{}` should contain at least one row",
                &sql
            )
            .as_ref(),
        );
        Ok(result)
    }

    ///
    /// Update a single rust value in the database.
    ///
    /// Example:
    /// ```
    /// use profugus::PGConnection;
    /// use tokio::prelude::*;
    /// use profugus::FromSql;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    ///     // Change a existing record in the database.
    ///     conn.update(Product { prod_id : 50, title: String::from("Rust ORM")}).await.expect("update failed");
    ///
    ///     let product : Product = conn.query("SELECT prod_id, title FROM Products where prod_id = 50", &[]).await.unwrap();
    ///     assert_eq!(product, Product{ prod_id: 50, title: String::from("Rust ORM")});
    ///     // Change it back to it's original value.
    ///     conn.update(Product { prod_id : 50, title: String::from("ACADEMY BAKED")}).await.expect("update failed");
    ///
    ///     let product : Product = conn.query("SELECT prod_id, title FROM Products where prod_id = 50", &[]).await.unwrap();
    ///     assert_eq!(product, Product{ prod_id: 50, title: String::from("ACADEMY BAKED")});
    /// }
    /// ```
    pub async fn update<T>(self, item: T) -> Result<T, Error>
    where
        T: Sized + ToSql,
    {
        let sql = format!(
            "UPDATE {table_name} SET X = $1, $2 WHERE {primary_key} = (${count}) RETURNING *",
            table_name = T::get_table_name(),
            primary_key = T::get_primary_key(),
            count = T::get_argument_count()
        );
        let insert = self.client.lock().prepare(sql.as_str());
        let insert = insert.await?;

        let result = {
            self.client
                .lock()
                .query(&insert, &[&item.get_primary_key_value()])
        };
        Ok(result
            .map_ok(|row| T::from_row(&row))
            .try_collect::<Vec<T>>()
            .await?
            .pop()
            .expect("at least it should return a row"))
    }

    ///
    /// Update multiple rust values in the database.
    ///
    /// Example:
    /// ```
    /// use profugus::PGConnection;
    /// use tokio::prelude::*;
    /// use profugus::FromSql;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    ///     let new_products = vec!(
    ///             Product{ prod_id: 60, title: String::from("Rust ACADEMY") },
    ///             Product{ prod_id: 61, title: String::from("SQL ACADEMY") },
    ///             Product{ prod_id: 62, title: String::from("Backend development training") },
    ///         );
    ///     // Change a existing record in the database.
    ///     conn.update_multiple(new_products).await.expect("update failed");
    ///
    ///     let products : Vec<Product> = conn.query("SELECT prod_id, title FROM Products where prod_id in (60, 61, 62)", &[]).await.unwrap();
    ///     assert_eq!(products, new_products);
    ///
    ///     let old_products = vec!(
    ///             Product{ prod_id: 50, title: String::from("ACADEMY BEAST") },
    ///             Product{ prod_id: 50, title: String::from("ACADEMY BEAUTY") },
    ///             Product{ prod_id: 50, title: String::from("ACADEMY BED") },
    ///         );
    ///     // Change it back to it's original value.
    ///     conn.update_multiple(old_products).await.expect("update failed");
    ///
    ///     let product_list : Vec<Product> = conn.query("SELECT prod_id, title FROM Products where prod_id in (60, 61, 62)", &[]).await.unwrap();
    ///     assert_eq!(product_list, old_products);
    /// }
    /// ```
    pub async fn update_multiple<T>(self, items: Vec<T>) -> Result<(), Error>
    where
        T: Sized + ToSql,
    {
        unimplemented!();
    }

    ///
    /// Create a new row in the database.
    ///
    /// Example:
    /// ```
    /// #![feature(custom_attribute)]
    /// use profugus::PGConnection;
    /// use tokio::prelude::*;
    /// use profugus::FromSql;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     #[profugus(primary_key)]
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    ///     let new_product = Product {prod_id: 0, title: String::from("Sql insert lesson")};
    ///     let id = conn.create(new_product).await.unwrap().get_id();
    ///     let product = conn.query("SELECT prod_id, title from Products where prod_id = $1", &[id]);
    ///
    ///     assert_eq!(new_product, product);
    ///
    /// }
    /// ```
    pub async fn create<T>(self, item: T) -> Result<T, Error>
    where
        T: Sized + ToSql + FromSql,
    {
        let sql = format!(
            "INSERT INTO {table_name} ({fields}) values ({prepared_values}) RETURNING *",
            table_name = T::get_table_name(),
            fields = T::get_fields(),
            prepared_values = T::get_prepared_arguments_list(),
        );
        let insert = self.client.lock().prepare(sql.as_str());
        let insert = insert.await?;

        let result = { self.client.lock().query(&insert, &item.get_query_params()) };
        Ok(result
            .map_ok(|row| T::from_row(&row))
            .try_collect::<Vec<T>>()
            .await?
            .pop()
            .expect("at least it should return a row"))
    }

    ///
    /// Create new rows in the database.
    ///
    /// Example:
    /// ```
    /// use profugus::PGConnection;
    /// use tokio::prelude::*;
    /// use profugus::FromSql;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    ///     let new_products = vec!(
    ///         Product {prod_id: 0, title: String::from("Sql insert lesson")},
    ///         Product {prod_id: 0, title: String::from("Rust macro lesson")},
    ///         Product {prod_id: 0, title: String::from("Postgres data types lesson")};
    ///     );
    ///     let products = conn.create(new_product).await.unwrap();
    ///
    ///     assert_eq!(new_products, products);
    ///
    ///     conn.delete(products).await.unwrap();
    /// }
    /// ```
    pub async fn create_multiple<T>(self, items: Vec<T>) -> Result<Vec<T>, Error>
    where
        T: Sized + ToSql + FromSql,
    {
        let sql = format!(
            "INSERT INTO {table_name} ({fields}) values {prepared_values} RETURNING *",
            table_name = T::get_table_name(),
            fields = T::get_fields(),
            prepared_values =
                generate_prepared_arguments_list(T::get_argument_count(), items.len()),
        );
        let insert = self.client.lock().prepare(sql.as_str());
        let insert = insert.await?;

        let params: Vec<&dyn ToSqlItem> = items
            .iter()
            .map(|item| item.get_query_params())
            .flatten()
            .collect();
        let result = { self.client.lock().query(&insert, &params) };
        Ok(result
            .map_ok(|row| T::from_row(&row))
            .try_collect::<Vec<T>>()
            .await?)
    }

    ///
    /// Deletes a item.
    ///
    /// Example:
    /// ```
    /// use profugus::PGConnection;
    /// use tokio::prelude::*;
    /// use profugus::FromSql;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    ///     let new_product = Product {prod_id: 0, title: String::from("Sql insert lesson")};
    ///     let id = conn.create(new_product).await.unwrap().get_id();
    ///     let product = conn.query("SELECT prod_id, title from Products where prod_id = $1", &[id]);
    ///
    ///     assert_eq!(new_product, product);
    ///
    ///     conn.delete(product).await.unwrap();
    /// }
    /// ```
    pub async fn delete<T: traits::FromSql + traits::ToSql>(self, item: T) -> Result<T, Error>
    where
        <T as traits::ToSql>::PK: tokio_postgres::types::ToSql,
    {
        let sql = format!(
            "DELETE FROM {table_name} WHERE {primary_key} IN ($1) RETURNING *",
            table_name = T::get_table_name(),
            primary_key = T::get_primary_key()
        );
        let insert = self.client.lock().prepare(sql.as_str());
        let insert = insert.await?;

        let result = {
            self.client
                .lock()
                .query(&insert, &[&item.get_primary_key_value()])
        };
        Ok(result
            .map_ok(|row| T::from_row(&row))
            .try_collect::<Vec<T>>()
            .await?
            .pop()
            .expect("at least it should return a row"))
    }

    ///
    /// Deletes a list of items.
    ///
    /// Example:
    /// ```
    /// use profugus::PGConnection;
    /// use tokio::prelude::*;
    /// use profugus::FromSql;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    ///     let new_products = vec!(
    ///         Product {prod_id: 0, title: String::from("Sql insert lesson")},
    ///         Product {prod_id: 0, title: String::from("Rust macro lesson")},
    ///         Product {prod_id: 0, title: String::from("Postgres data types lesson")};
    ///     );
    ///     let id = conn.create(new_product).await.unwrap().get_id();
    ///     let products = conn.query_multiple("SELECT prod_id, title from Products where prod_id = $1", &[id]);
    ///
    ///     assert_eq!(new_products, products);
    ///
    ///     conn.delete(products).await.unwrap();
    /// }
    /// ```
    pub async fn delete_multiple<T: traits::FromSql + traits::ToSql>(
        self,
        items: Vec<T>,
    ) -> Result<Vec<T>, Error>
    where
        <T as traits::ToSql>::PK: tokio_postgres::types::ToSql + Sized,
    {
        //        let sql = format!(
        //            "DELETE FROM {table_name} WHERE {primary_key} IN ({argument_list}) RETURNING *",
        //            table_name = T::get_table_name(),
        //            primary_key = T::get_primary_key(),
        //            argument_list = generate_single_prepared_arguments_list(items.len())
        //        );
        //        let insert = self.client.lock().prepare(sql.as_str());
        //        let insert = insert.await?;
        //        // TODO: make this work:
        //        let params: Vec<_> = items
        //            .iter()
        //            .map(|item| &item.get_primary_key_value())
        //            .collect();
        //        let result = { self.client.lock().query(&insert, &params[..])};
        //        Ok(result
        //            .map_ok(|row| T::from_row(&row))
        //            .try_collect::<Vec<T>>()
        //            .await?)
        unimplemented!()
    }
}
///
/// Generates a string of prepared statement placeholder arguments.
///
fn generate_prepared_arguments_list(item_length: usize, no_of_items: usize) -> String {
    let mut arguments_list: String = String::new();
    let argument_num = item_length * no_of_items;
    let mut first: bool = true;

    for i in 1..argument_num + 1 {
        if (i - 1) % item_length == 0 {
            if first {
                first = false;
            } else {
                arguments_list.push_str("),");
            }
            arguments_list.push('(');
        } else {
            arguments_list.push(',');
        }
        arguments_list.push('$');
        arguments_list.push_str(&*i.to_string());
    }
    arguments_list.push(')');
    arguments_list
}

fn generate_single_prepared_arguments_list(no_of_items: usize) -> String {
    let mut arguments_list: String = String::new();
    arguments_list.push('(');
    for i in 1..no_of_items + 1 {
        arguments_list.push('$');
        arguments_list.push_str(&*i.to_string());
        match i == no_of_items {
            true => {}
            false => arguments_list.push(','),
        }
    }
    arguments_list.push(')');
    arguments_list
}
