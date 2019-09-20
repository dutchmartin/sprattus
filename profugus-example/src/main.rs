#![feature(custom_attribute)]
use profugus::PGConnection;
use profugus::*;

use futures::TryStreamExt;

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
            prod_id: 1,
            title: String::from("Sql insert lesson"),
        },
        Product {
            prod_id: 2,
            title: String::from("my little pony"),
        },
        Product {
            prod_id: 3,
            title: String::from("sheep scissors"),
        },
    ];
    let product = Product {
        prod_id: 2,
        title: String::from("boom-box"),
    };

    //    let product: Vec<Product> = conn.create_multiple(products).await.unwrap();
    //    dbg!(&product);
    let product: Vec<Product> = conn
        .query_multiple_stream("SELECT * from Products limit 10", &[])
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    dbg!(&product);

    //    let deleted = conn.delete_multiple(product).await.unwrap();
    //    dbg!(deleted);
}
