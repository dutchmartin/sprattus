extern crate proc_macro;

mod functions;
mod from_sql;
mod to_sql;

use crate::to_sql::*;
use crate::from_sql::SqlField;
use crate::functions::*;
use proc_macro2::{Literal, TokenTree::Group};
use quote::quote;
use syn::export::TokenStream2;
use syn::{parse_macro_input, Data::Struct, DeriveInput};


/// Automatically implements the `ToSql` trait for a given struct.
#[proc_macro_derive(ToSql, attributes(profugus))]
pub fn to_sql(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let name = &derive_input.ident;

    // Set table name to to either the defined attribute value, or fall back on the structs name
    let table_name: String = match get_table_name_from_attributes(derive_input.attrs) {
        Some(table_name) => table_name,
        None => name.to_string(),
    };
    let mut fields_info: Vec<StructFieldData> = Vec::new();

    match derive_input.data {
        Struct(data) => {
            for field in data.fields.clone() {
                let field_name = get_field_name(&field);
                let field_name = match find_field_table_name(&field) {
                    Some(name) => StructName::Renamed {
                        original: (field_name),
                        new: (name),
                    },
                    None => StructName::Named { name: (field_name) },
                };
                let key_type = find_key_type(&field);
                let field_type = get_ident_name_from_path(&field.ty);
                let pg_field_type = get_postgres_datatype(field_type.to_string());

                fields_info.push(StructFieldData {
                    name: (field_name),
                    key_type,
                    field_type,
                    pg_field_type,
                })
            }
        }
        _ => panic!(format!(
            "Deriving on {}, which is not a struct, is not supported",
            name.to_string()
        )),
    };
    build_to_sql_implementation(&name, table_name, &mut fields_info)
}

/// Automatically implements the `FromSql` trait for a given struct.
#[proc_macro_derive(FromSql, attributes(profugus))]
pub fn from_sql(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Gather data.
    let name = &input.ident;
    let mut fields: Vec<SqlField> = Vec::new();

    if let Struct(data) = input.data {
        'field_loop: for field in data.fields {
            'attribute_loop: for attr in field.attrs {
                if let Some(ident) = attr.path.segments.first() {
                    if ident.ident.eq("profugus") {
                        // Attr is ours, let's parse it.
                        for tokens in attr.tokens.into_iter() {
                            let group = match tokens {
                                Group(group) => group,
                                _ => panic!("cannot find a group of tokens to parse"),
                            };
                            let (key, value) = get_key_value_of_attribute(group);
                            match &field.ident {
                                Some(ident) => {
                                    // Validate if the rename attribute is used.
                                    if key.eq("name") {
                                        let sql_name = match value {
                                            None => Literal::string(ident.to_string().as_str()),
                                            Some(sql_value) => sql_value,
                                        };
                                        fields.push(SqlField {
                                            rust_name: ident.clone(),
                                            sql_name,
                                        });
                                        continue 'field_loop;
                                    } else {
                                        continue 'attribute_loop;
                                    }
                                }
                                _ => panic!("Cannot implement FromSql on a tuple struct"),
                            }
                        }
                    } else {
                        continue 'attribute_loop;
                    }
                }
            }
            if let Some(ident) = &field.ident {
                let name = &ident.to_string();
                fields.push(SqlField {
                    rust_name: ident.clone(),
                    sql_name: Literal::string(name.as_str()),
                });
                continue 'field_loop;
            }
        }
    } else {
        panic!(format!(
            "Deriving on {}, which is not a struct, is not supported",
            name.to_string()
        ))
    }

    // Build the lines for constructing the struct.
    let mut struct_lines: Vec<TokenStream2> = Vec::new();
    for field in fields {
        let rust_name = &field.rust_name;
        let sql_name = &field.sql_name;
        struct_lines.push(quote!(
            #rust_name : row.try_get(#sql_name)?
        ));
    }

    // Build the output.
    let expanded = quote! {
        impl FromSql for #name {
            fn from_row(row: &Row) -> Result<Self, Error> where Self: Sized {
                Ok(Self {
                    #(#struct_lines),*
                })
            }
        }
    };
    expanded.into()
}




