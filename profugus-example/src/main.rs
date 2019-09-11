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

#[derive(Identifiable, FromSql, Debug, Default)]
#[profugus(table = "count")]
struct Count {
    #[profugus(primary_key)]
    count: i64,
}
