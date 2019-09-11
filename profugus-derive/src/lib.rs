extern crate proc_macro;

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
#[proc_macro_derive(Identifiable)]
pub fn identifiable_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);

    // Gather data.
    let name = &derive_input.ident;

    match derive_input.data {
        Struct(data) => {
            // Check if the field contains a primary key attribute.
            'key_name_search: for field in &data.fields {
                dbg!(field);
                for attr in &field.attrs {
                    for segment in &attr.path.segments {
                        if segment.ident.to_string().eq("profugus")
                        /*todo add check for pk as argument*/
                        {
                            return build_identifiable_impl(name, get_field_name(field));
                        }
                    }
                }
            }
            // Check if the field contains a
            for field in &data.fields {
                let field_name = get_field_name(field);
                if field_name.to_string().contains("id") {
                    return build_identifiable_impl(name, field_name);
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

fn build_identifiable_impl(name: &Ident, primary_key: &Ident) -> proc_macro::TokenStream {
    let tokens = quote!(
    struct #name {
        count: i64,
    }
        impl Identifiable for #name {
            #[inline]
            fn get_primary_key() -> &'static str {
                return stringify!(#primary_key);
            }
        }
    );
    tokens.into()
}

fn contains_ident_with(name: &'static str, idents: Vec<Ident>) -> bool {
    for ident in idents {
        if ident.to_string().eq("primary_key") {
            return true;
        }
    }
    return false;
}
