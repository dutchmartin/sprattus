use profugus::*;
use chrono::NaiveDate;

#[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
#[profugus(table_name = "reorder")]
struct Reorder {
    #[profugus(primary_key)]
    #[profugus(name = "prod_id")]
    id: i32,
    date_low: NaiveDate,
    #[profugus(name = "quant_low")]
    quantity_low: i32,
    date_reordered: Option<NaiveDate>,
    #[profugus(name = "quant_reordered")]
    quantity_reordered: Option<i32>,
    date_expected: Option<NaiveDate>
}

#[tokio::main]
async fn main() {
    let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg")
        .await
        .unwrap();
}
