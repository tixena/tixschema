//! The std ranges and the std `Result` an item writes: the check, beside the item, that one
//! written by its bare name is the std type every surface describes, the refusal of one under
//! `#[serde(flatten)]`, and that of one as a map key in a build whose map key guard does not run.
//! Beside them, the refusal of a map under `#[serde(flatten)]` beside another member or a tag, or
//! as the lone slot of a variant under a tag alone, written there or filling a type parameter a
//! type flattens there, of a map keyed by a number or a `bool` at any depth below a flattened
//! member or in a variant of an enum serde buffers, written there or filling a type parameter read
//! there, and `serde_json`'s `Value` and `Map`: refused by their full path, and checked beside the
//! item by their bare name.

#[cfg(feature = "serde")]
use core::ptr;

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens as _, quote, quote_spanned};
use syn::ext::IdentExt as _;
use syn::spanned::Spanned as _;
use syn::{Fields, GenericArgument, Item, PathArguments, Type, TypePath};

use super::judged_read::unclaimed_parameter;
use crate::features::serde::parse_serde_key_omission;
use crate::field_type::{
    RangeKind, names_std_ops, names_std_result, written_std_range, written_std_result,
};
use crate::utils::{written_names, written_type};

#[cfg(any(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
use super::prefixed_guard_message;
#[cfg(feature = "serde")]
use super::{
    KeyAsked, RefusedKey, bson_refused_keys, flattening_levels, forwarded_parameter_position,
    is_bson_refused_key, item_generics, item_label, item_schema_ident, placed_tokens,
    refused_member_named, type_arguments_of, writes_an_optional, written_as_an_entry,
    written_spelling,
};
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
use super::{WRITTEN_AS_OBJECT, unwritable_map_key_message};
#[cfg(feature = "serde")]
use crate::features::serde::{
    Tagging, brand_field, has_serde_read_hook, parse_serde_field_attributes, struct_tag,
};
#[cfg(any(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
use crate::field_type::FieldDef;
#[cfg(feature = "serde")]
use crate::field_type::{
    FieldDefType, get_field_def, is_ownership_wrapper, is_transparent_wrapper,
};
#[cfg(feature = "serde")]
use crate::utils::{
    FlattenedBeside, FlattenedParameter, FlattenedRead, TextKeyParameter, TextKeysRead,
    flattened_parameters, is_open_map, open_map_parameters, text_key_parameters,
};

/// What every refusal of a key serde reads below a flattened member says of serde.
#[cfg(feature = "serde")]
const KEYS_BELOW_A_FLATTEN: &str = "serde reads every key below a flattened member as text, and \
     reads no number and no `bool` from text, so no entry of that map reads back.";

/// What every refusal of a key serde reads in an untagged enum says of serde.
#[cfg(feature = "serde")]
const KEYS_IN_AN_UNTAGGED_ENUM: &str = "serde reads an untagged enum from a copy of the payload \
     it buffers, where every key is text, and reads no number and no `bool` from text, so no \
     entry of that map reads back.";

/// What every refusal of a key serde reads in an enum under a tag alone says of serde.
#[cfg(feature = "serde")]
const KEYS_IN_A_TAGGED_ENUM: &str = "serde reads an enum under a tag alone from a copy of the \
     payload it buffers, where every key is text, and reads no number and no `bool` from text, \
     so no entry of that map reads back.";

/// A `serde_json` type that holds any JSON value, where a written type reaches it.
enum AnyJson<'ty> {
    /// `Value` with no arguments or `Map` with arguments, written by its bare name.
    Bare(&'static str, &'ty Type),
    /// `serde_json::Value` or `serde_json::Map`, written by its full path.
    Named(&'static str),
}

/// A member of an item that serde reads with every key as text.
#[cfg(feature = "serde")]
struct TextReadMember<'item> {
    field: &'item syn::Field,
    /// The item, the variant and the member, as a refusal of the member opens.
    subject: String,
    under: TextKeysRead,
}

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

/// The first of `serde_json`'s two types that hold any JSON value, `Value` and `Map`, `ty` reaches
/// by its full path, with or without a leading `::`.
pub fn any_json_named(ty: &Type) -> Option<&'static str> {
    any_json_reached(ty)
        .into_iter()
        .find_map(|reached| match reached {
            AnyJson::Named(name) => Some(name),
            AnyJson::Bare(..) => None,
        })
}

/// Each `Value` with no arguments and each `Map` with arguments `ty` reaches by its bare name, once
/// per spelling, where `kept` does not answer for the name and a check beside the declaration can
/// name the type: it names none of `parameters`, no `Self` and no lifetime but `'static`.
pub fn any_json_bare<'ty, F>(
    ty: &'ty Type,
    kept: F,
    parameters: &[String],
) -> Vec<(&'static str, &'ty Type)>
where
    F: Fn(&str) -> bool,
{
    let mut bare: Vec<(&'static str, &'ty Type)> = Vec::new();
    for reached in any_json_reached(ty) {
        if let AnyJson::Bare(name, written) = reached
            && !kept(name)
            && !names_what_a_check_cannot(written.to_token_stream(), parameters)
            && !bare.iter().any(|&(_, seen)| seen == written)
        {
            bare.push((name, written));
        }
    }
    bare
}

/// The check beside a declaration that `probed`, a bare `name` it writes, is no `serde_json` type:
/// the probe's inherent const answers only where it is, and the evaluation then fails with
/// `refusal`, spanned on `probed`.
pub fn any_json_check(name: &str, probed: &Type, refusal: &str) -> TokenStream {
    let held = if name == "Map" {
        quote! { ::serde_json::Map<::std::string::String, ::serde_json::Value> }
    } else {
        quote! { ::serde_json::Value }
    };
    let written = written_names(probed.to_token_stream());
    let probe = unclaimed_parameter("Probe", &written);
    let fallback = unclaimed_parameter("Fallback", &written);
    // The message is a format string: a brace in it is written twice.
    let message = refusal.replace('{', "{{").replace('}', "}}");
    let refused = quote_spanned! { probed.span()=>
        ::core::assert!(!#probe::<#probed>::ANY_JSON, #message);
    };
    quote! {
        const _: () = {
            struct #probe<T: ?::core::marker::Sized>(::core::marker::PhantomData<T>);
            trait #fallback {
                const ANY_JSON: bool = false;
            }
            impl<T: ?::core::marker::Sized> #fallback for #probe<T> {}
            impl #probe<#held> {
                const ANY_JSON: bool = true;
            }
            ::core::assert!(
                #probe::<#held>::ANY_JSON,
                "the probe answers for serde_json's own type"
            );
            #refused
        };
    }
}

/// Every `serde_json::Value` and `serde_json::Map` `ty` reaches by its full path or by its bare
/// name, in the order written: in a path's type arguments, a reference, an array, a slice, a tuple
/// and parentheses.
fn any_json_reached(ty: &Type) -> Vec<AnyJson<'_>> {
    let mut reached = Vec::new();
    collect_any_json(ty, &mut reached);
    reached
}

/// Adds what `ty` reaches of [`any_json_reached`] to `reached`.
fn collect_any_json<'ty>(ty: &'ty Type, reached: &mut Vec<AnyJson<'ty>>) {
    let written = written_type(ty);
    if let Type::Path(path) = written {
        reached.extend(any_json_path(written, path));
        for segment in &path.path.segments {
            if let PathArguments::AngleBracketed(arguments) = &segment.arguments {
                for argument in &arguments.args {
                    if let GenericArgument::Type(held) = argument {
                        collect_any_json(held, reached);
                    }
                }
            }
        }
    } else if let Type::Reference(reference) = written {
        collect_any_json(&reference.elem, reached);
    } else if let Type::Array(array) = written {
        collect_any_json(&array.elem, reached);
    } else if let Type::Slice(slice) = written {
        collect_any_json(&slice.elem, reached);
    } else if let Type::Tuple(tuple) = written {
        for position in &tuple.elems {
            collect_any_json(position, reached);
        }
    } else if let Type::Paren(paren) = written {
        collect_any_json(&paren.elem, reached);
    } else {
        // Every other type reaches neither name.
    }
}

/// What `path`, written as `written`, names of `serde_json`'s two types.
fn any_json_path<'ty>(written: &'ty Type, path: &TypePath) -> Option<AnyJson<'ty>> {
    if path.qself.is_some() {
        return None;
    }
    let segments: Vec<&syn::PathSegment> = path.path.segments.iter().collect();
    if let [owner, last] = segments.as_slice() {
        return if owner.ident == "serde_json" && owner.arguments.is_none() {
            any_json_name(&last.ident).map(AnyJson::Named)
        } else {
            None
        };
    }
    let [only] = segments.as_slice() else {
        return None;
    };
    if path.path.leading_colon.is_some() {
        return None;
    }
    match (any_json_name(&only.ident)?, &only.arguments) {
        ("Value", PathArguments::None) => Some(AnyJson::Bare("Value", written)),
        ("Map", PathArguments::AngleBracketed(_)) => Some(AnyJson::Bare("Map", written)),
        _ => None,
    }
}

/// `Value` or `Map`, where `ident` is one of the two.
fn any_json_name(ident: &Ident) -> Option<&'static str> {
    let name = ident.unraw();
    if name == "Value" {
        Some("Value")
    } else if name == "Map" {
        Some("Map")
    } else {
        None
    }
}

/// Whether `tokens` name one of `parameters`, `Self` or a lifetime other than `'static`, none of
/// which a check written beside the declaration can name.
fn names_what_a_check_cannot(tokens: TokenStream, parameters: &[String]) -> bool {
    let mut after_a_quote = false;
    tokens.into_iter().any(|token| {
        let unnameable = match &token {
            TokenTree::Group(group) => names_what_a_check_cannot(group.stream(), parameters),
            TokenTree::Ident(ident) => {
                let name = ident.unraw().to_string();
                if after_a_quote {
                    name != "static"
                } else {
                    name == "Self" || parameters.contains(&name)
                }
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => false,
        };
        after_a_quote = matches!(&token, TokenTree::Punct(punct) if punct.as_char() == '\'');
        unnameable
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

/// Refuses each member of `item` serde reads with every key as text, a member under
/// `#[serde(flatten)]` and a member of a variant of an untagged enum or of an enum under a tag
/// alone, that holds, at any depth, a map keyed by a number or a `bool`. A member that holds the
/// item itself reaches no map its own members do not.
#[cfg(feature = "serde")]
pub fn text_read_map_key_errors(item: &Item) -> Vec<TokenStream> {
    let own = item_schema_ident(item).map(ToString::to_string);
    let is_own = |key: &RefusedKey<'_>| match key {
        RefusedKey::Held(held) => Some(&held.model) == own.as_ref(),
        RefusedKey::Written(_) => false,
    };
    text_read_members(item)
        .into_iter()
        .filter_map(|member| {
            let held = match bson_refused_keys(&member.field.ty, KeyAsked::Refused)
                .into_iter()
                .find(|key| !is_own(key))?
            {
                RefusedKey::Written(key) => format!("a map keyed by `{}`", written_spelling(key)),
                RefusedKey::Held(held) => format!(
                    "`{}`, which holds a map keyed by a number or a `bool` ({})",
                    held.model, held.described
                ),
            };
            let holder = match member.under {
                TextKeysRead::Flattened => "`#[serde(flatten)]`",
                TextKeysRead::TaggedAlone => "an enum under a tag alone",
                TextKeysRead::Untagged => "an untagged enum",
            };
            let message = format!(
                "{}: {holder} cannot hold {held} \u{2014} {}",
                member.subject,
                text_keys_refused(member.under, "field")
            );
            Some(
                syn::Error::new_spanned(member.field, prefixed_guard_message(&message))
                    .to_compile_error(),
            )
        })
        .collect()
}

/// What a refusal of a map keyed by a number or a `bool` says after its dash, where serde reads
/// the map `under` that: why no entry reads back, and what to write instead of the `member`.
#[cfg(feature = "serde")]
fn text_keys_refused(under: TextKeysRead, member: &str) -> String {
    match under {
        TextKeysRead::Flattened => format!(
            "{KEYS_BELOW_A_FLATTEN} Key the map by a `String`, or write the {member} under a key \
             of its own."
        ),
        TextKeysRead::TaggedAlone => format!(
            "{KEYS_IN_A_TAGGED_ENUM} Key the map by a `String`, or name a `content` key: serde \
             writes the tag ahead of the content, and reads the content that follows it without \
             a copy."
        ),
        TextKeysRead::Untagged => format!("{KEYS_IN_AN_UNTAGGED_ENUM} Key the map by a `String`."),
    }
}

/// Each member of `item` serde reads with every key as text: a member under `#[serde(flatten)]`,
/// and every other member of a variant of an untagged enum or of an enum under a tag alone, which
/// serde reads from a copy of the payload it buffers. A member or a variant serde does not read,
/// and a member read through its author's hook, is not one.
#[cfg(feature = "serde")]
fn text_read_members(item: &Item) -> Vec<TextReadMember<'_>> {
    let label = item_label(item);
    let (levels, buffered): (Vec<(Option<&Ident>, &Fields)>, _) =
        if let Item::Struct(item_struct) = item {
            (vec![(None, &item_struct.fields)], None)
        } else if let Item::Enum(item_enum) = item {
            let buffered = match Tagging::of(item_enum) {
                Tagging::Internal { tag: _tag } => Some(TextKeysRead::TaggedAlone),
                Tagging::Untagged => Some(TextKeysRead::Untagged),
                Tagging::Adjacent {
                    content: _content,
                    tag: _adjacent,
                } => None,
                Tagging::External | Tagging::Plain => None,
            };
            let levels = item_enum
                .variants
                .iter()
                .filter(|variant| read_by_serde(&variant.attrs))
                .map(|variant| (Some(&variant.ident), &variant.fields))
                .collect();
            (levels, buffered)
        } else {
            return Vec::new();
        };
    let parameters = item_generics(item).map_or_default(parameter_names);
    let mut members = Vec::new();
    for (variant, fields) in levels {
        // The lone slot of a variant under a tag alone that writes a map is refused beside the
        // tag, whatever keys it.
        let beside_the_tag = buffered == Some(TextKeysRead::TaggedAlone)
            && matches!(fields, Fields::Unnamed(slots) if slots.unnamed.len() == 1);
        for (position, field) in fields.iter().enumerate() {
            if !read_by_serde(&field.attrs)
                || (beside_the_tag && writes_a_map(&field.ty, &parameters))
            {
                continue;
            }
            let under = if parse_serde_field_attributes(&field.attrs).flatten {
                TextKeysRead::Flattened
            } else if let Some(under) = buffered {
                under
            } else {
                continue;
            };
            members.push(TextReadMember {
                field,
                subject: format!(
                    "{label}: {}",
                    refused_member_named(variant, position, field)
                ),
                under,
            });
        }
    }
    members
}

/// Whether serde reads the variant or the member `attrs` are written on, and reads it itself
/// rather than through a function of its author's.
#[cfg(feature = "serde")]
fn read_by_serde(attrs: &[syn::Attribute]) -> bool {
    !parse_serde_key_omission(attrs).skips_deserializing && !has_serde_read_hook(attrs)
}

/// The type parameters `item` reads, in a value of its own, where serde reads every key as text,
/// as a map key or anywhere else: in each member [`text_read_members`] answers, and where it
/// fills, with one of them, a type that reads a parameter so.
#[cfg(feature = "serde")]
pub fn text_key_parameters_of(item: &Item) -> Vec<TextKeyParameter> {
    let (Some(ident), Some(generics)) = (item_schema_ident(item), item_generics(item)) else {
        return Vec::new();
    };
    let parameters = parameter_names(generics);
    if parameters.is_empty() {
        return Vec::new();
    }
    let flattens = ident.to_string();
    let named = |at: usize, read: FlattenedRead, under: TextKeysRead| TextKeyParameter {
        at,
        flattens: flattens.clone(),
        parameter: parameters[at].clone(),
        read,
        under,
    };
    let mut held = Vec::new();
    for member in text_read_members(item) {
        held.extend(
            parameters_read_below(&member.field.ty, generics)
                .into_iter()
                .map(|(at, read)| named(at, read, member.under)),
        );
    }
    let own = held.clone();
    let recorded = |name: &str| {
        if name == flattens {
            own.clone()
        } else {
            text_key_parameters(name)
        }
    };
    let mut fillings = Vec::new();
    for (_, ty) in written_types(item) {
        flattened_fillings(ty, &recorded, text_key_at, &mut fillings);
    }
    for (argument, filled) in fillings {
        let handed_on = key_parameter(argument, &parameters);
        let reads = match filled.read {
            FlattenedRead::Key => handed_on
                .map(|at| (at, FlattenedRead::Key))
                .into_iter()
                .collect(),
            FlattenedRead::Below => parameters_read_below(argument, generics),
        };
        for (at, read) in reads {
            // A parameter handed on whole stands where the type that reads it declares its own;
            // one handed on inside another type is read by the item itself.
            held.push(if handed_on == Some(at) && read == filled.read {
                TextKeyParameter {
                    at,
                    ..filled.clone()
                }
            } else {
                named(at, read, filled.under)
            });
        }
    }
    // One record per parameter and read: a filling that refuses it is refused once.
    let mut seen: Vec<(usize, FlattenedRead)> = Vec::new();
    held.retain(|filled| {
        let first = !seen.contains(&(filled.at, filled.read));
        seen.push((filled.at, filled.read));
        first
    });
    held
}

/// The positions of the type parameters of `generics` that `ty`, read where serde reads every key
/// as text, holds: as the key of a map, as the BSON walk finds one at any depth, and anywhere at
/// all.
#[cfg(feature = "serde")]
fn parameters_read_below(ty: &Type, generics: &syn::Generics) -> Vec<(usize, FlattenedRead)> {
    let declared: Vec<&Ident> = generics
        .type_params()
        .map(|declared| &declared.ident)
        .collect();
    let written = written_names(ty.to_token_stream());
    let placed: Option<Type> = syn::parse2(placed_tokens(ty.to_token_stream(), &declared)).ok();
    declared
        .iter()
        .enumerate()
        .flat_map(|(at, parameter)| {
            let keyed = placed.as_ref().is_some_and(|written_placed| {
                !bson_refused_keys(written_placed, KeyAsked::Parameter(at)).is_empty()
            });
            let held = written.contains(&parameter.unraw().to_string());
            keyed
                .then_some((at, FlattenedRead::Key))
                .into_iter()
                .chain(held.then_some((at, FlattenedRead::Below)))
        })
        .collect()
}

/// The position among `parameters` of `key`, where it is one of them, under any `Box`, `Rc`, `Arc`
/// and `Cow`.
#[cfg(feature = "serde")]
fn key_parameter(key: &Type, parameters: &[String]) -> Option<usize> {
    let mut current = written_type(key);
    loop {
        let Type::Path(named) = current else {
            return None;
        };
        if let Some(ident) = named.path.get_ident() {
            return parameters.iter().position(|parameter| ident == parameter);
        }
        let last = named.path.segments.last()?;
        if !is_ownership_wrapper(&last.ident.to_string()) {
            return None;
        }
        current = written_type(type_arguments_of(&last.arguments).next()?);
    }
}

/// The refusal of each filling, in a type `item` writes, of a type parameter a type reads where
/// serde reads every key as text, where it keys a map with a number or a `bool`, or holds a map so
/// keyed, spanned on the filling: in its members serde reads otherwise, which
/// [`text_read_map_key_errors`] leaves to this, in an alias's value, and in each of
/// `default_types`, where the entry fills the item's own parameter.
#[cfg(feature = "serde")]
pub fn filled_text_key_errors(item: &Item, default_types: &[(Ident, Type)]) -> Vec<TokenStream> {
    let Some(generics) = item_generics(item) else {
        return Vec::new();
    };
    let parameters = parameter_names(generics);
    let label = item_label(item);
    let flattens = item_schema_ident(item).map(ToString::to_string);
    let own = text_key_parameters_of(item);
    let recorded = |name: &str| {
        if flattens.as_deref() == Some(name) {
            own.clone()
        } else {
            text_key_parameters(name)
        }
    };
    // What follows "fills it" in the refusal of a filling, where it is refused.
    let refused_by = |argument: &Type, filled: &TextKeyParameter| match filled.read {
        FlattenedRead::Key => (key_parameter(argument, &parameters).is_none()
            && is_bson_refused_key(argument))
        .then(String::new),
        FlattenedRead::Below => {
            bson_refused_keys(argument, KeyAsked::Refused)
                .first()
                .map(|refused_key| match refused_key {
                    RefusedKey::Written(key) => {
                        format!(" with a map keyed by `{}`", written_spelling(key))
                    }
                    RefusedKey::Held(held) if held.model == written_spelling(argument) => format!(
                        ", which holds a map keyed by a number or a `bool` ({})",
                        held.described
                    ),
                    RefusedKey::Held(held) => format!(
                        " with `{}`, which holds a map keyed by a number or a `bool` ({})",
                        held.model, held.described
                    ),
                })
        }
    };
    let text_read = text_read_member_types(item);
    let mut refused: Vec<(String, &Type, TextKeyParameter, String)> = Vec::new();
    for (member, ty) in written_types(item)
        .into_iter()
        .filter(|(_, ty)| !text_read.iter().any(|read| ptr::eq(*read, *ty)))
    {
        let subject = member.map_or_else(
            || format!("{label}: "),
            |named| format!("{label}: {named}: "),
        );
        let mut fillings = Vec::new();
        flattened_fillings(ty, &recorded, text_key_at, &mut fillings);
        for (argument, filled) in fillings {
            if let Some(filling) = refused_by(argument, &filled) {
                refused.push((subject.clone(), argument, filled, filling));
            }
        }
    }
    for (entry, filling) in default_types {
        let subject = format!("{label}: `default_types` entry `{entry}`: ");
        let at = parameters.iter().position(|parameter| entry == parameter);
        for filled in own.iter().filter(|filled| Some(filled.at) == at) {
            if let Some(filled_with) = refused_by(filling, filled) {
                refused.push((subject.clone(), filling, filled.clone(), filled_with));
            }
        }
        let mut fillings = Vec::new();
        flattened_fillings(filling, &recorded, text_key_at, &mut fillings);
        for (argument, filled) in fillings {
            if let Some(filled_with) = refused_by(argument, &filled) {
                refused.push((subject.clone(), argument, filled, filled_with));
            }
        }
    }
    refused
        .into_iter()
        .map(|(subject, argument, filled, filled_with)| {
            let reads = match filled.read {
                FlattenedRead::Key => "as the key of a map ",
                FlattenedRead::Below => "",
            };
            let read_where = match filled.under {
                TextKeysRead::Flattened => "below `#[serde(flatten)]`",
                TextKeysRead::TaggedAlone => "in an enum under a tag alone",
                TextKeysRead::Untagged => "in an untagged enum",
            };
            let message = format!(
                "{subject}`{}` reads its type parameter `{}` {reads}{read_where}, and `{}` fills \
                 it{filled_with} \u{2014} {}",
                filled.flattens,
                filled.parameter,
                written_spelling(argument),
                text_keys_refused(filled.under, "member")
            );
            syn::Error::new_spanned(argument, prefixed_guard_message(&message)).to_compile_error()
        })
        .collect()
}

/// The type of each member of `item` serde reads with every key as text, which
/// [`text_read_map_key_errors`] answers for whole.
#[cfg(feature = "serde")]
fn text_read_member_types(item: &Item) -> Vec<&Type> {
    text_read_members(item)
        .into_iter()
        .map(|member| &member.field.ty)
        .collect()
}

/// Refuses `slot`, the lone slot of `variant` of `item_enum` under the tag `tag` alone, where it
/// writes a map: serde writes the slot's value beside the tag, so the tag and the map's entries
/// share one level, as for a map flattened beside the tag.
#[cfg(feature = "serde")]
pub fn tagged_slot_map_error(
    item_enum: &syn::ItemEnum,
    variant: &syn::Variant,
    slot: &syn::Field,
    tag: &str,
) -> Option<TokenStream> {
    if !writes_a_map(&slot.ty, &parameter_names(&item_enum.generics)) {
        return None;
    }
    let message = format!(
        "type `{}`: {}: `{}` writes a map, which cannot sit beside the tag `{tag}` \u{2014} serde \
         writes the tag and the map's entries at one level, and no TypeScript type describes both. \
         Name a `content` key so the map gets an object of its own, or write the map under a key \
         of its own.",
        item_enum.ident.unraw(),
        refused_member_named(Some(&variant.ident), 0, slot),
        written_spelling(&slot.ty)
    );
    Some(syn::Error::new_spanned(slot, prefixed_guard_message(&message)).to_compile_error())
}

/// Whether `item` is a type serde writes, flattened, as the entries of a map: a struct whose one
/// member on the wire is a flattened map, a brand or a struct of one slot over a map, an alias of
/// one, and an untagged enum with a variant serde writes as one.
#[cfg(feature = "serde")]
pub fn writes_an_open_map(item: &Item) -> bool {
    let parameters = item_generics(item).map_or_default(parameter_names);
    written_as_itself(item)
        .into_iter()
        .any(|ty| writes_a_map(ty, &parameters))
}

/// The positions of the type parameters of `item` whose filling serde writes, flattened, as the
/// entries of a value of `item`, where `item` writes it as it writes a map in
/// [`writes_an_open_map`].
#[cfg(feature = "serde")]
pub fn open_map_parameter_positions(item: &Item) -> Vec<usize> {
    let mut positions: Vec<usize> = written_as_itself(item)
        .into_iter()
        .flat_map(|ty| parameters_written_as(item, ty))
        .collect();
    positions.sort_unstable();
    positions.dedup();
    positions
}

/// The types serde writes, flattened, as the entries of a value of `item` itself: a brand's value,
/// the one slot of a struct, the one member on the wire of a struct with no tag of its own where
/// it is flattened, an alias's value, and the same of each variant of an untagged enum.
#[cfg(feature = "serde")]
fn written_as_itself(item: &Item) -> Vec<&Type> {
    if let Item::Struct(item_struct) = item {
        if let Some(inner) = brand_field(item_struct) {
            return vec![&inner.ty];
        }
        // A struct's own tag stands beside its flattened member, which `filled_map_errors` refuses
        // where a map fills it.
        if struct_tag(item_struct).is_some() {
            return Vec::new();
        }
        entries_of_its_own(&item_struct.fields)
    } else if let Item::Enum(item_enum) = item {
        if !matches!(Tagging::of(item_enum), Tagging::Untagged) {
            return Vec::new();
        }
        item_enum
            .variants
            .iter()
            .filter(|variant| !parse_serde_key_omission(&variant.attrs).absent_from_wire())
            .flat_map(|variant| entries_of_its_own(&variant.fields))
            .collect()
    } else if let Item::Type(item_type) = item {
        vec![&*item_type.ty]
    } else {
        Vec::new()
    }
}

/// The type of the member of `fields` serde writes as the value that holds it: its one slot, or
/// its one member on the wire where that member is flattened.
#[cfg(feature = "serde")]
fn entries_of_its_own(fields: &Fields) -> Vec<&Type> {
    if let Fields::Unnamed(slots) = fields {
        return if slots.unnamed.len() == 1 {
            slots.unnamed.iter().map(|slot| &slot.ty).collect()
        } else {
            Vec::new()
        };
    }
    let mut written = fields
        .iter()
        .filter(|field| !parse_serde_key_omission(&field.attrs).absent_from_wire());
    match (written.next(), written.next()) {
        (Some(only), None) if parse_serde_field_attributes(&only.attrs).flatten => vec![&only.ty],
        _ => Vec::new(),
    }
}

/// The positions among `item`'s type parameters of each one serde writes `ty` as: through
/// `Option` and the transparent wrappers, through what [`forwarded_parameter_position`] reads, and
/// through a type declared above at each position it writes as itself.
#[cfg(feature = "serde")]
fn parameters_written_as(item: &Item, ty: &Type) -> Vec<usize> {
    let held = unwrapped(ty);
    if let Some(at) = forwarded_parameter_position(item, held) {
        return vec![at];
    }
    let Type::Path(named) = held else {
        return Vec::new();
    };
    let Some(last) = named.path.segments.last().filter(|_| named.qself.is_none()) else {
        return Vec::new();
    };
    let arguments: Vec<&Type> = type_arguments_of(&last.arguments).collect();
    open_map_parameters(&last.ident.to_string())
        .into_iter()
        .filter_map(|at| arguments.get(at))
        .flat_map(|argument| parameters_written_as(item, argument))
        .collect()
}

/// `ty` under every `Option` and transparent wrapper it is written in, as [`get_field_def`] reads
/// through them.
#[cfg(feature = "serde")]
fn unwrapped(ty: &Type) -> &Type {
    let mut current = written_type(ty);
    while let Type::Path(named) = current
        && let Some(last) = named.path.segments.last()
        && (last.ident == "Option" || is_transparent_wrapper(&last.ident.to_string()))
        && let Some(inner) = type_arguments_of(&last.arguments).next()
    {
        current = written_type(inner);
    }
    current
}

/// Whether `field` is a named field under `#[serde(flatten)]` whose type writes a map.
#[cfg(feature = "serde")]
fn flattened_map(field: &syn::Field, parameters: &[String]) -> bool {
    parse_serde_field_attributes(&field.attrs).flatten && writes_a_map(&field.ty, parameters)
}

/// Whether serde writes `ty` as a map, through `Option` and the transparent wrappers: a map, or a
/// type declared above that writes one, itself or at what fills its type parameter.
#[cfg(feature = "serde")]
fn writes_a_map(ty: &Type, parameters: &[String]) -> bool {
    let mut def = get_field_def("", ty, "");
    def.erase_type_parameters(parameters);
    def_writes_a_map(&def)
}

/// Whether serde writes `def` as a map: a map, a name recorded as an open map, or a name recorded
/// as writing its argument at a position as itself, filled there with a map.
#[cfg(feature = "serde")]
pub fn def_writes_a_map(def: &FieldDef) -> bool {
    if def.array_depth != 0 {
        return false;
    }
    if let FieldDefType::SiblingType(name, arguments) = &def.field_type {
        return is_open_map(name)
            || open_map_parameters(name)
                .into_iter()
                .any(|at| arguments.get(at).is_some_and(def_writes_a_map));
    }
    matches!(def.field_type, FieldDefType::Map(_, _))
}

/// The type parameters `item` flattens, each with the key serde writes beside it at the same level
/// where it writes one: at each of its own levels, and where it fills, with one of them, a type
/// that flattens a parameter.
#[cfg(feature = "serde")]
pub fn flattened_parameters_of(item: &Item) -> Vec<FlattenedParameter> {
    let (Some(ident), Some(generics)) = (item_schema_ident(item), item_generics(item)) else {
        return Vec::new();
    };
    let parameters = parameter_names(generics);
    if parameters.is_empty() {
        return Vec::new();
    }
    let flattens = ident.to_string();
    let mut held = Vec::new();
    for (_, fields, tag) in flattening_levels(item) {
        let written: Vec<&syn::Field> = fields
            .iter()
            .filter(|field| !parse_serde_key_omission(&field.attrs).absent_from_wire())
            .collect();
        // serde writes the lone slot of a variant under a tag alone beside the tag, as it writes a
        // flattened member there.
        let slot_beside_tag =
            tag.is_some() && matches!(fields, Fields::Unnamed(slots) if slots.unnamed.len() == 1);
        for (index, field) in written.iter().enumerate() {
            if !slot_beside_tag && !parse_serde_field_attributes(&field.attrs).flatten {
                continue;
            }
            let beside = written
                .iter()
                .enumerate()
                .find(|(other, _)| *other != index)
                .and_then(|(_, other)| other.ident.as_ref())
                .map(|member| FlattenedBeside::Member(member.unraw().to_string()))
                .or_else(|| tag.clone().map(FlattenedBeside::Tag));
            for at in parameters_written_as(item, &field.ty) {
                held.push(FlattenedParameter {
                    at,
                    beside: beside.clone(),
                    flattens: flattens.clone(),
                    parameter: parameters[at].clone(),
                });
            }
        }
    }
    let own = held.clone();
    let recorded = |name: &str| {
        if name == flattens {
            own.clone()
        } else {
            flattened_parameters(name)
        }
    };
    let mut fillings = Vec::new();
    for (_, ty) in written_types(item) {
        flattened_fillings(ty, &recorded, flattened_at, &mut fillings);
    }
    for (argument, filled) in fillings {
        for at in parameters_written_as(item, argument) {
            held.push(FlattenedParameter {
                at,
                ..filled.clone()
            });
        }
    }
    // One record per parameter, one beside a key ahead of one alone: a filling is refused once,
    // and a map that fills it wherever it sits beside a key.
    let mut kept: Vec<FlattenedParameter> = Vec::new();
    for filled in held {
        match kept.iter_mut().find(|other| other.at == filled.at) {
            None => kept.push(filled),
            Some(other) if other.beside.is_none() && filled.beside.is_some() => *other = filled,
            Some(_) => {}
        }
    }
    kept
}

/// Each argument that fills, in a type `item` writes, a type parameter a type flattens, where
/// `refuses` answers for it, beside the subject its refusal opens with, what is recorded of the
/// parameter and the answer: in its members, in an alias's value, and in each of `default_types`,
/// where the entry fills the item's own parameter.
#[cfg(feature = "serde")]
fn refused_fillings<'item, T, R>(
    item: &'item Item,
    default_types: &'item [(Ident, Type)],
    refuses: R,
) -> Vec<(String, &'item Type, FlattenedParameter, T)>
where
    R: Fn(&Type, &[String], &FlattenedParameter) -> Option<T>,
{
    let Some(generics) = item_generics(item) else {
        return Vec::new();
    };
    let parameters = parameter_names(generics);
    let label = item_label(item);
    let flattens = item_schema_ident(item).map(ToString::to_string);
    let own = flattened_parameters_of(item);
    let recorded = |name: &str| {
        if flattens.as_deref() == Some(name) {
            own.clone()
        } else {
            flattened_parameters(name)
        }
    };
    let mut answered = Vec::new();
    let mut refuse = |subject: &str, argument: &'item Type, filled: FlattenedParameter| {
        if let Some(answer) = refuses(argument, &parameters, &filled) {
            answered.push((subject.to_owned(), argument, filled, answer));
        }
    };
    for (member, ty) in written_types(item) {
        let subject = member.map_or_else(
            || format!("{label}: "),
            |named| format!("{label}: {named}: "),
        );
        let mut fillings = Vec::new();
        flattened_fillings(ty, &recorded, flattened_at, &mut fillings);
        for (argument, filled) in fillings {
            refuse(&subject, argument, filled);
        }
    }
    for (entry, filling) in default_types {
        let subject = format!("{label}: `default_types` entry `{entry}`: ");
        let at = parameters.iter().position(|parameter| entry == parameter);
        for filled in own.iter().filter(|filled| Some(filled.at) == at) {
            refuse(&subject, filling, filled.clone());
        }
        let mut fillings = Vec::new();
        flattened_fillings(filling, &recorded, flattened_at, &mut fillings);
        for (argument, filled) in fillings {
            refuse(&subject, argument, filled);
        }
    }
    answered
}

/// The refusal of each map that fills, in a type `item` writes, a type parameter a type flattens
/// beside a key serde writes at the same level, spanned on the map: in its members, in an alias's
/// value, and in each of `default_types`, where the entry fills the item's own parameter.
#[cfg(feature = "serde")]
pub fn filled_map_errors(item: &Item, default_types: &[(Ident, Type)]) -> Vec<TokenStream> {
    refused_fillings(item, default_types, |argument, parameters, filled| {
        let beside = filled.beside.clone()?;
        writes_a_map(argument, parameters).then_some(beside)
    })
    .into_iter()
    .map(|(subject, argument, filled, beside)| {
        let message = filled_map_message(&subject, argument, &filled, &beside);
        syn::Error::new_spanned(argument, prefixed_guard_message(&message)).to_compile_error()
    })
    .collect()
}

/// The refusal of `argument`, a map, filling the parameter `filled` holds `beside` a key.
#[cfg(feature = "serde")]
fn filled_map_message(
    subject: &str,
    argument: &Type,
    filled: &FlattenedParameter,
    beside: &FlattenedBeside,
) -> String {
    let flattened = format!(
        "{subject}`{}` flattens its type parameter `{}`",
        filled.flattens, filled.parameter
    );
    let filling = written_spelling(argument);
    match beside {
        FlattenedBeside::Member(member) => format!(
            "{flattened} beside `{member}`, and `{filling}` fills it with a map \u{2014} serde \
             writes the map's entries at one level with `{member}`, and no TypeScript type \
             describes both. Write the map under a key of its own."
        ),
        FlattenedBeside::Tag(key) => format!(
            "{flattened} beside the tag `{key}`, and `{filling}` fills it with a map \u{2014} \
             serde writes the tag and the map's entries at one level, and no TypeScript type \
             describes both. Write the map under a key of its own."
        ),
    }
}

/// The refusal of each enum serde writes a variant of as an entry of its name over `null` that
/// fills, in a type `item` writes, a type parameter a type flattens, spanned on the enum: in its
/// members, in an alias's value, and in each of `default_types`. The type's description is
/// written once for every filling, and takes the enum's own value, the variant's bare name. An
/// `Option` beside a tag is left to serde, which refuses every write of one there.
#[cfg(feature = "serde")]
pub fn filled_entry_errors(item: &Item, default_types: &[(Ident, Type)]) -> Vec<TokenStream> {
    refused_fillings(item, default_types, |argument, parameters, filled| {
        let mut def = get_field_def("", argument, "");
        def.erase_type_parameters(parameters);
        if matches!(filled.beside, Some(FlattenedBeside::Tag(_))) && writes_an_optional(&def) {
            return None;
        }
        written_as_an_entry(&def)
    })
    .into_iter()
    .map(|(subject, argument, filled, named)| {
        let message = filled_entry_message(&subject, argument, &filled, named);
        syn::Error::new_spanned(argument, prefixed_guard_message(&message)).to_compile_error()
    })
    .collect()
}

/// The refusal of `argument`, a value serde writes a variant of as an entry of its name over
/// `null`, filling the parameter `filled` holds. `named` says the value is an externally tagged
/// enum, which every surface describes by its entries where it is named in the parameter's place.
#[cfg(feature = "serde")]
fn filled_entry_message(
    subject: &str,
    argument: &Type,
    filled: &FlattenedParameter,
    named: bool,
) -> String {
    let flattens = &filled.flattens;
    let beside = match &filled.beside {
        Some(FlattenedBeside::Member(member)) => format!(" beside `{member}`"),
        Some(FlattenedBeside::Tag(key)) => format!(" beside the tag `{key}`"),
        None => String::new(),
    };
    let filling = written_spelling(argument);
    let advice = if named {
        "Name the enum where the type parameter stands, which every surface describes by its \
         entries, or write it under a key of its own."
    } else {
        "Write it under a key of its own."
    };
    format!(
        "{subject}`{flattens}` flattens its type parameter `{}`{beside}, and `{filling}` fills it \
         with a value serde writes a variant of there as an entry of its name over null \u{2014} \
         the TypeScript type and the Zod schema of `{flattens}`, written once for every filling, \
         describe that variant as its bare name. {advice}",
        filled.parameter
    )
}

/// Each type `item` writes, beside the member that writes it: every member on the wire of a struct
/// and of each variant on the wire of an enum, and an alias's value under no member.
#[cfg(feature = "serde")]
fn written_types(item: &Item) -> Vec<(Option<String>, &Type)> {
    let members: Vec<(Option<&Ident>, &Fields)> = if let Item::Struct(item_struct) = item {
        vec![(None, &item_struct.fields)]
    } else if let Item::Enum(item_enum) = item {
        item_enum
            .variants
            .iter()
            .filter(|variant| !parse_serde_key_omission(&variant.attrs).absent_from_wire())
            .map(|variant| (Some(&variant.ident), &variant.fields))
            .collect()
    } else if let Item::Type(item_type) = item {
        return vec![(None, &*item_type.ty)];
    } else {
        Vec::new()
    };
    members
        .into_iter()
        .flat_map(|(variant, fields)| {
            fields
                .iter()
                .enumerate()
                .filter(|(_, field)| !parse_serde_key_omission(&field.attrs).absent_from_wire())
                .map(move |(position, field)| {
                    (
                        Some(refused_member_named(variant, position, field)),
                        &field.ty,
                    )
                })
        })
        .collect()
}

/// Each argument `ty` fills a type parameter with, at any depth, where `recorded` answers for the
/// type it is handed to, by that type's name, beside what is recorded of the parameter, whose
/// position `at` reads.
#[cfg(feature = "serde")]
fn flattened_fillings<'ty, R>(
    ty: &'ty Type,
    recorded: &dyn Fn(&str) -> Vec<R>,
    at: fn(&R) -> usize,
    found: &mut Vec<(&'ty Type, R)>,
) {
    match written_type(ty) {
        Type::Path(named) => {
            let Some(last) = named.path.segments.last() else {
                return;
            };
            // serde writes no value of what a marker is written with.
            if last.ident == "PhantomData" {
                return;
            }
            let arguments: Vec<&Type> = type_arguments_of(&last.arguments).collect();
            for filled in recorded(&last.ident.to_string()) {
                if let Some(argument) = arguments.get(at(&filled)) {
                    found.push((argument, filled));
                }
            }
            for segment in &named.path.segments {
                for argument in type_arguments_of(&segment.arguments) {
                    flattened_fillings(argument, recorded, at, found);
                }
            }
        }
        Type::Array(array) => flattened_fillings(&array.elem, recorded, at, found),
        Type::Slice(slice) => flattened_fillings(&slice.elem, recorded, at, found),
        Type::Paren(paren) => flattened_fillings(&paren.elem, recorded, at, found),
        Type::Reference(reference) => flattened_fillings(&reference.elem, recorded, at, found),
        Type::Tuple(tuple) => {
            for elem in &tuple.elems {
                flattened_fillings(elem, recorded, at, found);
            }
        }
        // None of these holds a value serde writes as a type argument's filling.
        Type::FnPtr(_)
        | Type::Group(_)
        | Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Never(_)
        | Type::Ptr(_)
        | Type::TraitObject(_)
        | Type::Verbatim(_)
        | _ => {}
    }
}

/// The position a record of a flattened parameter holds.
#[cfg(feature = "serde")]
const fn flattened_at(filled: &FlattenedParameter) -> usize {
    filled.at
}

/// The position a record of a parameter read as a flattened map or its key holds.
#[cfg(feature = "serde")]
const fn text_key_at(filled: &TextKeyParameter) -> usize {
    filled.at
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
