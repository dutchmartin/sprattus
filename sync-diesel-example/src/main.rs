#[macro_use]
extern crate diesel;
extern crate num_bigint;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;
use bigdecimal::BigDecimal;
use num_bigint::{BigInt, Sign};

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
    create_product(&conn);
    let products = get_products(&conn);
    for product in products {
        dbg!(product);
    }
}

fn create_product(conn: &PgConnection){
    diesel::insert_into(products::table)
        .values((
            products::category.eq(2),
            products::title.eq(String::from("Baby shark")),
            products::actor.eq(String::from("Martin")),
            products::price.eq(BigDecimal::new(BigInt::from_bytes_be(Sign::Plus, &[1_u8]), 2)),
            products::special.eq(Option::None::<i16>),
            products::common_prod_id.eq(12345)
            ))
        .returning(products::prod_id)
        .get_result::<i32>(&*conn).expect("could not insert into db");
}

fn get_products(conn: &PgConnection) -> Vec<Product> {
    products::table
        .select(products::all_columns)
        .order(products::prod_id.asc())
        .load(&*conn)
        .expect("Could not get values form db")
}
