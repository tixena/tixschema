//! The keys of a member a strict holder flattens whose type is declared below the holder or in
//! another crate, which no client emitter sees when it writes the holder's read. The read names a
//! placeholder in their place, and the holder's `*_definition()` writes there the keys the type
//! publishes in `__TIXSCHEMA_KEYS`, read in a `const` beside the holder, where the type's path
//! resolves as the holder's field wrote it. A type no `#[model_schema]` expanded is refused in
//! such a holder by the `const` beside it, so every type named here publishes them.

use core::cell::RefCell;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;

use crate::field_type::{FieldDef, FieldDefType};
use crate::model_schema::flatten_order::unmet_member;

thread_local! {
    /// The members of the item being expanded whose keys its reads name by a placeholder.
    static MEMBERS: RefCell<Vec<FieldDef>> = const { RefCell::new(Vec::new()) };
}

/// Forgets the members of the item expanded before.
pub fn reset() {
    MEMBERS.with(|members| members.borrow_mut().clear());
}

fn placeholder(index: usize) -> String {
    format!("\u{1}tixschema-keys-{index}\u{1}")
}

/// The keys of the flattened member typed `def`, `Option` or not, as the one placeholder its
/// holder's `*_definition()` writes them in place of, where its type is one the expansion has not
/// met.
pub fn of(def: &FieldDef) -> Option<Vec<String>> {
    let mut held = def.clone();
    held.nullable_levels
        .retain(|&level| level != held.array_depth);
    if !unmet_member(&held) {
        return None;
    }
    let FieldDefType::SiblingType(name, _) = &held.field_type else {
        return None;
    };
    let index = MEMBERS.with(|members| {
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
    });
    Some(vec![placeholder(index)])
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
    for (index, def) in members.iter().enumerate() {
        let FieldDefType::SiblingType(name, _) = &def.field_type else {
            continue;
        };
        let span = def.type_span;
        let ty = def.scope.reaching(&Ident::new(name, span), span);
        let held = Ident::new(
            &format!(
                "__TIXSCHEMA_{}_KEYS_{index}",
                module.to_string().to_uppercase()
            ),
            Span::call_site(),
        );
        beside.extend(quote! {
            const #held: &[&str] = {
                trait TixschemaUnmet {
                    const __TIXSCHEMA_KEYS: &'static [&'static str] = &[];
                }
                impl<T: ?::core::marker::Sized> TixschemaUnmet for T {}
                <#ty>::__TIXSCHEMA_KEYS
            };
        });
        let needle = quoted(&placeholder(index));
        pairs.extend(quote! { (#needle, super::#held), });
    }
    let mark = Literal::character(quote);
    let replaced = escaped.iter().map(|&(read, escape)| {
        let read_char = Literal::character(read);
        quote! { .replace(#read_char, #escape) }
    });
    let key = Ident::new("key", Span::call_site());
    let statements = quote! {
        for (tixschema_needle, tixschema_keys) in [#pairs] {
            let listed: ::std::vec::Vec<::std::string::String> = tixschema_keys
                .iter()
                .map(|#key| ::std::format!("{}{}{}", #mark, #key #(#replaced)*, #mark))
                .collect();
            #written = #written.replace(tixschema_needle, &listed.join(", "));
        }
    };
    (beside, statements)
}
