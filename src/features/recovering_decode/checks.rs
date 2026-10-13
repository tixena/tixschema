//! `decode_with_check_messages`: what the checks answer of a value serde built, each message under
//! the place the payload holds its subject at.
//!
//! A check takes the typed value, which the walker never holds: it reads the raw one. So the
//! checks run after a read serde accepts, and each message they answer is turned into an issue
//! against the raw value the read was made from.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt as _;
use syn::{Field, Fields, ItemEnum, ItemStruct, PathArguments, Type};

use super::enums::Tagging;
use super::{hooked_leaf, type_arguments, value_slot};
use crate::features::serde::{
    has_serde_transparent, parse_serde_field_attributes, parse_serde_field_hooks,
    parse_serde_key_omission, parse_written_renames,
};
use crate::field_type::{
    FieldDefType, classify_variant, get_field_def, is_map_wrapper, is_ownership_wrapper,
    is_refused_sequence_wrapper, is_sequence_wrapper,
};
use crate::model_schema::checks::{ItemChecks, member_calls};
use crate::model_schema::{
    BoundMember, MemberPlace, bound_member, build_member_check_arms, helper_name_stem,
    nested_under_fn, variant_check_pattern,
};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{Declared, declared, member_checks, own_checks, written_type};

/// The list `decode_with_check_messages` collects each message in.
const SINK: &str = "messages";

/// How deep an alias of an alias is read before the type is taken as it stands.
const ALIAS_DEPTH: usize = 8;

/// One step of the place a value is held at: a key or a position serde writes, or a binding of
/// the walk, which holds the index of a list's item or the key of a map's entry.
#[derive(Clone)]
enum Step {
    Bound(Ident),
    Written(String),
}

/// One member of the type as its checks are run: where it is read from, the type it is written
/// as, and where serde writes it.
struct Member<'item> {
    /// The expression the member is read from: a reference to it.
    checked: TokenStream,
    field: &'item Field,
    /// The keys and positions serde writes the member under, outermost first. Empty for a member
    /// written as the value of what holds it.
    place: Vec<String>,
    /// What the member's helpers are named from, which is what its checks are recorded under.
    stem: String,
}

impl Member<'_> {
    /// What the models the member holds answer, each message under the place it is held at.
    /// `None` for a member that holds none, and for one read through a hook of its author's: no
    /// walk says where such a member writes what it holds.
    fn held(&self, module_name: &str, parameters: &[String]) -> Option<TokenStream> {
        let hooks = parse_serde_field_hooks(&self.field.attrs);
        if hooked_leaf(&self.field.ty, &hooks, module_name, parameters).is_some() {
            return None;
        }
        let head = binding("held", 0);
        let place: Vec<Step> = self.place.iter().cloned().map(Step::Written).collect();
        let walked = held_messages(&self.field.ty, &head, &place, 1, 0)?;
        let checked = &self.checked;
        Some(quote! {
            {
                let #head = #checked;
                #walked
            }
        })
    }

    /// What the checks the member names answer, each message under the member's place.
    fn own(&self, module_name: &str) -> Option<TokenStream> {
        let place = MemberPlace {
            checked: self.checked.clone(),
            reported: (!self.place.is_empty()).then(|| self.place.join(".")),
        };
        member_calls(
            &place,
            &self.field.ty,
            &member_checks(module_name, &self.stem),
            SINK,
        )
    }
}

/// `decode_with_check_messages` of a struct, written from the checks recorded for it as it was
/// expanded.
pub fn struct_check_messages(
    item_struct: &ItemStruct,
    module_name: &str,
    parameters: &[String],
) -> TokenStream {
    let members = struct_members(item_struct);
    let of_members = members.iter().filter_map(|member| member.own(module_name));
    let of_held = members
        .iter()
        .filter_map(|member| member.held(module_name, parameters));
    method(
        &quote! { #(#of_members)* },
        module_name,
        &quote! { #(#of_held)* },
    )
}

/// `decode_with_check_messages` of an enum: the checks of the variant the value holds, the enum's
/// own, and those of the models that variant holds, each where the enum's tagging writes it.
pub fn enum_check_messages(
    item_enum: &ItemEnum,
    module_name: &str,
    parameters: &[String],
) -> TokenStream {
    let tagging = Tagging::of(item_enum);
    let mut of_members: Vec<(TokenStream, Vec<TokenStream>)> = Vec::new();
    let mut of_held: Vec<(TokenStream, Vec<TokenStream>)> = Vec::new();
    for variant in &item_enum.variants {
        let members = variant_members(item_enum, variant, &tagging);
        of_members.push(arm(variant, &members, |member| member.own(module_name)));
        of_held.push(arm(variant, &members, |member| {
            member.held(module_name, parameters)
        }));
    }
    method(&matched(of_members), module_name, &matched(of_held))
}

/// The pattern `variant` is matched by and what each of `members` answers under it: only a member
/// that answers is bound.
fn arm<F>(
    variant: &syn::Variant,
    members: &[(Member<'_>, usize)],
    answer: F,
) -> (TokenStream, Vec<TokenStream>)
where
    F: Fn(&Member<'_>) -> Option<TokenStream>,
{
    let (bound, bodies): (Vec<BoundMember>, Vec<TokenStream>) = members
        .iter()
        .filter_map(|(member, index)| Some((bound_member(member.field, *index), answer(member)?)))
        .unzip();
    (
        variant_check_pattern(
            &variant.ident,
            &classify_variant(variant),
            variant.fields.len(),
            &bound,
        ),
        bodies,
    )
}

/// The `match` that runs what each variant's members answer, or nothing where none answers.
fn matched(per_variant: Vec<(TokenStream, Vec<TokenStream>)>) -> TokenStream {
    let arms = build_member_check_arms(per_variant);
    if arms.is_empty() {
        return TokenStream::new();
    }
    quote! {
        match self {
            #(#arms),*
        }
    }
}

/// The `impl` that holds `method`, which is `decode_with_check_messages`, and what turns its
/// messages into issues. It is under the bounds the type's author wrote and no other, so the
/// checks of a type with a parameter run at every filling of it.
pub fn type_impl(
    name: &Ident,
    generics: &syn::Generics,
    module: &Ident,
    method: &TokenStream,
) -> TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let issues = issue_methods(module);
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #method
            #issues
        }
    }
}

/// The method itself: the members' messages, then the type's own, then those of the models it
/// holds.
fn method(of_members: &TokenStream, module_name: &str, of_held: &TokenStream) -> TokenStream {
    let named = own_checks(module_name);
    let own = ItemChecks {
        named: !named.is_empty(),
        own: &named,
    }
    .own_calls(SINK);
    quote! {
        /// The messages of the checks alone: its members', then its own, then those of every
        /// model it holds, each under the key serde writes.
        pub fn decode_with_check_messages(&self) -> ::std::vec::Vec<::std::string::String> {
            let mut messages: ::std::vec::Vec<::std::string::String> = ::std::vec::Vec::new();
            #of_members
            #(#own)*
            #of_held
            messages
        }
    }
}

/// `decode_with_value_check_issues`, and `decode_with_bson_check_issues` under `bson`: what turns
/// the messages of a value's checks into the issues of the read that built it. They are written
/// beside `decode_with_check_messages`, and name the issue's types in `module`.
fn issue_methods(module: &Ident) -> TokenStream {
    let of_bson = bson_issue_method(module);
    let lead = lead_fn();
    quote! {
        /// One `Invalid` per message the checks answered of the value serde built from `whole`.
        /// Its path is the message's lead read against `whole`, where a number under a list is
        /// an index and every other segment a key, and it holds what is found there, or `null`
        /// where the path leads nowhere.
        fn decode_with_value_check_issues(
            whole: &serde_json::Value,
            messages: &[::std::string::String],
        ) -> ::std::vec::Vec<#module::Issue<serde_json::Value>> {
            #lead
            messages
                .iter()
                .map(|message| {
                    let (lead, reason) = check_lead(message);
                    let mut found = ::core::option::Option::Some(whole);
                    let mut path = ::std::vec::Vec::new();
                    for segment in lead.split('.').filter(|segment| !segment.is_empty()) {
                        let index = match found {
                            ::core::option::Option::Some(serde_json::Value::Array(_)) => segment.parse::<usize>().ok(),
                            _ => ::core::option::Option::None,
                        };
                        found = found.and_then(|held| match index {
                            ::core::option::Option::Some(position) => held.get(position),
                            ::core::option::Option::None => held.get(segment),
                        });
                        path.push(index.map_or_else(|| #module::Segment::Key(segment.to_owned()), #module::Segment::Index));
                    }
                    #module::Issue::Invalid {
                        path: #module::Path(path),
                        expected: #module::Expected::Unknown,
                        found: found.cloned().unwrap_or(serde_json::Value::Null),
                        reason: reason.to_owned(),
                    }
                })
                .collect()
        }

        #of_bson
    }
}

/// `check_lead`: the place a check's message leads with and the text after it, as
/// `'a.b.1': text` writes them. A message with no lead is at the root.
fn lead_fn() -> TokenStream {
    quote! {
        fn check_lead(message: &str) -> (&str, &str) {
            message
                .strip_prefix('\'')
                .and_then(|rest| rest.split_once("': "))
                .unwrap_or(("", message))
        }
    }
}

/// `decode_with_bson_check_issues`.
#[cfg(feature = "bson")]
fn bson_issue_method(module: &Ident) -> TokenStream {
    let lead = lead_fn();
    quote! {
        /// One `Invalid` per message the checks answered of the value serde built from the BSON
        /// document `whole` holds, as `decode_with_value_check_issues` lists them for a JSON
        /// value.
        fn decode_with_bson_check_issues(
            whole: &bson::Bson,
            messages: &[::std::string::String],
        ) -> ::std::vec::Vec<#module::Issue<bson::Bson>> {
            #lead
            messages
                .iter()
                .map(|message| {
                    let (lead, reason) = check_lead(message);
                    let mut found = ::core::option::Option::Some(whole);
                    let mut path = ::std::vec::Vec::new();
                    for segment in lead.split('.').filter(|segment| !segment.is_empty()) {
                        let index = match found {
                            ::core::option::Option::Some(bson::Bson::Array(_)) => segment.parse::<usize>().ok(),
                            _ => ::core::option::Option::None,
                        };
                        found = found.and_then(|held| match (held, index) {
                            (bson::Bson::Array(items), ::core::option::Option::Some(position)) => items.get(position),
                            (bson::Bson::Document(entries), ::core::option::Option::None) => entries.get(segment),
                            _ => ::core::option::Option::None,
                        });
                        path.push(index.map_or_else(|| #module::Segment::Key(segment.to_owned()), #module::Segment::Index));
                    }
                    #module::Issue::Invalid {
                        path: #module::Path(path),
                        expected: #module::Expected::Unknown,
                        found: found.cloned().unwrap_or(bson::Bson::Null),
                        reason: reason.to_owned(),
                    }
                })
                .collect()
        }
    }
}

/// A build without `bson` reads no BSON value.
#[cfg(not(feature = "bson"))]
fn bson_issue_method(_module: &Ident) -> TokenStream {
    TokenStream::new()
}

/// A binding of the walk at `depth`, numbered so that no two levels of one walk collide.
fn binding(stem: &str, depth: usize) -> Ident {
    format_ident!("{stem}_{depth}")
}

/// The key serde writes `field` under: its own `rename`, or its name cased by `rename_all`.
fn written_key(field: &Field, rename_all: Option<&str>) -> Option<String> {
    let ident = field.ident.as_ref()?;
    Some(
        parse_written_renames(&field.attrs)
            .rename
            .unwrap_or_else(|| {
                resolve_rename_rule(rename_all).apply_to_field(&ident.unraw().to_string())
            }),
    )
}

/// What a member's helpers are named from: its Rust name, or a slot's position.
fn declared_as(field: &Field, index: usize) -> String {
    field
        .ident
        .as_ref()
        .map_or_else(|| index.to_string(), |member| member.unraw().to_string())
}

/// The members of `fields` serde reads, each beside its place among the ones declared, with the
/// place serde writes it at under `prefix`: a named member under its key, the only slot at
/// `prefix` itself where `lone_is_the_value` says serde writes it as the whole value, and any
/// other slot under its position among the slots serde reads.
fn read_members<'item>(
    fields: &'item Fields,
    prefix: &[String],
    rename_all: Option<&str>,
    lone_is_the_value: bool,
) -> Vec<(&'item Field, usize, Vec<String>)> {
    let mut position = 0_usize;
    let mut members = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if parse_serde_key_omission(&field.attrs).skips_deserializing {
            continue;
        }
        let mut place = prefix.to_vec();
        if let Some(key) = written_key(field, rename_all) {
            if !parse_serde_field_attributes(&field.attrs).flatten {
                place.push(key);
            }
        } else if !lone_is_the_value {
            place.push(position.to_string());
        } else {
            // The slot is the value serde writes, at the place of what holds it.
        }
        position = position.saturating_add(1);
        members.push((field, index, place));
    }
    members
}

/// The members of a struct, each read off `self`.
fn struct_members(item_struct: &ItemStruct) -> Vec<Member<'_>> {
    let rename_all = parse_written_renames(&item_struct.attrs).rename_all;
    // serde writes a struct of one slot, and a transparent one, as the value of that slot.
    let one_value = value_slot(item_struct).is_some();
    let transparent = has_serde_transparent(&item_struct.attrs);
    read_members(&item_struct.fields, &[], rename_all.as_deref(), one_value)
        .into_iter()
        .map(|(field, index, place)| {
            let checked = field.ident.as_ref().map_or_else(
                || {
                    let slot = syn::Index::from(index);
                    quote! { &self.#slot }
                },
                |member| quote! { &self.#member },
            );
            Member {
                checked,
                field,
                // A transparent struct's one field is its value, whatever it is named.
                place: if transparent { Vec::new() } else { place },
                stem: helper_name_stem(&declared_as(field, index), None),
            }
        })
        .collect()
}

/// The members of one variant, each read off the binding its arm makes, beside its place among
/// the ones the variant declares.
fn variant_members<'item>(
    item_enum: &'item ItemEnum,
    variant: &'item syn::Variant,
    tagging: &Tagging,
) -> Vec<(Member<'item>, usize)> {
    let container = parse_written_renames(&item_enum.attrs);
    let named = variant.ident.unraw().to_string();
    let prefix: Vec<String> = if let Tagging::Adjacent { content, tag: _tag } = tagging {
        vec![content.clone()]
    } else if matches!(tagging, Tagging::External) {
        vec![
            parse_written_renames(&variant.attrs)
                .rename
                .unwrap_or_else(|| {
                    resolve_rename_rule(container.rename_all.as_deref()).apply_to_variant(&named)
                }),
        ]
    } else {
        // serde writes what the variant holds beside the tag, or as the value itself.
        Vec::new()
    };
    let rename_all = parse_written_renames(&variant.attrs)
        .rename_all
        .or(container.rename_all_fields);
    let lone = matches!(&variant.fields, Fields::Unnamed(slots) if slots.unnamed.len() == 1);
    let variant_name = variant.ident.to_string();
    read_members(&variant.fields, &prefix, rename_all.as_deref(), lone)
        .into_iter()
        .map(|(field, index, place)| {
            let bound = bound_member(field, index).binding;
            (
                Member {
                    checked: quote! { #bound },
                    field,
                    place,
                    stem: helper_name_stem(&declared_as(field, index), Some(&variant_name)),
                },
                index,
            )
        })
        .collect()
}

/// `place` as the `&str` a message is nested under, or `None` for no place at all.
fn place_text(place: &[Step]) -> Option<TokenStream> {
    if place.is_empty() {
        return None;
    }
    let bound = place.iter().any(|step| matches!(step, Step::Bound(_)));
    // A key is text inside a format string, where a brace opens an argument.
    let template = place
        .iter()
        .map(|step| match step {
            Step::Bound(held) => format!("{{{held}}}"),
            Step::Written(key) if bound => key.replace('{', "{{").replace('}', "}}"),
            Step::Written(key) => key.clone(),
        })
        .collect::<Vec<_>>()
        .join(".");
    Some(if bound {
        quote! { &format!(#template) }
    } else {
        quote! { #template }
    })
}

/// `place` with one more step after it.
fn under(place: &[Step], step: Step) -> Vec<Step> {
    let mut longer = place.to_vec();
    longer.push(step);
    longer
}

/// What asks the value under `value` for the messages of its own checks. A type with no
/// `decode_with_check_messages` of its own answers none: a method of the type's own is found
/// ahead of the one the trait here gives a reference to anything.
fn asked(value: &Ident, place: &[Step]) -> TokenStream {
    let fallback = quote! {
        trait Unchecked {
            fn decode_with_check_messages(&self) -> ::std::vec::Vec<::std::string::String> {
                ::std::vec::Vec::new()
            }
        }
        impl<T: ?::core::marker::Sized> Unchecked for &T {}
    };
    let Some(under_place) = place_text(place) else {
        return quote! {
            {
                #fallback
                messages.extend(#value.decode_with_check_messages());
            }
        };
    };
    let naming = nested_under_fn();
    quote! {
        {
            #fallback
            #naming
            messages.extend(
                #value
                    .decode_with_check_messages()
                    .iter()
                    .map(|violation| nested_under(#under_place, violation)),
            );
        }
    }
}

/// What lists the messages of every model a value of type `ty` holds, read from `value`, a
/// reference to it, with each message under the place the model is held at. `None` where the
/// type holds no model a walk reaches. `aliases` counts the aliases read through to reach `ty`.
fn held_messages(
    ty: &Type,
    value: &Ident,
    place: &[Step],
    depth: usize,
    aliases: usize,
) -> Option<TokenStream> {
    let written = written_type(ty);
    let below = depth.saturating_add(1);
    let item = binding("held", depth);
    let listed = if let Type::Array(array) = written {
        Some(&*array.elem)
    } else if let Type::Slice(slice) = written {
        Some(&*slice.elem)
    } else {
        None
    };
    if let Some(of_items) = listed {
        return each_item(of_items, value, place, depth, aliases);
    }
    if let Type::Tuple(tuple) = written {
        let positions: Vec<TokenStream> = tuple
            .elems
            .iter()
            .enumerate()
            .filter_map(|(at, position)| {
                let step = Step::Written(at.to_string());
                let each = held_messages(position, &item, &under(place, step), below, aliases)?;
                let slot = syn::Index::from(at);
                Some(quote! {
                    {
                        let #item = &#value.#slot;
                        #each
                    }
                })
            })
            .collect();
        return (!positions.is_empty()).then(|| quote! { #(#positions)* });
    }
    let Type::Path(type_path) = written else {
        return None;
    };
    let last = type_path.path.segments.last()?;
    let name = last.ident.unraw().to_string();
    if matches!(last.arguments, PathArguments::None) {
        if let Some(Declared::Alias(aliased)) = declared(&name)
            && aliases < ALIAS_DEPTH
            && let Ok(named) = syn::parse_str::<Type>(&aliased)
        {
            return held_messages(&named, value, place, depth, aliases.saturating_add(1));
        }
        return holds_a_model(written).then(|| asked(value, place));
    }
    let mut arguments = type_arguments(&last.arguments);
    let (first, second) = (arguments.next(), arguments.next());
    if name == "Option" {
        let each = held_messages(first?, &item, place, below, aliases)?;
        return Some(quote! {
            if let ::core::option::Option::Some(#item) = #value {
                #each
            }
        });
    }
    if is_sequence_wrapper(&name) || is_refused_sequence_wrapper(&name) {
        return each_item(first?, value, place, depth, aliases);
    }
    if is_map_wrapper(&name) {
        let key = binding("key", depth);
        let step = Step::Bound(key.clone());
        let each = held_messages(second?, &item, &under(place, step), below, aliases)?;
        let keyed = written_key_items();
        return Some(quote! {
            {
                #keyed
                for (#key, #item) in #value.iter() {
                    if let ::core::option::Option::Some(#key) = #key.decode_with_key() {
                        #each
                    }
                }
            }
        });
    }
    if is_ownership_wrapper(&name) {
        let each = held_messages(first?, &item, place, below, aliases)?;
        return Some(quote! {
            {
                let #item = &**#value;
                #each
            }
        });
    }
    holds_a_model(written).then(|| asked(value, place))
}

/// [`held_messages`] for a list of `of_items` under `value`: each item at its index.
fn each_item(
    of_items: &Type,
    value: &Ident,
    place: &[Step],
    depth: usize,
    aliases: usize,
) -> Option<TokenStream> {
    let (item, index) = (binding("held", depth), binding("index", depth));
    let step = Step::Bound(index.clone());
    let each = held_messages(
        of_items,
        &item,
        &under(place, step),
        depth.saturating_add(1),
        aliases,
    )?;
    Some(quote! {
        for (#index, #item) in #value.iter().enumerate() {
            #each
        }
    })
}

/// Whether `ty` names a type declared elsewhere, which may carry checks of its own, and no type
/// the crate describes itself.
fn holds_a_model(ty: &Type) -> bool {
    matches!(
        get_field_def("", ty, "").field_type,
        FieldDefType::SiblingType(_, _)
    )
}

/// What answers a map's key as serde writes it, which is the key the raw value holds the entry
/// under. A key serde cannot be asked to write answers nothing, and its entry is passed over: the
/// trait for a reference is reached only where the one over `Serialize` does not hold.
fn written_key_items() -> TokenStream {
    let span = Span::call_site();
    let (written, unwritten) = (
        Ident::new("WrittenKey", span),
        Ident::new("UnwrittenKey", span),
    );
    quote! {
        trait #written {
            fn decode_with_key(&self) -> ::core::option::Option<::std::string::String>;
        }
        impl<K: ?::core::marker::Sized + serde::Serialize> #written for K {
            fn decode_with_key(&self) -> ::core::option::Option<::std::string::String> {
                match serde_json::to_value(self) {
                    ::core::result::Result::Ok(serde_json::Value::String(text)) => ::core::option::Option::Some(text),
                    ::core::result::Result::Ok(other) => ::core::option::Option::Some(other.to_string()),
                    ::core::result::Result::Err(_) => ::core::option::Option::None,
                }
            }
        }
        trait #unwritten {
            fn decode_with_key(&self) -> ::core::option::Option<::std::string::String> {
                ::core::option::Option::None
            }
        }
        impl<K: ?::core::marker::Sized> #unwritten for &K {}
    }
}
