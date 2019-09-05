use futures::{
    future::Future,
    stream::Stream
};
use tokio;
use profugus::*;

fn main() -> Result<(), Error> {
    let mut runtime = tokio::runtime::Runtime::new().expect("Unable to create a runtime");
    let conn = runtime.block_on(PGConnection::new("postgresql://localhost/dellstore2?user=tg"))?;
    let number_of_products : Vec<Count> = runtime.block_on(conn.query("select count(id) as count from products"))?;
    dbg!(number_of_products);
    Ok(())
}

#[derive(FromSql, Debug)]
struct Count {
    count: i64
}