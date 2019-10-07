extern crate proc_macro;

use crate::KeyType::{NoKey, PrimaryKey, PrimaryKeyCandidate};
use proc_macro2::TokenTree::{Group, Ident as Ident2, Punct};
use proc_macro2::{Ident, Literal, Span, TokenStream, TokenTree};
use quote::quote;
use syn::export::TokenStream2;
use syn::PathArguments::AngleBracketed;
use syn::Type::Path;
use syn::{parse_macro_input, Attribute, Data::Struct, DeriveInput, Field, GenericArgument, Type};

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

#[derive(Debug, Eq, PartialEq)]
enum KeyType {
    PrimaryKey,
    PrimaryKeyCandidate,
    NoKey,
}

enum StructName {
    Renamed { original: Ident, new: Literal },
    Named { name: Ident },
}

impl quote::ToTokens for StructName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match &self {
            StructName::Renamed { original, new: _ } => {
                let n = original.clone();
                tokens.extend(quote!(#n));
            }
            StructName::Named { name } => {
                let n = name.clone();
                tokens.extend(quote!(#n));
            }
        }
    }
}
impl ToString for StructName {
    fn to_string(&self) -> String {
        match self {
            StructName::Renamed { original: _, new } => new.to_string(),
            StructName::Named { name } => name.to_string(),
        }
    }
}

struct StructFieldData {
    pub name: StructName,
    pub key_type: KeyType,
    pub field_type: Ident,
    pub pg_field_type: String,
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

fn get_field_name(field: &Field) -> Ident {
    match &field.ident {
        Some(ident) => ident.clone(),
        _ => panic!("Could not find a name for one of the fields in your struct"),
    }
}
fn build_to_sql_implementation(
    name: &Ident,
    table_name: String,
    field_list: &mut Vec<StructFieldData>,
) -> proc_macro::TokenStream {
    let (primary_key, primary_key_type) = field_list
        .iter()
        .filter(|field| field.key_type == KeyType::PrimaryKey)
        .map(|field| (&field.name, &field.field_type))
        .next()
        .unwrap_or_else(|| {
            panic!("no field field with the 'primary_key' attribute found");
        });
    let primary_key_string = primary_key.to_string();
    let arguments_list_with_types = generate_argument_list_with_types(&field_list);

    let non_pk_field_list: Vec<&StructName> = field_list
        .iter()
        .filter(|field| field.key_type != KeyType::PrimaryKey)
        .map(|field| &field.name)
        .collect();

    let field_list_string = generate_field_list(
        non_pk_field_list
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>()
            .as_slice(),
    );

    let all_fields_list_string = generate_field_list(
        field_list
            .iter()
            .map(|field| field.name.to_string())
            .collect::<Vec<String>>()
            .as_slice(),
    );
    let field_list_len = non_pk_field_list.len();
    let prepared_arguments_list = generate_argument_list(field_list_len);

    let tokens = quote!(
        impl ToSql for #name {

            #[inline]
            fn get_table_name() -> &'static str {
                stringify!(#table_name)
            }

            #[inline]
            fn get_primary_key() -> &'static str {
                #primary_key_string
            }

            type PK = #primary_key_type;

            #[inline]
            fn get_primary_key_value(&self) -> Self::PK
            where
                Self::PK: ToSqlItem + Sized + Sync
            {
                self.#primary_key
            }

            #[inline]
            fn get_all_fields() -> &'static str {
                #all_fields_list_string
            }

            #[inline]
            fn get_fields() -> &'static str {
               #field_list_string
            }

            #[inline]
            fn get_values_of_all_fields(&self) -> Vec<&(dyn ToSqlItem + Sync)> {
                vec![&self.#primary_key,#(&self.#non_pk_field_list),*]
            }

            #[inline]
            fn get_query_params(&self) -> Vec<&(dyn ToSqlItem + Sync)> {
                vec![#(&self.#non_pk_field_list),*]
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

fn get_key_value_of_attribute(tokens: proc_macro2::Group) -> (Ident, Option<Literal>) {
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
fn generate_field_list(field_list: &[String]) -> String {
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

fn get_ident_name_from_path(path: &Type) -> Ident {
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

fn is_profugus_attribute(attribute: &Attribute) -> bool {
    match attribute.path.get_ident() {
        Some(name) => name.eq("profugus"),
        _ => false,
    }
}

fn generate_argument_list_with_types(fields: &Vec<StructFieldData>) -> String {
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

fn find_field_table_name(field: &Field) -> Option<Literal> {
    'attribute_loop: for attribute in field.attrs.clone() {
        if !is_profugus_attribute(&attribute) {
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

fn find_key_type(field: &Field) -> KeyType {
    'attribute_loop: for attribute in field.attrs.clone() {
        if !is_profugus_attribute(&attribute) {
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
        "NaiveTime" => String::from("TIME"),
        "NaiveDate" => String::from("DATE"),
        "Uuid" => String::from("UUID"),
        "NaiveDateTime" => String::from("TIMESTAMP"),
        "Json" => String::from("JSON"),
        "MacAddress" => String::from("MACADDR"),
        _ => panic!("unsupported type"),
    }
}
