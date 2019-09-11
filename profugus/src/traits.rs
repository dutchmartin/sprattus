use tokio_postgres::Row;

pub trait FromSql {
    fn from_row(row: &Row) -> Self;
}

pub trait Identifiable {
    fn get_primary_key() -> &'static str;
}
