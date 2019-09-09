use profugus::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg").await?;
    let number_of_products: Vec<Count> = conn
        .query_multiple("select count(*) as count from products", &[])
        .await?;
    dbg!(number_of_products);
    Ok(())
}

#[derive(FromSql, Debug, Default)]
struct Count {
    count: i64,
}
