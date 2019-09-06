use tokio;
use profugus::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await?;
    let number_of_products : Vec<Count> = conn.query("select count(*) as count from products").await?;
    dbg!(number_of_products);
    Ok(())
}

#[derive(FromSql, Debug)]
struct Count {
    count: i64
}
