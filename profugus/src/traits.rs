use std::sync::Arc;
use tokio_postgres::types::ToSql as ToSqlItem;
use tokio_postgres::Row;

pub trait FromSql {
    ///
    /// Implementors of this method create an instance of Self with the content of a Row.
    ///
    fn from_row(row: &Row) -> Self;
}

pub trait ToSql {
    ///
    /// Returns the name of the table.
    ///
    fn get_table_name() -> &'static str;
    ///
    /// Returns the name of the primary key.
    ///
    fn get_primary_key() -> &'static str;

    ///
    /// The fields that contain the data of the table.
    /// The primary key is excluded from this list.
    ///
    fn get_fields() -> &'static [&'static str];

    ///
    /// The method that implements converting the fields
    /// into a array of items that implement the ToSql trait of rust_postgres.
    ///
    fn get_query_params(self) -> Arc<[Box<dyn ToSqlItem>]>;

    ///
    /// Returns the formatted prepared statement list.
    /// Example: "$1, $2"
    ///
    fn get_prepared_arguments_list() -> &'static str;
}
