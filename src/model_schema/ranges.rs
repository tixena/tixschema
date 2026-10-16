//! The std ranges an item writes: the check, beside the item, that a range written by its bare name
//! is the std type every surface describes, and the refusal of a range as a map key in a build
//! whose map key guard does not run.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Fields, GenericArgument, Item, PathArguments, Type, TypePath};

use crate::features::serde::parse_serde_key_omission;
use crate::field_type::{RangeKind, names_std_ops, written_std_range};
use crate::utils::written_type;

#[cfg(any(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
use super::prefixed_guard_message;
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
use super::{WRITTEN_AS_OBJECT, unwritable_map_key_message};
#[cfg(feature = "serde")]
use crate::features::serde::parse_serde_field_attributes;
#[cfg(any(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
use crate::field_type::FieldDef;
#[cfg(feature = "serde")]
use crate::field_type::FieldDefType;

/// One check per std range `item` writes by a path other than `core::ops` and `std::ops`, in the
/// members serde reads or writes: a function that hands the std range back as the type the path
/// names, which fails with E0308 where the path names another type.
pub fn identity_checks(item: &Item) -> Vec<TokenStream> {
    let mut checks = Vec::new();
    for ty in described_types(item) {
        collect(ty, &mut checks);
    }
    checks
}

/// The type of each member of `item` serde reads or writes, and an alias's own target.
fn described_types(item: &Item) -> Vec<&Type> {
    if let Item::Struct(item_struct) = item {
        described_members(&item_struct.fields)
    } else if let Item::Enum(item_enum) = item {
        item_enum
            .variants
            .iter()
            .flat_map(|variant| described_members(&variant.fields))
            .collect()
    } else if let Item::Type(item_type) = item {
        vec![&*item_type.ty]
    } else {
        Vec::new()
    }
}

/// The type of each of `fields` serde reads or writes.
fn described_members(fields: &Fields) -> Vec<&Type> {
    fields
        .iter()
        .filter(|field| !parse_serde_key_omission(&field.attrs).absent_from_wire())
        .map(|field| &field.ty)
        .collect()
}

/// Adds the check of each std range `ty` writes, at every position the field definition reads it.
fn collect(ty: &Type, checks: &mut Vec<TokenStream>) {
    let written = written_type(ty);
    if let Type::Path(path) = written {
        if let Some(kind) = written_std_range(path)
            && !names_std_ops(&path.path)
        {
            checks.extend(identity_check(path, kind));
        }
        if let Some(last) = path.path.segments.last()
            && let PathArguments::AngleBracketed(arguments) = &last.arguments
        {
            for argument in &arguments.args {
                if let GenericArgument::Type(held) = argument {
                    collect(held, checks);
                }
            }
        }
    } else if let Type::Reference(reference) = written {
        collect(&reference.elem, checks);
    } else if let Type::Array(array) = written {
        collect(&array.elem, checks);
    } else if let Type::Slice(slice) = written {
        collect(&slice.elem, checks);
    } else if let Type::Tuple(tuple) = written {
        for position in &tuple.elems {
            collect(position, checks);
        }
    } else {
        // Every other type is read as no type the field definition describes.
    }
}

/// The check of the range `path` writes: the std range of a bound of the function's own, handed
/// back as `path` names it at that bound.
fn identity_check(path: &TypePath, kind: RangeKind) -> Option<TokenStream> {
    let std_name = Ident::new(kind.name(), Span::call_site());
    let bound = Ident::new("Bound", Span::call_site());
    let mut named = path.clone();
    let last = named.path.segments.last_mut()?;
    last.arguments = PathArguments::AngleBracketed(syn::parse_quote! { <#bound> });
    let mut unit = path.clone();
    unit.path.segments.last_mut()?.arguments =
        PathArguments::AngleBracketed(syn::parse_quote! { <()> });
    let held = quote_spanned! { path.span()=> held };
    Some(quote! {
        const _: fn(::core::ops::#std_name<()>) -> #unit = {
            fn written_std_range<#bound>(held: ::core::ops::#std_name<#bound>) -> #named {
                #held
            }
            written_std_range::<()>
        };
    })
}

/// Refuses a range under `#[serde(flatten)]`: serde writes its bounds among the members of the
/// object that holds it, which no surface describes.
#[cfg(feature = "serde")]
pub fn flattened_range_error(
    field: &syn::Field,
    field_def: &FieldDef,
    label: &str,
) -> Option<TokenStream> {
    let FieldDefType::Range(kind, _) = &field_def.field_type else {
        return None;
    };
    if !parse_serde_field_attributes(&field.attrs).flatten {
        return None;
    }
    let message = format!(
        "{label}: `#[serde(flatten)]` cannot hold a `{}<_>` — serde writes its bounds among the \
         members of the object that holds it, beside the other fields, and no surface describes \
         them there. Write the range under a key of its own.",
        kind.name()
    );
    Some(syn::Error::new_spanned(field, prefixed_guard_message(&message)).to_compile_error())
}

/// Refuses a field whose type reaches a map keyed by a std range, in the words of the map key
/// guard, which a build with no schema surface does not run: serde writes a range as an object.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
pub fn map_key_error(field: &syn::Field, field_def: &FieldDef, label: &str) -> Option<TokenStream> {
    let kind = field_def.range_map_key()?;
    let key_name = format!("{}<_>", kind.name());
    let message = unwritable_map_key_message(label, &key_name, WRITTEN_AS_OBJECT);
    Some(syn::Error::new_spanned(field, prefixed_guard_message(&message)).to_compile_error())
}

#[cfg(test)]
mod tests;
