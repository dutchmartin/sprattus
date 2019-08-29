use bigdecimal::*;
use crate::schema::*;

#[derive(Queryable, Debug, Associations, Insertable)]
pub struct Product {
    #[column_name="prod_id"]
    id: i32,
    category: i32,
    title: String,
    actor: String,
    price: BigDecimal,
    special: Option<i16>,
    common_prod_id: i32
}