use crate::*;
use futures::{Stream, TryStreamExt};
use futures_util::future::FutureExt;
use futures_util::stream::StreamExt;
use futures_util::try_future::TryFutureExt;
use parking_lot::*;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use strfmt::strfmt;
use tokio;
use tokio_postgres::*;

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
        self.query_multiple_stream(sql, args)
            .await?
            .try_collect::<Vec<T>>()
            .await
    }
    //TODO: comments for explaination.
    pub async fn query_multiple_stream<T>(
        self,
        sql: &str,
        args: &[&dyn ToSqlItem],
    ) -> Result<impl Stream<Item = Result<T, Error>>, Error>
    where
        T: FromSql,
    {
        let statement = self.client.lock().prepare(sql).await?;
        let result = { self.client.lock().query(&statement, args) };
        Ok(result.map(|row_result| -> Result<T, Error> {
            match row_result {
                Ok(row) => T::from_row(&row),
                Err(e) => Err(e),
            }
        }))
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
        let mut boxed_future = self.query_multiple_stream(sql, args).await?.boxed();
        let mut pinned_fut = Pin::new(&mut boxed_future);
        Ok(pinned_fut
            .try_next()
            .await?
            .expect("expected at least one item"))
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
    pub async fn update<T: traits::FromSql + traits::ToSql>(self, item: T) -> Result<T, Error>
    where
        <T as traits::ToSql>::PK: tokio_postgres::types::ToSql,
    {
        // FIXME: change this to a const fn, see https://github.com/rust-lang/rust/issues/57563
        let sql_template = if T::get_prepared_arguments_list() == "$1" {
            "UPDATE {table_name} SET {fields} = {prepared_values} WHERE {primary_key} = $1 RETURNING *"
        } else {
            "UPDATE {table_name} SET ({fields}) = ({prepared_values}) WHERE {primary_key} = $1 RETURNING *"
        };
        let mut sql_vars = HashMap::with_capacity(12);
        sql_vars.insert(String::from("table_name"), T::get_table_name());
        sql_vars.insert(String::from("fields"), T::get_fields());
        sql_vars.insert(String::from("primary_key"), T::get_primary_key());
        let prepared_values =
            generate_single_prepared_arguments_list(2, T::get_argument_count() + 1);
        sql_vars.insert(String::from("prepared_values"), prepared_values.as_ref());
        let sql = strfmt(sql_template, &sql_vars).unwrap();

        let insert = self.client.lock().prepare(&sql);
        let insert = insert.await?;
        let result = {
            self.client
                .lock()
                .query(&insert, &item.get_values_of_all_fields())
        };
        let mut boxed_fut = result.boxed();
        let mut pinned_fut = Pin::new(&mut boxed_fut);
        pinned_fut
            .try_next()
            .map_ok(|row| T::from_row(&row.expect("At least it should return one row")))
            .await?
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
    pub async fn update_multiple<T>(self, items: Vec<T>) -> Result<Vec<T>, Error>
    where
        T: Sized + ToSql + FromSql,
    {
        // TODO: change this to a const fn, see https://github.com/rust-lang/rust/issues/57563
        let sql_template = if T::get_prepared_arguments_list() == "$1" {
            "UPDATE {table_name} AS P SET {fields} = temp_table.{inner_fields} FROM \
             (VALUES {prepared_placeholders}) as temp_table({all_fields}) \
             WHERE P.{primary_key} = temp_table.{primary_key} \
             RETURNING *"
        } else {
            "UPDATE {table_name} AS P SET ({fields}) = (temp_table.{inner_fields}) FROM \
             (VALUES {prepared_placeholders}) as temp_table({all_fields}) \
             WHERE P.{primary_key} = temp_table.{primary_key} \
             RETURNING *"
        };
        let placeholders = generate_prepared_arguments_list_with_types::<T>(
            T::get_argument_count() + 1,
            items.len(),
        );
        let inner_fields = T::get_fields().replace(",", ",temp_table");
        let mut sql_vars = HashMap::with_capacity(12);
        sql_vars.insert(String::from("table_name"), T::get_table_name());
        sql_vars.insert(String::from("inner_fields"), inner_fields.as_str());
        sql_vars.insert(String::from("fields"), T::get_fields());
        sql_vars.insert(String::from("primary_key"), T::get_primary_key());
        sql_vars.insert(String::from("all_fields"), T::get_all_fields());
        sql_vars.insert(String::from("prepared_placeholders"), placeholders.as_str());
        let sql = strfmt(sql_template, &sql_vars).unwrap();
        let insert = self.client.lock().prepare(&sql);
        let insert = insert.await?;
        let params: Vec<&dyn ToSqlItem> = items
            .iter()
            .map(|item| item.get_values_of_all_fields())
            .flatten()
            .collect();

        let result = { self.client.lock().query(&insert, &params) };
        Ok(result
            .map(|row_result| -> Result<T, Error> {
                match row_result {
                    Ok(row) => T::from_row(&row),
                    Err(e) => Err(e),
                }
            })
            .try_collect::<Vec<T>>()
            .await?)
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
        let mut boxed_fut = result.boxed();
        let mut pinned_fut = Pin::new(&mut boxed_fut);
        pinned_fut
            .try_next()
            .map_ok(|row| T::from_row(&row.expect("At least it should return one row")))
            .await?
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
            .map(|row_result| -> Result<T, Error> {
                match row_result {
                    Ok(row) => T::from_row(&row),
                    Err(e) => Err(e),
                }
            })
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
        <T as traits::ToSql>::PK: tokio_postgres::types::ToSql + Copy,
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
        let mut boxed_fut = result.boxed();
        let mut pinned_fut = Pin::new(&mut boxed_fut);
        pinned_fut
            .try_next()
            .map_ok(|row| T::from_row(&row.expect("At least it should return one row")))
            .await?
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
    pub async fn delete_multiple<P, T>(self, items: Vec<T>) -> Result<Vec<T>, Error>
    where
        P: tokio_postgres::types::ToSql + Copy,
        T: traits::FromSql + traits::ToSql<PK = P>,
        <T as traits::ToSql>::PK: Copy,
    {
        let sql = format!(
            "DELETE FROM {table_name} WHERE {primary_key} IN ({argument_list}) RETURNING *",
            table_name = T::get_table_name(),
            primary_key = T::get_primary_key(),
            argument_list = generate_single_prepared_arguments_list(1, items.len())
        );
        let insert = self.client.lock().prepare(sql.as_str());
        let insert = insert.await?;
        let params: Vec<P> = items
            .iter()
            .map(|item| item.get_primary_key_value())
            .collect();
        let p = params
            .iter()
            .map(|i| i as &dyn tokio_postgres::types::ToSql)
            .collect::<Vec<_>>();
        let result = { self.client.lock().query(&insert, p.as_slice()) };
        Ok(result
            .map(|row_result| -> Result<T, Error> {
                match row_result {
                    Ok(row) => T::from_row(&row),
                    Err(e) => Err(e),
                }
            })
            .try_collect::<Vec<T>>()
            .await?)
    }
}
///
/// Generates a string of prepared statement placeholder arguments.
///
fn generate_prepared_arguments_list(item_length: usize, no_of_items: usize) -> String {
    let mut arguments_list: String = String::new();
    let range_end = item_length * no_of_items + 1;

    complete_prepared_arguments_list(&mut arguments_list, 1, range_end, item_length);
    arguments_list
}

fn generate_prepared_arguments_list_with_types<T>(item_length: usize, no_of_items: usize) -> String
where
    T: ToSql,
{
    let mut arguments_list: String =
        format!("({})", T::get_prepared_arguments_list_with_types());
    if no_of_items == 1 {
        return arguments_list;
    }
    let range_end = item_length * no_of_items + 1;
    arguments_list.push(',');
    complete_prepared_arguments_list(&mut arguments_list, item_length + 1, range_end, item_length);
    arguments_list
}

fn complete_prepared_arguments_list(
    arguments_list: &mut String,
    range_start: usize,
    range_end: usize,
    item_length: usize,
) {
    let mut first: bool = true;

    for i in range_start..range_end {
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
}

fn generate_single_prepared_arguments_list(start_num: usize, end_num: usize) -> String {
    let mut arguments_list: String = String::new();
    for i in start_num..=end_num {
        arguments_list.push('$');
        arguments_list.push_str(&*i.to_string());
        if i != end_num {
            arguments_list.push(',');
        }
    }
    arguments_list
}
