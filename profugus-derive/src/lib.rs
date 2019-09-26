extern crate proc_macro;

use proc_macro2::TokenTree::{Group, Ident as Ident2, Punct};
use proc_macro2::{Ident, Literal, Span, TokenTree};
use quote::quote;
use syn::export::TokenStream2;
use syn::Type::Path;
use syn::{parse_macro_input, Attribute, Data::Struct, DeriveInput, Field, Type};

#[derive(Debug)]
struct SqlField {
    pub rust_name: Ident,
    pub sql_name: Literal,
}

#[proc_macro_derive(FromSql, attributes(profugus))]
pub fn from_sql(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Gather data.
    let name = &input.ident;
    let mut fields: Vec<SqlField> = Vec::new();

    match input.data {
        Struct(data) => {
            for field in data.fields {
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
                                        if !key.eq("name") {
                                            fields.push(SqlField {
                                                rust_name: ident.clone(),
                                                sql_name: Literal::string(
                                                    ident.to_string().as_str(),
                                                ),
                                            });
                                            continue 'attribute_loop;
                                        }
                                        let sql_name = match value {
                                            None => Literal::string(ident.to_string().as_str()),
                                            Some(sql_value) => sql_value,
                                        };
                                        fields.push(SqlField {
                                            rust_name: ident.clone(),
                                            sql_name,
                                        });
                                    }
                                    _ => panic!("Cannot implement FromSql on a tuple struct"),
                                }
                            }
                        } else {
                            continue;
                        }
                    }
                }
            }
        }
        _ => panic!(format!(
            "Deriving on {}, which is not a struct, is not supported",
            name.to_string()
        )),
    };

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

struct StructField {
    pub attribute: Option<String>,
    pub name: Ident,
    pub field_type: Ident,
}

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
                        if segment.ident.to_string().eq("profugus") {
                            continue 'inner;
                        } else {
                            break 'key_name_search;
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
        Some(ident) => ident.clone(),
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
    field_list.retain(|el| el.to_string() != *primary_key.to_string());

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
            fn get_primary_key_value(&self) -> Self::PK
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
#[allow(clippy::unnecessary_operation)]
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
                            TokenTree::Literal(literal) => {
                                Some(literal.to_string().replace("\"", ""));
                            }
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

fn get_key_value_of_attribute(tokens: proc_macro2::Group) -> (Ident, Option<Literal>) {
    let mut name: Ident = Ident::new("none", Span::call_site());
    for token in tokens.stream() {
        match token {
            Ident2(ident) => {
                name = ident;
            }
            Punct(punct) => {
                if punct.as_char() != '=' {
                    return (name, None);
                }
            }
            TokenTree::Literal(literal) => {
                return (name, Some(literal));
            }
            _ => {}
        }
    }
    (name, None)
}

fn generate_argument_list(length: usize) -> String {
    let mut prepared_arguments_list = String::new();
    for i in 1..=length {
        if i == length {
            prepared_arguments_list.push_str(format!("${}", i).as_str());
        } else {
            prepared_arguments_list.push_str(format!("${},", i).as_str());
        }
    }
    prepared_arguments_list
}
fn generate_field_list(field_list: &[TokenStream2]) -> String {
    let mut field_list_str = String::new();
    for (i, field) in field_list.iter().enumerate() {
        if i == field_list.len() - 1 {
            field_list_str.push_str(field.to_string().as_str());
        } else {
            field_list_str.push_str(format!("{},", field.to_string().as_str()).as_str());
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
                if let TokenTree::Group(group) = token_tree {
                    return Some(group.to_string());
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
        if i == (fields.len() - 1) {
            prepared_arguments_list.push_str(format!("${}::{}", i + 1, pg_type).as_str());
        } else {
            prepared_arguments_list.push_str(format!("${}::{},", i + 1, pg_type).as_str());
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
