//! The refusal class each Dart, Swift and Kotlin type that judges a value declares in its own text,
//! `{Name}Refusal`, which `validate()`, `of(..)` and the type's judged read throw with the list of
//! broken rules, and the classes a service surface reads that list off.

use syn::{Item, Type};

use crate::field_type::{FieldDef, FieldDefType, get_field_def, is_sequence_wrapper};

/// The refusal class of the type published as `export_name`, without its type parameters.
pub fn refusal_name(export_name: &str) -> String {
    format!("{export_name}Refusal")
}

/// Every refusal class a read of `def` throws where the value it reads, or one a list, a set, a
/// map, an option, a tuple or a `Result` holds, breaks a rule: `own` answers those of a declared
/// type by its Rust ident, and nothing for a type the expansion has not met. A generic type judges
/// what fills its parameters itself, so its arguments add none.
pub fn reached(def: &FieldDef, own: &dyn Fn(&str) -> Vec<String>) -> Vec<String> {
    let mut found = Vec::new();
    collect(def, own, &mut found);
    found
}

/// The Rust ident of the struct, enum or alias `item` declares.
pub fn item_ident(item: &Item) -> Option<String> {
    if let Item::Struct(declared) = item {
        Some(declared.ident.to_string())
    } else if let Item::Enum(declared) = item {
        Some(declared.ident.to_string())
    } else if let Item::Type(declared) = item {
        Some(declared.ident.to_string())
    } else {
        None
    }
}

/// Every refusal class a read of `item` lets through from what a member, a slot, a variant or an
/// alias's target holds, where that value judges itself as it is read, a map's key among them.
pub fn held_by(item: &Item, own: &dyn Fn(&str) -> Vec<String>) -> Vec<String> {
    let held: Vec<&Type> = if let Item::Struct(declared) = item {
        declared.fields.iter().map(|field| &field.ty).collect()
    } else if let Item::Enum(declared) = item {
        declared
            .variants
            .iter()
            .flat_map(|variant| variant.fields.iter().map(|field| &field.ty))
            .collect()
    } else if let Item::Type(declared) = item {
        vec![declared.ty.as_ref()]
    } else {
        Vec::new()
    };
    let mut found = Vec::new();
    for written in held {
        collect(&get_field_def("", written, ""), own, &mut found);
    }
    found
}

fn collect(def: &FieldDef, own: &dyn Fn(&str) -> Vec<String>, found: &mut Vec<String>) {
    let held: Vec<&FieldDef> = if let FieldDefType::SiblingType(name, generics) = &def.field_type {
        if let [element] = generics.as_slice()
            && is_sequence_wrapper(name)
        {
            vec![element]
        } else {
            for refusal in own(name) {
                if !found.contains(&refusal) {
                    found.push(refusal);
                }
            }
            Vec::new()
        }
    } else if let FieldDefType::Map(key, value) | FieldDefType::Result(key, value) = &def.field_type
    {
        vec![key.as_ref(), value.as_ref()]
    } else if let FieldDefType::Tuple(elements) = &def.field_type {
        elements.iter().collect()
    } else {
        Vec::new()
    };
    for inner in held {
        collect(inner, own, found);
    }
}
