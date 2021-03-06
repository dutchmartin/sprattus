use crate::*;
use futures_util::future::FutureExt;
use futures_util::future::TryFutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use strfmt::strfmt;
use tokio;
use tokio_postgres::*;

/// Client for Postgres database manipulation.
///
///
#[derive(Clone)]
pub struct Connection {
    client: Arc<Client>,
}

impl Connection {
    ///
    /// Creates a new connection to the database.
    ///
    /// Example:
    /// ```no_run
    /// use sprattus::*;
    ///
    ///# #[tokio::main]
    ///# async fn main() -> Result<(), Error> {
    /// let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///# return Ok(())
    ///# }
    /// ```
    pub async fn new(connection_string: &str) -> Result<Self, Error> {
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await?;

        let connection = connection
            .map_err(|e| panic!("connection error: {}", e))
            .map(|conn| conn.unwrap());
        tokio::spawn(connection);
        Ok(Self {
            client: Arc::new(client),
        })
    }
    /// Executes a statement, returning the number of rows modified.
    ///
    /// If the statement does not modify any rows (e.g. `SELECT`), 0 is returned.
    ///
    /// # Panics
    ///
    /// Panics if the number of parameters provided does not match the number expected.
    pub async fn execute(&self, sql: &str, args: &[&(dyn ToSqlItem + Sync)]) -> Result<u64, Error> {
        let client = &self.client;
        client.execute(sql, args).await
    }

    /// Executes a sequence of SQL statements using the simple query protocol.
    ///
    /// Statements should be separated by semicolons. If an error occurs, execution of the sequence will stop at that
    /// point. This is intended for use when, for example, initializing a database schema.
    ///
    /// # Warning
    ///
    /// Prepared statements should be use for any query which contains user-specified data, as they provided the
    /// functionality to safely embed that data in the request. Do not form statements via string concatenation and pass
    /// them to this method!
    pub async fn batch_execute(&self, sql: &str) -> Result<(), Error> {
        let client = &self.client;
        let result = { client.batch_execute(&sql) };
        result.await
    }

    ///
    /// Query multiple rows of a table.
    ///
    /// Example:
    /// ```no_run
    ///# use sprattus::*;
    ///# use tokio::prelude::*;
    ///#
    ///# #[derive(FromSql, Eq, PartialEq, Debug)]
    ///# struct Product {
    ///#     #[sql(primary_key)]
    ///#     prod_id: i32,
    ///#     title: String
    ///# }
    ///# #[tokio::main]
    ///# async fn main() -> Result<(), Error> {
    ///#
    /// let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///#
    ///#
    ///#
    /// let product_list : Vec<Product> =
    ///    conn.query_multiple("SELECT * FROM Products LIMIT 3", &[]).await?;
    /// assert_eq!(product_list,
    ///     vec!(
    ///    Product {
    ///	    prod_id : 1,
    ///	    title : String::from("ACADEMY ACADEMY")
    ///    },
    ///	Product {
    ///	   prod_id : 2,
    ///	   title : String::from("ACADEMY ACE")
    ///    },
    ///	Product {
    ///	    prod_id : 3,
    ///	    title : String::from("ACADEMY ADAPTATION")
    ///	}));
    ///# Ok(())
    ///# }
    /// ```
    pub async fn query_multiple<T>(
        &self,
        sql: &str,
        args: &[&(dyn ToSqlItem + Sync)],
    ) -> Result<Vec<T>, Error>
    where
        T: FromSql,
    {
        self.client
            .query(sql, args)
            .map(|rows| rows?.iter().map(|row| T::from_row(row)).collect())
            .await
    }

    ///
    /// Get a single row of a table.
    ///
    /// Example:
    /// ```no_run
    /// use sprattus::*;
    /// use tokio::prelude::*;
    ///
    /// #[derive(FromSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     #[sql(primary_key)]
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///     let product : Product = conn.query("SELECT * FROM Products LIMIT 1", &[]).await?;
    ///     assert_eq!(product, Product{ prod_id: 1, title: String::from("ACADEMY ACADEMY")});
    ///     Ok(())
    /// }
    /// ```
    pub async fn query<T>(&self, sql: &str, args: &[&(dyn ToSqlItem + Sync)]) -> Result<T, Error>
    where
        T: FromSql,
    {
        let client = &self.client;
        T::from_row(&client.query_one(sql, args).await?)
    }

    ///
    /// Update a single rust value in the database.
    ///
    /// Example:
    /// ```no_run
    /// use sprattus::*;
    /// use tokio::prelude::*;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     #[sql(primary_key)]
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///     // Change a existing record in the database.
    ///     conn.update(&Product { prod_id : 50, title: String::from("Rust ORM")}).await?;
    ///
    ///     let product : Product = conn.query("SELECT * FROM Products where prod_id = 50", &[]).await?;
    ///     assert_eq!(product, Product{ prod_id: 50, title: String::from("Rust ORM")});
    ///     // Change it back to it's original value.
    ///     conn.update(&Product { prod_id : 50, title: String::from("ACADEMY BAKED")}).await?;
    ///
    ///     let product : Product = conn.query("SELECT * FROM Products where prod_id = 50", &[]).await?;
    ///     assert_eq!(product, Product{ prod_id: 50, title: String::from("ACADEMY BAKED")});
    ///     Ok(())
    /// }
    /// ```
    pub async fn update<T: traits::FromSql + traits::ToSql>(&self, item: &T) -> Result<T, Error>
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
        let client = &self.client;

        T::from_row(
            &client
                .query_one(sql.as_str(), item.get_values_of_all_fields().as_slice())
                .await?,
        )
    }

    ///
    /// Update multiple rust values in the database.
    ///
    /// Example:
    /// ```no_run
    /// use sprattus::*;
    /// use tokio::prelude::*;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     #[sql(primary_key)]
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///     let new_products = vec!(
    ///             Product{ prod_id: 60, title: String::from("Rust ACADEMY") },
    ///             Product{ prod_id: 61, title: String::from("SQL ACADEMY") },
    ///             Product{ prod_id: 62, title: String::from("Backend development training") },
    ///         );
    ///     // Change a existing record in the database.
    ///     conn.update_multiple(&new_products).await?;
    ///     let sql = "SELECT * FROM Products where prod_id in (60, 61, 62)";
    ///     let products: Vec<Product> = conn.query_multiple(sql, &[]).await?;
    ///     assert_eq!(products, new_products);
    ///     Ok(())
    /// }
    /// ```
    pub async fn update_multiple<T>(&self, items: &[T]) -> Result<Vec<T>, Error>
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
        let inner_fields = T::get_fields().replace(",", ",temp_table.");
        let mut sql_vars = HashMap::with_capacity(12);
        sql_vars.insert(String::from("table_name"), T::get_table_name());
        sql_vars.insert(String::from("inner_fields"), inner_fields.as_str());
        sql_vars.insert(String::from("fields"), T::get_fields());
        sql_vars.insert(String::from("primary_key"), T::get_primary_key());
        sql_vars.insert(String::from("all_fields"), T::get_all_fields());
        sql_vars.insert(String::from("prepared_placeholders"), placeholders.as_str());
        let sql = strfmt(sql_template, &sql_vars).unwrap();
        let params: Vec<&(dyn ToSqlItem + Sync)> = items
            .iter()
            .map(|item| item.get_values_of_all_fields())
            .flatten()
            .collect();
        let client = &self.client;
        client
            .query(sql.as_str(), params.as_slice())
            .map(|rows| rows?.iter().map(|row| T::from_row(row)).collect())
            .await
    }

    ///
    /// Create a new row in the database.
    ///
    /// Example:
    /// ```no_run
    /// use sprattus::*;
    /// use tokio::prelude::*;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     #[sql(primary_key)]
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///     let new_product = Product {prod_id: 0, title: String::from("Sql insert lesson")};
    ///     let product = conn.create(&new_product).await?;
    ///
    ///     assert_eq!(new_product, product);
    ///     Ok(())
    /// }
    /// ```
    pub async fn create<T>(&self, item: &T) -> Result<T, Error>
    where
        T: Sized + ToSql + FromSql,
    {
        let sql = format!(
            "INSERT INTO {table_name} ({fields}) values ({prepared_values}) RETURNING *",
            table_name = T::get_table_name(),
            fields = T::get_fields(),
            prepared_values = T::get_prepared_arguments_list(),
        );
        let client = &self.client;

        T::from_row(
            &client
                .query_one(sql.as_str(), item.get_query_params().as_slice())
                .await?,
        )
    }

    ///
    /// Create new rows in the database.
    ///
    /// Example:
    /// ```no_run
    /// use sprattus::*;
    /// use tokio::prelude::*;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     #[sql(primary_key)]
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///     let new_products = vec!(
    ///         Product {prod_id: 0, title: String::from("Sql insert lesson")},
    ///         Product {prod_id: 0, title: String::from("Rust macro lesson")},
    ///         Product {prod_id: 0, title: String::from("Postgres data types lesson")}
    ///     );
    ///     let products = conn.create_multiple(&new_products).await?;
    ///
    ///     assert_eq!(&new_products, &products);
    ///
    ///     conn.delete_multiple(&products).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_multiple<T>(&self, items: &[T]) -> Result<Vec<T>, Error>
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

        let params: Vec<&(dyn ToSqlItem + Sync)> = items
            .iter()
            .map(|item| item.get_query_params())
            .flatten()
            .collect();
        let client = &self.client;
        client
            .query(sql.as_str(), params.as_slice())
            .map(|rows| rows?.iter().map(|row| T::from_row(row)).collect())
            .await
    }

    ///
    /// Deletes a item.
    ///
    /// Example:
    /// ```no_run
    /// use sprattus::*;
    /// use tokio::prelude::*;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     #[sql(primary_key)]
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///
    ///     let new_product = Product {prod_id: 0, title: String::from("Sql insert lesson")};
    ///     let product = conn.create(&new_product).await?;
    ///     let deleted_product = conn.delete(&product).await?;
    ///
    ///     assert_eq!(&product, &deleted_product);
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete<T: traits::FromSql + traits::ToSql>(&self, item: &T) -> Result<T, Error>
    where
        <T as traits::ToSql>::PK: tokio_postgres::types::ToSql + Sync,
    {
        let sql = format!(
            "DELETE FROM {table_name} WHERE {primary_key} IN ($1) RETURNING *",
            table_name = T::get_table_name(),
            primary_key = T::get_primary_key()
        );
        let client = &self.client;
        T::from_row(
            &client
                .query_one(sql.as_str(), &[&item.get_primary_key_value()])
                .await?,
        )
    }

    ///
    /// Deletes a list of items.
    ///
    /// Example:
    /// ```no_run
    /// use sprattus::*;
    /// use tokio::prelude::*;
    ///
    /// #[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
    /// struct Product {
    ///     #[sql(primary_key)]
    ///     prod_id: i32,
    ///     title: String
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Error> {
    ///     let conn = Connection::new("postgresql://localhost?user=tg").await?;
    ///     let new_products = vec!(
    ///         Product {prod_id: 0, title: String::from("Sql insert lesson")},
    ///         Product {prod_id: 0, title: String::from("Rust macro lesson")},
    ///         Product {prod_id: 0, title: String::from("Postgres data types lesson")}
    ///     );
    ///     let created_products = conn.create_multiple(&new_products).await?;
    ///
    ///     let deleted_products = conn.delete_multiple(&created_products).await?;
    ///     assert_eq!(&created_products, &deleted_products);
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_multiple<P, T>(&self, items: &[T]) -> Result<Vec<T>, Error>
    where
        P: tokio_postgres::types::ToSql,
        T: traits::FromSql + traits::ToSql<PK = P>,
        <T as traits::ToSql>::PK: Sync,
    {
        let sql = format!(
            "DELETE FROM {table_name} WHERE {primary_key} IN ({argument_list}) RETURNING *",
            table_name = T::get_table_name(),
            primary_key = T::get_primary_key(),
            argument_list = generate_single_prepared_arguments_list(1, items.len())
        );
        let params: Vec<P> = items
            .iter()
            .map(|item| item.get_primary_key_value())
            .collect();
        let p = params
            .iter()
            .map(|i| i as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect::<Vec<_>>();
        let client = &self.client;
        client
            .query(sql.as_str(), p.as_slice())
            .map(|rows| rows?.iter().map(|row| T::from_row(row)).collect())
            .await
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
    let mut arguments_list: String = format!("({})", T::get_prepared_arguments_list_with_types());
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
