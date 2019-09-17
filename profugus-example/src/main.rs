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
    let products = vec![
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
        },
    ];
    let product = Product {
        prod_id: 69,
        title: String::from("bla"),
    };

    let product: Product = conn.update(product).await.unwrap();
    dbg!(&product);

    //    let deleted = conn.delete_multiple(product).await.unwrap();
    //    dbg!(deleted);
}
