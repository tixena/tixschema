//! The Swift type name a bare `syn::Type` reference renders as — a message, a success, a
//! declared error, or a `header_in`/`header_out`/`part` binding's own argument type. Mirrors
//! `crate::features::dart::dart_typename`, reading through the same [`FieldDef`] walk every
//! field's own type goes through, but kept apart from `crate::features::swift`'s own `Codable`
//! codec generation: that module answers for a *declared item*'s fields, threading an `aux`
//! vector of nested tuple and map-key wrapper structs through every leaf.
//!
//! [`super::swift_http_client`], and [`super::swift_ws_client`] for an operation declaring
//! `header_out`/`error_header_out`, never ask `JSONDecoder` to decode a whole tuple: each decodes
//! the body and reads its header elements separately and assembles the Swift tuple by hand, so
//! the tuple named here is a bare `(A, B)`, and no wrapper struct is ever generated. A headerless
//! `swift_ws_client` operation reaches for [`crate::features::swift::swift_reference_type`]
//! instead, whose `aux` wrapper gives a bare tuple or non-string-keyed map the `Codable`
//! conformance a whole-payload decode needs.

use crate::features::swift::lookup_swift_name;
use crate::field_type::{FieldDef, FieldDefType, get_field_def, is_sequence_wrapper};
use syn::Type;

/// `ty`'s own Swift type name, read through the same `FieldDef` walk every field's type goes
/// through — so a reference to a sibling `#[model_schema()]` type resolves to its published
/// Swift name exactly as it would inside an ordinary field.
pub(super) fn swift_typename_of(ty: &Type) -> String {
    swift_typename(&get_field_def("value", ty, ""))
}

fn swift_typename(field: &FieldDef) -> String {
    let base = swift_base(field);
    if field.is_optional() {
        format!("{base}?")
    } else {
        base
    }
}

fn swift_base(field: &FieldDef) -> String {
    let scalar = match &field.field_type {
        FieldDefType::Unknown => "any Sendable".to_owned(),
        FieldDefType::TypeParam(name) => name.clone(),
        FieldDefType::Tuple(elements) => {
            let rendered = elements
                .iter()
                .map(swift_typename)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({rendered})")
        }
        FieldDefType::SiblingType(name, generics) => {
            if let [element] = generics.as_slice()
                && is_sequence_wrapper(name)
            {
                return swift_base(&field.collection_element_field(element));
            }
            let class_name = lookup_swift_name(name).unwrap_or_else(|| name.clone());
            if generics.is_empty() {
                class_name
            } else {
                let arguments = generics
                    .iter()
                    .map(swift_typename)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{class_name}<{arguments}>")
            }
        }
        FieldDefType::Map(key, value) => {
            format!("[{}: {}]", swift_typename(key), swift_typename(value))
        }
        FieldDefType::Boolean | FieldDefType::BooleanLiteral(_) => "Bool".to_owned(),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            "String".to_owned()
        }
        FieldDefType::NumberLiteral(_) | FieldDefType::F64 => "Double".to_owned(),
        FieldDefType::F32 => "Float".to_owned(),
        FieldDefType::U8 => "UInt8".to_owned(),
        FieldDefType::U16 => "UInt16".to_owned(),
        FieldDefType::U32 => "UInt32".to_owned(),
        FieldDefType::U64 => "UInt64".to_owned(),
        FieldDefType::I8 => "Int8".to_owned(),
        FieldDefType::I16 => "Int16".to_owned(),
        FieldDefType::I32 => "Int32".to_owned(),
        FieldDefType::I64 => "Int64".to_owned(),
        FieldDefType::Usize | FieldDefType::Isize => "Int".to_owned(),
        #[cfg(feature = "object_id")]
        FieldDefType::ObjectId => "ObjectId".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            "String".to_owned()
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => "Date".to_owned(),
    };
    (0..field.array_depth).fold(scalar, |wrapped, level| {
        let item = if field.is_nullable_at(level) {
            format!("{wrapped}?")
        } else {
            wrapped
        };
        format!("[{item}]")
    })
}
