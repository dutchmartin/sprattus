use tokio_postgres::Row;
use tokio_postgres::types::ToSql as PGToSql;

pub trait FromSql {
    ///
    /// Implementors of this method create an instance of Self with the content of a Row.
    ///
    fn from_row(row: &Row) -> Self;
}

pub trait ToSql {

    ///
    /// The name of the primary key.
    ///
    const PRIMARY_KEY: &'static str;

    ///
    /// The fields that contain the data of the table.
    /// The primary key is excluded from this list.
    ///
    const FIELDS : &'static [&'static str];

    ///
    /// The method that implements converting the fields
    /// into a array of items that implement the ToSql trait of rust_postgres.
    ///
    fn get_query_params(self) -> &[dyn PGToSql];

}
