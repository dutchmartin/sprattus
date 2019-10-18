use proc_macro2::{Ident, Literal};

#[derive(Debug)]
pub (crate) struct SqlField {
    pub rust_name: Ident,
    pub sql_name: Literal,
}

