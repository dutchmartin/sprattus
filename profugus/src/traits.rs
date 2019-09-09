use tokio_postgres::Row;

pub trait FromSql {
    fn from_row(row: &Row) -> Self;
}