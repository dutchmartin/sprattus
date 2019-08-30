use bigdecimal::*;

#[derive(Debug)]
pub struct Product {
    id: i32,
    category: i32,
    title: String,
    actor: String,
    price: BigDecimal,
    special: Option<i16>,
    common_prod_id: i32
}