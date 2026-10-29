//! The brands and aliases a map can be keyed by, which the emitter records as it writes them so a
//! map written below reads each key as the type it names: an alias as its target, a brand through
//! its `value`.

use core::cell::RefCell;
use std::collections::HashMap;

use crate::field_type::{FieldDef, FieldDefType};

/// A declared item a map key can name.
#[derive(Clone)]
enum Named {
    Alias(FieldDef),
    Brand(Brand),
}

#[derive(Clone)]
struct Brand {
    hashable: bool,
    value: FieldDef,
}

thread_local! {
    static NAMED: RefCell<HashMap<String, Named>> = RefCell::new(HashMap::new());
}

fn note(rust_ident: &str, declared: Option<Named>) {
    NAMED.with(|recorded| match declared {
        Some(entry) => {
            recorded.borrow_mut().insert(rust_ident.to_owned(), entry);
        }
        None => {
            recorded.borrow_mut().remove(rust_ident);
        }
    });
}

/// Records `target`, what the alias `rust_ident` names, or forgets the alias for `None`.
pub(super) fn note_alias(rust_ident: &str, target: Option<&FieldDef>) {
    note(rust_ident, target.cloned().map(Named::Alias));
}

/// Records `value`, what the brand `rust_ident` holds, and whether the Swift struct declares
/// `Hashable`, or forgets the brand for `None`.
pub(super) fn note_brand(rust_ident: &str, brand: Option<(&FieldDef, bool)>) {
    note(
        rust_ident,
        brand.map(|(value, hashable)| {
            Named::Brand(Brand {
                hashable,
                value: value.clone(),
            })
        }),
    );
}

fn named_by(def: &FieldDef) -> Option<Named> {
    if def.array_depth > 0 || def.is_optional() {
        return None;
    }
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return None;
    };
    if !arguments.is_empty() {
        return None;
    }
    NAMED.with(|recorded| recorded.borrow().get(name).cloned())
}

/// `key` with every alias it names written out as its target.
pub(super) fn peeled(key: &FieldDef) -> FieldDef {
    let mut peeled = key.clone();
    while let Some(Named::Alias(target)) = named_by(&peeled) {
        if target.array_depth > 0 || target.is_optional() {
            break;
        }
        peeled = target;
    }
    peeled
}

/// The Rust ident and the value of the brand `key` names, once its aliases are peeled.
pub(super) fn brand_of(key: &FieldDef) -> Option<(String, FieldDef)> {
    let peeled = peeled(key);
    let FieldDefType::SiblingType(name, _) = &peeled.field_type else {
        return None;
    };
    match named_by(&peeled)? {
        Named::Brand(brand) => Some((name.clone(), brand.value)),
        Named::Alias(_) => None,
    }
}

/// The type `key` is read and written as once its aliases are peeled and its brands are opened.
pub(super) fn leaf(key: &FieldDef) -> FieldDef {
    let mut leaf = peeled(key);
    while let Some((_, value)) = brand_of(&leaf) {
        leaf = peeled(&value);
    }
    leaf
}

/// Whether a Swift struct that holds `value` can declare `Hashable`: only what Swift hashes by
/// itself is held, so a type this module has not seen keeps the struct as it was.
pub(super) fn hashable(value: &FieldDef) -> bool {
    if value.array_depth > 0 || value.is_optional() {
        return false;
    }
    if let Some(named) = named_by(value) {
        return match named {
            Named::Brand(brand) => brand.hashable,
            Named::Alias(target) => hashable(&target),
        };
    }
    match &value.field_type {
        FieldDefType::Boolean
        | FieldDefType::Char
        | FieldDefType::String
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize => true,
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime => true,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => true,
        FieldDefType::StringLiteral(_)
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::SiblingType(_, _)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::Map(_, _)
        | FieldDefType::Range(_, _)
        | FieldDefType::Result(_, _)
        | FieldDefType::Tuple(_)
        | FieldDefType::U64
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::Usize => false,
    }
}
