extern crate proc_macro;

use crate::functions::*;
use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum KeyType {
    PrimaryKey,
    PrimaryKeyCandidate,
    NoKey,
}

pub(crate) enum StructName {
    Renamed { original: Ident, new: Literal },
    Named { name: Ident },
}
pub(crate) struct StructFieldData {
    pub name: StructName,
    pub key_type: KeyType,
    pub field_type: Ident,
    pub pg_field_type: String,
}

impl quote::ToTokens for StructName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match &self {
            StructName::Renamed { original, .. } => {
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
            StructName::Renamed { new, .. } => new.to_string(),
            StructName::Named { name } => name.to_string(),
        }
    }
}

pub(crate) fn build_to_sql_implementation(
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
