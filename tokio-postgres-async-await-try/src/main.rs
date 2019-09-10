use bigdecimal::*;
use futures_util::try_future::TryFutureExt;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::prelude::*;
use tokio_postgres::{row::Row, Client, NoTls, Statement};

mod models;

use crate::models::*;
use futures::TryStreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (mut client, connection) =
        tokio_postgres::connect("postgresql://localhost/dellstore2?user=tg", NoTls).await?;
    let connection = connection
        .map_err(|e| eprintln!("connection error: {}", e))
        .map(|result| result.unwrap());
    tokio::spawn(connection);
    let client_ref = Arc::new(Mutex::new(client));

    let statement = prepare_statement(
        client_ref.clone(),
        "SELECT * FROM products ORDER BY prod_id DESC limit 5",
    )
    .await;

    let result = get_products(client_ref.clone(), statement.unwrap()).await;
    dbg!(result);
    Ok(())
}

async fn prepare_statement(
    client_ref: Arc<Mutex<Client>>,
    statement: &str,
) -> Box<Result<Statement, tokio_postgres::Error>> {
    match client_ref
        .lock()
        .unwrap()
        .prepare("SELECT * FROM products ORDER BY prod_id DESC limit 5")
        .await
    {
        Ok(Statement) => {
            return Box::new(Ok(Statement));
        }
        Err(e) => {
            return Box::new(Err(e));
        }
    }
}

async fn get_products(
    client_ref: Arc<Mutex<Client>>,
    statement: Statement,
) -> Result<Vec<Product>, tokio_postgres::Error> {
    client_ref
        .lock()
        .unwrap()
        .query(&statement, &[]) // returns a stream<Item = Result<Row, tokio_postgres::Error>>
        .map_ok(|row: Row| Product {
            id: row.get(0),
            category: row.get(1),
            title: row.get(2),
            actor: row.get(3),
            price: BigDecimal::from(0),
            special: row.get(5),
            common_prod_id: row.get(6),
        })
        .try_collect::<Vec<_>>()
        .await
}
