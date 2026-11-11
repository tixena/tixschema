//! The `#[serde(deny_unknown_fields)]` the macro writes on a struct and on an enum, so that the
//! Rust read of a value refuses a key no member declares, as every other surface does.
//!
//! The attribute stays off a type that is `nonstrict`, off one the author already wrote it on, and
//! off one that flattens a member serde leaves the keys of in the object it reads them from:
//! serde would refuse the very keys such a member is declared to hold. A strict struct whose serde
//! read then refuses no such key, under a tag of its own or beside a flattened member that reads
//! past one, is refused, so that it is marked `nonstrict`, and so is a strict enum a variant of
//! which flattens such a member.

use super::flatten_order::{
    Culprit, a_variant_refuses_a_stray, held_open_refusal, open_member_refusal,
    open_variant_refusal, refused_by_a_member, takes_only,
};
use super::{ModelSchemaArgs, prefixed_guard_message};
use crate::features::serde::{
    Tagging, derived_serde_attributes, has_serde_transparent, parse_serde_field_attributes,
    parse_serde_key_omission, parse_serde_type_attributes, read_hook, struct_tag,
    variant_on_the_wire,
};
use crate::utils::{record_unconsumed, to_snake_case};
use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt as _;
use syn::punctuated::Punctuated;
use syn::token::Paren;
use syn::{Attribute, Fields, Item, ItemEnum, ItemStruct, Token};

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

/// Whether serde reads a member of `fields`, under a key of its own or flattened. Without one, its
/// derive would declare an enum of no variant for the keys, which `clippy::empty_enums` refuses in
/// the consumer's crate.
fn reads_a_member(fields: &Fields) -> bool {
    fields
        .iter()
        .any(|field| !parse_serde_key_omission(&field.attrs).skips_deserializing)
}

/// Whether the macro writes the attribute on a container that carries `attrs`.
fn writes_deny(attrs: &[Attribute], args: &ModelSchemaArgs) -> bool {
    !args.nonstrict
        && args.preprocess.is_empty()
        && derives_serde(attrs)
        && written_deny(attrs).is_none()
        && !has_serde_transparent(attrs)
}

/// Whether the macro writes `#[serde(deny_unknown_fields)]` on `item_struct`: a struct of named
/// members, one of which serde reads, whose flattened members all take their own keys.
pub(super) fn closes_struct(item_struct: &ItemStruct, args: &ModelSchemaArgs) -> bool {
    matches!(item_struct.fields, Fields::Named(_))
        && reads_a_member(&item_struct.fields)
        && !writes_an_unread_key(&item_struct.fields)
        && struct_tag(item_struct).is_none()
        && writes_deny(&item_struct.attrs, args)
        && takes_only(&item_struct.fields, &item_struct.generics)
}

/// Whether the macro writes `#[serde(deny_unknown_fields)]` on `item_enum`.
pub(super) fn closes_enum(item_enum: &ItemEnum, args: &ModelSchemaArgs) -> bool {
    closable_but_for_flattening(item_enum, args)
        && item_enum
            .variants
            .iter()
            .all(|variant| takes_only(&variant.fields, &item_enum.generics))
}

/// Whether the macro would write `#[serde(deny_unknown_fields)]` on `item_enum` but for a member
/// a variant flattens that reads every key still left.
pub(super) fn closable_but_for_flattening(item_enum: &ItemEnum, args: &ModelSchemaArgs) -> bool {
    !item_enum.variants.is_empty()
        && writes_deny(&item_enum.attrs, args)
        && item_enum.variants.iter().all(|variant| {
            !written_not_read(&variant.attrs)
                && !writes_an_unread_key(&variant.fields)
                && (!matches!(variant.fields, Fields::Named(_)) || reads_a_member(&variant.fields))
        })
}

/// The unit variants of `item_enum` the macro reads through [`unit_read_name`], so that they refuse
/// a key beside the enum's tag: those serde reads of an enum under a tag alone the macro or its
/// author closes, that name no read and are not the `#[serde(other)]` variant.
fn refusing_units<'item>(
    item_enum: &'item ItemEnum,
    args: &ModelSchemaArgs,
) -> Vec<&'item syn::Variant> {
    let closed =
        closes_enum(item_enum, args) || (closed_by_hand(&item_enum.attrs) && !args.nonstrict);
    if !matches!(Tagging::of(item_enum), Tagging::Internal { tag: _tag }) || !closed {
        return Vec::new();
    }
    item_enum
        .variants
        .iter()
        .filter(|variant| {
            matches!(variant.fields, Fields::Unit)
                && variant_on_the_wire(variant)
                && !parse_serde_key_omission(&variant.attrs).skips_deserializing
                && read_hook(&variant.attrs).is_none()
                && !names_other(&variant.attrs)
        })
        .collect()
}

/// Whether `attrs` mark a variant `#[serde(other)]`.
fn names_other(attrs: &[Attribute]) -> bool {
    let mut other = false;
    for attr in derived_serde_attributes(attrs) {
        attr.parse_nested_meta(|nested| {
            other |= nested.path.is_ident("other");
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
            log::trace!("Failed to parse serde variant attribute: {e}");
        });
    }
    other
}

/// The name of the read the macro hangs on each unit variant of `item_enum` it closes under a tag
/// alone: serde reads such a variant past every key beside the tag.
fn unit_read_name(item_enum: &ItemEnum) -> String {
    format!(
        "model_schema_{}_unit_refuses_a_key",
        to_snake_case(&item_enum.ident.unraw().to_string())
    )
}

/// Whether the macro reads the unit variants of `item_enum` refusing a key beside its tag.
pub(super) fn refuses_beside_a_unit(item_enum: &ItemEnum, args: &ModelSchemaArgs) -> bool {
    !refusing_units(item_enum, args).is_empty()
}

/// The read [`refusing_units`] hang on the unit variants of `item`, where it has one: it reads
/// `{}` and the unit a format writes, and refuses the first key it is handed.
pub(super) fn unit_reads(item: &Item, args: &ModelSchemaArgs) -> TokenStream {
    let Item::Enum(item_enum) = item else {
        return TokenStream::new();
    };
    if refusing_units(item_enum, args).is_empty() {
        return TokenStream::new();
    }
    let name = proc_macro2::Ident::new(&unit_read_name(item_enum), proc_macro2::Span::call_site());
    let doc = format!(
        " The read of a unit variant of `{}`, which refuses a key beside the tag.",
        item_enum.ident.unraw()
    );
    quote! {
        #[doc = #doc]
        fn #name<'de, D: ::serde::Deserializer<'de>>(deserializer: D) -> ::core::result::Result<(), D::Error> {
            struct UnknownKey(::core::convert::Infallible);
            impl<'de> ::serde::Deserialize<'de> for UnknownKey {
                fn deserialize<K: ::serde::Deserializer<'de>>(keys: K) -> ::core::result::Result<Self, K::Error> {
                    struct KeyVisitor;
                    impl<'de> ::serde::de::Visitor<'de> for KeyVisitor {
                        type Value = UnknownKey;
                        fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            formatter.write_str("no key")
                        }
                        fn visit_str<E: ::serde::de::Error>(self, key: &str) -> ::core::result::Result<Self::Value, E> {
                            ::core::result::Result::Err(E::unknown_field(key, &[]))
                        }
                    }
                    keys.deserialize_identifier(KeyVisitor)
                }
            }
            struct UnitVisitor;
            impl<'de> ::serde::de::Visitor<'de> for UnitVisitor {
                type Value = ();
                fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    formatter.write_str("no key beside the tag")
                }
                fn visit_unit<E: ::serde::de::Error>(self) -> ::core::result::Result<Self::Value, E> {
                    ::core::result::Result::Ok(())
                }
                fn visit_map<A: ::serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> ::core::result::Result<Self::Value, A::Error> {
                    match map.next_key::<UnknownKey>()? {
                        ::core::option::Option::Some(unknown) => match unknown.0 {},
                        ::core::option::Option::None => ::core::result::Result::Ok(()),
                    }
                }
            }
            deserializer.deserialize_any(UnitVisitor)
        }
    }
}

/// Whether every surface describes `item`'s own object as reading past a key no member declares:
/// where it is `nonstrict`, but for a struct a member of which it flattens refuses the key, and an
/// enum a variant of which refuses it, as serde refuses it there.
pub(super) fn reads_past_a_stray(item: &Item, args: &ModelSchemaArgs) -> bool {
    if !args.nonstrict {
        return false;
    }
    if let Item::Struct(item_struct) = item {
        !refused_by_a_member(&item_struct.fields, &item_struct.generics)
    } else if let Item::Enum(item_enum) = item {
        !a_variant_refuses_a_stray(item_enum)
    } else {
        true
    }
}

/// Whether the author wrote `#[serde(deny_unknown_fields)]` among `attrs`.
pub(super) fn closed_by_hand(attrs: &[Attribute]) -> bool {
    written_deny(attrs).is_some()
}

/// The struct as a type that refuses a key it does not declare, where it refuses one: closed by
/// its author or by the macro, or a struct of no member whose read the macro writes.
pub(super) fn struct_culprit(item_struct: &ItemStruct, args: &ModelSchemaArgs) -> Option<Culprit> {
    let by_hand = closed_by_hand(&item_struct.attrs);
    let read_written = item_struct.fields.is_empty() && !args.nonstrict;
    (by_hand || read_written || closes_struct(item_struct, args)).then(|| Culprit {
        by_hand,
        holds_more: false,
        name: item_struct.ident.to_string(),
    })
}

/// The enum as a type that refuses a key it does not declare, where it refuses one.
pub(super) fn enum_culprit(item_enum: &ItemEnum, args: &ModelSchemaArgs) -> Option<Culprit> {
    let by_hand = closed_by_hand(&item_enum.attrs);
    (by_hand || closes_enum(item_enum, args)).then(|| Culprit {
        by_hand,
        holds_more: false,
        name: item_enum.ident.to_string(),
    })
}

/// Writes `#[serde(deny_unknown_fields)]` on `item_struct` where the macro closes it.
pub(super) fn close_struct(item_struct: &mut ItemStruct, args: &ModelSchemaArgs) {
    if closes_struct(item_struct, args) {
        item_struct
            .attrs
            .push(syn::parse_quote! { #[serde(deny_unknown_fields)] });
    }
}

/// Writes `#[serde(deny_unknown_fields)]` on `item_enum` where the macro closes it, and hangs the
/// read of [`unit_reads`] on each of its unit variants [`refusing_units`] names.
pub(super) fn close_enum(item_enum: &mut ItemEnum, args: &ModelSchemaArgs) {
    let units: Vec<String> = refusing_units(item_enum, args)
        .iter()
        .map(|variant| variant.ident.to_string())
        .collect();
    if !units.is_empty() {
        let read = syn::LitStr::new(&unit_read_name(item_enum), proc_macro2::Span::call_site());
        for variant in &mut item_enum.variants {
            if units.contains(&variant.ident.to_string()) {
                variant
                    .attrs
                    .push(syn::parse_quote! { #[serde(deserialize_with = #read)] });
            }
        }
    }
    if closes_enum(item_enum, args) {
        item_enum
            .attrs
            .push(syn::parse_quote! { #[serde(deny_unknown_fields)] });
    }
}

/// The refusal of a strict struct under a tag of its own: serde cannot refuse a key beside the tag,
/// as `deny_unknown_fields` there refuses the tag itself, so the struct says `nonstrict` and every
/// surface reads past such a key as serde does.
fn own_tag_error(
    item_struct: &ItemStruct,
    args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    let (key, _named) = struct_tag(item_struct)?;
    if args.nonstrict || !args.preprocess.is_empty() || !derives_serde(&item_struct.attrs) {
        return None;
    }
    let written = item_struct.attrs.iter().find(|attr| {
        attr.path().is_ident("serde")
            && quote::ToTokens::to_token_stream(attr)
                .to_string()
                .contains("tag")
    })?;
    let name = item_struct.ident.unraw();
    Some(
        syn::Error::new_spanned(
            written,
            prefixed_guard_message(&format!(
                "type `{name}` is under a tag of its own, `{key}`, and serde cannot refuse a key \
                 no member declares beside it: `#[serde(deny_unknown_fields)]` there refuses the \
                 tag itself. Mark `{name}` `#[model_schema(nonstrict)]`, so every surface reads \
                 past such a key as serde does"
            )),
        )
        .to_compile_error(),
    )
}

/// The refusal of a strict struct that flattens a member which reads every key still left and
/// refuses none, where no member it flattens refuses such a key: serde cannot refuse a key no
/// member declares beside it, so the struct says `nonstrict` and every surface reads past one.
fn reads_past_a_stray_error(
    item_struct: &ItemStruct,
    args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    if !matches!(item_struct.fields, Fields::Named(_))
        || !reads_a_member(&item_struct.fields)
        || struct_tag(item_struct).is_some()
        || !writes_deny(&item_struct.attrs, args)
        || refused_by_a_member(&item_struct.fields, &item_struct.generics)
    {
        return None;
    }
    let (field, message) = open_member_refusal(item_struct)?;
    Some(syn::Error::new_spanned(field, prefixed_guard_message(&message)).to_compile_error())
}

/// The refusal of a strict enum a struct variant of which flattens a member that reads every key
/// still left and refuses none, where no variant refuses such a key. [`held_open_refusal`] answers
/// instead where another level of the enum would refuse one, with the newtype fix.
fn variant_reads_past_a_stray_error(
    item_enum: &ItemEnum,
    args: &ModelSchemaArgs,
) -> Option<proc_macro2::TokenStream> {
    if !writes_deny(&item_enum.attrs, args)
        || a_variant_refuses_a_stray(item_enum)
        || held_open_refusal(item_enum, args).is_some()
    {
        return None;
    }
    let (field, message) = open_variant_refusal(item_enum)?;
    Some(syn::Error::new_spanned(field, prefixed_guard_message(&message)).to_compile_error())
}

/// The refusal of a strict struct or enum serde cannot close, by [`own_tag_error`],
/// [`reads_past_a_stray_error`] and [`variant_reads_past_a_stray_error`], and of a `nonstrict` type
/// whose author also wrote `deny_unknown_fields`, which asks for the opposite.
pub(super) fn conflict_errors(
    item: &Item,
    args: &ModelSchemaArgs,
) -> Vec<proc_macro2::TokenStream> {
    let (attrs, cannot_close) = if let Item::Struct(item_struct) = item {
        (
            &item_struct.attrs,
            own_tag_error(item_struct, args)
                .or_else(|| reads_past_a_stray_error(item_struct, args)),
        )
    } else if let Item::Enum(item_enum) = item {
        (
            &item_enum.attrs,
            variant_reads_past_a_stray_error(item_enum, args),
        )
    } else {
        return Vec::new();
    };
    if let Some(refused) = cannot_close {
        return vec![refused];
    }
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
