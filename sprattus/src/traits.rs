use tokio_postgres::types::ToSql as ToSqlItem;
use tokio_postgres::{Error, Row};


/// Arranges deserialization from Postgres table values to a Rust struct.
pub trait FromSql {
    ///
    /// Implementors of this method create an instance of Self with the content of a Row.
    ///
    fn from_row(row: &Row) -> Result<Self, Error>
    where
        Self: Sized;
}

/// All required methods to create, update and delete the struct it's implemented for.
pub trait ToSql {
    ///
    /// Returns the name of the table.
    ///
    fn get_table_name() -> &'static str;
    ///
    /// Returns the Postgres name of the primary key.
    ///
    fn get_primary_key() -> &'static str;

    /// Represents the Rust type of the primary key.
    type PK;

    /// Returns the value of the primary key.
    fn get_primary_key_value(&self) -> Self::PK
    where
        Self::PK: ToSqlItem + Sized + Sync;

    ///
    /// The fields that contain the data of the table.
    /// The primary key is excluded from this list.
    ///
    fn get_fields() -> &'static str;

    /// Returns a comma separated list with the Postgres names of all fields.
    fn get_all_fields() -> &'static str;

    /// Returns a vector of references to all values of the implemented struct.
    fn get_values_of_all_fields(&self) -> Vec<&(dyn ToSqlItem + Sync)>;

    ///
    /// The method that implements converting the fields
    /// into a array of items that implement the ToSql trait of rust_postgres.
    ///
    fn get_query_params(&self) -> Vec<&(dyn ToSqlItem + Sync)>;

    ///
    /// Returns the formatted prepared statement list.
    ///
    /// Example return value: `$1, $2`
    ///
    fn get_prepared_arguments_list() -> &'static str;

    ///
    /// Returns the formatted prepared statement list with Postgres types.
    ///
    /// Example return value: `$1::INT, $2::VARCHAR`
    ///
    fn get_prepared_arguments_list_with_types() -> &'static str;

    /// Returns the amount of fields excluding the primary key.
    fn get_argument_count() -> usize;
}
