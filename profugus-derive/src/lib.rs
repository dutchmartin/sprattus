extern crate proc_macro;

use proc_macro2::TokenTree::{Group, Ident as Ident2, Literal, Punct};
use proc_macro2::{Ident, TokenTree};
use quote::quote;
use syn::export::TokenStream2;
use syn::{parse_macro_input, Attribute, Data::Struct, DeriveInput, Field};

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

// TODO: remove attributes in the derived struct so the feature flag #![feature(custom_attribute)] is not needed.
#[proc_macro_derive(ToSql)]
pub fn to_sql_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    //dbg!(&derive_input);
    let name = &derive_input.ident;

    // Set table name to to either the defined attribute value, or fall back on the structs name
    let table_name: String = match get_table_name_from_attributes(derive_input.attrs) {
        Some(table_name) => table_name,
        None => name.to_string(),
    };

    // derive
    match derive_input.data {
        Struct(data) => {
            // Check if the field contains a primary key attribute.
            'key_name_search: for field in &data.fields {
                for attr in &field.attrs {
                    for segment in &attr.path.segments {
                        if segment.ident.to_string().eq("profugus")
                        // TODO: add check for pk as argument so we are sure we found #[profugus(primary_key)
                        {
                            return build_to_sql_impl(name, get_field_name(field), table_name);
                        }
                    }
                }
            }
            // Check if the field contains a field with `id` in the name.
            for field in &data.fields {
                let field_name = get_field_name(field);
                if field_name.to_string().contains("id") {
                    return build_to_sql_impl(name, field_name, table_name);
                }
            }

            panic!("no field with a name containing `id` or field with the 'primary_key' attribute found");
        }
        _ => panic!(format!(
            "Deriving on {}, which is not a struct, is not supported",
            name.to_string()
        )),
    };
}

fn get_field_name(field: &Field) -> &Ident {
    match &field.ident {
        Some(ident) => {
            return ident;
        }
        _ => panic!("Could not find a name for one of the fields in your struct"),
    }
}

fn build_to_sql_impl(
    name: &Ident,
    primary_key: &Ident,
    table_name: String,
) -> proc_macro::TokenStream {
    let tokens = quote!(
            impl ToSql for #name {

                #[inline]
                fn get_table_name() -> &'static str {
                    #table_name
                }

                #[inline]
                fn get_primary_key() -> &'static str {
                    stringify!(#primary_key)
                }

                #[inline]
                fn get_fields() -> &'static [&'static str] {
                   ["TO", "DO"]
                }

    //            fn get_query_params -> Arc<[Box<dyn ToSqlItem>]> {
    //                unimplemented!()
    //            }
            }
        );
    tokens.into()
}

fn contains_ident_with(name: &'static str, idents: Vec<Ident>) -> bool {
    for ident in idents {
        if ident.to_string().eq(name) {
            return true;
        }
    }
    return false;
}
#[inline]
fn get_table_name_from_attributes(attributes: Vec<Attribute>) -> Option<String> {
    for attribute in attributes {
        match attribute.path.segments.first() {
            Some(segment) => {
                if !segment.ident.to_string().eq("profugus") {
                    continue;
                }
            }
            None => continue,
        }
        'table_name_search: for item in attribute.tokens {
            match item {
                Group(group) => {
                    for token in group.stream() {
                        match token {
                            Ident2(ident) => {
                                if !ident.to_string().eq("table") {
                                    break 'table_name_search;
                                }
                            }
                            Punct(punct) => {
                                if punct.as_char() != '=' {
                                    break 'table_name_search;
                                }
                            }
                            Literal(literal) => return Some(literal.to_string().replace("\"", "")),
                            _ => break 'table_name_search,
                        }
                    }
                }
                _ => break,
            }
        }
    }
    None
}
