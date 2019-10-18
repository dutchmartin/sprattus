extern crate proc_macro;

use crate::to_sql::KeyType::{NoKey, PrimaryKey, PrimaryKeyCandidate};
use crate::to_sql::*;
use proc_macro2::TokenTree::{Group, Ident as Ident2, Punct};
use proc_macro2::{Ident, Literal, Span, TokenTree};
use syn::PathArguments::AngleBracketed;
use syn::Type::Path;
use syn::{Attribute, Field, GenericArgument, Type};


pub (crate) fn get_field_name(field: &Field) -> Ident {
    match &field.ident {
        Some(ident) => ident.clone(),
        _ => panic!("Could not find a name for one of the fields in your struct"),
    }
}


#[allow(clippy::unnecessary_operation)]
pub (crate) fn get_table_name_from_attributes(attributes: Vec<Attribute>) -> Option<String> {
    for attribute in attributes {
        match attribute.path.segments.first() {
            Some(segment) => {
                if !segment.ident.to_string().eq("sql") {
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
                                return Some(literal.to_string().replace("\"", ""));
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

pub (crate) fn get_key_value_of_attribute(tokens: proc_macro2::Group) -> (Ident, Option<Literal>) {
    let mut name: Ident = Ident::new("temp", Span::call_site());
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

pub (crate) fn generate_argument_list(length: usize) -> String {
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
pub (crate) fn generate_field_list(field_list: &[String]) -> String {
    let mut field_list_str = String::new();
    for (i, field) in field_list.iter().enumerate() {
        let field = if !field.starts_with('"') {
            format!("\"{}\"", field.as_str())
        } else {
            field.clone()
        };
        if i == field_list.len() - 1 {
            field_list_str.push_str(field.as_str());
        } else {
            field_list_str.push_str(format!("{},", field.as_str()).as_str());
        }
    }
    field_list_str
}

pub (crate) fn get_ident_name_from_path(path: &Type) -> Ident {
    match path {
        Path(path) => match path.path.get_ident() {
            Some(ident) => ident.clone(),
            None => {
                // Handle generic types like Option<T>.
                if let Some(path_segement) = &path.path.segments.first() {
                    if let AngleBracketed(arguments) = &path_segement.arguments {
                        if let Some(GenericArgument::Type(generic_type)) = arguments.args.first() {
                            return get_ident_name_from_path(generic_type);
                        }
                    }
                }
                panic!("Could not infer type information of your struct")
            }
        },
        _ => panic!("not found a path"),
    }
}

pub (crate) fn is_sprattus_attribute(attribute: &Attribute) -> bool {
    match attribute.path.get_ident() {
        Some(name) => name.eq("sql"),
        _ => false,
    }
}

pub (crate) fn generate_argument_list_with_types(fields: &[StructFieldData]) -> String {
    let mut prepared_arguments_list = String::new();
    for (i, pg_type) in fields.iter().map(|field| &field.pg_field_type).enumerate() {
        if i == (fields.len() - 1) {
            prepared_arguments_list.push_str(format!("${}::{}", i + 1, pg_type).as_str());
        } else {
            prepared_arguments_list.push_str(format!("${}::{},", i + 1, pg_type).as_str());
        }
    }
    prepared_arguments_list
}

pub (crate) fn find_field_table_name(field: &Field) -> Option<Literal> {
    'attribute_loop: for attribute in field.attrs.clone() {
        if !is_sprattus_attribute(&attribute) {
            continue;
        }
        for token in attribute.tokens {
            match token {
                Group(group) => match get_key_value_of_attribute(group) {
                    (ident, Some(name)) => {
                        if ident.to_string().eq("name") {
                            return Some(name);
                        }
                    }
                    _ => continue 'attribute_loop,
                },
                _ => {
                    continue 'attribute_loop;
                }
            }
        }
    }
    None
}

pub (crate) fn find_key_type(field: &Field) -> KeyType {
    'attribute_loop: for attribute in field.attrs.clone() {
        if !is_sprattus_attribute(&attribute) {
            continue;
        }
        for token in attribute.tokens {
            match token {
                Group(group) => match get_key_value_of_attribute(group) {
                    (ident, Some(_name)) => {
                        if ident.to_string().eq("primary_key") {
                            return PrimaryKey;
                        }
                    }
                    (ident, None) => {
                        if ident.to_string().eq("primary_key") {
                            return PrimaryKey;
                        }
                    }
                },
                _ => {
                    continue 'attribute_loop;
                }
            }
        }
    }
    if let Some(name) = &field.ident {
        if name.to_string().contains("id") {
            return PrimaryKeyCandidate;
        }
    }
    NoKey
}

pub (crate) fn get_postgres_datatype(rust_type: String) -> String {
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
        "NaiveTime" => String::from("TIME"),
        "NaiveDate" => String::from("DATE"),
        "Uuid" => String::from("UUID"),
        "NaiveDateTime" => String::from("TIMESTAMP"),
        "Json" => String::from("JSON"),
        "MacAddress" => String::from("MACADDR"),
        _ => panic!("unsupported type"),
    }
}