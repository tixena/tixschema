//! The `#[serde(deny_unknown_fields)]` the macro writes on a struct and on an enum, so that the
//! Rust read of a value refuses a key no member declares, as every other surface does.
//!
//! The attribute stays off a type that is `nonstrict`, off one the author already wrote it on, and
//! off one that flattens a member whose keys the expansion cannot see: serde would refuse the very
//! keys such a member is declared to hold.

use super::{FieldDef, FieldDefType, ModelSchemaArgs, get_field_def, prefixed_guard_message};
use crate::features::serde::{
    derived_serde_attributes, has_serde_transparent, parse_serde_field_attributes,
    parse_serde_field_hooks, parse_serde_key_omission, parse_serde_type_attributes, struct_tag,
};
use crate::utils::{
    Declared, declared, flattened_parameters, is_open_map, is_unconsumed, record_unconsumed,
};
use syn::punctuated::Punctuated;
use syn::token::Paren;
use syn::{Attribute, Field, Fields, Item, ItemEnum, ItemStruct, Token};

/// How many aliases deep a flattened member is followed before its keys are called unseen.
const ALIAS_DEPTH: usize = 8;

/// The path of the `deny_unknown_fields` serde reads on a container, where its author wrote it.
fn written_deny(attrs: &[Attribute]) -> Option<syn::Path> {
    let mut found = None;
    for attr in derived_serde_attributes(attrs) {
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("deny_unknown_fields") {
                found.get_or_insert(nested.path);
            } else if nested.input.peek(Token![=]) {
                nested.value()?.parse::<syn::Expr>()?;
            } else if nested.input.peek(Paren) {
                nested.input.parse::<proc_macro2::Group>()?;
            } else {
                // A bare key such as `untagged`: nothing to step over.
            }
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde container attribute: {e}");
        });
    }
    found
}

/// Whether serde's own derive, rather than another crate's namesake, is among what `attrs` derive.
fn derives_serde(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .any(|path| {
            let mut names = path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string());
            let (first, second, third) = (names.next(), names.next(), names.next());
            let derive = |name: &str| name == "Serialize" || name == "Deserialize";
            match (first.as_deref(), second.as_deref(), third) {
                (Some(only), None, None) | (Some("serde"), Some(only), None) => derive(only),
                _ => false,
            }
        })
}

/// Whether serde writes the item `attrs` belong to under a key and never reads it: a key its
/// own writes carry, which `deny_unknown_fields` would refuse.
fn written_not_read(attrs: &[Attribute]) -> bool {
    let (mut unread, mut unwritten) = (false, false);
    for attr in derived_serde_attributes(attrs) {
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("skip_deserializing") {
                unread = true;
            } else if nested.path.is_ident("skip") || nested.path.is_ident("skip_serializing") {
                unwritten = true;
            } else {
                // Any other key leaves the member read and written.
            }
            if nested.input.peek(Token![=]) {
                nested.value()?.parse::<syn::Expr>()?;
            } else if nested.input.peek(Paren) {
                nested.input.parse::<proc_macro2::Group>()?;
            } else {
                // A bare key: nothing to step over.
            }
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde attribute: {e}");
        });
    }
    unread && !unwritten
}

/// Whether a member of `fields` is written under a key and never read.
fn writes_an_unread_key(fields: &Fields) -> bool {
    fields.iter().any(|field| written_not_read(&field.attrs))
}

/// Whether the keys `def` writes, flattened, are all known to the expansion.
fn keys_known(def: &FieldDef, depth: usize) -> bool {
    if def.array_depth != 0 {
        return false;
    }
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return false;
    };
    match declared(name) {
        Some(Declared::Model(_)) => {
            !is_open_map(name)
                && !is_unconsumed(name)
                && flattened_parameters(name).iter().all(|parameter| {
                    arguments
                        .get(parameter.at)
                        .is_some_and(|filling| keys_known(filling, depth))
                })
        }
        Some(Declared::Alias(target)) if depth < ALIAS_DEPTH => {
            syn::parse_str::<syn::Type>(&target)
                .is_ok_and(|ty| keys_known(&get_field_def(name, &ty, ""), depth.saturating_add(1)))
        }
        Some(Declared::Alias(_) | Declared::OneValue(_)) | None => false,
    }
}

/// Whether serde leaves the keys of a flattened `fields` in the object it reads them from.
fn flattens_something(fields: &Fields) -> bool {
    fields
        .iter()
        .any(|field| parse_serde_field_attributes(&field.attrs).flatten)
}

/// Records whether serde takes the keys of `item` off the object a holder flattens it from: it
/// takes those of a struct that flattens nothing and has no tag of its own and of an enum under no
/// tag or under a tag and a content key, and it leaves those of an enum under a tag alone, an
/// untagged one, a struct under a tag of its own and any type that flattens a member of its own.
pub(super) fn record(item: &Item, args: &ModelSchemaArgs) {
    let corrected = !args.preprocess.is_empty();
    if let Item::Struct(item_struct) = item {
        record_unconsumed(
            &item_struct.ident.to_string(),
            corrected
                || flattens_something(&item_struct.fields)
                || struct_tag(item_struct).is_some(),
        );
    } else if let Item::Enum(item_enum) = item {
        let written = parse_serde_type_attributes(&item_enum.attrs);
        let held_apart = !written.untagged && written.tag.is_some() == written.content.is_some();
        let unit_entry = written.tag.is_none()
            && item_enum
                .variants
                .iter()
                .any(|variant| matches!(variant.fields, Fields::Unit));
        record_unconsumed(
            &item_enum.ident.to_string(),
            corrected
                || !held_apart
                || unit_entry
                || item_enum
                    .variants
                    .iter()
                    .any(|variant| flattens_something(&variant.fields)),
        );
    } else {
        // An alias is read through what it names.
    }
}

/// Whether a function reads `field`: one serde calls or one the type names with `model_schema_prop`.
fn read_by_a_function(field: &Field) -> bool {
    let hooks = parse_serde_field_hooks(&field.attrs);
    hooks.with.is_some()
        || hooks.deserialize_with.is_some()
        || field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("model_schema_prop"))
}

/// Whether serde reads a member of `fields` under a key of its own. Without one, its derive would
/// declare an enum of no variant for the keys, which `clippy::empty_enums` refuses in the
/// consumer's crate.
fn reads_a_key(fields: &Fields) -> bool {
    fields.iter().any(|field| {
        !parse_serde_field_attributes(&field.attrs).flatten
            && !parse_serde_key_omission(&field.attrs).skips_deserializing
    })
}

/// Whether `ty` names one of the type parameters `generics` declares, anywhere in it.
fn names_a_parameter(ty: &syn::Type, generics: &syn::Generics) -> bool {
    let parameters: Vec<String> = generics
        .type_params()
        .map(|parameter| parameter.ident.to_string())
        .collect();
    let mut pending: Vec<proc_macro2::TokenStream> = vec![quote::ToTokens::to_token_stream(ty)];
    while let Some(tokens) = pending.pop() {
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Ident(ident) => {
                    if parameters.contains(&ident.to_string()) {
                        return true;
                    }
                }
                proc_macro2::TokenTree::Group(group) => pending.push(group.stream()),
                proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {}
            }
        }
    }
    false
}

/// Whether every member `fields` flattens holds keys the expansion can see.
fn flattened_keys_known(fields: &Fields, generics: &syn::Generics) -> bool {
    fields
        .iter()
        .filter(|field| parse_serde_field_attributes(&field.attrs).flatten)
        .all(|field| {
            !read_by_a_function(field)
                && !names_a_parameter(&field.ty, generics)
                && keys_known(&get_field_def("flattened", &field.ty, ""), 0)
        })
}

/// Whether the macro writes the attribute on a container that carries `attrs`.
fn writes_deny(attrs: &[Attribute], args: &ModelSchemaArgs) -> bool {
    !args.nonstrict
        && args.preprocess.is_empty()
        && derives_serde(attrs)
        && written_deny(attrs).is_none()
        && !has_serde_transparent(attrs)
}

/// Writes `#[serde(deny_unknown_fields)]` on `item_struct` where it is a struct of named members
/// the macro closes.
pub(super) fn close_struct(item_struct: &mut ItemStruct, args: &ModelSchemaArgs) {
    if matches!(item_struct.fields, Fields::Named(_))
        && reads_a_key(&item_struct.fields)
        && !writes_an_unread_key(&item_struct.fields)
        && struct_tag(item_struct).is_none()
        && writes_deny(&item_struct.attrs, args)
        && flattened_keys_known(&item_struct.fields, &item_struct.generics)
    {
        item_struct
            .attrs
            .push(syn::parse_quote! { #[serde(deny_unknown_fields)] });
    }
}

/// Writes `#[serde(deny_unknown_fields)]` on `item_enum` where the macro closes it.
pub(super) fn close_enum(item_enum: &mut ItemEnum, args: &ModelSchemaArgs) {
    if !item_enum.variants.is_empty()
        && writes_deny(&item_enum.attrs, args)
        && item_enum.variants.iter().all(|variant| {
            !written_not_read(&variant.attrs)
                && !writes_an_unread_key(&variant.fields)
                && flattened_keys_known(&variant.fields, &item_enum.generics)
                && (!matches!(variant.fields, Fields::Named(_)) || reads_a_key(&variant.fields))
        })
    {
        item_enum
            .attrs
            .push(syn::parse_quote! { #[serde(deny_unknown_fields)] });
    }
}

/// The refusal of a `nonstrict` type whose author also wrote `deny_unknown_fields`, which asks
/// for the opposite.
pub(super) fn conflict_errors(
    item: &Item,
    args: &ModelSchemaArgs,
) -> Vec<proc_macro2::TokenStream> {
    let attrs = if let Item::Struct(item_struct) = item {
        &item_struct.attrs
    } else if let Item::Enum(item_enum) = item {
        &item_enum.attrs
    } else {
        return Vec::new();
    };
    let Some(written) = written_deny(attrs).filter(|_| args.nonstrict) else {
        return Vec::new();
    };
    vec![
        syn::Error::new_spanned(
            written,
            prefixed_guard_message(
                "`nonstrict` and `#[serde(deny_unknown_fields)]` are both written on this type: \
                 `nonstrict` reads past a key no member declares and `deny_unknown_fields` \
                 refuses it, so remove one of them",
            ),
        )
        .to_compile_error(),
    ]
}
