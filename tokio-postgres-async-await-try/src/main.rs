use std::error::Error;
use std::io::{Error as OperationError, ErrorKind};
use tokio::prelude::*;
use tokio_postgres::{
    NoTls,
    Client,
    row::Row,
};
use futures_util::try_future::TryFutureExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (mut client, mut connection) = tokio_postgres::connect("postgresql://localhost/dellstore2?user=tg", NoTls).await?;

    connection.map_err(|e| eprintln!("connection error: {}", e));

    async move {
        get_categories(&mut client);
    };

    Ok(())
}

async fn get_categories(client: &mut Client) {
    // Now we can prepare a simple statement that just returns its parameter.
    let statement = client.prepare("SELECT categoryname from categories")

        .and_then(|statement|
            client.query(&statement, &[])
        ).collect()
        .and_then(|row|
            return row.get(0)
        ).map_err(|e| eprintln!("connection error: {}", e));

    let category: String = row.get(0);
    category
        ()
}

async fn get_categories_2(client: &mut Client) {
    // Now we can prepare a simple statement that just returns its parameter.
    client.prepare("SELECT $1::TEXT")
        .map(|statement| (client, statement))

        .and_then(|(mut client, statement)| {
            // And then execute it, returning a Stream of Rows which we collect into a Vec
            client.query(&statement, &[&"hello world"]).collect()
        })

        // Now we can check that we got back the same string we sent over.
        .map(|rows| {
            let value: &str = rows[0].get(0);
        })

        // And report any errors that happened.
        .map_err(|e| {
            eprintln!("error: {}", e);
        });
}
