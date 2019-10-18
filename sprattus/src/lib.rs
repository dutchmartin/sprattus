//! A crate for easy Postgres database communication.
//!
//! # Getting started
//! 
//! Add sprattus to your cargo.toml:  
//! ```toml
//! sprattus = "0.1"
//! ```
//! Create a table in Postgres:
//! ```sql
//! CREATE TABLE fruits(
//!    id SERIAL PRIMARY KEY,
//!    name VARCHAR NOT NULL
//! );
//! ```
//! 
//! Create a struct corresponding to the created table:
//! ```rust
//! struct Fruit {
//!     id: i32,
//!     name: String
//! }
//! ```
//! And finally add the sprattus macro's and annotations:
//! ```rust
//! use sprattus::*;
//! 
//! #[derive(ToSql, FromSql, Debug)]
//! #[sprattus(table = "fruits")]
//! struct Fruit {
//!     #[sprattus(primary_key)]
//!     id: i32,
//!     name: String
//! }
//! ```
//! And now your ready to use the client in combination with you freshly created struct!
//! 
//! ```rust
//! use tokio::prelude::*;
//! use sprattus::*;
//! 
//! #[derive(ToSql, FromSql)]
//! #[sprattus(table = "fruits")]
//! struct Fruit {
//!     #[sprattus(primary_key)]
//!     id: i32,
//!     name: String
//! }
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Error>{
//!     let conn = Connection::new("postgresql://! localhost/dellstore2?user=tg").await?;
//!     let fruit = Fruit{
//!         id: 0,
//!         name: String::from("apple")
//!     };
//!     // Insert created fruit into the database.
//!     let created_fruit = conn.create(fruit).await?;
//!     dbg!(created_fruit);
//!     Ok(())
//! }
//! ```
//!
//! # Types
//! The following Rust types are provided by this crate for use in user created Rust structs, along with the
//! corresponding Postgres types:
//!
//! | Rust type                         | Postgres type(s)                              |
//! |-----------------------------------|-----------------------------------------------|
//! | `bool`                            | BOOL                                          |
//! | `i8`                              | "char"                                        |
//! | `i16`                             | SMALLINT, SMALLSERIAL                         |
//! | `i32`                             | INT, SERIAL                                   |
//! | `u32`                             | OID                                           |
//! | `i64`                             | BIGINT, BIGSERIAL                             |
//! | `f32`                             | REAL                                          |
//! | `f64`                             | DOUBLE PRECISION                              |
//! | `&str`/`String`                   | VARCHAR, CHAR(n), TEXT, CITEXT, NAME, UNKNOWN |
//! | `&[u8]`/`Vec<u8>`                 | BYTEA                                         |
//! | `HashMap<String, Option<String>>` | HSTORE                                        |
//! | `SystemTime`                      | TIMESTAMP, TIMESTAMP WITH TIME ZONE           |
//! | `IpAddr`                          | INET                                          |
//!
//! In addition, some implementations are provided for types in third party
//! crates. These are disabled by default; to opt into one of these
//! implementations, activate the Cargo feature corresponding to the crate's
//! name prefixed by `with-`. For example, the `with-serde_json-1` feature enables
//! the implementation for the `serde_json::Value` type.
//!
//! | Rust type                       | Postgres type(s)                    |
//! |---------------------------------|-------------------------------------|
//! | `chrono::NaiveDateTime`         | TIMESTAMP                           |
//! | `chrono::DateTime<Utc>`         | TIMESTAMP WITH TIME ZONE            |
//! | `chrono::DateTime<Local>`       | TIMESTAMP WITH TIME ZONE            |
//! | `chrono::DateTime<FixedOffset>` | TIMESTAMP WITH TIME ZONE            |
//! | `chrono::NaiveDate`             | DATE                                |
//! | `chrono::NaiveTime`             | TIME                                |
//! | `eui48::MacAddress`             | MACADDR                             |
//! | `geo_types::Point<f64>`         | POINT                               |
//! | `geo_types::Rect<f64>`          | BOX                                 |
//! | `geo_types::LineString<f64>`    | PATH                                |
//! | `serde_json::Value`             | JSON, JSONB                         |
//! | `uuid::Uuid`                    | UUID                                |
//! | `bit_vec::BitVec`               | BIT, VARBIT                         |
//! | `eui48::MacAddress`             | MACADDR                             |
//!
//! ### Nullability
//!
//! In addition to the types listed above, `FromSqlItem` is implemented for
//! `Option<T>` where `T` implements `FromSqlItem`. An `Option<T>` represents a
//! nullable Postgres value.
//!
//! # Annotations
//!
//! On user created structs, there are several options configurable by using annotiations.
//! ### Renaming fields
//! In any case of having not the same name for a field in the database and in Rust, use the rename annotation.
//! ```
//! struct Product {
//!     #[sprattus(primary_key)]
//!     id: i32,
//!     name: String,
//!     // Renames the postgres field 'product_price' to costs.
//!     #[sprattus(name = "product_price")]
//!     costs: f64
//! }
//! ```
//! ### Selecting a primary key
//! Every struct that wants to use the `ToSql` derive macro needs to have a primary key.
//! Therefore there is a annotion available for that.
//! ```
//! struct User {
//!     // Annotates id as primary key of the table.
//!     #[sprattus(primary_key)]
//!     id: i32,
//!     name: String,
//! }
//! ```
//! ### Selecting a database table
//! In many cases, the name of your Rust struct will not correspond with the table in Postgres.
//! To solve that problem, there is a attribute to select the table belonging to the created struct:
//! ```rust
//! // This tells sprattus to use the 'houses' table in Postgres.
//! #[sprattus(table = "houses")]
//! struct House {
//!     id: i32,
//!     address: String,
//!     city: String,
//!     country: String,
//! }
//! ```

mod connection;
mod traits;

pub use self::connection::Connection;
pub use self::traits::{FromSql, ToSql};
pub use sprattus_derive::{FromSql, ToSql};
pub use tokio_postgres::types::ToSql as ToSqlItem;
pub use tokio_postgres::{Error, Row};
