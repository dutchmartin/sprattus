use bigdecimal::*;

#[derive(Debug)]
pub struct Product {
    pub id: i32,
    pub category: i32,
    pub title: String,
    pub actor: String,
    pub price: BigDecimal,
    pub special: Option<i16>,
    pub common_prod_id: i32
}