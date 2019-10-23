Sprattus, a async Rust ORM for Postgres
================
[![Build Status](https://api.travis-ci.com/dutchmartin/sprattus.svg?branch=master)](https://travis-ci.com/dutchmartin/sprattus)

Sprattus is a crate that let's you easily do async CRUD operations on your Postgres database with Rust structs.

## Getting started

Add sprattus to your cargo.toml:  
```toml
sprattus = "0.0.1"
```
Create a table in Postgres:
```sql
CREATE TABLE fruits(
   id SERIAL PRIMARY KEY,
   name VARCHAR NOT NULL
);
```

Create a struct corresponding to the created table:
```rust
struct Fruit {
    id: i32,
    name: String
}
```
And finally add the sprattus macro's and annotations:
```rust
use sprattus::*;

#[derive(ToSql, FromSql, Debug)]
#[sql(table = "fruits")]
struct Fruit {
    #[sql(primary_key)]
    id: i32,
    name: String
}
```
And now you're ready to use the client in combination with you freshly created struct!

```rust
use tokio::prelude::*;
use sprattus::*;

#[derive(ToSql, FromSql)]
#[sql(table = "fruits")]
struct Fruit {
    #[sql(primary_key)]
    id: i32,
    name: String
}

#[tokio::main]
async fn main() -> Result<(), Error>{
    let conn = PGConnection::new("postgresql://localhost?user=postgres").await?;
    let fruit = Fruit {
        id: 0,
        name: String::from("apple")
    };
    let created_fruit = conn.create(fruit).await?;
    dbg!(created_fruit);
    Ok(())
}
```

Please check out the [docs](https://docs.rs/sprattus) for further reference.

## name
The name sprattus is the genus of the fish named [sprat](https://en.wikipedia.org/wiki/Sprat). It is a fitting name because of the schooling behavour of the sprat: 
> Sprats travel asynchronous from each other in large schools with other fish and swim continuously throughout the day

