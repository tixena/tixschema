//! `mcp_form_value()` and `mcp_form_flat()`: whether a model type is a choice among plain names,
//! and whether it is a form MCP can show, which takes text, numbers, yes or no, and choices.
//!
//! A type that names another reads the answer recorded when that one was expanded, and takes
//! `false` for one it has not met, so a field never fails the build of the type that holds it.

use super::{Item, Tagging, type_parameters_in_scope};
use crate::features::serde::parse_serde_field_attributes;
use crate::utils::{record_mcp_form_value, recorded_mcp_form_value};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, GenericArgument, PathArguments, Type};

const SCALARS: [&str; 14] = [
    "String", "bool", "f32", "f64", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64",
    "usize",
];

/// The two `const fn` an item earns. An alias earns none: it resolves to the type it stands for.
pub fn items(item: &Item) -> TokenStream {
    let (name, generics, value, flat) = match item {
        Item::Struct(declared) => (
            &declared.ident,
            &declared.generics,
            false,
            struct_flat(declared),
        ),
        Item::Enum(declared) => (
            &declared.ident,
            &declared.generics,
            of_enum(declared),
            false,
        ),
        Item::Const(_)
        | Item::ExternCrate(_)
        | Item::Fn(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Macro(_)
        | Item::Mod(_)
        | Item::Static(_)
        | Item::Trait(_)
        | Item::TraitAlias(_)
        | Item::Type(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_)
        | _ => return TokenStream::new(),
    };
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            pub const fn mcp_form_value() -> bool {
                #value
            }

            pub const fn mcp_form_flat() -> bool {
                #flat
            }
        }
    }
}

/// Records the answer for `item`, for the forms that name it.
pub fn record(item: &Item) {
    let (name, value) = match item {
        Item::Enum(declared) => (&declared.ident, of_enum(declared)),
        Item::Struct(declared) => (&declared.ident, false),
        Item::Type(declared) => (&declared.ident, is_form_value(&declared.ty)),
        Item::Const(_)
        | Item::ExternCrate(_)
        | Item::Fn(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Macro(_)
        | Item::Mod(_)
        | Item::Static(_)
        | Item::Trait(_)
        | Item::TraitAlias(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_)
        | _ => return,
    };
    record_mcp_form_value(&name.to_string(), value);
}

fn of_enum(declared: &syn::ItemEnum) -> bool {
    matches!(Tagging::of(declared), Tagging::Plain)
}

fn struct_flat(declared: &syn::ItemStruct) -> bool {
    let Fields::Named(fields) = &declared.fields else {
        return false;
    };
    type_parameters_in_scope(&declared.generics).is_empty()
        && fields.named.iter().all(|field| {
            !parse_serde_field_attributes(&field.attrs).flatten && is_form_field(&field.ty)
        })
}

fn is_form_field(ty: &Type) -> bool {
    if let Some(inner) = only_argument(ty, "Option") {
        return is_form_field(inner);
    }
    if let Some(inner) = only_argument(ty, "Vec") {
        return is_form_value(inner);
    }
    is_scalar(ty) || is_form_value(ty)
}

fn is_form_value(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|last| recorded_mcp_form_value(&last.ident.to_string()) == Some(true))
}

fn only_argument<'ty>(ty: &'ty Type, wrapper: &str) -> Option<&'ty Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    if last.ident != wrapper {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    match arguments.args.first() {
        Some(GenericArgument::Type(inner)) if arguments.args.len() == 1 => Some(inner),
        _ => None,
    }
}

fn is_scalar(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.qself.is_none()
        && path.path.segments.len() == 1
        && path.path.segments[0].arguments.is_none()
        && SCALARS.iter().any(|scalar| path.path.segments[0].ident == scalar))
}
