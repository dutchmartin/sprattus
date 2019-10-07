# profugus, the async Rust ORM for Postgres
###### The Future .awaits

profugus is a crate that let's you easily do async CRUD operations on your Postgres database with Rust structs.


### Getting started

Add profugus to your cargo.toml:  
```toml
profugus = "0.1"
```
Create a table in Postgres:
```postgresql
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
And finally add the profugus macro's and annotations:
```rust
use profugus::*;

#[derive(ToSql, FromSql, Debug)]
#[profugus(table = "fruits")]
struct Fruit {
    #[profugus(primary_key)]
    id: i32,
    name: String
}
```
And now your ready to use the client in combination with you freshly created struct!

```rust
use tokio::prelude::*;
use profugus::*;

#[derive(ToSql, FromSql)]
#[profugus(table = "fruits")]
struct Fruit {
    #[profugus(primary_key)]
    id: i32,
    name: String
}

#[tokio::main]
async fn main() -> Result<(), Error>{
    let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await.unwrap();
    let fruit = Fruit{
        id: 0,
        name: String::from("apple")
    };
    let created_fruit = conn.create(fruit).await?;
    dbg!(created_fruit);
    Ok(())
}
```

Please check out the [docs](https://docs.rs) for further reference.