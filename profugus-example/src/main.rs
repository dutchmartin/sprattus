#![feature(custom_attribute)]
use profugus::PGConnection;
use profugus::*;

#[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
struct Product {
    #[profugus(primary_key)]
    prod_id: i32,
    title: String,
}

#[tokio::main]
async fn main() {
    let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg")
        .await
        .unwrap();
    let new_products = Vec!(
        Product {
            prod_id: 0,
            title: String::from("Sql insert lesson"),
        },
        Product {
            prod_id: 0,
            title: String::from("something"),
        },
        Product {
            prod_id: 0,
            title: String::from("bla"),
        });

    let product: vec<Product> = conn.create_multiple(new_product).await.unwrap();
    dbg!(product);
}
