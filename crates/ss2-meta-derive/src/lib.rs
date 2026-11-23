//! `#[derive(MetaDeserialize)]`: typed structs from the generic `Value` tree of `ss2-meta`
//! (roadmap T02-008b).
//!
//! ```ignore
//! #[derive(MetaDeserialize)]
//! #[meta(class = "CBaseTexture")]                     // optional: the class the value must be
//! struct TextureHeader {
//!     #[meta(owner = "CBaseTexture", name = "tx_pixWidth")]
//!     width: u32,
//!     #[meta(owner = "CBaseTexture", name = "tx_pixHeight")]
//!     height: u32,
//!     #[meta(owner = "CBaseTexture", name = "tx_ctFrames", default)]   // absent member: Default
//!     frames: u32,
//! }
//! ```
//!
//! A field is read from member `name` declared by class `owner` (the member names alone are not
//! unique: a derived class may reuse `"1"`). `owner` defaults to the struct's `class`. The field
//! type must implement `ss2_meta::FromValue`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

/// Derive `ss2_meta::FromValue` for a struct with named fields.
#[proc_macro_derive(MetaDeserialize, attributes(meta))]
pub fn derive_meta_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            name,
            "MetaDeserialize can only be derived for structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            name,
            "MetaDeserialize needs a struct with named fields",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "MetaDeserialize does not support generic structs",
        ));
    }

    let mut class: Option<LitStr> = None;
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("meta")) {
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("class") {
                class = Some(m.value()?.parse()?);
                Ok(())
            } else {
                Err(m.error("unknown key; the struct accepts `class = \"...\"`"))
            }
        })?;
    }

    let mut inits = Vec::new();
    for field in &fields.named {
        let ident = field.ident.as_ref().expect("named fields have identifiers");
        let (mut owner, mut member, mut default) = (None::<LitStr>, None::<LitStr>, false);
        for attr in field.attrs.iter().filter(|a| a.path().is_ident("meta")) {
            attr.parse_nested_meta(|m| {
                if m.path.is_ident("owner") {
                    owner = Some(m.value()?.parse()?);
                } else if m.path.is_ident("name") {
                    member = Some(m.value()?.parse()?);
                } else if m.path.is_ident("default") {
                    default = true;
                } else {
                    return Err(m.error(
                        "unknown key; use `owner = \"...\"`, `name = \"...\"` or `default`",
                    ));
                }
                Ok(())
            })?;
        }
        let Some(member) = member else {
            return Err(syn::Error::new_spanned(
                ident,
                "this field needs `#[meta(name = \"...\")]`",
            ));
        };
        let Some(owner) = owner.or_else(|| class.clone()) else {
            return Err(syn::Error::new_spanned(
                ident,
                "this field needs `#[meta(owner = \"...\")]` (or a struct-level `#[meta(class = \"...\")]`)",
            ));
        };
        let field_name = ident.to_string();
        let ty = &field.ty;
        let read = if default {
            quote! { ::ss2_meta::__private::field_or_default::<#ty>(types, s, #owner, #member, #field_name)? }
        } else {
            quote! { ::ss2_meta::__private::field::<#ty>(types, s, #owner, #member, #field_name)? }
        };
        inits.push(quote! { #ident: #read });
    }

    let struct_name = name.to_string();
    let class_check = class.map(|c| {
        quote! { ::ss2_meta::__private::check_class(types, s, #c, #struct_name)?; }
    });
    Ok(quote! {
        impl ::ss2_meta::FromValue for #name {
            fn from_value(
                value: &::ss2_meta::Value,
                types: &::ss2_meta::TypeSystem,
            ) -> ::ss2_meta::Result<Self> {
                let s = ::ss2_meta::__private::expect_struct(value, #struct_name)?;
                #class_check
                Ok(Self { #(#inits),* })
            }
        }
    })
}
