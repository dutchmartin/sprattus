use sprattus::*;

/// This struct just contains keywords as names, just to test.
#[derive(Eq, PartialEq, Debug, ToSql, FromSql)]
struct Collate {
    #[sql(primary_key)]
    id: i32,
    column: bool,
    desc: bool,
    constraint: Option<i32>,
    current_user: String,
    fetch: String,
}

pub async fn test_if_keywords_are_escaped(conn: Connection) -> Result<(), Error> {
    print!("\n Testing if keywords are properly escaped ... \n\n");

    let fixture = vec![
        Collate {
            id: 1,
            column: true,
            desc: false,
            constraint: Some(103),
            current_user: String::from("Martin"),
            fetch: String::from("example.com"),
        },
        Collate {
            id: 2,
            column: true,
            desc: true,
            constraint: Some(7689),
            current_user: String::from("Steven"),
            fetch: String::from("google.com"),
        },
        Collate {
            id: 3,
            column: false,
            desc: true,
            constraint: Some(543_432),
            current_user: String::from("Superman"),
            fetch: String::from("tweedegolf.nl"),
        },
    ];

    let update_fixture = vec![
        Collate {
            id: 1,
            column: false,
            desc: false,
            constraint: Some(4535),
            current_user: String::from("Martin"),
            fetch: String::from("martijngroeneveldt.nl"),
        },
        Collate {
            id: 2,
            column: false,
            desc: true,
            constraint: Some(7_645_389),
            current_user: String::from("Steven"),
            fetch: String::from("google.com"),
        },
        Collate {
            id: 3,
            column: false,
            desc: false,
            constraint: None,
            current_user: String::from("Batman"),
            fetch: String::from("tweedegolf.nl"),
        },
    ];
    // Setup table
    conn.batch_execute(
        "DROP TABLE IF EXISTS \"Collate\";
        CREATE TABLE \"Collate\" (
	    \"id\" serial NOT NULL,
	    \"column\" bool NOT NULL,
	    \"desc\" bool NOT NULL,
	    \"constraint\" int4 NULL,
	    \"current_user\" varchar NOT NULL,
	    \"fetch\" varchar NOT NULL);",
    )
    .await?;

    // Insert test
    let created_items = conn.create_multiple(&fixture).await?;
    assert_eq!(created_items, fixture);
    println!("Insert succeeded");

    // Query test
    let queried_reorders = conn
        .query_multiple::<Collate>("SELECT * FROM  \"Collate\" WHERE id IN (1,2,3,4,5)", &[])
        .await?;
    assert_eq!(queried_reorders, fixture);
    println!("Query succeeded");

    // Update test
    let updated_items = conn.update_multiple(&update_fixture).await?;
    assert_eq!(updated_items, update_fixture);
    println!("Update succeeded");

    // Delete test
    let deleted_items = conn.delete_multiple(&update_fixture).await?;
    assert_eq!(deleted_items, update_fixture);
    println!("Delete succeeded");

    Ok(())
}
