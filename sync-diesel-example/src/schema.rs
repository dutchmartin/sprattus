table! {
    categories (category) {
        category -> Int4,
        categoryname -> Varchar,
    }
}

table! {
    customers (customerid) {
        customerid -> Int4,
        firstname -> Varchar,
        lastname -> Varchar,
        address1 -> Varchar,
        address2 -> Nullable<Varchar>,
        city -> Varchar,
        state -> Nullable<Varchar>,
        zip -> Nullable<Int4>,
        country -> Varchar,
        region -> Int2,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        creditcardtype -> Int4,
        creditcard -> Varchar,
        creditcardexpiration -> Varchar,
        username -> Varchar,
        password -> Varchar,
        age -> Nullable<Int2>,
        income -> Nullable<Int4>,
        gender -> Nullable<Varchar>,
    }
}

table! {
    inventory (prod_id) {
        prod_id -> Int4,
        quan_in_stock -> Int4,
        sales -> Int4,
    }
}

table! {
    orders (orderid) {
        orderid -> Int4,
        orderdate -> Date,
        customerid -> Nullable<Int4>,
        netamount -> Numeric,
        tax -> Numeric,
        totalamount -> Numeric,
    }
}

table! {
    products (prod_id) {
        prod_id -> Int4,
        category -> Int4,
        title -> Varchar,
        actor -> Varchar,
        price -> Numeric,
        special -> Nullable<Int2>,
        common_prod_id -> Int4,
    }
}

joinable!(orders -> customers (customerid));

allow_tables_to_appear_in_same_query!(
    categories,
    customers,
    inventory,
    orders,
    products,
);
