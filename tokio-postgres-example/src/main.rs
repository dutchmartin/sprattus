use futures::future::Future;
use futures::stream::Stream;
use tokio_postgres::NoTls;
use std::error::Error;

fn main() {
    let fut =
        // Connect to the database
        tokio_postgres::connect("postgresql://localhost/dellstore2?user=tg", NoTls)

            .map(|(client, connection)| {
                // The connection object performs the actual communication with the database,
                // so spawn it off to run on its own.
                let connection = connection.map_err(|e| eprintln!("connection error: {}", e));
                tokio::spawn(connection);

                // The client is what you use to make requests.
                client
            })

            .and_then(|mut client| {
                // Now we can prepare a simple statement that just returns its parameter.
                client.prepare("SELECT categoryname from categories")
                    .map(|statement| (client, statement))
            })

            .and_then(|(mut client, statement)| {
                // And then execute it, returning a Stream of Rows which we collect into a Vec
                client.query(&statement, &[]).collect()
            })

            // Now we can check that we got back the same string we sent over.
            .map(|rows| {
                for (i, row) in rows.iter().enumerate() {
                    let value: String = row.get("categoryname");
                    println!("{}| {}", i+1, value)
                };
            })

            // And report any errors that happened.
            .map_err(|e| {
                eprintln!("error: {}", e);
            });
    // By default, tokio_postgres uses the tokio crate as its runtime.
    tokio::run(fut);
}

//async fn getCategories() -> Result<Vec<String>, Box<Error>>
//{
//    tokio_postgres::connect("postgresql://localhost/dellstore2?user=tg", NoTls)
//
//        .map(|(client, connection)| {
//            // The connection object performs the actual communication with the database,
//            // so spawn it off to run on its own.
//            let connection = connection.map_err(|e| eprintln!("connection error: {}", e));
//            tokio::spawn(connection);
//
//            // The client is what you use to make requests.
//            client
//        })
//
//        .and_then(|mut client| {
//            // Now we can prepare a simple statement that just returns its parameter.
//            client.prepare("SELECT categoryname from categories")
//                .map(|statement| (client, statement))
//        })
//
//        .and_then(|(mut client, statement)| {
//            // And then execute it, returning a Stream of Rows which we collect into a Vec
//            let result = client.query(&statement, &[]).collect().map(|row|{
//                let category: String = row.get(0);
//                category
//            });
//        })
//}