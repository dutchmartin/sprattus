#![feature(associated_type_bounds)]
/// Profugus postgres orm
mod connection;
mod traits;

pub use self::connection::PGConnection;
pub use self::traits::{FromSql, ToSql};
pub use profugus_derive::{FromSql, ToSql};
pub use tokio_postgres::types::ToSql as ToSqlItem;
pub use tokio_postgres::{Error, Row};
