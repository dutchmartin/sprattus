use chrono::*;
use profugus::*;

#[derive(FromSql, ToSql, Eq, PartialEq, Debug)]
#[profugus(table = "reorder")]
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
    date_expected: Option<NaiveDate>,
}

#[tokio::main]
async fn main() -> Result<(), Error>{
    let conn = PGConnection::new("postgresql://localhost/dellstore2?user=tg")
        .await
        .unwrap();

    let reorders = vec![
        Reorder {
            id: 1,
            date_low: NaiveDate::from_ymd(1944,11,6),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(10001),
            date_expected: None
        },
        Reorder {
            id: 2,
            date_low: NaiveDate::from_ymd(1945,5,5),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(1),
            date_expected: None
        },
        Reorder {
            id: 3,
            date_low: NaiveDate::from_ymd(1969,11,6),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(300),
            date_expected: None
        },
        Reorder {
            id: 4,
            date_low: NaiveDate::from_ymd(1989,11,6),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(4),
            date_expected: None
        },
        Reorder {
            id: 5,
            date_low: NaiveDate::from_ymd(1998,11,26),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(5),
            date_expected: None
        }
    ];
    let reorders_update = vec![
        Reorder {
            id: 1,
            date_low: NaiveDate::from_ymd(1944,11,6),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(20002),
            date_expected: Some(NaiveDate::from_ymd(1944,11,7))
        },
        Reorder {
            id: 2,
            date_low: NaiveDate::from_ymd(1945,5,5),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: None,
            date_expected: None
        },
        Reorder {
            id: 3,
            date_low: NaiveDate::from_ymd(1969,11,6),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(300),
            date_expected: None
        },
        Reorder {
            id: 4,
            date_low: NaiveDate::from_ymd(1989,11,6),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(4),
            date_expected: None
        },
        Reorder {
            id: 5,
            date_low: NaiveDate::from_ymd(1998,11,26),
            quantity_low: 0,
            date_reordered: None,
            quantity_reordered: Some(3),
            date_expected: Some(NaiveDate::from_ymd(1998,12,26))
        }
    ];

    // Insert test
    let created_reorders = conn.create_multiple(&reorders).await?;
    assert_eq!(created_reorders, reorders);

    // Query test
    let queried_reorders = conn.query_multiple::<Reorder>("SELECT * FROM reorders WHERE prod_id IN VALUES(1,2,3,4,5)",&[]).await?;
    assert_eq!(queried_reorders, reorders);

    // Update test
    let updated_reorders = conn.update_multiple(&reorders_update).await?;
    assert_eq!(updated_reorders, reorders_update);

    // Delete test
    let deleted_reorders = conn.delete_multiple(&reorders_update).await?;
    assert_eq!(deleted_reorders, reorders_update);

    Ok(())

}
