//! The std ranges and the std `Result` an item writes: the check, beside the item, that one
//! written by its bare name is the std type every surface describes, the refusal of one under
//! `#[serde(flatten)]`, and that of one as a map key in a build whose map key guard does not run.
//! Beside them, the refusal of a map under `#[serde(flatten)]` beside another member or a tag.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Fields, GenericArgument, Item, PathArguments, Type, TypePath};

use crate::features::serde::parse_serde_key_omission;
use crate::field_type::{
    RangeKind, names_std_ops, names_std_result, written_std_range, written_std_result,
};
use crate::utils::written_type;

#[cfg(any(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
use super::prefixed_guard_message;
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
use super::{WRITTEN_AS_OBJECT, unwritable_map_key_message};
#[cfg(feature = "serde")]
use crate::features::serde::{brand_field, parse_serde_field_attributes};
#[cfg(any(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
use crate::field_type::FieldDef;
#[cfg(feature = "serde")]
use crate::field_type::{FieldDefType, get_field_def};
#[cfg(feature = "serde")]
use crate::utils::is_open_map;
#[cfg(feature = "serde")]
use syn::ext::IdentExt as _;

/// One check per std range and std `Result` `item` writes by a path other than the std module that
/// declares it, in the members serde reads or writes: a function that hands the std type back as
/// the type the path names, which fails with E0308 where the path names another type.
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
        if written_std_result(path).is_some() && !names_std_result(&path.path) {
            checks.extend(result_identity_check(path));
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

/// The check of the `Result` `path` writes: the std `Result` of two types of the function's own,
/// handed back as `path` names it at those types.
fn result_identity_check(path: &TypePath) -> Option<TokenStream> {
    let (ok, err) = (
        Ident::new("T", Span::call_site()),
        Ident::new("E", Span::call_site()),
    );
    let mut named = path.clone();
    named.path.segments.last_mut()?.arguments =
        PathArguments::AngleBracketed(syn::parse_quote! { <#ok, #err> });
    let mut unit = path.clone();
    unit.path.segments.last_mut()?.arguments =
        PathArguments::AngleBracketed(syn::parse_quote! { <(), ()> });
    let held = quote_spanned! { path.span()=> held };
    Some(quote! {
        const _: fn(::core::result::Result<(), ()>) -> #unit = {
            fn is_std_result<#ok, #err>(held: ::core::result::Result<#ok, #err>) -> #named {
                #held
            }
            is_std_result::<(), ()>
        };
    })
}

/// Refuses a `Result` under `#[serde(flatten)]`: serde writes the key of its arm among the members
/// of the object that holds it, which no surface describes.
#[cfg(feature = "serde")]
pub fn flattened_result_error(
    field: &syn::Field,
    field_def: &FieldDef,
    label: &str,
) -> Option<TokenStream> {
    if !matches!(field_def.field_type, FieldDefType::Result(_, _))
        || !parse_serde_field_attributes(&field.attrs).flatten
    {
        return None;
    }
    let message = format!(
        "{label}: `#[serde(flatten)]` cannot hold a `Result<_, _>` — serde writes the key of its \
         arm among the members of the object that holds it, beside the other fields, and no \
         surface describes it there. Write the result under a key of its own."
    );
    Some(syn::Error::new_spanned(field, prefixed_guard_message(&message)).to_compile_error())
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

/// Refuses each map under `#[serde(flatten)]` among `fields` beside another member that writes a
/// key, or beside `tag`, the tag serde writes at the level of `fields`: serde writes the map's
/// entries at one level with that key, which no TypeScript type describes. `subject` leads each
/// message; `generics` are the item's own.
#[cfg(feature = "serde")]
pub fn flattened_map_beside_errors(
    subject: &str,
    fields: &Fields,
    generics: &syn::Generics,
    tag: Option<&str>,
) -> Vec<TokenStream> {
    let parameters = parameter_names(generics);
    let written: Vec<&syn::Field> = fields
        .iter()
        .filter(|field| !parse_serde_key_omission(&field.attrs).absent_from_wire())
        .collect();
    written
        .iter()
        .enumerate()
        .filter(|(_, field)| flattened_map(field, &parameters))
        .filter_map(|(index, field)| {
            let ident = field.ident.as_ref()?;
            let beside = written
                .iter()
                .enumerate()
                .find(|(other, _)| *other != index)
                .and_then(|(_, other)| other.ident.as_ref());
            let message = match (beside, tag) {
                (Some(member), _) => format!(
                    "{subject}field `{}`: `#[serde(flatten)]` cannot hold a map beside other \
                     members \u{2014} serde writes the map's entries at one level with `{}`, and \
                     no TypeScript type describes both. Write the map under a key of its own.",
                    ident.unraw(),
                    member.unraw()
                ),
                (None, Some(key)) => format!(
                    "{subject}field `{}`: `#[serde(flatten)]` cannot hold a map beside the tag \
                     `{key}` \u{2014} serde writes the tag and the map's entries at one level, \
                     and no TypeScript type describes both. Write the map under a key of its own.",
                    ident.unraw()
                ),
                (None, None) => return None,
            };
            Some(
                syn::Error::new_spanned(field, prefixed_guard_message(&message)).to_compile_error(),
            )
        })
        .collect()
}

/// Whether `item` is a type serde writes, flattened, as the entries of a map: a struct whose one
/// member on the wire is a flattened map, a brand or a struct of one slot over a map, and an alias
/// of one.
#[cfg(feature = "serde")]
pub fn writes_an_open_map(item: &Item) -> bool {
    if let Item::Struct(item_struct) = item {
        let parameters = parameter_names(&item_struct.generics);
        if let Some(inner) = brand_field(item_struct) {
            return writes_a_map(&inner.ty, &parameters);
        }
        if let Fields::Unnamed(slots) = &item_struct.fields {
            return slots.unnamed.len() == 1
                && slots
                    .unnamed
                    .iter()
                    .all(|slot| writes_a_map(&slot.ty, &parameters));
        }
        let mut written = item_struct
            .fields
            .iter()
            .filter(|field| !parse_serde_key_omission(&field.attrs).absent_from_wire());
        matches!(
            (written.next(), written.next()),
            (Some(only), None) if flattened_map(only, &parameters)
        )
    } else if let Item::Type(item_type) = item {
        writes_a_map(&item_type.ty, &parameter_names(&item_type.generics))
    } else {
        false
    }
}

/// Whether `field` is a named field under `#[serde(flatten)]` whose type writes a map.
#[cfg(feature = "serde")]
fn flattened_map(field: &syn::Field, parameters: &[String]) -> bool {
    parse_serde_field_attributes(&field.attrs).flatten && writes_a_map(&field.ty, parameters)
}

/// Whether serde writes `ty` as a map, through `Option` and the transparent wrappers: a map, or a
/// type declared above that writes one.
#[cfg(feature = "serde")]
fn writes_a_map(ty: &Type, parameters: &[String]) -> bool {
    let mut def = get_field_def("", ty, "");
    def.erase_type_parameters(parameters);
    def.array_depth == 0
        && (matches!(def.field_type, FieldDefType::Map(_, _))
            || matches!(&def.field_type, FieldDefType::SiblingType(name, _) if is_open_map(name)))
}

/// The names of the type parameters `generics` declares.
#[cfg(feature = "serde")]
fn parameter_names(generics: &syn::Generics) -> Vec<String> {
    generics
        .type_params()
        .map(|parameter| parameter.ident.to_string())
        .collect()
}

/// Refuses a field whose type reaches a map keyed by a std range or a std `Result`, in the words of
/// the map key guard, which a build with no schema surface does not run: serde writes either as an
/// object.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
pub fn map_key_error(field: &syn::Field, field_def: &FieldDef, label: &str) -> Option<TokenStream> {
    let key_name = field_def.object_map_key()?;
    let message = unwritable_map_key_message(label, &key_name, WRITTEN_AS_OBJECT);
    Some(syn::Error::new_spanned(field, prefixed_guard_message(&message)).to_compile_error())
}

#[cfg(test)]
mod tests;
