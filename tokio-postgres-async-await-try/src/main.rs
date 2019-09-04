use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::prelude::*;
use tokio_postgres::{NoTls, Client, row::Row, Statement};
use futures_util::try_future::TryFutureExt;
use bigdecimal::*;

mod models;

use crate::models::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (mut client, connection) = tokio_postgres::connect("postgresql://localhost/dellstore2?user=tg", NoTls).await?;
    let connection = connection.map_err(|e| eprintln!("connection error: {}", e)).await;
    let client_ref = Arc::new(Mutex::new(client));
    let statement = prepare_statement(client_ref.clone(), "SELECT * FROM products ORDER BY prod_id DESC limit 5").await;
    let result = get_products(client_ref.clone(), statement.unwrap()).await;

    Ok(())
}

async fn prepare_statement(client_ref: Arc<Mutex<Client>>, statement: &str) -> Box<Result<Statement, tokio_postgres::Error>> {
    match client_ref
        .lock()
        .unwrap()
        .prepare("SELECT * FROM products ORDER BY prod_id DESC limit 5").await {
        Ok(Statement) => {
            return Box::new(Ok(Statement));
        }
        Err(e) => {
            return Box::new(Err(e));
        }
    }
}

async fn get_products(client_ref: Arc<Mutex<Client>>, statement: Statement) -> Box<Vec<Option<Product>>> {
    client_ref
        .lock()
        .unwrap()
        .query(&statement, &[]) // returns a stream<Item = Result<Row, tokio_postgres::Error>>
        .collect()
        .map(|row_result: Result<Row, tokio_postgres::Error> /*type interference breaks here*/| {
            // Could be https://github.com/rust-lang-nursery/futures-rs/issues/1833
            match row_result {
                Ok(row) => {
                    Some(Product {
                        id: row.get(0),
                        category: row.get(1),
                        title: row.get(2),
                        actor: row.get(3),
                        price: BigDecimal::from(0),
                        special: row.get(5),
                        common_prod_id: row.get(6),
                    })
                }
                Err(e) => {
                    eprintln!("Error in getting a item: {}", e);
                    None
                }
            }
        }).collect()
}