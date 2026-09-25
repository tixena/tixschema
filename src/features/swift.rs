//! Swift type and `Codable`-codec generation.
//!
//! Emits one `swift_definition()` method per `#[model_schema]` item — a Swift `struct` or `enum`
//! conforming to `Codable, Sendable`, generating the way the Dart backend generates:
//! `swift_schema_dispatch` is called directly from `exec_model_schema`, ahead of the
//! `process_struct`/`process_enum`/`process_type_alias` dispatch that consumes the item, and reads
//! its own borrow of it. Where Dart hand-writes `fromJson`/`toJson` for every field, Swift's own
//! `Codable` synthesis covers the common shapes once a `CodingKeys` enum carries the wire
//! spelling; a hand-written `init(from:)`/`encode(to:)` is written only where synthesis cannot
//! reach — a `nullable` field, a non-string map key, a tuple, and the three enum shapes serde
//! writes as something other than `{"case": payload}`.

use core::cell::RefCell;
use core::fmt::Write as _;
use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, Item, ItemEnum, ItemStruct, ItemType, Variant};

use crate::features::model_schema_prop::parse_model_schema_prop_attributes;
use crate::features::serde::parse_serde_key_omission;
use crate::field_type::{
    FieldDef, FieldDefType, VariantKind, classify_variant, get_field_def, is_plain_enum,
    is_sequence_wrapper,
};
use crate::rename_rule::{RenameRule, resolve_rename_rule};
use crate::utils::{
    MapKeyWire, compute_alias_export_name, compute_item_export_name, to_snake_case,
    type_parameters_in_scope,
};

#[cfg(feature = "serde")]
use crate::features::serde::{parse_serde_field_attributes, parse_serde_type_attributes};

/// One field this module has decided belongs on the wire, resolved to the Swift property it
/// earns: its Rust name, its lower-camel Swift name, its wire name, whether the key always
/// reaches the wire, whether it is a `#[serde(flatten)]` source, and the shape that drives its
/// decode/encode statements.
struct SwiftField {
    field_def: FieldDef,
    flatten: bool,
    real_type: String,
    required: bool,
    shape: SwiftFieldShape,
    swift_name: String,
    wire_name: String,
}

/// The container attributes read off any enum, in the shape `process_enum` itself dispatches on.
struct EnumTagAttrs {
    content: Option<String>,
    rename_all: Option<String>,
    rename_all_fields: Option<String>,
    tag: Option<String>,
    untagged: bool,
}

/// What a field's own leaf value needs beyond a plain `container.decode`/`.encode` call — the
/// question every custom `init(from:)`/`encode(to:)` this module writes exists to answer for at
/// least one field.
#[derive(Clone)]
enum LeafConversion {
    /// A `DateTime<Tz>` field under `#[model_schema_prop(as_number)]`: the wire is still the ISO
    /// 8601 string chrono writes, the Swift value is epoch milliseconds.
    #[cfg(feature = "chrono")]
    DateTimeAsNumber,
    /// A `DateTime<Tz>` field: the wire is an ISO 8601 string with fractional seconds, which
    /// `JSONDecoder`'s default strategy cannot parse into `Date` without a formatter.
    #[cfg(feature = "chrono")]
    DateTimePlain,
    /// A map whose key is not written as a JSON object key can decode directly: `wrapper` names
    /// the per-field struct that carries the string round trip.
    MapWrapper(String),
    /// The leaf reads and writes exactly as its own Swift type spells it.
    None,
}

/// A field's fully resolved Swift shape: the property type callers see, the type a `container`
/// call actually decodes, and what (if anything) turns one into the other.
#[derive(Clone)]
struct SwiftFieldShape {
    leaf_conversion: LeafConversion,
    real_leaf: String,
    wire_leaf: String,
}

thread_local! {
    /// The Swift type name each Rust ident publishes — the one thing a reference to a sibling item
    /// needs. Swift resolves a reference across the whole file regardless of declaration order, so
    /// this carries no forward-reference bookkeeping.
    static SWIFT_NAMES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// The Swift tokens `item` earns, given the `name = "..."` override an author declared on it.
/// Dispatches on the item's own shape; an item this module has nothing to say about earns nothing.
pub fn swift_schema_dispatch(item: &Item, name_override: Option<&str>) -> TokenStream {
    if let Item::Struct(item_struct) = item {
        struct_swift_tokens(item_struct, name_override)
    } else if let Item::Enum(item_enum) = item {
        enum_swift_tokens(item_enum, name_override)
    } else if let Item::Type(item_type) = item {
        alias_swift_tokens(item_type, name_override)
    } else {
        TokenStream::new()
    }
}

/// Whether `field`'s own declared type has no Swift mapping — a `u64` or `usize`, refused rather
/// than mapped because a decoder in the wild could disagree about the top half of the range.
pub const fn refuses_swift(field: &FieldDef) -> bool {
    matches!(field.field_type, FieldDefType::U64 | FieldDefType::Usize)
}

/// The Swift name registered for `rust_ident`, or `None` for a type declared below the one
/// asking — harmless, since a renamed item always re-publishes its ident too, as a `typealias`.
pub fn lookup_swift_name(rust_ident: &str) -> Option<String> {
    SWIFT_NAMES.with(|names| names.borrow().get(rust_ident).cloned())
}

/// Records the Swift name `rust_ident` publishes under, for a later sibling reference to read
/// back.
fn register_swift_name(rust_ident: &str, export_name: &str) {
    SWIFT_NAMES.with(|names| {
        names
            .borrow_mut()
            .insert(rust_ident.to_owned(), export_name.to_owned());
    });
}

/// Whether `attrs` carries a bare `#[serde(transparent)]` — duplicated from `features::dart` since
/// it is a dozen lines of plain `syn` parsing with no feature dependency of its own.
fn has_serde_transparent(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            let mut found = false;
            let _: syn::Result<()> = attr.parse_nested_meta(|nested| {
                if nested.path.is_ident("transparent") {
                    found = true;
                }
                Ok(())
            });
            if found {
                return true;
            }
        }
    }
    false
}

/// The module `{ident}_swift` publishes `swift_definition()` from — never a direct inherent
/// `impl {ident}`, for the same reason `dart::dart_module_tokens` gives: a Rust type alias is not
/// a new type, and a module name is never collapsed through one the way an impl target is.
fn swift_module_tokens(
    rust_ident: &str,
    span: proc_macro2::Span,
    swift_source: &str,
) -> TokenStream {
    let module_ident = swift_module_ident(rust_ident, span);
    quote! {
        pub mod #module_ident {
            pub fn swift_definition() -> String {
                #swift_source.to_owned()
            }
        }
    }
}

/// The `{ident}_swift` module ident an item's own `swift_definition()` publishes from.
pub fn swift_module_ident(rust_ident: &str, span: proc_macro2::Span) -> Ident {
    Ident::new(&format!("{}_swift", to_snake_case(rust_ident)), span)
}

/// The `typealias {rust_ident}{generics} = {export_name}{generics}` a renamed item re-publishes
/// under its own Rust ident — the alias a reference declared above the rename still resolves
/// through. Empty for an item that already publishes under its own ident.
fn ident_typealias(rust_ident: &str, export_name: &str, generic_params: &str) -> String {
    if rust_ident == export_name {
        String::new()
    } else {
        let bare = bare_generic_params(generic_params);
        format!("\n\npublic typealias {rust_ident}{bare} = {export_name}{bare};")
    }
}

/// `<T, U>` with every parameter's own conformance stripped — the spelling a `typealias` and a
/// reference site write, as opposed to a declaration, which writes [`swift_generic_params`].
fn bare_generic_params(generic_params: &str) -> String {
    if generic_params.is_empty() {
        return String::new();
    }
    let names: Vec<&str> = generic_params
        .trim_start_matches('<')
        .trim_end_matches('>')
        .split(", ")
        .map(|parameter| parameter.split(':').next().unwrap_or(parameter).trim())
        .collect();
    format!("<{}>", names.join(", "))
}

/// `<T: Codable & Sendable>` for the type parameters `generics` declares, or the empty string for
/// none — every parameter bound to `Codable & Sendable` since `Codable`'s own synthesis, and the
/// `Sendable` conformance this module always declares, both need it.
fn swift_generic_params(generics: &syn::Generics) -> String {
    let parameters = type_parameters_in_scope(generics);
    if parameters.is_empty() {
        String::new()
    } else {
        let bounded = parameters
            .iter()
            .map(|parameter| format!("{parameter}: Codable & Sendable"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("<{bounded}>")
    }
}

/// `parameter`, upper-camel-cased: `idType` -> `IdType` — used to name a per-field auxiliary type
/// after the field it belongs to.
fn swift_upper_camel(parameter: &str) -> String {
    let mut characters = parameter.chars();
    characters.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_uppercase(), characters.as_str())
    })
}

/// One field's `#[model_schema_prop(...)]` metadata, folded into the `FieldDef` `get_field_def`
/// built for it, exactly as `dart::field_def_with_prop_meta` does for the other backend.
fn field_def_with_prop_meta(name: &str, ty: &syn::Type, attrs: &[syn::Attribute]) -> FieldDef {
    let mut field_def = get_field_def(name, ty, "");
    field_def.model_schema_prop_meta = Some(parse_model_schema_prop_attributes(attrs));
    field_def
}

/// The `#[serde(rename = "...")]` a field or variant earns, honored only where the `serde`
/// feature reads serde attributes at all.
#[cfg(feature = "serde")]
fn rename_override(attrs: &[syn::Attribute]) -> Option<String> {
    parse_serde_field_attributes(attrs).rename
}

#[cfg(not(feature = "serde"))]
const fn rename_override(_attrs: &[syn::Attribute]) -> Option<String> {
    None
}

/// Whether `attrs` carries `#[serde(flatten)]`.
#[cfg(feature = "serde")]
fn field_is_flatten(attrs: &[syn::Attribute]) -> bool {
    parse_serde_field_attributes(attrs).flatten
}

#[cfg(not(feature = "serde"))]
const fn field_is_flatten(_attrs: &[syn::Attribute]) -> bool {
    false
}

/// The wire name a field with Rust name `rust_name` and its own `rename` writes under, once
/// `rule` has had its say — mirrors `dart::wire_field_name`.
fn wire_field_name(rust_name: &str, rename: Option<&str>, rule: RenameRule) -> String {
    rename.map_or_else(|| rule.apply_to_field(rust_name), ToOwned::to_owned)
}

/// A container's own `rename_all`, or [`RenameRule::None`] without the `serde` feature.
#[cfg(feature = "serde")]
fn container_rename_rule(attrs: &[syn::Attribute]) -> RenameRule {
    let meta = parse_serde_type_attributes(attrs);
    resolve_rename_rule(meta.rename_all.as_deref())
}

#[cfg(not(feature = "serde"))]
fn container_rename_rule(_attrs: &[syn::Attribute]) -> RenameRule {
    resolve_rename_rule(None)
}

/// The container attributes an enum's own dispatch reads.
#[cfg(feature = "serde")]
fn enum_tag_attrs(attrs: &[syn::Attribute]) -> EnumTagAttrs {
    let meta = parse_serde_type_attributes(attrs);
    EnumTagAttrs {
        content: meta.content,
        rename_all: meta.rename_all,
        rename_all_fields: meta.rename_all_fields,
        tag: meta.tag,
        untagged: meta.untagged,
    }
}

#[cfg(not(feature = "serde"))]
const fn enum_tag_attrs(_attrs: &[syn::Attribute]) -> EnumTagAttrs {
    EnumTagAttrs {
        content: None,
        rename_all: None,
        rename_all_fields: None,
        tag: None,
        untagged: false,
    }
}

// ---------------------------------------------------------------------------------------------
// Width table and per-field shape resolution.
// ---------------------------------------------------------------------------------------------

/// Whether `field` carries `#[model_schema_prop(as_number)]`.
#[cfg(feature = "chrono")]
fn has_as_number(field: &FieldDef) -> bool {
    field
        .model_schema_prop_meta
        .as_ref()
        .is_some_and(|meta| meta.as_number)
}

/// The Swift type a map key with wire form `key` names when it needs no wrapper — a String, or a
/// type whose own `Codable` conformance already spells a JSON object key (`Int` included).
fn swift_map_key_scalar(key: &FieldDef) -> String {
    match &key.field_type {
        FieldDefType::Boolean => "Bool".to_owned(),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            "String".to_owned()
        }
        FieldDefType::U8 => "UInt8".to_owned(),
        FieldDefType::U16 => "UInt16".to_owned(),
        FieldDefType::U32 => "UInt32".to_owned(),
        FieldDefType::I8 => "Int8".to_owned(),
        FieldDefType::I16 => "Int16".to_owned(),
        FieldDefType::I32 => "Int32".to_owned(),
        FieldDefType::I64 => "Int64".to_owned(),
        FieldDefType::Isize => "Int".to_owned(),
        FieldDefType::SiblingType(name, _) => {
            lookup_swift_name(name).unwrap_or_else(|| name.clone())
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => "Date".to_owned(),
        FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Map(_, _)
        | FieldDefType::Tuple(_)
        | FieldDefType::U64
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::Usize => "String".to_owned(),
        #[cfg(feature = "object_id")]
        FieldDefType::ObjectId => "String".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            "String".to_owned()
        }
    }
}

/// The expression reading one map key back from the `String` a wrapper's `DynamicCodingKey`
/// always carries it as, and the expression writing one back into that `String` — the pair
/// mirrors `dart::dart_map_key_decode`/`dart_map_key_encode`.
fn swift_map_key_codec(key: &FieldDef) -> (String, String) {
    match &key.field_type {
        FieldDefType::Boolean => (
            "(wireKey == \"true\" ? true : wireKey == \"false\" ? false : nil)".to_owned(),
            "(key ? \"true\" : \"false\")".to_owned(),
        ),
        FieldDefType::SiblingType(name, _) => {
            let type_name = lookup_swift_name(name).unwrap_or_else(|| name.clone());
            (
                format!("{type_name}(rawValue: wireKey)"),
                "key.rawValue".to_owned(),
            )
        }
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize => {
            let swift_type = swift_map_key_scalar(key);
            (format!("{swift_type}(wireKey)"), "String(key)".to_owned())
        }
        FieldDefType::Char
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Map(_, _)
        | FieldDefType::Tuple(_)
        | FieldDefType::U64
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::Usize => ("wireKey".to_owned(), "key".to_owned()),
        #[cfg(feature = "object_id")]
        FieldDefType::ObjectId => ("wireKey".to_owned(), "key".to_owned()),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            ("wireKey".to_owned(), "key".to_owned())
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => (
            "(isoFormatterForKeys.date(from: wireKey) ?? Date(timeIntervalSince1970: 0))"
                .to_owned(),
            "isoFormatterForKeys.string(from: key)".to_owned(),
        ),
    }
}

/// The per-field wrapper struct a non-string-keyed map earns: a `Codable` type over its own
/// `[Key: Value]` `storage`, decoding and encoding through a locally nested `DynamicCodingKey` so
/// two different fields' wrappers never collide even when their generated text is concatenated.
fn map_key_wrapper_struct(
    wrapper_name: &str,
    key_type: &str,
    value_type: &str,
    key: &FieldDef,
) -> String {
    let (decode_key, encode_key) = swift_map_key_codec(key);
    #[cfg(feature = "chrono")]
    let formatter_decl = if matches!(key.field_type, FieldDefType::DateTime) {
        "private let isoFormatterForKeys: ISO8601DateFormatter = { let f = ISO8601DateFormatter(); f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; return f }();"
    } else {
        ""
    };
    #[cfg(not(feature = "chrono"))]
    let formatter_decl = "";
    format!(
        "public struct {wrapper_name}: Codable, Sendable {{ \
         var storage: [{key_type}: {value_type}]; \
         {formatter_decl} \
         private struct DynamicCodingKey: CodingKey {{ \
         var stringValue: String; init?(stringValue: String) {{ self.stringValue = stringValue }}; \
         var intValue: Int? {{ nil }}; init?(intValue: Int) {{ nil }} }}; \
         init(storage: [{key_type}: {value_type}]) {{ self.storage = storage }}; \
         public init(from decoder: Decoder) throws {{ \
         let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
         var result: [{key_type}: {value_type}] = [:]; \
         for codingKey in container.allKeys {{ \
         let wireKey = codingKey.stringValue; \
         guard let key = {decode_key} else {{ \
         throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, debugDescription: \"bad key \\(wireKey)\") }}; \
         result[key] = try container.decode({value_type}.self, forKey: codingKey) }}; \
         storage = result }}; \
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: DynamicCodingKey.self); \
         for (key, value) in storage {{ \
         let wireKey = {encode_key}; \
         guard let codingKey = DynamicCodingKey(stringValue: wireKey) else {{ continue }}; \
         try container.encode(value, forKey: codingKey) }} }} }};"
    )
}

/// The Swift tuple-slot struct a `Tuple` field or a `TupleMultiple` variant payload earns: no
/// bare Swift tuple conforms to `Codable`, so each occurrence gets its own unkeyed-container
/// struct, named after the field it belongs to so two occurrences never collide.
fn tuple_struct(struct_name: &str, generic_params: &str, elements: &[(String, String)]) -> String {
    let slot_names: Vec<String> = (0..elements.len())
        .map(|index| format!("slot{index}"))
        .collect();
    let decls = slot_names
        .iter()
        .zip(elements)
        .fold(String::new(), |mut acc, (slot, (_, ty))| {
            write!(acc, "let {slot}: {ty}; ").unwrap();
            acc
        });
    let ctor_params = slot_names
        .iter()
        .zip(elements)
        .map(|(slot, (_, ty))| format!("{slot}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");
    let ctor_assigns = slot_names.iter().fold(String::new(), |mut acc, slot| {
        write!(acc, "self.{slot} = {slot}; ").unwrap();
        acc
    });
    let decode_assigns =
        slot_names
            .iter()
            .zip(elements)
            .fold(String::new(), |mut acc, (slot, (decode, _))| {
                write!(acc, "{slot} = try container.decode({decode}.self); ").unwrap();
                acc
            });
    let encode_calls = slot_names.iter().fold(String::new(), |mut acc, slot| {
        write!(acc, "try container.encode({slot}); ").unwrap();
        acc
    });
    format!(
        "public struct {struct_name}{generic_params}: Codable, Sendable {{ \
         {decls} \
         init({ctor_params}) {{ {ctor_assigns} }}; \
         public init(from decoder: Decoder) throws {{ \
         var container = try decoder.unkeyedContainer(); \
         {decode_assigns} }}; \
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.unkeyedContainer(); \
         {encode_calls} }} }};"
    )
}

/// `field`'s shape before array/optional wrapping: the property type, the type a `container` call
/// decodes, and what turns the second into the first, generating an auxiliary struct into `aux`
/// when the leaf needs one of its own.
fn swift_field_shape(field: &FieldDef, name_hint: &str, aux: &mut Vec<String>) -> SwiftFieldShape {
    match &field.field_type {
        FieldDefType::Unknown => direct_shape("String".to_owned()),
        FieldDefType::TypeParam(name) => direct_shape(name.clone()),
        FieldDefType::Tuple(elements) => tuple_shape(elements, name_hint, aux),
        FieldDefType::SiblingType(name, generics) => sibling_shape(field, name, generics, aux),
        FieldDefType::Map(key, value) => map_shape(key, value, name_hint, aux),
        FieldDefType::Boolean | FieldDefType::BooleanLiteral(_) => direct_shape("Bool".to_owned()),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            direct_shape("String".to_owned())
        }
        FieldDefType::NumberLiteral(_) | FieldDefType::F64 => direct_shape("Double".to_owned()),
        FieldDefType::F32 => direct_shape("Float".to_owned()),
        FieldDefType::U8 => direct_shape("UInt8".to_owned()),
        FieldDefType::U16 => direct_shape("UInt16".to_owned()),
        FieldDefType::U32 => direct_shape("UInt32".to_owned()),
        FieldDefType::U64 => direct_shape("UInt64".to_owned()),
        FieldDefType::I8 => direct_shape("Int8".to_owned()),
        FieldDefType::I16 => direct_shape("Int16".to_owned()),
        FieldDefType::I32 => direct_shape("Int32".to_owned()),
        FieldDefType::I64 => direct_shape("Int64".to_owned()),
        FieldDefType::Usize | FieldDefType::Isize => direct_shape("Int".to_owned()),
        #[cfg(feature = "object_id")]
        FieldDefType::ObjectId => direct_shape("ObjectId".to_owned()),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            direct_shape("String".to_owned())
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => datetime_shape(field),
    }
}

/// A leaf whose wire type is exactly its real type — no conversion, the common case.
fn direct_shape(swift_type: String) -> SwiftFieldShape {
    SwiftFieldShape {
        leaf_conversion: LeafConversion::None,
        real_leaf: swift_type.clone(),
        wire_leaf: swift_type,
    }
}

/// The `DateTime<Tz>` leaf: `Date` (or `Int64` under `as_number`) is the real type, the ISO 8601
/// string chrono writes is the wire type.
#[cfg(feature = "chrono")]
fn datetime_shape(field: &FieldDef) -> SwiftFieldShape {
    if has_as_number(field) {
        SwiftFieldShape {
            leaf_conversion: LeafConversion::DateTimeAsNumber,
            real_leaf: "Int64".to_owned(),
            wire_leaf: "String".to_owned(),
        }
    } else {
        SwiftFieldShape {
            leaf_conversion: LeafConversion::DateTimePlain,
            real_leaf: "Date".to_owned(),
            wire_leaf: "String".to_owned(),
        }
    }
}

/// A `Tuple` leaf: the aux struct is both the real and the wire type, so no conversion runs — the
/// struct's own `Codable` already speaks the array serde writes.
fn tuple_shape(elements: &[FieldDef], name_hint: &str, aux: &mut Vec<String>) -> SwiftFieldShape {
    let struct_name = format!("{name_hint}Tuple");
    let rendered: Vec<(String, String)> = elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let element_type =
                swift_full_real_type(element, &format!("{struct_name}Slot{index}"), aux);
            (element_type.clone(), element_type)
        })
        .collect();
    aux.push(tuple_struct(&struct_name, "", &rendered));
    direct_shape(struct_name)
}

/// A `SiblingType` leaf: a sequence wrapper (`Vec`, `HashSet`, …) re-enters as the arrayed field
/// it stands for; otherwise the referenced item's own published name, with generic arguments
/// rendered the same way a field's own type is.
fn sibling_shape(
    field: &FieldDef,
    name: &str,
    generics: &[FieldDef],
    aux: &mut Vec<String>,
) -> SwiftFieldShape {
    if let [element] = generics
        && is_sequence_wrapper(name)
    {
        let combined = field.collection_element_field(element);
        return swift_field_shape(&combined, "Element", aux);
    }
    let class_name = lookup_swift_name(name).unwrap_or_else(|| name.to_owned());
    if generics.is_empty() {
        return direct_shape(class_name);
    }
    let arguments = generics
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            swift_full_real_type(argument, &format!("{class_name}Argument{index}"), aux)
        })
        .collect::<Vec<_>>()
        .join(", ");
    direct_shape(format!("{class_name}<{arguments}>"))
}

/// A `Map` leaf: a string-shaped key stays a plain `[String: Value]` (or `[Int: Value]`, both
/// natively `Codable`); any other key earns a wrapper struct.
fn map_shape(
    key: &FieldDef,
    value: &FieldDef,
    name_hint: &str,
    aux: &mut Vec<String>,
) -> SwiftFieldShape {
    let value_type = swift_full_real_type(value, &format!("{name_hint}Value"), aux);
    if matches!(key.map_key_wire(), MapKeyWire::Named)
        && matches!(
            key.field_type,
            FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_)
        )
    {
        return direct_shape(format!("[String: {value_type}]"));
    }
    if matches!(key.map_key_wire(), MapKeyWire::Named)
        && matches!(key.field_type, FieldDefType::Isize)
    {
        return direct_shape(format!("[Int: {value_type}]"));
    }
    let key_type = swift_map_key_scalar(key);
    let wrapper_name = format!("{name_hint}KeyedMap");
    aux.push(map_key_wrapper_struct(
        &wrapper_name,
        &key_type,
        &value_type,
        key,
    ));
    SwiftFieldShape {
        leaf_conversion: LeafConversion::MapWrapper(wrapper_name.clone()),
        real_leaf: format!("[{key_type}: {value_type}]"),
        wire_leaf: wrapper_name,
    }
}

/// `field`'s shape together with its full public Swift type: [`swift_field_shape`]'s real leaf,
/// wrapped in one `[...]` per array level, then the field's own outer `?`. The one seam that
/// pushes a field's auxiliary declarations, so every caller needing both calls this once.
fn swift_resolve(
    field: &FieldDef,
    name_hint: &str,
    aux: &mut Vec<String>,
) -> (String, SwiftFieldShape) {
    let shape = swift_field_shape(field, name_hint, aux);
    let wrapped = wrap_array_levels(field, &shape.real_leaf);
    let real_type = if field.is_optional() {
        format!("{wrapped}?")
    } else {
        wrapped
    };
    (real_type, shape)
}

/// [`swift_resolve`]'s type alone, for a caller that has no use for the shape.
fn swift_full_real_type(field: &FieldDef, name_hint: &str, aux: &mut Vec<String>) -> String {
    swift_resolve(field, name_hint, aux).0
}

/// The Swift type name a field earns, together with any auxiliary declarations it needs (a
/// tuple-slot struct, a non-string map-key wrapper) — exposed for a `#[service_schema]` client to
/// name a message, success or error type by the same spelling its own declaration publishes.
#[cfg(all(feature = "serde", feature = "swift"))]
pub fn swift_reference_type(field: &FieldDef, name_hint: &str) -> (String, Vec<String>) {
    let mut aux = Vec::new();
    let real_type = swift_full_real_type(field, name_hint, &mut aux);
    (real_type, aux)
}

/// The wire counterpart of [`swift_full_real_type`]'s array wrapping, without the field's own
/// outer optionality — a caller reading the wire value handles that separately, since
/// `decodeIfPresent` already returns the optional it names.
fn swift_wire_array_type(field: &FieldDef, shape: &SwiftFieldShape) -> String {
    wrap_array_levels(field, &shape.wire_leaf)
}

/// `[T]` folded once per array level `field` declares, an inner level's own nullability carried
/// as `?` inside the brackets.
fn wrap_array_levels(field: &FieldDef, leaf: &str) -> String {
    (0..field.array_depth).fold(leaf.to_owned(), |wrapped, level| {
        let item = if field.is_nullable_at(level) {
            format!("{wrapped}?")
        } else {
            wrapped
        };
        format!("[{item}]")
    })
}

// ---------------------------------------------------------------------------------------------
// Decode/encode statement builders.
// ---------------------------------------------------------------------------------------------

/// The leaf conversion reading `expr` (a value of [`SwiftFieldShape::wire_leaf`]'s type) into
/// [`SwiftFieldShape::real_leaf`]'s type.
fn decode_leaf(shape: &SwiftFieldShape, expr: &str) -> String {
    match &shape.leaf_conversion {
        LeafConversion::None => expr.to_owned(),
        LeafConversion::MapWrapper(_) => format!("{expr}.storage"),
        #[cfg(feature = "chrono")]
        LeafConversion::DateTimePlain => {
            format!("try Self.swiftSchemaParseIso8601Date({expr}, codingPath: decoder.codingPath)")
        }
        #[cfg(feature = "chrono")]
        LeafConversion::DateTimeAsNumber => format!(
            "try Self.swiftSchemaParseIso8601Millis({expr}, codingPath: decoder.codingPath)"
        ),
    }
}

/// The leaf conversion writing `expr` (a value of [`SwiftFieldShape::real_leaf`]'s type) as
/// [`SwiftFieldShape::wire_leaf`]'s type — the encode counterpart of [`decode_leaf`].
fn encode_leaf(shape: &SwiftFieldShape, expr: &str) -> String {
    match &shape.leaf_conversion {
        LeafConversion::None => expr.to_owned(),
        LeafConversion::MapWrapper(wrapper) => format!("{wrapper}(storage: {expr})"),
        #[cfg(feature = "chrono")]
        LeafConversion::DateTimePlain => format!("Self.swiftSchemaFormatIso8601Date({expr})"),
        #[cfg(feature = "chrono")]
        LeafConversion::DateTimeAsNumber => format!("Self.swiftSchemaFormatIso8601Millis({expr})"),
    }
}

/// [`decode_leaf`], peeled outward through `level` array levels via `.map`, `$0` reused at every
/// level the way `dart::dart_decode_at` reuses `e` — mirrors the array-wrap fold in
/// [`wrap_array_levels`] the other direction.
fn decode_conversion(field: &FieldDef, shape: &SwiftFieldShape, level: u8, expr: &str) -> String {
    if matches!(shape.leaf_conversion, LeafConversion::None) {
        return expr.to_owned();
    }
    if level == 0 {
        return decode_leaf(shape, expr);
    }
    let inner_level = level - 1;
    let inner = decode_conversion(field, shape, inner_level, "$0");
    if field.is_nullable_at(inner_level) {
        format!("try {expr}.map {{ try $0.map {{ {inner} }} }}")
    } else {
        format!("try {expr}.map {{ {inner} }}")
    }
}

/// The encode counterpart of [`decode_conversion`].
fn encode_conversion(field: &FieldDef, shape: &SwiftFieldShape, level: u8, expr: &str) -> String {
    if matches!(shape.leaf_conversion, LeafConversion::None) {
        return expr.to_owned();
    }
    if level == 0 {
        return encode_leaf(shape, expr);
    }
    let inner_level = level - 1;
    let inner = encode_conversion(field, shape, inner_level, "$0");
    if field.is_nullable_at(inner_level) {
        format!("{expr}.map {{ $0.map {{ {inner} }} }}")
    } else {
        format!("{expr}.map {{ {inner} }}")
    }
}

/// The `self.{name} = ...` statement a field earns inside a hand-written `init(from:)`.
fn decode_statement(field: &SwiftField, container: &str) -> String {
    if field.flatten {
        return format!(
            "self.{swift} = try {real_type}(from: decoder)",
            swift = field.swift_name,
            real_type = field.real_type,
        );
    }
    if matches!(field.shape.leaf_conversion, LeafConversion::None) {
        return if field.required {
            format!(
                "self.{swift} = try {container}.decode({real_type}.self, forKey: .{swift})",
                swift = field.swift_name,
                real_type = field.real_type,
            )
        } else {
            let inner = field
                .real_type
                .strip_suffix('?')
                .unwrap_or(&field.real_type);
            format!(
                "self.{swift} = try {container}.decodeIfPresent({inner}.self, forKey: .{swift})",
                swift = field.swift_name
            )
        };
    }
    let wire_array = swift_wire_array_type(&field.field_def, &field.shape);
    let read = if field.required {
        if field.field_def.is_optional() {
            format!(
                "try {container}.decode({wire_array}?.self, forKey: .{swift})",
                swift = field.swift_name
            )
        } else {
            format!(
                "try {container}.decode({wire_array}.self, forKey: .{swift})",
                swift = field.swift_name
            )
        }
    } else {
        format!(
            "try {container}.decodeIfPresent({wire_array}.self, forKey: .{swift})",
            swift = field.swift_name
        )
    };
    let local = format!("wire{}", swift_upper_camel(&field.swift_name));
    let convert = if field.field_def.is_optional() {
        let inner = decode_conversion(
            &field.field_def,
            &field.shape,
            field.field_def.array_depth,
            "$0",
        );
        format!("try {local}.map {{ {inner} }}")
    } else {
        decode_conversion(
            &field.field_def,
            &field.shape,
            field.field_def.array_depth,
            &local,
        )
    };
    format!(
        "let {local} = {read}; self.{} = {convert}",
        field.swift_name
    )
}

/// The `try container.encode...` statement a field earns inside a hand-written `encode(to:)`.
fn encode_statement(field: &SwiftField, container: &str) -> String {
    if field.flatten {
        return format!("try {}.encode(to: encoder)", field.swift_name);
    }
    let source = if matches!(field.shape.leaf_conversion, LeafConversion::None) {
        field.swift_name.clone()
    } else {
        let convert = if field.field_def.is_optional() {
            let inner = encode_conversion(
                &field.field_def,
                &field.shape,
                field.field_def.array_depth,
                "$0",
            );
            format!("{}.map {{ {inner} }}", field.swift_name)
        } else {
            encode_conversion(
                &field.field_def,
                &field.shape,
                field.field_def.array_depth,
                &field.swift_name,
            )
        };
        let local = format!("wire{}", swift_upper_camel(&field.swift_name));
        return if field.required {
            format!(
                "let {local} = {convert}; try {container}.encode({local}, forKey: .{swift})",
                swift = field.swift_name
            )
        } else {
            format!(
                "let {local} = {convert}; try {container}.encodeIfPresent({local}, forKey: .{swift})",
                swift = field.swift_name
            )
        };
    };
    if field.required {
        format!(
            "try {container}.encode({source}, forKey: .{swift})",
            swift = field.swift_name
        )
    } else {
        format!(
            "try {container}.encodeIfPresent({source}, forKey: .{swift})",
            swift = field.swift_name
        )
    }
}

/// The per-struct static helpers a `DateTime` field's decode/encode calls into — one shared
/// formatter with fractional seconds, so the emitted round trip is byte-for-byte the ISO 8601
/// text chrono wrote.
#[cfg(feature = "chrono")]
fn datetime_helpers(export_name: &str) -> String {
    format!(
        "extension {export_name} {{ \
         private static let swiftSchemaIso8601Formatter: ISO8601DateFormatter = {{ \
         let f = ISO8601DateFormatter(); f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; return f }}(); \
         private static func swiftSchemaParseIso8601Date(_ text: String, codingPath: [CodingKey]) throws -> Date {{ \
         guard let date = swiftSchemaIso8601Formatter.date(from: text) else {{ \
         throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: codingPath, debugDescription: \"bad ISO8601 date: \\(text)\")) }}; \
         return date }}; \
         private static func swiftSchemaFormatIso8601Date(_ date: Date) -> String {{ swiftSchemaIso8601Formatter.string(from: date) }}; \
         private static func swiftSchemaParseIso8601Millis(_ text: String, codingPath: [CodingKey]) throws -> Int64 {{ \
         let date = try swiftSchemaParseIso8601Date(text, codingPath: codingPath); \
         return Int64((date.timeIntervalSince1970 * 1000).rounded()) }}; \
         private static func swiftSchemaFormatIso8601Millis(_ millis: Int64) -> String {{ \
         swiftSchemaIso8601Formatter.string(from: Date(timeIntervalSince1970: Double(millis) / 1000)) }} }};"
    )
}

/// Whether any field needs [`datetime_helpers`] — a scalar or array/optional `DateTime<Tz>`
/// leaf, whose custom decode/encode reads them.
#[cfg(feature = "chrono")]
fn fields_need_datetime_helpers(fields: &[SwiftField]) -> bool {
    fields.iter().any(|field| {
        matches!(
            field.shape.leaf_conversion,
            LeafConversion::DateTimePlain | LeafConversion::DateTimeAsNumber
        )
    })
}

#[cfg(feature = "chrono")]
fn datetime_helpers_for(export_name: &str, fields: &[SwiftField]) -> String {
    if fields_need_datetime_helpers(fields) {
        datetime_helpers(export_name)
    } else {
        String::new()
    }
}

#[cfg(not(feature = "chrono"))]
const fn datetime_helpers_for(_export_name: &str, _fields: &[SwiftField]) -> String {
    String::new()
}

// ---------------------------------------------------------------------------------------------
// Struct field collection and class body assembly — shared by a named-field struct and a
// struct-shaped enum variant's own payload type.
// ---------------------------------------------------------------------------------------------

/// Walks a named-field struct's or a struct-shaped enum variant's fields into [`SwiftField`]s,
/// dropping any field a serde attribute takes off the wire in both directions — mirrors
/// `dart::collect_dart_fields`.
fn collect_swift_fields(
    fields: &Fields,
    rule: RenameRule,
    type_parameters: &[String],
    name_hint: &str,
    aux: &mut Vec<String>,
) -> Vec<SwiftField> {
    let Fields::Named(named) = fields else {
        return Vec::new();
    };
    let mut collected = Vec::new();
    for field in &named.named {
        let Some(ident) = field.ident.as_ref() else {
            continue;
        };
        let rust_name = ident.to_string();
        let omission = parse_serde_key_omission(&field.attrs);
        if omission.absent_from_wire() {
            continue;
        }
        let wire_name = wire_field_name(&rust_name, rename_override(&field.attrs).as_deref(), rule);
        let mut field_def = field_def_with_prop_meta(&rust_name, &field.ty, &field.attrs);
        field_def.erase_type_parameters(type_parameters);
        if omission.omits_key && !field_def.is_optional() {
            field_def.nullable_levels.push(field_def.array_depth);
        }
        let swift_name = RenameRule::CamelCase.apply_to_field(&rust_name);
        let field_hint = format!("{name_hint}{}", swift_upper_camel(&swift_name));
        let (real_type, shape) = swift_resolve(&field_def, &field_hint, aux);
        collected.push(SwiftField {
            field_def,
            flatten: field_is_flatten(&field.attrs),
            real_type,
            required: !omission.omits_key,
            shape,
            swift_name,
            wire_name,
        });
    }
    collected
}

/// Whether `field` is declared `#[model_schema_prop(nullable)]` — required key, encode always
/// written even when `nil`.
fn field_is_nullable_flag(field: &SwiftField) -> bool {
    field
        .field_def
        .model_schema_prop_meta
        .as_ref()
        .is_some_and(|meta| meta.nullable)
}

/// The `enum CodingKeys: String, CodingKey { ... }` a struct or a struct-shaped variant payload
/// earns — every field listed, a `= "wire"` suffix only where the wire spelling differs from the
/// Swift property name, `extra` cases (a merged tag key) listed first.
///
/// Empty for a shape with no field and no extra case — Swift refuses a raw-typed enum with no
/// cases, and needs none: `Codable`'s own synthesis already writes and reads `{}` for it.
fn swift_coding_keys(fields: &[SwiftField], extra: &[(String, String)]) -> String {
    if fields.is_empty() && extra.is_empty() {
        return String::new();
    }
    let mut cases = String::new();
    for (name, wire) in extra {
        write!(cases, "case {name} = \"{wire}\"; ").unwrap();
    }
    for field in fields {
        if field.swift_name == field.wire_name {
            write!(cases, "case {}; ", field.swift_name).unwrap();
        } else {
            write!(
                cases,
                "case {} = \"{}\"; ",
                field.swift_name, field.wire_name
            )
            .unwrap();
        }
    }
    format!("enum CodingKeys: String, CodingKey {{ {cases} }};")
}

/// The body (properties, `CodingKeys`, and — only where synthesis cannot reach — `init(from:)`,
/// `encode(to:)` and the memberwise initializer it costs) for a set of [`SwiftField`]s.
/// `extra_coding_keys` lists cases with no matching field (an internally-tagged variant's tag).
fn struct_body_content(fields: &[SwiftField], extra_coding_keys: &[(String, String)]) -> String {
    let props = fields.iter().fold(String::new(), |mut acc, field| {
        write!(
            acc,
            "public let {}: {}; ",
            field.swift_name, field.real_type
        )
        .unwrap();
        acc
    });
    let coding_keys = swift_coding_keys(fields, extra_coding_keys);
    let needs_decode = fields
        .iter()
        .any(|field| field.flatten || !matches!(field.shape.leaf_conversion, LeafConversion::None));
    let needs_encode = needs_decode || fields.iter().any(field_is_nullable_flag);

    let memberwise_init = if needs_decode {
        memberwise_initializer(fields)
    } else {
        String::new()
    };
    let init_method = if needs_decode {
        let statements = fields
            .iter()
            .map(|field| decode_statement(field, "container"))
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "public init(from decoder: Decoder) throws {{ \
             let container = try decoder.container(keyedBy: CodingKeys.self); {statements} }}; "
        )
    } else {
        String::new()
    };
    let encode_method = if needs_encode {
        let statements = fields
            .iter()
            .map(|field| encode_statement(field, "container"))
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "public func encode(to encoder: Encoder) throws {{ \
             var container = encoder.container(keyedBy: CodingKeys.self); {statements} }} "
        )
    } else {
        String::new()
    };
    format!("{props}{coding_keys} {memberwise_init}{init_method}{encode_method}")
}

/// The explicit memberwise `public init(...)` a custom `init(from:)` costs a struct — Swift
/// synthesizes one only when no initializer is declared at all.
fn memberwise_initializer(fields: &[SwiftField]) -> String {
    let params = fields
        .iter()
        .map(|field| format!("{}: {}", field.swift_name, field.real_type))
        .collect::<Vec<_>>()
        .join(", ");
    let assigns = fields.iter().fold(String::new(), |mut acc, field| {
        write!(acc, "self.{0} = {0}; ", field.swift_name).unwrap();
        acc
    });
    format!("public init({params}) {{ {assigns} }}; ")
}

/// The `public struct {export_name}{generics}: Codable, Sendable { ... }` a named-field struct or
/// a struct-shaped variant payload earns, with the `DateTime` helpers appended when a field needs
/// them.
fn struct_declaration(export_name: &str, generic_params: &str, fields: &[SwiftField]) -> String {
    let content = struct_body_content(fields, &[]);
    let helpers = datetime_helpers_for(export_name, fields);
    format!(
        "public struct {export_name}{generic_params}: Codable, Sendable {{ {content} }}; {helpers}"
    )
}

// ---------------------------------------------------------------------------------------------
// Structs, branded newtypes, tuple structs and aliases.
// ---------------------------------------------------------------------------------------------

/// The Swift tokens a named-field struct earns: a struct, plus a `typealias` under its own Rust
/// ident when `name = "..."` moved its published name elsewhere.
fn struct_swift_tokens(item_struct: &ItemStruct, name_override: Option<&str>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    if has_serde_transparent(&item_struct.attrs) && is_single_slot(&item_struct.fields) {
        let value_field = single_slot_field(&item_struct.fields, &type_parameters);
        return value_wrapper_tokens(
            &item_struct.ident,
            &item_struct.generics,
            name_override,
            &value_field,
        );
    }
    if matches!(item_struct.fields, Fields::Unnamed(_)) {
        return tuple_struct_swift_tokens(item_struct, name_override);
    }

    let rust_ident = item_struct.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, name_override);
    register_swift_name(&rust_ident, &export_name);

    let rule = container_rename_rule(&item_struct.attrs);
    let generic_params = swift_generic_params(&item_struct.generics);
    let mut aux = Vec::new();
    let fields = collect_swift_fields(
        &item_struct.fields,
        rule,
        &type_parameters,
        &export_name,
        &mut aux,
    );
    let body = struct_declaration(&export_name, &generic_params, &fields);
    let typealias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let swift_source = if aux.is_empty() {
        format!("{body}{typealias}")
    } else {
        format!("{} {body}{typealias}", aux.join(" "))
    };

    swift_module_tokens(&rust_ident, item_struct.ident.span(), &swift_source)
}

/// Whether `fields` is a tuple shape (unnamed) with exactly one slot — the shape a branded
/// newtype and a bare-value tuple struct share on the wire.
fn is_single_slot(fields: &Fields) -> bool {
    matches!(fields, Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1)
}

/// The `FieldDef` of a single-slot tuple shape's one field, its type parameters already erased.
fn single_slot_field(fields: &Fields, type_parameters: &[String]) -> FieldDef {
    let Fields::Unnamed(unnamed) = fields else {
        return get_field_def("value", &syn::parse_quote!(()), "");
    };
    let Some(slot) = unnamed.unnamed.first() else {
        return get_field_def("value", &syn::parse_quote!(()), "");
    };
    let mut field_def = field_def_with_prop_meta("value", &slot.ty, &slot.attrs);
    field_def.erase_type_parameters(type_parameters);
    field_def
}

/// The Swift tokens for a value that carries no shape of its own on the wire beyond one wrapped
/// value: a branded newtype, a non-branded single-slot tuple struct, or a type alias. All three
/// publish a struct with one `public let value` and a single-value-container codec.
fn value_wrapper_tokens(
    ident: &Ident,
    generics: &syn::Generics,
    name_override: Option<&str>,
    value_field: &FieldDef,
) -> TokenStream {
    let rust_ident = ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, name_override);
    register_swift_name(&rust_ident, &export_name);

    let generic_params = swift_generic_params(generics);
    let mut aux = Vec::new();
    let (real_type, shape) = swift_resolve(value_field, &export_name, &mut aux);

    let decode = if matches!(shape.leaf_conversion, LeafConversion::None) {
        format!("try container.decode({real_type}.self)")
    } else {
        let wire_type = swift_wire_array_type(value_field, &shape);
        let read = if value_field.is_optional() {
            format!("try container.decode({wire_type}?.self)")
        } else {
            format!("try container.decode({wire_type}.self)")
        };
        let convert = if value_field.is_optional() {
            let inner = decode_conversion(value_field, &shape, value_field.array_depth, "$0");
            format!("try wireValue.map {{ {inner} }}")
        } else {
            decode_conversion(value_field, &shape, value_field.array_depth, "wireValue")
        };
        format!("{{ let wireValue = {read}; return {convert} }}()")
    };
    let encode = if matches!(shape.leaf_conversion, LeafConversion::None) {
        "try container.encode(value)".to_owned()
    } else if value_field.is_optional() {
        let inner = encode_conversion(value_field, &shape, value_field.array_depth, "$0");
        format!("try container.encode(value.map {{ {inner} }})")
    } else {
        let converted = encode_conversion(value_field, &shape, value_field.array_depth, "value");
        format!("try container.encode({converted})")
    };
    let helpers = datetime_helpers_for(
        &export_name,
        &[SwiftField {
            field_def: value_field.clone(),
            flatten: false,
            real_type: real_type.clone(),
            required: true,
            shape,
            swift_name: "value".to_owned(),
            wire_name: "value".to_owned(),
        }],
    );
    let body = format!(
        "public struct {export_name}{generic_params}: Codable, Sendable {{ \
         public let value: {real_type}; \
         public init(value: {real_type}) {{ self.value = value }}; \
         public init(from decoder: Decoder) throws {{ \
         let container = try decoder.singleValueContainer(); self.value = {decode} }}; \
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.singleValueContainer(); {encode} }} }}; {helpers}"
    );
    let typealias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let swift_source = if aux.is_empty() {
        format!("{body}{typealias}")
    } else {
        format!("{} {body}{typealias}", aux.join(" "))
    };

    swift_module_tokens(&rust_ident, ident.span(), &swift_source)
}

/// The Swift tokens for a non-branded tuple struct: the single-slot shape shares
/// [`value_wrapper_tokens`] with a branded newtype; a wider tuple struct wraps the fixed-size
/// tuple its slots describe.
fn tuple_struct_swift_tokens(item_struct: &ItemStruct, name_override: Option<&str>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let Fields::Unnamed(unnamed) = &item_struct.fields else {
        return TokenStream::new();
    };
    let slots: Vec<FieldDef> = unnamed
        .unnamed
        .iter()
        .filter(|slot| !parse_serde_key_omission(&slot.attrs).absent_from_wire())
        .map(|slot| {
            let mut field_def = field_def_with_prop_meta("slot", &slot.ty, &slot.attrs);
            field_def.erase_type_parameters(&type_parameters);
            field_def
        })
        .collect();
    let tuple_field = FieldDef {
        absent_from_wire: false,
        array_depth: 0,
        array_lengths: Vec::new(),
        docs: String::new(),
        field_type: FieldDefType::Tuple(slots),
        model_schema_prop_meta: None,
        name: "value".to_owned(),
        nullable_levels: Vec::new(),
        omits_value: false,
        #[cfg(feature = "jsonschema")]
        type_span: proc_macro2::Span::call_site(),
    };
    value_wrapper_tokens(
        &item_struct.ident,
        &item_struct.generics,
        name_override,
        &tuple_field,
    )
}

/// The Swift tokens a `type X = ...;` alias earns: `public typealias {export_name} = {target};`.
fn alias_swift_tokens(item_type: &ItemType, name_override: Option<&str>) -> TokenStream {
    let export_name = compute_alias_export_name(&item_type.ident.to_string(), name_override);
    register_swift_name(&item_type.ident.to_string(), &export_name);
    let type_parameters = type_parameters_in_scope(&item_type.generics);
    let mut target = get_field_def(&export_name, &item_type.ty, "");
    target.erase_type_parameters(&type_parameters);
    let generic_params = swift_generic_params(&item_type.generics);
    let mut aux = Vec::new();
    let target_type = swift_full_real_type(&target, &export_name, &mut aux);
    let body = format!("public typealias {export_name}{generic_params} = {target_type};");
    let rust_ident = item_type.ident.to_string();
    let typealias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let swift_source = if aux.is_empty() {
        format!("{body}{typealias}")
    } else {
        format!("{} {body}{typealias}", aux.join(" "))
    };
    swift_module_tokens(&rust_ident, item_type.ident.span(), &swift_source)
}

// ---------------------------------------------------------------------------------------------
// Enums: plain, externally tagged (the default), internally tagged, adjacently tagged, untagged.
// ---------------------------------------------------------------------------------------------

/// One variant's own payload, resolved to at most one associated value: `None` for a `Unit`
/// variant, an existing type for a `TupleSingle` slot, or a freshly generated struct (pushed
/// into `aux`) for a `Named` payload or a folded `TupleMultiple` tuple.
fn resolve_variant_payload(
    variant: &Variant,
    field_rule: RenameRule,
    type_parameters: &[String],
    export_name: &str,
    aux: &mut Vec<String>,
) -> Option<String> {
    match classify_variant(variant) {
        VariantKind::Unit => None,
        VariantKind::Named => {
            let struct_name = format!("{export_name}{}Payload", variant.ident);
            let fields = collect_swift_fields(
                &variant.fields,
                field_rule,
                type_parameters,
                &struct_name,
                aux,
            );
            aux.push(struct_declaration(&struct_name, "", &fields));
            Some(struct_name)
        }
        VariantKind::TupleSingle => {
            let slot = variant.fields.iter().next();
            let mut field_def = slot.map_or_else(
                || get_field_def("value", &syn::parse_quote!(()), ""),
                |field| field_def_with_prop_meta("value", &field.ty, &field.attrs),
            );
            field_def.erase_type_parameters(type_parameters);
            let hint = format!("{export_name}{}Value", variant.ident);
            Some(swift_full_real_type(&field_def, &hint, aux))
        }
        VariantKind::TupleMultiple => {
            let slots: Vec<FieldDef> = variant
                .fields
                .iter()
                .map(|field| {
                    let mut field_def = field_def_with_prop_meta("slot", &field.ty, &field.attrs);
                    field_def.erase_type_parameters(type_parameters);
                    field_def
                })
                .collect();
            let tuple_field = FieldDef {
                absent_from_wire: false,
                array_depth: 0,
                array_lengths: Vec::new(),
                docs: String::new(),
                field_type: FieldDefType::Tuple(slots),
                model_schema_prop_meta: None,
                name: "value".to_owned(),
                nullable_levels: Vec::new(),
                omits_value: false,
                #[cfg(feature = "jsonschema")]
                type_span: proc_macro2::Span::call_site(),
            };
            let hint = format!("{export_name}{}", variant.ident);
            Some(swift_full_real_type(&tuple_field, &hint, aux))
        }
    }
}

/// One `case {name}({payload});` or `case {name};` declaration.
fn variant_case(case_name: &str, payload_type: Option<&str>) -> String {
    payload_type.map_or_else(
        || format!("case {case_name}; "),
        |ty| format!("case {case_name}({ty}); "),
    )
}

/// The Swift tokens a plain (all-unit, string-wire) enum earns: a `String`-backed enum, the wire
/// spelling carried directly as each case's raw value.
fn plain_enum_swift_source(item_enum: &ItemEnum, rust_ident: &str, export_name: &str) -> String {
    let rule = container_rename_rule(&item_enum.attrs);
    let cases = item_enum
        .variants
        .iter()
        .fold(String::new(), |mut acc, variant| {
            let rust_name = variant.ident.to_string();
            let case_name = RenameRule::CamelCase.apply_to_variant(&rust_name);
            let wire = rename_override(&variant.attrs)
                .unwrap_or_else(|| rule.apply_to_variant(&rust_name));
            write!(acc, "case {case_name} = \"{wire}\"; ").unwrap();
            acc
        });
    let body = format!("public enum {export_name}: String, Codable, Sendable {{ {cases} }}");
    let typealias = ident_typealias(rust_ident, export_name, "");
    format!("{body}{typealias}")
}

/// The Swift tokens the externally tagged shape earns — serde's default once a variant carries
/// data. Swift's own associated-value synthesis wraps a payload under an auto-generated `_0` key
/// rather than serde's bare `{"Foo": payload}`, so this writes the codec by hand instead.
fn external_tagged_enum_swift_source(
    item_enum: &ItemEnum,
    rust_ident: &str,
    export_name: &str,
    tag_attrs: &EnumTagAttrs,
    type_parameters: &[String],
    generic_params: &str,
    aux: &mut Vec<String>,
) -> String {
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let mut cases = String::new();
    let mut unit_decode_arms = String::new();
    let mut object_decode_arms = String::new();
    let mut encode_arms = String::new();
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = RenameRule::CamelCase.apply_to_variant(&rust_name);
        let wire_tag = rename_override(&variant.attrs)
            .unwrap_or_else(|| variant_rule.apply_to_variant(&rust_name));
        let payload_type =
            resolve_variant_payload(variant, field_rule, type_parameters, export_name, aux);
        cases.push_str(&variant_case(&case_name, payload_type.as_deref()));
        if let Some(ty) = &payload_type {
            write!(
                object_decode_arms,
                "case \"{wire_tag}\": self = .{case_name}(try container.decode({ty}.self, forKey: key)); "
            )
            .unwrap();
            write!(
                encode_arms,
                "case .{case_name}(let payload): var container = encoder.container(keyedBy: SwiftSchemaExternalCodingKey.self); \
                 try container.encode(payload, forKey: SwiftSchemaExternalCodingKey(stringValue: \"{wire_tag}\")!); "
            )
            .unwrap();
        } else {
            write!(
                unit_decode_arms,
                "case \"{wire_tag}\": self = .{case_name}; return; "
            )
            .unwrap();
            write!(
                encode_arms,
                "case .{case_name}: var single = encoder.singleValueContainer(); try single.encode(\"{wire_tag}\"); "
            )
            .unwrap();
        }
    }
    let key_type = "private struct SwiftSchemaExternalCodingKey: CodingKey { \
         var stringValue: String; init?(stringValue: String) { self.stringValue = stringValue }; \
         var intValue: Int? { nil }; init?(intValue: Int) { nil } };"
        .to_owned();
    let init_method = format!(
        "public init(from decoder: Decoder) throws {{ \
         if let single = try? decoder.singleValueContainer(), let tag = try? single.decode(String.self) {{ \
         switch tag {{ {unit_decode_arms} default: break }} }}; \
         let container = try decoder.container(keyedBy: SwiftSchemaExternalCodingKey.self); \
         guard let key = container.allKeys.first else {{ \
         throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"empty object for {export_name}\")) }}; \
         switch key.stringValue {{ {object_decode_arms} \
         default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(key.stringValue)\")) }} }};"
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ switch self {{ {encode_arms} }} }};"
    );
    let body = format!(
        "public enum {export_name}{generic_params}: Codable, Sendable {{ {cases} {key_type} {init_method} {encode_method} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{typealias}")
}

/// The Swift tokens the internally tagged shape earns (`tag = "..."`, no `content`): the tag
/// merges into the same object a `Named` payload's own fields sit in, so a variant's own payload
/// struct decodes/encodes through the *same* `decoder`/`encoder` the enum's own tag read from.
fn internal_tagged_enum_swift_source(
    item_enum: &ItemEnum,
    rust_ident: &str,
    export_name: &str,
    tag_attrs: &EnumTagAttrs,
    type_parameters: &[String],
    generic_params: &str,
    aux: &mut Vec<String>,
) -> String {
    let tag_key = tag_attrs.tag.as_deref().unwrap_or("type");
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let mut cases = String::new();
    let mut decode_arms = String::new();
    let mut encode_arms = String::new();
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = RenameRule::CamelCase.apply_to_variant(&rust_name);
        let wire_tag = rename_override(&variant.attrs)
            .unwrap_or_else(|| variant_rule.apply_to_variant(&rust_name));
        let payload_type =
            resolve_variant_payload(variant, field_rule, type_parameters, export_name, aux);
        cases.push_str(&variant_case(&case_name, payload_type.as_deref()));
        if let Some(ty) = &payload_type {
            write!(
                decode_arms,
                "case \"{wire_tag}\": self = .{case_name}(try {ty}(from: decoder)); "
            )
            .unwrap();
            write!(
                encode_arms,
                "case .{case_name}(let payload): try container.encode(\"{wire_tag}\", forKey: .swiftSchemaTag); \
                 try payload.encode(to: encoder); "
            )
            .unwrap();
        } else {
            write!(decode_arms, "case \"{wire_tag}\": self = .{case_name}; ").unwrap();
            write!(
                encode_arms,
                "case .{case_name}: try container.encode(\"{wire_tag}\", forKey: .swiftSchemaTag); "
            )
            .unwrap();
        }
    }
    let tag_coding_key = format!(
        "private enum SwiftSchemaTagCodingKeys: String, CodingKey {{ case swiftSchemaTag = \"{tag_key}\" }};"
    );
    let init_method = format!(
        "public init(from decoder: Decoder) throws {{ \
         let container = try decoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); \
         let tag = try container.decode(String.self, forKey: .swiftSchemaTag); \
         switch tag {{ {decode_arms} default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(tag)\")) }} }};"
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); \
         switch self {{ {encode_arms} }} }}"
    );
    let body = format!(
        "public enum {export_name}{generic_params}: Codable, Sendable {{ {cases} {tag_coding_key} {init_method} {encode_method} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{typealias}")
}

/// The Swift tokens the adjacently tagged shape earns (`tag = "...", content = "..."`): the tag
/// and the payload sit at two keys of the same object, so a variant's own payload decodes and
/// encodes through the `content` key directly, needing no shared-decoder trick.
fn adjacent_tagged_enum_swift_source(
    item_enum: &ItemEnum,
    rust_ident: &str,
    export_name: &str,
    tag_attrs: &EnumTagAttrs,
    type_parameters: &[String],
    generic_params: &str,
    aux: &mut Vec<String>,
) -> String {
    let tag_key = tag_attrs.tag.as_deref().unwrap_or("type");
    let content_key = tag_attrs.content.as_deref().unwrap_or("content");
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let mut cases = String::new();
    let mut decode_arms = String::new();
    let mut encode_arms = String::new();
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = RenameRule::CamelCase.apply_to_variant(&rust_name);
        let wire_tag = rename_override(&variant.attrs)
            .unwrap_or_else(|| variant_rule.apply_to_variant(&rust_name));
        let payload_type =
            resolve_variant_payload(variant, field_rule, type_parameters, export_name, aux);
        cases.push_str(&variant_case(&case_name, payload_type.as_deref()));
        if let Some(ty) = &payload_type {
            write!(
                decode_arms,
                "case \"{wire_tag}\": self = .{case_name}(try container.decode({ty}.self, forKey: .swiftSchemaContent)); "
            )
            .unwrap();
            write!(
                encode_arms,
                "case .{case_name}(let payload): try container.encode(\"{wire_tag}\", forKey: .swiftSchemaTag); \
                 try container.encode(payload, forKey: .swiftSchemaContent); "
            )
            .unwrap();
        } else {
            write!(decode_arms, "case \"{wire_tag}\": self = .{case_name}; ").unwrap();
            write!(
                encode_arms,
                "case .{case_name}: try container.encode(\"{wire_tag}\", forKey: .swiftSchemaTag); "
            )
            .unwrap();
        }
    }
    let tagging_keys = format!(
        "private enum SwiftSchemaAdjacentCodingKeys: String, CodingKey {{ \
         case swiftSchemaContent = \"{content_key}\", swiftSchemaTag = \"{tag_key}\" }};"
    );
    let init_method = format!(
        "public init(from decoder: Decoder) throws {{ \
         let container = try decoder.container(keyedBy: SwiftSchemaAdjacentCodingKeys.self); \
         let tag = try container.decode(String.self, forKey: .swiftSchemaTag); \
         switch tag {{ {decode_arms} default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(tag)\")) }} }};"
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: SwiftSchemaAdjacentCodingKeys.self); \
         switch self {{ {encode_arms} }} }}"
    );
    let body = format!(
        "public enum {export_name}{generic_params}: Codable, Sendable {{ {cases} {tagging_keys} {init_method} {encode_method} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{typealias}")
}

/// The Swift tokens a `#[serde(untagged)]` enum earns: `init(from:)` tries each variant's own
/// decode in turn, returning the first that does not throw.
fn untagged_enum_swift_source(
    item_enum: &ItemEnum,
    rust_ident: &str,
    export_name: &str,
    tag_attrs: &EnumTagAttrs,
    type_parameters: &[String],
    generic_params: &str,
    aux: &mut Vec<String>,
) -> String {
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let mut cases = String::new();
    let mut decode_attempts = String::new();
    let mut encode_arms = String::new();
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = RenameRule::CamelCase.apply_to_variant(&rust_name);
        let payload_type =
            resolve_variant_payload(variant, field_rule, type_parameters, export_name, aux);
        cases.push_str(&variant_case(&case_name, payload_type.as_deref()));
        if let Some(ty) = &payload_type {
            write!(
                decode_attempts,
                "if let value = try? {ty}(from: decoder) {{ self = .{case_name}(value); return }}; "
            )
            .unwrap();
            write!(
                encode_arms,
                "case .{case_name}(let value): try value.encode(to: encoder); "
            )
            .unwrap();
        } else {
            write!(
                decode_attempts,
                "if let single = try? decoder.singleValueContainer(), single.decodeNil() {{ self = .{case_name}; return }}; "
            )
            .unwrap();
            write!(
                encode_arms,
                "case .{case_name}: var single = encoder.singleValueContainer(); try single.encodeNil(); "
            )
            .unwrap();
        }
    }
    let init_method = format!(
        "public init(from decoder: Decoder) throws {{ {decode_attempts} \
         throw DecodingError.typeMismatch({export_name}.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"no untagged member matched\")) }};"
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ switch self {{ {encode_arms} }} }}"
    );
    let body = format!(
        "public enum {export_name}{generic_params}: Codable, Sendable {{ {cases} {init_method} {encode_method} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{typealias}")
}

/// The Swift tokens an enum earns: a `String`-backed enum for an all-unit, untagged-by-default
/// declaration; otherwise the tagging shape `process_enum` itself dispatches the declaration to.
fn enum_swift_tokens(item_enum: &ItemEnum, name_override: Option<&str>) -> TokenStream {
    let rust_ident = item_enum.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, name_override);
    register_swift_name(&rust_ident, &export_name);
    let tag_attrs = enum_tag_attrs(&item_enum.attrs);
    let writes_bare_variant_names =
        tag_attrs.tag.is_none() && tag_attrs.content.is_none() && !tag_attrs.untagged;
    let type_parameters = type_parameters_in_scope(&item_enum.generics);
    let generic_params = swift_generic_params(&item_enum.generics);
    let mut aux = Vec::new();
    let swift_source = if is_plain_enum(item_enum) && writes_bare_variant_names {
        plain_enum_swift_source(item_enum, &rust_ident, &export_name)
    } else if tag_attrs.untagged {
        untagged_enum_swift_source(
            item_enum,
            &rust_ident,
            &export_name,
            &tag_attrs,
            &type_parameters,
            &generic_params,
            &mut aux,
        )
    } else if tag_attrs.tag.is_some() && tag_attrs.content.is_some() {
        adjacent_tagged_enum_swift_source(
            item_enum,
            &rust_ident,
            &export_name,
            &tag_attrs,
            &type_parameters,
            &generic_params,
            &mut aux,
        )
    } else if tag_attrs.tag.is_some() {
        internal_tagged_enum_swift_source(
            item_enum,
            &rust_ident,
            &export_name,
            &tag_attrs,
            &type_parameters,
            &generic_params,
            &mut aux,
        )
    } else {
        external_tagged_enum_swift_source(
            item_enum,
            &rust_ident,
            &export_name,
            &tag_attrs,
            &type_parameters,
            &generic_params,
            &mut aux,
        )
    };
    let full_source = if aux.is_empty() {
        swift_source
    } else {
        format!("{} {swift_source}", aux.join(" "))
    };
    swift_module_tokens(&rust_ident, item_enum.ident.span(), &full_source)
}
