//! The keys of a member a strict holder flattens whose type is declared below the holder or in
//! another crate, which no client emitter sees when it writes the holder's read. The read names a
//! placeholder in their place, and the holder's `*_definition()` writes there the keys the type
//! publishes in `__TIXSCHEMA_KEYS`, read in a `const` beside the holder, where the type's path
//! resolves as the holder's field wrote it. A type no `#[model_schema]` expanded is refused in
//! such a holder by the `const` beside it, so every type named here publishes them.

extern crate alloc;

use alloc::collections::BTreeSet;
use core::cell::RefCell;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

use crate::field_type::{FieldDef, FieldDefType};
use crate::model_schema::flatten_order::unmet_member;

thread_local! {
    /// The members of the item being expanded whose keys its reads name by a placeholder.
    static MEMBERS: RefCell<Vec<FieldDef>> = const { RefCell::new(Vec::new()) };
    /// The places among [`MEMBERS`] whose reads name the entries of the member by a placeholder
    /// too.
    static ENTRIED: RefCell<BTreeSet<usize>> = const { RefCell::new(BTreeSet::new()) };
    /// The places among [`MEMBERS`] whose reads name the keys the member takes by a placeholder
    /// too.
    static TAKING: RefCell<BTreeSet<usize>> = const { RefCell::new(BTreeSet::new()) };
    /// The places among [`MEMBERS`] whose reads name the unit variants of the member by a
    /// placeholder too.
    static UNITED: RefCell<BTreeSet<usize>> = const { RefCell::new(BTreeSet::new()) };
}

/// Forgets the members of the item expanded before.
pub fn reset() {
    MEMBERS.with(|members| members.borrow_mut().clear());
    ENTRIED.with(|entried| entried.borrow_mut().clear());
    TAKING.with(|taking| taking.borrow_mut().clear());
    UNITED.with(|united| united.borrow_mut().clear());
}

fn placeholder(index: usize) -> String {
    format!("\u{1}tixschema-keys-{index}\u{1}")
}

fn taken_placeholder(index: usize) -> String {
    format!("\u{1}tixschema-taken-{index}\u{1}")
}

fn units_placeholder(index: usize) -> String {
    format!("\u{1}tixschema-units-{index}\u{1}")
}

fn entries_placeholder(index: usize) -> String {
    format!("\u{1}tixschema-entries-{index}\u{1}")
}

/// Whether the flattened member typed `def`, `Option` or not, has a type the expansion has not met.
pub fn is_unmet(def: &FieldDef) -> bool {
    let mut held = def.clone();
    held.nullable_levels
        .retain(|&level| level != held.array_depth);
    unmet_member(&held)
}

/// The place of the member typed `def` among the members whose reads name a placeholder, where its
/// type is one the expansion has not met.
fn registered(def: &FieldDef) -> Option<usize> {
    if !is_unmet(def) {
        return None;
    }
    let mut held = def.clone();
    held.nullable_levels
        .retain(|&level| level != held.array_depth);
    let FieldDefType::SiblingType(name, _) = &held.field_type else {
        return None;
    };
    Some(MEMBERS.with(|members| {
        let mut listed = members.borrow_mut();
        listed
            .iter()
            .position(|member| {
                matches!(&member.field_type, FieldDefType::SiblingType(other, _) if other == name)
            })
            .unwrap_or_else(|| {
                listed.push(held.clone());
                listed.len() - 1
            })
    }))
}

/// The keys of the flattened member typed `def`, `Option` or not, as the one placeholder its
/// holder's `*_definition()` writes them in place of, where its type is one the expansion has not
/// met.
pub fn of(def: &FieldDef) -> Option<Vec<String>> {
    registered(def).map(|index| vec![placeholder(index)])
}

/// The names the unit variants of the flattened member typed `def` are read under, as the one
/// placeholder its holder's `*_definition()` writes them in place of, where its type is one the
/// expansion has not met.
pub fn units_of(def: &FieldDef) -> Option<Vec<String>> {
    let index = registered(def)?;
    UNITED.with(|united| united.borrow_mut().insert(index));
    Some(vec![units_placeholder(index)])
}

/// The keys the flattened member typed `def` takes off the object its holder hands it, as the one
/// placeholder its holder's `*_definition()` writes them in place of, where its type is one the
/// expansion has not met: a struct's own keys and an enum's tag and content key, none for any
/// other type.
pub fn taken_of(def: &FieldDef) -> Option<Vec<String>> {
    let index = registered(def)?;
    TAKING.with(|taking| taking.borrow_mut().insert(index));
    Some(vec![taken_placeholder(index)])
}

/// The names the variants of the flattened member typed `def` are read under, as the one
/// placeholder its holder's `*_definition()` writes them in place of, one bracketed list per
/// variant, where its type is one the expansion has not met. The list is empty where the type is
/// no externally tagged enum.
pub fn entries_of(def: &FieldDef) -> Option<String> {
    let index = registered(def)?;
    ENTRIED.with(|entried| entried.borrow_mut().insert(index));
    Some(entries_placeholder(index))
}

/// The `const` of `module` that holds what the type `ty` publishes in the hidden constant named
/// `label`, of type `shape`, and what declares it: the constant a type no `#[model_schema]`
/// expanded answers with none.
fn published(
    module: &Ident,
    index: usize,
    (label, shape): (&str, &TokenStream),
    ty: &TokenStream,
) -> (Ident, TokenStream) {
    let held = Ident::new(
        &format!(
            "__TIXSCHEMA_{}_{label}_{index}",
            module.to_string().to_uppercase()
        ),
        Span::call_site(),
    );
    let constant = Ident::new(&format!("__TIXSCHEMA_{label}"), Span::call_site());
    let fallback = Ident::new(
        &format!("TixschemaUnmet{}", if label == "KEYS" { "" } else { label }),
        Span::call_site(),
    );
    let declared = quote! {
        const #held: #shape = {
            trait #fallback {
                const #constant: #shape = &[];
            }
            impl<T: ?::core::marker::Sized> #fallback for T {}
            <#ty>::#constant
        };
    };
    (held, declared)
}

/// What fills the keys of each member in place of its placeholder in the text of `module`: the
/// `const`s beside the module that read them, and the statements that write them into `written`,
/// the `String` the text is held in. `quoted` is how the client wrote the placeholder, and each key
/// is written between `quote`, every character of `escaped` replaced. None where the item names no
/// such member.
pub fn filled(
    written: &Ident,
    module: &Ident,
    quoted: fn(&str) -> String,
    quote: char,
    escaped: &[(char, &str)],
) -> (TokenStream, TokenStream) {
    let members = MEMBERS.with(|listed| listed.borrow().clone());
    if members.is_empty() {
        return (TokenStream::new(), TokenStream::new());
    }
    let mut beside = TokenStream::new();
    let mut pairs = TokenStream::new();
    let mut entry_pairs = TokenStream::new();
    let entried = ENTRIED.with(|listed| listed.borrow().clone());
    let taking = TAKING.with(|listed| listed.borrow().clone());
    let united = UNITED.with(|listed| listed.borrow().clone());
    for (index, def) in members.iter().enumerate() {
        let FieldDefType::SiblingType(name, _) = &def.field_type else {
            continue;
        };
        let span = def.type_span;
        let ty = def.scope.reaching(&Ident::new(name, span), span);
        let mut publish = |label: &str, listed: &TokenStream, placeholder: String| {
            let (held, declared) = published(module, index, (label, listed), &ty);
            beside.extend(declared);
            let needle = quoted(&placeholder);
            quote! { (#needle, super::#held), }
        };
        pairs.extend(publish(
            "KEYS",
            &quote! { &'static [&'static str] },
            placeholder(index),
        ));
        if taking.contains(&index) {
            pairs.extend(publish(
                "TAKEN",
                &quote! { &'static [&'static str] },
                taken_placeholder(index),
            ));
        }
        if united.contains(&index) {
            pairs.extend(publish(
                "UNITS",
                &quote! { &'static [&'static str] },
                units_placeholder(index),
            ));
        }
        if entried.contains(&index) {
            entry_pairs.extend(publish(
                "ENTRIES",
                &quote! { &'static [&'static [&'static str]] },
                entries_placeholder(index),
            ));
        }
    }
    let mark = Literal::character(quote);
    let replaced: Vec<TokenStream> = escaped
        .iter()
        .map(|&(read, escape)| {
            let read_char = Literal::character(read);
            quote! { .replace(#read_char, #escape) }
        })
        .collect();
    let key = Ident::new("key", Span::call_site());
    let entries_filled = if entry_pairs.is_empty() {
        TokenStream::new()
    } else {
        quote! {
            for (tixschema_needle, tixschema_variants) in [#entry_pairs] {
                let listed: ::std::vec::Vec<::std::string::String> = tixschema_variants
                    .iter()
                    .map(|tixschema_names| {
                        let names: ::std::vec::Vec<::std::string::String> = tixschema_names
                            .iter()
                            .map(|#key| ::std::format!("{}{}{}", #mark, #key #(#replaced)*, #mark))
                            .collect();
                        ::std::format!("[{}]", names.join(", "))
                    })
                    .collect();
                #written = #written.replace(tixschema_needle, &listed.join(", "));
            }
        }
    };
    let statements = quote! {
        for (tixschema_needle, tixschema_keys) in [#pairs] {
            let listed: ::std::vec::Vec<::std::string::String> = tixschema_keys
                .iter()
                .map(|#key| ::std::format!("{}{}{}", #mark, #key #(#replaced)*, #mark))
                .collect();
            #written = #written.replace(tixschema_needle, &listed.join(", "));
        }
        #entries_filled
    };
    (beside, statements)
}
