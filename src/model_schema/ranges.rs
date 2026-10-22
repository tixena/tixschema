//! The std ranges and the std `Result` an item writes: the check, beside the item, that one
//! written by its bare name is the std type every surface describes, the refusal of one under
//! `#[serde(flatten)]`, and that of one as a map key in a build whose map key guard does not run.
//! Beside them, the refusal of a map under `#[serde(flatten)]` beside another member or a tag, or
//! as the lone slot of a variant under a tag alone, written there or filling a type parameter a
//! type flattens there, and `serde_json`'s `Value` and `Map`: refused by their full path, and
//! checked beside the item by their bare name.

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
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
use super::{WRITTEN_AS_OBJECT, unwritable_map_key_message};
#[cfg(feature = "serde")]
use super::{
    flattening_levels, forwarded_parameter_position, item_generics, item_label, item_schema_ident,
    refused_member_named, type_arguments_of, written_spelling,
};
#[cfg(feature = "serde")]
use crate::features::serde::{Tagging, brand_field, parse_serde_field_attributes, struct_tag};
#[cfg(any(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
use crate::field_type::FieldDef;
#[cfg(feature = "serde")]
use crate::field_type::{FieldDefType, get_field_def, is_transparent_wrapper};
#[cfg(feature = "serde")]
use crate::utils::{
    FlattenedBeside, FlattenedParameter, flattened_beside_parameters, is_open_map,
    open_map_parameters,
};

/// A `serde_json` type that holds any JSON value, where a written type reaches it.
enum AnyJson<'ty> {
    /// `Value` with no arguments or `Map` with arguments, written by its bare name.
    Bare(&'static str, &'ty Type),
    /// `serde_json::Value` or `serde_json::Map`, written by its full path.
    Named(&'static str),
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

/// The type parameters `item` flattens beside a key serde writes at the same level: at each of its
/// own levels, and where it fills, with one of them, a type that flattens a parameter so.
#[cfg(feature = "serde")]
pub fn flattened_beside_parameters_of(item: &Item) -> Vec<FlattenedParameter> {
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
            let Some(written_beside) = beside else {
                continue;
            };
            for at in parameters_written_as(item, &field.ty) {
                held.push(FlattenedParameter {
                    at,
                    beside: written_beside.clone(),
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
            flattened_beside_parameters(name)
        }
    };
    let mut fillings = Vec::new();
    for (_, ty) in written_types(item) {
        flattened_fillings(ty, &recorded, &mut fillings);
    }
    for (argument, filled) in fillings {
        for at in parameters_written_as(item, argument) {
            held.push(FlattenedParameter {
                at,
                ..filled.clone()
            });
        }
    }
    // One record per parameter: a map that fills it is refused once.
    let mut seen: Vec<usize> = Vec::new();
    held.retain(|filled| {
        let first = !seen.contains(&filled.at);
        seen.push(filled.at);
        first
    });
    held
}

/// The refusal of each map that fills, in a type `item` writes, a type parameter a type flattens
/// beside a key serde writes at the same level, spanned on the map: in its members, in an alias's
/// value, and in each of `default_types`, where the entry fills the item's own parameter.
#[cfg(feature = "serde")]
pub fn filled_map_errors(item: &Item, default_types: &[(Ident, Type)]) -> Vec<TokenStream> {
    let Some(generics) = item_generics(item) else {
        return Vec::new();
    };
    let parameters = parameter_names(generics);
    let label = item_label(item);
    let flattens = item_schema_ident(item).map(ToString::to_string);
    let own = flattened_beside_parameters_of(item);
    let recorded = |name: &str| {
        if flattens.as_deref() == Some(name) {
            own.clone()
        } else {
            flattened_beside_parameters(name)
        }
    };
    let mut refused: Vec<(String, &Type, FlattenedParameter)> = Vec::new();
    for (member, ty) in written_types(item) {
        let subject = member.map_or_else(
            || format!("{label}: "),
            |named| format!("{label}: {named}: "),
        );
        let mut fillings = Vec::new();
        flattened_fillings(ty, &recorded, &mut fillings);
        refused.extend(
            fillings
                .into_iter()
                .filter(|(argument, _)| writes_a_map(argument, &parameters))
                .map(|(argument, filled)| (subject.clone(), argument, filled)),
        );
    }
    for (entry, filling) in default_types {
        let subject = format!("{label}: `default_types` entry `{entry}`: ");
        let at = parameters.iter().position(|parameter| entry == parameter);
        if writes_a_map(filling, &parameters) {
            refused.extend(
                own.iter()
                    .filter(|filled| Some(filled.at) == at)
                    .map(|filled| (subject.clone(), filling, filled.clone())),
            );
        }
        let mut fillings = Vec::new();
        flattened_fillings(filling, &recorded, &mut fillings);
        refused.extend(
            fillings
                .into_iter()
                .filter(|(argument, _)| writes_a_map(argument, &parameters))
                .map(|(argument, filled)| (subject.clone(), argument, filled)),
        );
    }
    refused
        .into_iter()
        .map(|(subject, argument, filled)| {
            let message = filled_map_message(&subject, argument, &filled);
            syn::Error::new_spanned(argument, prefixed_guard_message(&message)).to_compile_error()
        })
        .collect()
}

/// The refusal of `argument`, a map, filling the parameter `filled` holds.
#[cfg(feature = "serde")]
fn filled_map_message(subject: &str, argument: &Type, filled: &FlattenedParameter) -> String {
    let flattened = format!(
        "{subject}`{}` flattens its type parameter `{}`",
        filled.flattens, filled.parameter
    );
    let filling = written_spelling(argument);
    match &filled.beside {
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

/// Each argument `ty` fills a type parameter with, at any depth, where the type it is handed to
/// flattens that parameter beside a key, as `recorded` answers for that type's name, beside what is
/// recorded of the parameter.
#[cfg(feature = "serde")]
fn flattened_fillings<'ty>(
    ty: &'ty Type,
    recorded: &dyn Fn(&str) -> Vec<FlattenedParameter>,
    found: &mut Vec<(&'ty Type, FlattenedParameter)>,
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
                if let Some(argument) = arguments.get(filled.at) {
                    found.push((argument, filled));
                }
            }
            for segment in &named.path.segments {
                for argument in type_arguments_of(&segment.arguments) {
                    flattened_fillings(argument, recorded, found);
                }
            }
        }
        Type::Array(array) => flattened_fillings(&array.elem, recorded, found),
        Type::Slice(slice) => flattened_fillings(&slice.elem, recorded, found),
        Type::Paren(paren) => flattened_fillings(&paren.elem, recorded, found),
        Type::Reference(reference) => flattened_fillings(&reference.elem, recorded, found),
        Type::Tuple(tuple) => {
            for elem in &tuple.elems {
                flattened_fillings(elem, recorded, found);
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
