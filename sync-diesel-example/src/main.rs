#[macro_use]
extern crate diesel;
extern crate num_bigint;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::dsl::*;
use dotenv::dotenv;
use std::env;


mod models;
mod schema;

use crate::schema::*;
use crate::models::*;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

fn main() {
    let conn = establish_connection();
    let products = get_products(&conn);
    for product in products {
        dbg!(product);
    }
}

fn create_product(conn: &PgConnection){


}

fn get_products(conn: &PgConnection) -> Vec<Product> {
    products::table.select(products::all_columns).load(&*conn).unwrap()
}