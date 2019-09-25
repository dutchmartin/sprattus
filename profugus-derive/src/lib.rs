extern crate proc_macro;

use proc_macro2::TokenTree::{Group, Ident as Ident2, Literal, Punct};
use proc_macro2::{Ident, TokenTree};
use quote::quote;
use syn::export::TokenStream2;
use syn::Type::Path;
use syn::{parse_macro_input, Attribute, Data::Struct, DeriveInput, Field, Type};

#[proc_macro_derive(FromSql, attributes(profugus))]
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
        impl FromSql for #name {
            fn from_row(row: &Row) -> Result<Self, Error> where Self: Sized {
                Ok(Self {
                    #(#fields : row.try_get(stringify!(#fields))?),*
                })
            }
        }
    };

    // Hand the output tokens back to the compiler
    proc_macro::TokenStream::from(expanded)
}

struct StructField {
    pub attribute: Option<String>,
    pub name: Ident,
    pub field_type: Ident,
}

// TODO: remove attributes in the derived struct so the feature flag #![feature(custom_attribute)] is not needed.
#[proc_macro_derive(ToSql, attributes(profugus))]
pub fn to_sql_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    let name = &derive_input.ident;

    // Set table name to to either the defined attribute value, or fall back on the structs name
    let table_name: String = match get_table_name_from_attributes(derive_input.attrs) {
        Some(table_name) => table_name,
        None => name.to_string(),
    };

    let mut fields: Vec<TokenStream2> = Vec::new();

    // derive
    match derive_input.data {
        Struct(data) => {
            for field in data.fields.clone() {
                match field.ident {
                    Some(ident) => {
                        fields.push(TokenStream2::from(TokenTree::from(ident)));
                    }
                    _ => panic!("Cannot implement FromSql on a tuple struct"),
                }
            }
            let mut field_data: Vec<StructField> = Vec::new();
            for field in &data.fields {
                field_data.push(StructField {
                    attribute: get_attribute_name(&field),
                    name: get_field_name(field),
                    field_type: get_ident_name_from_path(&field.ty),
                });
            }
            // Check if the field contains a primary key attribute.
            'key_name_search: for field in &data.fields {
                for attr in &field.attrs {
                    'inner: for segment in &attr.path.segments {
                        match segment.ident.to_string().eq("profugus") {
                            true => break 'inner,
                            false => break 'key_name_search,
                        }
                    }
                    if attr.tokens.to_string().contains("primary_key") {
                        return build_to_sql_impl(
                            name,
                            &get_field_name(field),
                            &get_ident_name_from_path(&field.ty),
                            table_name,
                            &mut fields,
                            field_data,
                        );
                    }
                }
            }
            // Check if the field contains a field with `id` in the name.
            for field in &data.fields {
                let field_name = &get_field_name(field);
                let field_type = get_ident_name_from_path(&field.ty);
                if field_name.to_string().contains("id") {
                    return build_to_sql_impl(
                        name,
                        field_name,
                        &field_type,
                        table_name,
                        &mut fields,
                        field_data,
                    );
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

fn get_field_name(field: &Field) -> Ident {
    match &field.ident {
        Some(ident) => {
            return ident.clone();
        }
        _ => panic!("Could not find a name for one of the fields in your struct"),
    }
}

fn build_to_sql_impl(
    name: &Ident,
    primary_key: &Ident,
    primary_key_type: &Ident,
    table_name: String,
    field_list: &mut Vec<TokenStream2>,
    field_structs: Vec<StructField>,
) -> proc_macro::TokenStream {
    // Remove primary key from fields list
    field_list.retain(|el| el.to_string() != primary_key.to_string());

    let prepared_arguments_list = generate_argument_list(field_list.len());
    let field_list_string = generate_field_list(&field_list);
    let all_field_list_string = primary_key.to_string() + "," + &field_list_string;
    let field_list_len = field_list.len();
    let arguments_list_with_types = generate_argument_list_with_types(field_structs);
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

            type PK = #primary_key_type;

            #[inline]
            fn get_primary_key_value(&self) -> #primary_key_type
                where
        Self::PK: ToSqlItem + Sized + Copy
        {
                self.#primary_key
            }
            #[inline]
            fn get_all_fields() -> &'static str {
                #all_field_list_string
            }

            #[inline]
            fn get_fields() -> &'static str {
               #field_list_string
            }

            #[inline]
            fn get_values_of_all_fields(&self) -> Vec<&dyn ToSqlItem> {
                vec![&self.#primary_key,#(&self.#field_list),*]
            }

            #[inline]
            fn get_query_params(&self) -> Vec<&dyn ToSqlItem> {
                vec![#(&self.#field_list),*]
            }

            #[inline]
            fn get_prepared_arguments_list() -> &'static str {
                #prepared_arguments_list
            }

            #[inline]
            fn get_prepared_arguments_list_with_types() -> &'static str {
                #arguments_list_with_types
            }

            #[inline]
            fn get_argument_count() -> usize {
                #field_list_len
            }
        }
    );
    tokens.into()
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

fn generate_argument_list(length: usize) -> String {
    let mut prepared_arguments_list = String::new();
    for i in 1..length + 1 {
        match i == length {
            true => prepared_arguments_list.push_str(format!("${}", i).as_str()),
            false => prepared_arguments_list.push_str(format!("${},", i).as_str()),
        }
    }
    prepared_arguments_list
}
fn generate_field_list(field_list: &Vec<TokenStream2>) -> String {
    let mut field_list_str = String::new();
    for (i, field) in field_list.iter().enumerate() {
        match i == field_list.len() - 1 {
            true => field_list_str.push_str(field.to_string().as_str()),
            false => field_list_str.push_str(format!("{},", field.to_string().as_str()).as_str()),
        }
    }
    field_list_str
}

fn get_ident_name_from_path(path: &Type) -> Ident {
    //TODO: add support for all types.
    match path {
        Path(path) => path.path.get_ident().unwrap().clone(),
        _ => panic!("not found a path"),
    }
}

fn get_attribute_name(field: &Field) -> Option<String> {
    let profugus_attributes: Vec<&Attribute> = field
        .attrs
        .iter()
        .filter(|attribute| is_profugus_attribute(attribute))
        .collect();
    match profugus_attributes.first() {
        Some(attribute) => {
            for token_tree in attribute.tokens.clone() {
                match token_tree {
                    TokenTree::Group(group) => {
                        return Some(group.to_string());
                    }
                    _ => {}
                }
            }
            None
        }
        _ => None,
    }
}

fn is_profugus_attribute(attribute: &Attribute) -> bool {
    match attribute.path.get_ident() {
        Some(name) => name.eq("profugus"),
        _ => false,
    }
}

fn generate_argument_list_with_types(fields: Vec<StructField>) -> String {
    let mut prepared_arguments_list = String::new();
    for (i, field) in fields.iter().enumerate() {
        let pg_type = get_postgres_datatype(field.field_type.to_string());
        match i == (fields.len() - 1) {
            true => prepared_arguments_list.push_str(format!("${}::{}", i + 1, pg_type).as_str()),
            false => prepared_arguments_list.push_str(format!("${}::{},", i + 1, pg_type).as_str()),
        }
    }
    prepared_arguments_list
}

fn get_postgres_datatype(rust_type: String) -> String {
    match rust_type.as_str() {
        "bool" => String::from("BOOL"),
        "str" => String::from("VARCHAR"),
        "i8" => String::from("CHAR"),
        "i16" => String::from("SMALLINT"),
        "i32" => String::from("INT"),
        "u32" => String::from("OID"),
        "i64" => String::from("BIGINT"),
        "f32" => String::from("REAL"),
        "f64" => String::from("DOUBLE PRECISION"),
        "String" => String::from("VARCHAR"),
        _ => panic!("unsupported type"),
    }
}
