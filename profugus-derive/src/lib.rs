extern crate proc_macro;

use proc_macro2::TokenTree;
use quote::quote;
use syn::export::TokenStream2;
use syn::{parse_macro_input, Data::Struct, DeriveInput};

#[proc_macro_derive(FromSql)]
pub fn from_sql(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Gather data.
    let name = &input.ident;
    let mut fields: Vec<TokenStream2> = Vec::new();

    match input.data {
        Struct(data) => {
            for field in data.fields {
                match field.ident {
                    Some(ident) => {
                        fields.push(TokenStream2::from(TokenTree::from(ident)));
                    }
                    _ => panic!("Cannot implement FromSql on a tuple struct"),
                }
            }
        }
        _ => panic!(format!(
            "Deriving on {}, which is not a struct, is not supported",
            name.to_string()
        )),
    };

    // Build the output.
    let expanded = quote! {
        use profugus::Row;

        impl FromSql for #name {
            fn from_row(row: &Row) -> Self {
                Self {
                    #(#fields : row.get(stringify!(#fields))),*
                }
            }
        }
    };

    // Hand the output tokens back to the compiler
    proc_macro::TokenStream::from(expanded)
}
