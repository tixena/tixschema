//! Swift type and `Codable`-codec generation.

mod preprocess;
mod read;
mod validate;

use core::cell::RefCell;
use core::fmt::Write as _;
use core::iter;
use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt as _;
use syn::{Field, Fields, Ident, Item, ItemEnum, ItemStruct, ItemType, Variant};

use crate::features::model_schema_prop::{FunctionName, ModelSchemaPropMeta};
use crate::features::serde::defaults::{SerdeDefaults, substituted};
use crate::features::serde::{brand_field, parse_serde_key_omission, struct_tag};
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

/// The words Swift reserves in a declaration, a statement or an expression. A word reserved in one
/// context only, such as `get` or `type`, is not among them.
const SWIFT_RESERVED: [&str; 51] = [
    "as",
    "associatedtype",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "continue",
    "default",
    "defer",
    "deinit",
    "do",
    "else",
    "enum",
    "extension",
    "fallthrough",
    "false",
    "fileprivate",
    "for",
    "func",
    "guard",
    "if",
    "import",
    "in",
    "init",
    "inout",
    "internal",
    "is",
    "let",
    "nil",
    "operator",
    "precedencegroup",
    "private",
    "protocol",
    "public",
    "repeat",
    "rethrows",
    "return",
    "static",
    "struct",
    "subscript",
    "super",
    "switch",
    "throw",
    "throws",
    "true",
    "try",
    "typealias",
    "var",
    "where",
    "while",
];

/// One field this module has decided belongs on the wire, resolved to the Swift property it
/// earns: its Rust name, its lower-camel Swift name, its wire name, whether the key always
/// reaches the wire, whether it is a `#[serde(flatten)]` source, and the shape that drives its
/// decode/encode statements.
struct SwiftField {
    /// Where the JSON of the default serde reads for an absent key is written.
    default_at: Option<String>,
    field_def: FieldDef,
    flatten: bool,
    /// What the types written for the field are named from.
    hint: String,
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

/// What `#[model_schema(...)]` says of a type that the Swift emitter reads.
pub struct SwiftTypeArgs<'args> {
    /// The checks the type names, in the order written.
    pub check: &'args [FunctionName],
    /// The default type declared per type parameter.
    pub default_types: &'args [(Ident, syn::Type)],
    pub max_length: Option<usize>,
    /// A member's own `model_schema_prop`, held to the string check of the enum around it.
    pub member_meta: &'args dyn Fn(&Field) -> ModelSchemaPropMeta,
    pub min_length: Option<usize>,
    pub name_override: Option<&'args str>,
    pub pattern: Option<&'args str>,
    /// The functions the type names on itself, in the order written: each takes the payload.
    pub preprocess: &'args [FunctionName],
}

/// What every tagging of an enum is written from.
struct EnumShape<'item> {
    args: &'item SwiftTypeArgs<'item>,
    /// The defaults of the members of the enum's variants.
    defaults: &'item SerdeDefaults,
    export_name: &'item str,
    generic_params: &'item str,
    item_enum: &'item ItemEnum,
    rust_ident: &'item str,
    tag_attrs: &'item EnumTagAttrs,
    type_parameters: &'item [String],
}

/// What a struct of one `value` wraps.
enum Wrapped {
    /// The value of a brand, with the bounds the brand carries and the member it answers its text
    /// with.
    Brand {
        bounds: Vec<String>,
        text_member: Option<String>,
    },
    /// The one slot of a tuple struct that declares one, which serde writes as the struct.
    Slot,
    /// The tuple of a tuple struct of several slots.
    Tuple,
}

/// One variant's payload: the type its case holds, what the case adds to `validate()`, and how
/// the payload is read.
struct VariantPayload {
    /// What chooses the variant in an untagged enum: the bounds of its own members, on `payload`.
    bound: Vec<String>,
    /// The one slot of the variant, where it is read and written as the type it is written as.
    converted: Option<Shaped>,
    /// The one slot of the variant, with what its types are named from, where it names a function.
    corrected: Option<(FieldDef, String)>,
    held: Option<String>,
    rules: Vec<String>,
    /// Whether the payload is a type written for the variant that reads the keys the types around
    /// it took.
    takes_keys: bool,
    /// The one slot of the variant, with what its types are named from, where `validate()` runs
    /// the `validate()` of a value it holds.
    walked: Option<(FieldDef, String)>,
    /// Whether the payload is a type written for the variant.
    written: bool,
}

/// Where the payload of one variant is read from, and what its read is named from.
struct VariantRead<'read> {
    /// The key type of the container the payload is read from, where its key is written `.name`.
    keys: &'read str,
    source: &'read read::Source,
    /// What the locals and the type written for the payload are named from.
    stem: &'read str,
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
    /// A map Swift keys natively whose values convert: each value is turned through its own shape.
    Values(Box<Shaped>),
}

/// A value read and written through its shape: the value as the emitter declares it, and the shape
/// that says what turns the type it is written as into its own.
#[derive(Clone)]
struct Shaped {
    def: FieldDef,
    shape: SwiftFieldShape,
}

/// How Swift reads the keys of a map.
enum MapKeys {
    /// As the `String` a `[String: Value]` is keyed by.
    Text,
    /// As the `Int` a `[Int: Value]` is keyed by.
    Whole,
    /// Through a wrapper written for the field.
    Wrapped,
}

/// A field's fully resolved Swift shape: the property type callers see, the type a `container`
/// call actually decodes, and what (if anything) turns one into the other.
#[derive(Clone)]
struct SwiftFieldShape {
    leaf_conversion: LeafConversion,
    real_leaf: String,
    wire_leaf: String,
}

impl VariantPayload {
    /// The statements that read the payload and hand it to `sink`: a type written for the variant
    /// and a value `validate()` walks unjudged, a slot that names a function from what the
    /// function answers, and any other through its shape. A type written for such a slot joins
    /// `twins`.
    fn read(
        &self,
        from: &VariantRead<'_>,
        sink: &dyn Fn(&str) -> String,
        twins: &mut Vec<String>,
    ) -> String {
        let held = self.held.as_deref().unwrap_or_default();
        let source = from.source;
        if self.written {
            return self
                .takes_keys
                .then(|| read::beside(source, held, sink))
                .flatten()
                .unwrap_or_else(|| read::held(source, held, sink));
        }
        if let Some((def, hint)) = &self.corrected {
            return corrected_member(
                &preprocess::Member {
                    def,
                    functions: preprocess::functions(def),
                    hint,
                    keys: from.keys,
                    plain: preprocess::Plain::Declared,
                    real_type: held,
                    stem: from.stem,
                    walked: self.walked.is_some(),
                },
                self.converted.as_ref(),
                source,
                sink,
                twins,
            );
        }
        match (&self.walked, &self.converted) {
            (Some((def, hint)), _) => {
                read::unjudged(def, hint, source, sink, &mut read::Locals::default())
            }
            (None, Some(slot)) => slot.read(held, from.stem, source, sink),
            (None, None) => sink(&format!("try {}", source.plain(held))),
        }
    }

    /// What the enum writes for the payload `payload` names: itself, or the value of the type its
    /// slot is written as.
    fn written(&self, payload: &str) -> String {
        self.converted
            .as_ref()
            .map_or_else(|| payload.to_owned(), |slot| slot.written(payload))
    }
}

impl Shaped {
    /// What `local`, the value read as the type it is written as, is turned into.
    fn converted(&self, local: &str) -> String {
        converted_from(&self.def, &self.shape, local)
    }

    /// Whether the value is read and written as a type other than its own.
    const fn converts(&self) -> bool {
        !matches!(self.shape.leaf_conversion, LeafConversion::None)
    }

    /// The statements that read the value from `source` and hand it to `sink`: as `real_type`, its
    /// own, or as the type it is written as, held in `wire{Stem}` and turned into its own.
    fn read(
        &self,
        real_type: &str,
        stem: &str,
        source: &read::Source,
        sink: &dyn Fn(&str) -> String,
    ) -> String {
        if !self.converts() {
            return sink(&format!("try {}", source.plain(real_type)));
        }
        let local = format!("wire{}", swift_upper_camel(stem));
        format!(
            "let {local} = try {}; {}",
            source.plain(&wire_type(&self.def, &self.shape)),
            sink(&self.converted(&local))
        )
    }

    /// What `held`, the value, is written as: the value of the type it is written as.
    fn written(&self, held: &str) -> String {
        written_from(&self.def, &self.shape, held)
    }
}

thread_local! {
    /// The Swift type name each Rust ident publishes — the one thing a reference to a sibling item
    /// needs. Swift resolves a reference across the whole file regardless of declaration order, so
    /// this carries no forward-reference bookkeeping.
    static SWIFT_NAMES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// The Swift tokens `item` earns, given what `#[model_schema(...)]` says of it. Dispatches on the
/// item's own shape; an item this module has nothing to say about earns nothing.
pub fn swift_schema_dispatch(item: &Item, args: &SwiftTypeArgs<'_>) -> TokenStream {
    if let Item::Struct(item_struct) = item {
        struct_swift_tokens(item_struct, args)
    } else if let Item::Enum(item_enum) = item {
        enum_swift_tokens(item_enum, args)
    } else if let Item::Type(item_type) = item {
        alias_swift_tokens(item_type, args.name_override)
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

/// The module `{ident}_swift` publishes `swift_definition()` from — never a direct inherent
/// `impl {ident}`, for the same reason `dart::dart_module_tokens` gives: a Rust type alias is not
/// a new type, and a module name is never collapsed through one the way an impl target is.
///
/// Where a member is read as a default, the module also declares what is handed the JSON of the
/// defaults, which `swift_definition()` writes in place of each placeholder, between the quotes of
/// a Swift string.
fn swift_module_tokens(
    rust_ident: &str,
    span: proc_macro2::Span,
    swift_source: &str,
    defaults: &SerdeDefaults,
) -> TokenStream {
    let module_ident = swift_module_ident(rust_ident, span);
    let holder = defaults.holder();
    if holder.is_empty() {
        return quote! {
            pub mod #module_ident {
                pub fn swift_definition() -> ::std::string::String {
                    #swift_source.to_owned()
                }
            }
        };
    }
    let written = Ident::new("written", proc_macro2::Span::call_site());
    let replaced = substituted(
        &quote! { SerdeDefaults },
        &written,
        &[('\\', "\\\\"), ('"', "\\\"")],
    );
    let values = defaults.values_impl(&module_ident);
    quote! {
        pub mod #module_ident {
            #holder

            pub fn swift_definition() -> ::std::string::String {
                let mut #written = ::std::string::String::from(#swift_source);
                #replaced
                #written
            }
        }

        #values
    }
}

/// The `{ident}_swift` module ident an item's own `swift_definition()` publishes from.
pub fn swift_module_ident(rust_ident: &str, span: proc_macro2::Span) -> Ident {
    Ident::new(&format!("{}_swift", to_snake_case(rust_ident)), span)
}

/// The `typealias {rust_ident}<T: Codable & Sendable> = {export_name}<T>` a renamed item
/// re-publishes under its own Rust ident, the bound written again: Swift does not carry it over to
/// a generic `typealias`. Empty for an item that already publishes under its own ident.
fn ident_typealias(rust_ident: &str, export_name: &str, generic_params: &str) -> String {
    if rust_ident == export_name {
        String::new()
    } else {
        let held = held_at(export_name, generic_params);
        format!("\n\npublic typealias {rust_ident}{generic_params} = {held};")
    }
}

/// `<T, U>` with every parameter's own conformance stripped — the spelling a reference site
/// writes, as opposed to a declaration, which writes [`swift_generic_params`].
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

/// [`bounded_generic_params`] for the type parameters `generics` declares.
fn swift_generic_params(generics: &syn::Generics) -> String {
    bounded_generic_params(&type_parameters_in_scope(generics))
}

/// `<T: Codable & Sendable>` for `parameters`, or the empty string for none — every parameter
/// bound to `Codable & Sendable` since `Codable`'s own synthesis, and the `Sendable` conformance
/// this module always declares, both need it.
fn bounded_generic_params(parameters: &[String]) -> String {
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

/// `name` at the parameters `generic_params` declares: the spelling of a type position.
fn held_at(name: &str, generic_params: &str) -> String {
    format!("{name}{}", bare_generic_params(generic_params))
}

/// A member's name without the backticks [`swift_member`] may have put around it: what a longer
/// identifier is built from, and what the wire name is compared with.
fn swift_bare(member: &str) -> &str {
    member.trim_matches('`')
}

/// `name` as a Swift property, enum case, method or parameter: between backticks where Swift
/// reserves the word, which is the name itself to Swift and to `Codable`.
pub fn swift_member(name: &str) -> String {
    if SWIFT_RESERVED.contains(&name) {
        format!("`{name}`")
    } else {
        name.to_owned()
    }
}

/// `name` as a property of an emitted struct: [`swift_member`], and a property named `validate`
/// moved off the method every emitted type has. The key on the wire stays the one serde writes.
fn swift_property(name: &str) -> String {
    if name == "validate" {
        "validate_".to_owned()
    } else {
        swift_member(name)
    }
}

/// The property an emitted struct declares for the field with Rust name `rust_name`: what a
/// client reading that field off the message writes.
#[cfg(feature = "serde")]
pub fn swift_field_member(rust_name: &str) -> String {
    swift_property(&RenameRule::CamelCase.apply_to_field(rust_name))
}

/// `parameter`, upper-camel-cased: `idType` -> `IdType` — used to name a per-field auxiliary type
/// after the field it belongs to.
fn swift_upper_camel(parameter: &str) -> String {
    let mut characters = parameter.chars();
    characters.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_uppercase(), characters.as_str())
    })
}

/// One member's `#[model_schema_prop(...)]` metadata, held to the string check of the enum around
/// it, folded into the `FieldDef` `get_field_def` built for it.
fn member_def(args: &SwiftTypeArgs<'_>, name: &str, member: &Field) -> FieldDef {
    let mut field_def = get_field_def(name, &member.ty, "");
    field_def.model_schema_prop_meta = Some((args.member_meta)(member));
    arrayed(field_def)
}

/// `field` with a std sequence read as the array level serde writes for it, through a sequence of
/// sequences: `HashSet<T>` is `[T]`, as `Vec<T>` is.
fn arrayed(field: FieldDef) -> FieldDef {
    if let FieldDefType::SiblingType(name, arguments) = &field.field_type
        && let [element] = arguments.as_slice()
        && is_sequence_wrapper(name)
    {
        return arrayed(field.collection_element_field(element));
    }
    field
}

/// Every type parameter `defs` hold at any depth, once each, in the order a walk first reaches
/// them.
fn parameters_reached(defs: &[&FieldDef]) -> Vec<String> {
    let mut reached = Vec::new();
    for def in defs {
        reach_parameters(def, &mut reached);
    }
    reached
}

/// Adds to `reached` each type parameter `def` holds that it does not list yet.
fn reach_parameters(def: &FieldDef, reached: &mut Vec<String>) {
    match &def.field_type {
        FieldDefType::TypeParam(name) => {
            if !reached.contains(name) {
                reached.push(name.clone());
            }
        }
        FieldDefType::SiblingType(_, arguments) => {
            for argument in arguments {
                reach_parameters(argument, reached);
            }
        }
        FieldDefType::Map(key, value) => {
            reach_parameters(key, reached);
            reach_parameters(value, reached);
        }
        FieldDefType::Tuple(elements) => {
            for element in elements {
                reach_parameters(element, reached);
            }
        }
        FieldDefType::Unknown
        | FieldDefType::StringLiteral(_)
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Boolean
        | FieldDefType::Char
        | FieldDefType::String
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64 => {}
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => {}
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => {}
    }
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
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => "String".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            "String".to_owned()
        }
    }
}

/// The expression parsing one map key from the `String` a wrapper's `DynamicCodingKey` always
/// carries it as, `None` where the key is held as that text itself, and the expression writing one
/// back into that `String` — the pair mirrors `dart::dart_map_key_decode`/`dart_map_key_encode`.
fn swift_map_key_codec(key: &FieldDef) -> (Option<String>, String) {
    match &key.field_type {
        FieldDefType::Boolean => (
            Some("(wireKey == \"true\" ? true : wireKey == \"false\" ? false : nil)".to_owned()),
            "(key ? \"true\" : \"false\")".to_owned(),
        ),
        FieldDefType::SiblingType(name, _) => {
            let type_name = lookup_swift_name(name).unwrap_or_else(|| name.clone());
            (
                Some(format!("{type_name}(rawValue: wireKey)")),
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
            (Some(format!("{swift_type}(wireKey)")), "String(key)".to_owned())
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
        | FieldDefType::Usize => (None, "key".to_owned()),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => (None, "key".to_owned()),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            (None, "key".to_owned())
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => (
            Some("Self.isoFormatterForKeys.date(from: wireKey)".to_owned()),
            "Self.isoFormatterForKeys.string(from: key)".to_owned(),
        ),
    }
}

/// The per-field wrapper struct a non-string-keyed map earns: a `Codable` type over its own
/// `[Key: Value]` `storage`, decoding and encoding through a locally nested `DynamicCodingKey` so
/// two different fields' wrappers never collide even when their generated text is concatenated.
/// Each value is read and written through its shape.
fn map_key_wrapper_struct(
    wrapper_name: &str,
    generic_params: &str,
    key_type: &str,
    value_type: &str,
    key: &FieldDef,
    value: &Shaped,
    unjudged: Option<&str>,
) -> String {
    let (decode_key, encode_key) = swift_map_key_codec(key);
    let bound_key = decode_key.map_or_else(
        || "let key = wireKey; ".to_owned(),
        |parsed| {
            format!(
                "guard let key = {parsed} else {{ \
                 throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, debugDescription: \"bad key \\(wireKey)\") }}; "
            )
        },
    );
    let own_read = unjudged.map_or_else(String::new, |entry| {
        format!(
            "{}{} {{ \
             let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
             var result: [{key_type}: {value_type}] = [:]; \
             for codingKey in container.allKeys {{ \
             let wireKey = codingKey.stringValue; \
             {bound_key}{entry} }}; \
             storage = result }}; ",
            read::any_key(entry),
            read::UNJUDGED,
        )
    });
    let entry = value.read(
        value_type,
        "value",
        &read::Source::Keyed {
            container: "container".to_owned(),
            key: "codingKey".to_owned(),
            presence: read::Presence::Known,
        },
        &|held| format!("result[key] = {held}"),
    );
    let written = value.written("value");
    // Static, as the read uses it before `storage` is set, and computed, as a generic type stores
    // no static property and a stored `ISO8601DateFormatter` is not `Sendable`.
    #[cfg(feature = "chrono")]
    let formatter_decl = if matches!(key.field_type, FieldDefType::DateTime) {
        "private static var isoFormatterForKeys: ISO8601DateFormatter { let f = ISO8601DateFormatter(); f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; return f };"
    } else {
        ""
    };
    #[cfg(not(feature = "chrono"))]
    let formatter_decl = "";
    format!(
        "public struct {wrapper_name}{generic_params}: Codable, Sendable {{ \
         var storage: [{key_type}: {value_type}]; \
         {formatter_decl} \
         private struct DynamicCodingKey: CodingKey {{ \
         var stringValue: String; init?(stringValue: String) {{ self.stringValue = stringValue }}; \
         var intValue: Int? {{ nil }}; init?(intValue: Int) {{ nil }} }}; \
         init(storage: [{key_type}: {value_type}]) {{ self.storage = storage }}; \
         {own_read}\
         public init(from decoder: Decoder) throws {{ \
         let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
         var result: [{key_type}: {value_type}] = [:]; \
         for codingKey in container.allKeys {{ \
         let wireKey = codingKey.stringValue; \
         {bound_key}{entry} }}; \
         storage = result }}; \
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: DynamicCodingKey.self); \
         for (key, value) in storage {{ \
         let wireKey = {encode_key}; \
         guard let codingKey = DynamicCodingKey(stringValue: wireKey) else {{ continue }}; \
         try container.encode({written}, forKey: codingKey) }} }} }};"
    )
}

/// The Swift tuple-slot struct a `Tuple` field or a `TupleMultiple` variant payload earns: no
/// bare Swift tuple conforms to `Codable`, so each occurrence gets its own unkeyed-container
/// struct, named after the field it belongs to so two occurrences never collide. `elements` is
/// each slot's type, and what the slot is written as.
fn tuple_struct(
    struct_name: &str,
    generic_params: &str,
    elements: &[(String, String)],
    reads: &str,
    twins: Vec<String>,
    validate: &str,
) -> String {
    let slot_names: Vec<String> = (0..elements.len())
        .map(|index| format!("slot{index}"))
        .collect();
    let decls = slot_names
        .iter()
        .zip(elements)
        .fold(String::new(), |mut acc, (slot, (ty, _))| {
            write!(acc, "let {slot}: {ty}; ").unwrap();
            acc
        });
    let ctor_params = slot_names
        .iter()
        .zip(elements)
        .map(|(slot, (ty, _))| format!("{slot}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");
    let ctor_assigns = slot_names.iter().fold(String::new(), |mut acc, slot| {
        write!(acc, "self.{slot} = {slot}; ").unwrap();
        acc
    });
    let encode_calls = elements
        .iter()
        .fold(String::new(), |mut acc, (_, written)| {
            write!(acc, "try container.encode({written}); ").unwrap();
            acc
        });
    let both = read::both(
        &format!("var container = try decoder.unkeyedContainer(); {reads}"),
        &preprocess::Written {
            named: struct_name,
            twins,
            ..preprocess::Written::default()
        },
    );
    format!(
        "public struct {struct_name}{generic_params}: Codable, Sendable {{ \
         {decls} \
         public init({ctor_params}) {{ {ctor_assigns} }}; \
         {both}\
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.unkeyedContainer(); \
         {encode_calls} }}; {validate} }};"
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
        FieldDefType::SiblingType(name, generics) => sibling_shape(name, generics, aux),
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
        #[cfg(feature = "mongodb")]
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
/// struct's own `Codable` already speaks the array serde writes, each slot read and written
/// through its shape. The struct declares the parameters its slots reach.
fn tuple_shape(elements: &[FieldDef], name_hint: &str, aux: &mut Vec<String>) -> SwiftFieldShape {
    let struct_name = format!("{name_hint}Tuple");
    let mut locals = read::Locals::default();
    let mut reads = String::new();
    let mut twins = Vec::new();
    let mut slots = Vec::new();
    let rendered: Vec<(String, String)> = elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let hint = format!("{struct_name}Slot{index}");
            let (element_type, slot) = slot_resolve(element, &hint, aux);
            let stem = format!("slot{index}");
            let source = read::Source::Listed("container".to_owned());
            let held = |read: &str| format!("{stem} = {read}");
            let walked = validate::walks(element, false);
            let functions = preprocess::functions(element);
            let read = if !functions.is_empty() {
                corrected_member(
                    &preprocess::Member {
                        def: element,
                        functions,
                        hint: &hint,
                        keys: "",
                        plain: preprocess::Plain::Declared,
                        real_type: &element_type,
                        stem: &stem,
                        walked,
                    },
                    Some(&slot),
                    &source,
                    &held,
                    &mut twins,
                )
            } else if walked {
                read::unjudged(element, &hint, &source, &held, &mut locals)
            } else {
                slot.read(&element_type, &stem, &source, &held)
            };
            write!(reads, "{read}; ").unwrap();
            let written = slot.written(&stem);
            slots.push(slot.shape);
            (element_type, written)
        })
        .collect();
    let rules = validate::slot_rules(&struct_name, elements);
    let generic_params =
        bounded_generic_params(&parameters_reached(&elements.iter().collect::<Vec<_>>()));
    aux.push(tuple_struct(
        &struct_name,
        &generic_params,
        &rendered,
        &reads,
        twins,
        &validate::method(&rules),
    ));
    let shapes: Vec<&SwiftFieldShape> = slots.iter().collect();
    let helpers = datetime_helpers_for(&struct_name, &generic_params, &shapes);
    if !helpers.is_empty() {
        aux.push(helpers);
    }
    direct_shape(held_at(&struct_name, &generic_params))
}

/// A slot, a value serde writes with no key, as it is declared and read: its Swift type, and its
/// shape. An `as_number` date is read and written as the `Int64` it is declared as, which is the
/// number serde writes.
fn slot_resolve(written: &FieldDef, name_hint: &str, aux: &mut Vec<String>) -> (String, Shaped) {
    let def = arrayed(written.clone());
    let (real_type, resolved) = swift_resolve(&def, name_hint, aux);
    #[cfg(feature = "chrono")]
    if matches!(resolved.leaf_conversion, LeafConversion::DateTimeAsNumber) {
        let shape = direct_shape(resolved.real_leaf);
        return (real_type, Shaped { def, shape });
    }
    (
        real_type,
        Shaped {
            def,
            shape: resolved,
        },
    )
}

/// A `SiblingType` leaf: the referenced item's own published name, with generic arguments rendered
/// the same way a field's own type is. An argument whose read calls the date helpers is held in a
/// wrapper that carries them, as the type it fills reads it through `Decodable`.
fn sibling_shape(name: &str, generics: &[FieldDef], aux: &mut Vec<String>) -> SwiftFieldShape {
    let class_name = lookup_swift_name(name).unwrap_or_else(|| name.to_owned());
    if generics.is_empty() {
        return direct_shape(class_name);
    }
    let arguments = generics
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            let hint = format!("{class_name}Argument{index}");
            let def = arrayed(argument.clone());
            let (real_type, shape) = swift_resolve(&def, &hint, aux);
            if reads_a_date(&shape) {
                dated_argument(&hint, &real_type, &Shaped { def, shape }, aux)
            } else {
                real_type
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    direct_shape(format!("{class_name}<{arguments}>"))
}

/// The wrapper `{hint}Date` a type argument of the Swift type `real_type` is held in, pushed into
/// `aux` with the date helpers its read and its write call: a single value, read and written as
/// the type `argument` is written as.
fn dated_argument(hint: &str, real_type: &str, argument: &Shaped, aux: &mut Vec<String>) -> String {
    let name = format!("{hint}Date");
    let reads = read::both(
        &format!(
            "let container = try decoder.singleValueContainer(); \
             let wireValue = try container.decode({}.self); self.value = {}",
            wire_type(&argument.def, &argument.shape),
            argument.converted("wireValue")
        ),
        &preprocess::Written {
            named: &name,
            ..preprocess::Written::default()
        },
    );
    aux.push(format!(
        "public struct {name}: Codable, Sendable {{ \
         public let value: {real_type}; \
         public init(value: {real_type}) {{ self.value = value }}; \
         {reads}\
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.singleValueContainer(); try container.encode({}) }}; {} }};",
        argument.written("value"),
        validate::method(&[])
    ));
    aux.push(datetime_helpers_for(&name, "", &[&argument.shape]));
    name
}

/// How Swift reads a map under `key`: a string-shaped key and an `isize` are the key of a native
/// dictionary, and any other is read through a wrapper.
fn map_keys(key: &FieldDef) -> MapKeys {
    if !matches!(key.map_key_wire(), MapKeyWire::Named) {
        return MapKeys::Wrapped;
    }
    match key.field_type {
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => MapKeys::Text,
        FieldDefType::Isize => MapKeys::Whole,
        FieldDefType::SiblingType(_, _)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Map(_, _)
        | FieldDefType::Tuple(_)
        | FieldDefType::Boolean
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::Usize => MapKeys::Wrapped,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => MapKeys::Wrapped,
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => MapKeys::Wrapped,
    }
}

/// The wrapper a map under `key` and over `value` is read through, written for `name_hint`: its
/// name, and the parameters it declares, those its key and value reach.
fn keyed_map_wrapper(name_hint: &str, key: &FieldDef, value: &FieldDef) -> (String, String) {
    (
        format!("{name_hint}KeyedMap"),
        bounded_generic_params(&parameters_reached(&[key, value])),
    )
}

/// A `Map` leaf: a string-shaped key stays a plain `[String: Value]` (or `[Int: Value]`, both
/// natively `Codable`), read as the map of what its values are written as where they convert; any
/// other key earns a wrapper struct.
fn map_shape(
    key: &FieldDef,
    value: &FieldDef,
    name_hint: &str,
    aux: &mut Vec<String>,
) -> SwiftFieldShape {
    let value_hint = format!("{name_hint}Value");
    let def = arrayed(value.clone());
    let (value_type, shape) = swift_resolve(&def, &value_hint, aux);
    let entry = Shaped { def, shape };
    let native = match map_keys(key) {
        MapKeys::Text => Some("String"),
        MapKeys::Whole => Some("Int"),
        MapKeys::Wrapped => None,
    };
    if let Some(keyed_by) = native {
        if !entry.converts() {
            return direct_shape(format!("[{keyed_by}: {value_type}]"));
        }
        let wire = wire_type(&entry.def, &entry.shape);
        return SwiftFieldShape {
            leaf_conversion: LeafConversion::Values(Box::new(entry)),
            real_leaf: format!("[{keyed_by}: {value_type}]"),
            wire_leaf: format!("[{keyed_by}: {wire}]"),
        };
    }
    let key_type = swift_map_key_scalar(key);
    let (wrapper_name, generic_params) = keyed_map_wrapper(name_hint, key, value);
    let unjudged = validate::reaches(value).then(|| {
        let source = read::Source::Keyed {
            container: "container".to_owned(),
            key: "codingKey".to_owned(),
            presence: read::Presence::Known,
        };
        read::unjudged(
            value,
            &value_hint,
            &source,
            &|held| format!("result[key] = {held}"),
            &mut read::Locals::default(),
        )
    });
    aux.push(map_key_wrapper_struct(
        &wrapper_name,
        &generic_params,
        &key_type,
        &value_type,
        key,
        &entry,
        unjudged.as_deref(),
    ));
    let helpers = datetime_helpers_for(&wrapper_name, &generic_params, &[&entry.shape]);
    if !helpers.is_empty() {
        aux.push(helpers);
    }
    let wrapper = held_at(&wrapper_name, &generic_params);
    SwiftFieldShape {
        leaf_conversion: LeafConversion::MapWrapper(wrapper.clone()),
        real_leaf: format!("[{key_type}: {value_type}]"),
        wire_leaf: wrapper,
    }
}

/// `field`'s shape together with its full public Swift type: [`swift_field_shape`]'s real leaf,
/// wrapped in one `[...]` per array level, then the field's own outer `?`. The one seam that
/// pushes a field's auxiliary declarations, so every caller needing both calls this once. `written`
/// is [`arrayed`] first, for what no member is collected into: a type argument, a map's value, a
/// tuple's position and a service's message.
fn swift_resolve(
    written: &FieldDef,
    name_hint: &str,
    aux: &mut Vec<String>,
) -> (String, SwiftFieldShape) {
    let field = &arrayed(written.clone());
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
        LeafConversion::Values(value) => {
            let turned = converted_from(&value.def, &value.shape, "$0");
            // Swift reads a closure with a `try` in it as one that throws.
            let tried = if turned.contains("try ") { "try " } else { "" };
            format!("{tried}{expr}.mapValues {{ {turned} }}")
        }
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
        LeafConversion::Values(value) => format!(
            "{expr}.mapValues {{ {} }}",
            written_from(&value.def, &value.shape, "$0")
        ),
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

/// What `local`, a value of `def` read as [`wire_type`], is turned into: a value of its own type.
fn converted_from(def: &FieldDef, shape: &SwiftFieldShape, local: &str) -> String {
    if matches!(shape.leaf_conversion, LeafConversion::None) {
        return local.to_owned();
    }
    if def.is_optional() {
        let inner = decode_conversion(def, shape, def.array_depth, "$0");
        format!("try {local}.map {{ {inner} }}")
    } else {
        decode_conversion(def, shape, def.array_depth, local)
    }
}

/// What `held`, a value of `def`, is written as: a value of [`wire_type`].
fn written_from(def: &FieldDef, shape: &SwiftFieldShape, held: &str) -> String {
    if matches!(shape.leaf_conversion, LeafConversion::None) {
        return held.to_owned();
    }
    if def.is_optional() {
        let inner = encode_conversion(def, shape, def.array_depth, "$0");
        format!("{held}.map {{ {inner} }}")
    } else {
        encode_conversion(def, shape, def.array_depth, held)
    }
}

/// The type a value of `def` whose shape is `shape` is read and written as, its outer `?` included.
fn wire_type(def: &FieldDef, shape: &SwiftFieldShape) -> String {
    let wire = swift_wire_array_type(def, shape);
    if def.is_optional() {
        format!("{wire}?")
    } else {
        wire
    }
}

/// The statements that read a field in the type's own read, ending on `self.{name} = ...`. A
/// field `validate()` walks is read unjudged, and one that names a function from what it
/// answers. `synthesized` says Swift wrote the read before the emitter did, where an optional
/// member took an absent key as `nil` whether its key is required or not.
fn decode_statement(
    field: &SwiftField,
    container: &str,
    synthesized: bool,
    taken: &str,
    locals: &mut read::Locals,
    twins: &mut Vec<String>,
) -> String {
    let absent_is_nil = field.field_def.is_optional() && (synthesized || !field.required);
    let source = if field.flatten {
        read::Source::Beside(taken.to_owned())
    } else {
        read::Source::Keyed {
            container: container.to_owned(),
            key: format!(".{}", field.swift_name),
            presence: if absent_is_nil {
                read::Presence::Nil
            } else {
                read::Presence::Refused
            },
        }
    };
    let present = present_statement(field, &source, absent_is_nil, container, locals, twins);
    match &field.default_at {
        Some(at) => format!(
            "if {container}.contains(.{name}) {{ {present} }} else {{ {} }}",
            default_statement(field, at, container, twins),
            name = field.swift_name,
        ),
        None => present,
    }
}

/// The statements that read `field` from `source`, where the payload holds its key or it has no
/// default.
fn present_statement(
    field: &SwiftField,
    source: &read::Source,
    absent_is_nil: bool,
    container: &str,
    locals: &mut read::Locals,
    twins: &mut Vec<String>,
) -> String {
    let member = |held: &str| format!("self.{} = {held}", field.swift_name);
    let walked = validate::walks(&field.field_def, false);
    let functions = preprocess::functions(&field.field_def);
    if !functions.is_empty() {
        return corrected_field(field, functions, walked, source, twins);
    }
    if walked || (field.flatten && preprocess::beside(&field.field_def)) {
        return read::unjudged(&field.field_def, &field.hint, source, &member, locals);
    }
    if field.flatten {
        return format!(
            "self.{swift} = try {real_type}(from: decoder)",
            swift = field.swift_name,
            real_type = field.real_type,
        );
    }
    keyed_read(field, container, absent_is_nil)
}

/// The statements that read `field` from the JSON of its default, written at `at`: as it is read
/// from a payload under its key, with no function called for it.
fn default_statement(
    field: &SwiftField,
    at: &str,
    container: &str,
    twins: &mut Vec<String>,
) -> String {
    let wire = swift_wire_array_type(&field.field_def, &field.shape);
    let convert = |local: &str| converted(field, local);
    let plain = if matches!(field.shape.leaf_conversion, LeafConversion::None) {
        preprocess::Plain::Declared
    } else {
        preprocess::Plain::Wire {
            convert: &convert,
            wire: &wire,
        }
    };
    let stem = format!("{}Default", swift_bare(&field.swift_name));
    preprocess::defaulted(
        &preprocess::Member {
            def: &field.field_def,
            functions: &[],
            hint: &field.hint,
            keys: "CodingKeys",
            plain,
            real_type: &field.real_type,
            stem: &stem,
            walked: validate::walks(&field.field_def, false),
        },
        at,
        &format!("{container}.codingPath + [CodingKeys.{}]", field.swift_name),
        &|held| format!("self.{} = {held}", field.swift_name),
        twins,
    )
}

/// The statements that read from `source` a field that names `functions`, from what they answer:
/// as its own type, or as the type it is written as, turned into its own.
fn corrected_field(
    field: &SwiftField,
    functions: &[FunctionName],
    walked: bool,
    source: &read::Source,
    twins: &mut Vec<String>,
) -> String {
    let wire = wire_type(&field.field_def, &field.shape);
    let convert = |local: &str| converted(field, local);
    let plain = if matches!(field.shape.leaf_conversion, LeafConversion::None) {
        preprocess::Plain::Declared
    } else {
        preprocess::Plain::Wire {
            convert: &convert,
            wire: &wire,
        }
    };
    preprocess::member(
        &preprocess::Member {
            def: &field.field_def,
            functions,
            hint: &field.hint,
            keys: "CodingKeys",
            plain,
            real_type: &field.real_type,
            stem: swift_bare(&field.swift_name),
            walked,
        },
        source,
        &|held| format!("self.{} = {held}", field.swift_name),
        twins,
    )
}

/// [`preprocess::member`] for `member`, a slot: read as it is declared, or as the type it is
/// written as where `shaped` turns that into its own.
fn corrected_member(
    member: &preprocess::Member<'_>,
    shaped: Option<&Shaped>,
    source: &read::Source,
    sink: &dyn Fn(&str) -> String,
    twins: &mut Vec<String>,
) -> String {
    let Some(slot) = shaped.filter(|slot| slot.converts()) else {
        return preprocess::member(member, source, sink, twins);
    };
    let wire = wire_type(&slot.def, &slot.shape);
    let convert = |local: &str| slot.converted(local);
    preprocess::member(
        &preprocess::Member {
            plain: preprocess::Plain::Wire {
                convert: &convert,
                wire: &wire,
            },
            ..*member
        },
        source,
        sink,
        twins,
    )
}

/// What `local` holds, a field read as the type it is written as, turned into the field's own.
fn converted(field: &SwiftField, local: &str) -> String {
    converted_from(&field.field_def, &field.shape, local)
}

/// The statements that read under its key a field `validate()` walks nothing of: as its own type,
/// or as the type it is written as, turned into its own.
fn keyed_read(field: &SwiftField, container: &str, absent_is_nil: bool) -> String {
    if matches!(field.shape.leaf_conversion, LeafConversion::None) {
        return if absent_is_nil {
            let inner = field
                .real_type
                .strip_suffix('?')
                .unwrap_or(&field.real_type);
            format!(
                "self.{swift} = try {container}.decodeIfPresent({inner}.self, forKey: .{swift})",
                swift = field.swift_name
            )
        } else {
            format!(
                "self.{swift} = try {container}.decode({real_type}.self, forKey: .{swift})",
                swift = field.swift_name,
                real_type = field.real_type,
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
    let local = format!("wire{}", swift_upper_camel(swift_bare(&field.swift_name)));
    format!(
        "let {local} = {read}; self.{} = {}",
        field.swift_name,
        converted(field, &local)
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
        let convert = written_from(&field.field_def, &field.shape, &field.swift_name);
        let local = format!("wire{}", swift_upper_camel(swift_bare(&field.swift_name)));
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

/// The per-struct static helpers a `DateTime` field's decode/encode calls into — a formatter with
/// fractional seconds, so the emitted round trip is byte-for-byte the ISO 8601 text chrono wrote.
/// Swift refuses a static stored property in a generic type, so a type that declares a parameter
/// builds its formatter on each call, and any other shares one.
#[cfg(feature = "chrono")]
fn datetime_helpers(export_name: &str, declares_a_parameter: bool) -> String {
    let formatter = if declares_a_parameter {
        "private static var swiftSchemaIso8601Formatter: ISO8601DateFormatter { \
         let f = ISO8601DateFormatter(); f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; return f };"
    } else {
        "private static let swiftSchemaIso8601Formatter: ISO8601DateFormatter = { \
         let f = ISO8601DateFormatter(); f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; return f }();"
    };
    format!(
        "extension {export_name} {{ {formatter} \
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

/// Whether reading a value of `shape` calls [`datetime_helpers`]: a `DateTime<Tz>` leaf under any
/// list or optional, or one held as the value of a map Swift keys natively.
#[cfg(feature = "chrono")]
fn reads_a_date(shape: &SwiftFieldShape) -> bool {
    match &shape.leaf_conversion {
        LeafConversion::DateTimePlain | LeafConversion::DateTimeAsNumber => true,
        LeafConversion::Values(value) => reads_a_date(&value.shape),
        LeafConversion::MapWrapper(_) | LeafConversion::None => false,
    }
}

#[cfg(not(feature = "chrono"))]
const fn reads_a_date(_shape: &SwiftFieldShape) -> bool {
    false
}

/// [`datetime_helpers`] for `export_name`, where the read of one of `shapes` calls them.
#[cfg(feature = "chrono")]
fn datetime_helpers_for(
    export_name: &str,
    generic_params: &str,
    shapes: &[&SwiftFieldShape],
) -> String {
    if shapes.iter().any(|shape| reads_a_date(shape)) {
        datetime_helpers(export_name, !generic_params.is_empty())
    } else {
        String::new()
    }
}

#[cfg(not(feature = "chrono"))]
const fn datetime_helpers_for(
    _export_name: &str,
    _generic_params: &str,
    _shapes: &[&SwiftFieldShape],
) -> String {
    String::new()
}

/// Walks a named-field struct's or a struct-shaped enum variant's fields into [`SwiftField`]s,
/// dropping any field a serde attribute takes off the wire in both directions — mirrors
/// `dart::collect_dart_fields`.
fn collect_swift_fields(
    fields: &Fields,
    rule: RenameRule,
    type_parameters: &[String],
    name_hint: &str,
    args: &SwiftTypeArgs<'_>,
    aux: &mut Vec<String>,
    default_at: &dyn Fn(&str) -> Option<String>,
) -> Vec<SwiftField> {
    let Fields::Named(named) = fields else {
        return Vec::new();
    };
    let mut collected = Vec::new();
    for field in &named.named {
        let Some(ident) = field.ident.as_ref() else {
            continue;
        };
        let rust_name = ident.unraw().to_string();
        let omission = parse_serde_key_omission(&field.attrs);
        if omission.absent_from_wire() {
            continue;
        }
        let wire_name = wire_field_name(&rust_name, rename_override(&field.attrs).as_deref(), rule);
        let mut field_def = member_def(args, &rust_name, field);
        field_def.erase_type_parameters(type_parameters);
        if omission.omits_key && !field_def.is_optional() {
            field_def.nullable_levels.push(field_def.array_depth);
        }
        let swift_name = RenameRule::CamelCase.apply_to_field(&rust_name);
        let field_hint = format!("{name_hint}{}", swift_upper_camel(&swift_name));
        let (real_type, shape) = swift_resolve(&field_def, &field_hint, aux);
        collected.push(SwiftField {
            default_at: default_at(&rust_name),
            field_def,
            flatten: field_is_flatten(&field.attrs),
            hint: field_hint,
            real_type,
            required: !omission.omits_key,
            shape,
            swift_name: swift_property(&swift_name),
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
        if swift_bare(&field.swift_name) == field.wire_name {
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

/// The body of a set of [`SwiftField`]s: properties, `CodingKeys`, the memberwise initializer,
/// the reads, `encode(to:)` only where synthesis cannot reach, then `validate`, the method every
/// emitted type has. `extra_coding_keys` lists cases with no matching field (an
/// internally-tagged variant's tag), and `corrects` the functions the struct `owner` names.
fn struct_body_content(
    fields: &[SwiftField],
    extra_coding_keys: &[(String, String)],
    own_tag: Option<&(String, String)>,
    validate: &str,
    owner: &str,
    corrects: &[FunctionName],
) -> String {
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
    // A struct's own tag has no property to synthesize a codec from, so the tag is encoded under
    // a key set of its own, and decoding reads past it.
    let unsynthesized = own_tag.is_some()
        || fields.iter().any(|field| {
            field.flatten || !matches!(field.shape.leaf_conversion, LeafConversion::None)
        });
    let needs_encode = unsynthesized || fields.iter().any(field_is_nullable_flag);

    let memberwise_init = memberwise_initializer(fields);
    let mut locals = read::Locals::default();
    let mut twins = Vec::new();
    let taken = preprocess::taken_with(
        fields
            .iter()
            .filter(|field| !field.flatten)
            .map(|field| field.wire_name.as_str()),
    );
    let members = fields
        .iter()
        .map(|field| {
            decode_statement(
                field,
                "container",
                !unsynthesized,
                &taken,
                &mut locals,
                &mut twins,
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let keyed = if fields.is_empty() {
        ""
    } else {
        "let container = try decoder.container(keyedBy: CodingKeys.self); "
    };
    let reads = read::both(
        &format!("{keyed}{members}"),
        &preprocess::Written {
            beside: "",
            named: owner,
            own: corrects,
            twins,
        },
    );
    let encode_method = if needs_encode {
        let statements = fields
            .iter()
            .map(|field| encode_statement(field, "container"))
            .collect::<Vec<_>>()
            .join("; ");
        let container = if fields.is_empty() {
            ""
        } else {
            "var container = encoder.container(keyedBy: CodingKeys.self); "
        };
        let tag = own_tag.map_or_else(String::new, |(_, named)| {
            format!(
                "var tagged = encoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); \
                 try tagged.encode(\"{named}\", forKey: .swiftSchemaTag); "
            )
        });
        format!(
            "public func encode(to encoder: Encoder) throws {{ {tag}{container}{statements} }}; "
        )
    } else {
        String::new()
    };
    let tag_keys = own_tag.map_or_else(String::new, |(key, _)| {
        format!(
            "private enum SwiftSchemaTagCodingKeys: String, CodingKey {{ \
             case swiftSchemaTag = \"{key}\" }}; "
        )
    });
    format!("{props}{coding_keys} {tag_keys}{memberwise_init}{reads}{encode_method}{validate}")
}

/// The explicit memberwise `public init(...)` a written `init(from:)` costs a struct — Swift
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

/// What the fields of a struct add to its `validate()`, in the order declared: a field under the
/// key serde writes it under, and a flattened one under none.
fn field_rules(owner: &str, fields: &[SwiftField]) -> Vec<String> {
    fields
        .iter()
        .flat_map(|field| {
            validate::member_rules(
                owner,
                &validate::Checked {
                    access: validate::property_read(&field.swift_name),
                    def: &field.field_def,
                    runs_both: false,
                    step: (!field.flatten).then(|| field.wire_name.clone()),
                },
            )
        })
        .collect()
}

/// The `public struct {export_name}{generics}: Codable, Sendable { ... }` a named-field struct or
/// a struct-shaped variant payload earns, with the `DateTime` helpers appended when a field needs
/// them. `rules` is what its `validate()` runs, and `corrects` the functions it names on itself.
fn struct_declaration(
    export_name: &str,
    generic_params: &str,
    fields: &[SwiftField],
    own_tag: Option<&(String, String)>,
    rules: &[String],
    corrects: &[FunctionName],
) -> String {
    let content = struct_body_content(
        fields,
        &[],
        own_tag,
        &validate::method(rules),
        export_name,
        corrects,
    );
    let shapes: Vec<&SwiftFieldShape> = fields.iter().map(|field| &field.shape).collect();
    let helpers = datetime_helpers_for(export_name, generic_params, &shapes);
    format!(
        "public struct {export_name}{generic_params}: Codable, Sendable {{ {content} }}; {helpers}"
    )
}

/// The Swift tokens a named-field struct earns: a struct, plus a `typealias` under its own Rust
/// ident when `name = "..."` moved its published name elsewhere.
fn struct_swift_tokens(item_struct: &ItemStruct, args: &SwiftTypeArgs<'_>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let rust_ident = item_struct.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    if let Some(held) = brand_field(item_struct) {
        let value_field = brand_value_field(held, &type_parameters, args);
        let bounds = validate::brand_rules(&export_name, &value_field, args);
        let mut rules = bounds.clone();
        rules.extend(validate::own_checks(args.check));
        return value_wrapper_tokens(
            &item_struct.ident,
            &item_struct.generics,
            args,
            &value_field,
            &Wrapped::Brand {
                bounds,
                text_member: validate::brand_text_member(&value_field),
            },
            &rules,
        );
    }
    if matches!(item_struct.fields, Fields::Unnamed(_)) {
        return tuple_struct_swift_tokens(item_struct, args);
    }

    register_swift_name(&rust_ident, &export_name);
    validate::note(&rust_ident, None, true);

    let rule = container_rename_rule(&item_struct.attrs);
    let generic_params = swift_generic_params(&item_struct.generics);
    let mut aux = Vec::new();
    let defaults = SerdeDefaults::of(item_struct);
    let fields = collect_swift_fields(
        &item_struct.fields,
        rule,
        &type_parameters,
        &export_name,
        args,
        &mut aux,
        &|field| defaults.at(None, field),
    );
    let own_tag = struct_tag(item_struct);
    let mut rules = field_rules(&export_name, &fields);
    rules.extend(validate::own_checks(args.check));
    validate::note(&rust_ident, None, !rules.is_empty());
    let body = struct_declaration(
        &export_name,
        &generic_params,
        &fields,
        own_tag.as_ref(),
        &rules,
        args.preprocess,
    );
    preprocess::note(&rust_ident, read::takes_keys(&body));
    let typealias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let swift_source = if aux.is_empty() {
        format!("{body}{typealias}")
    } else {
        format!("{} {body}{typealias}", aux.join(" "))
    };

    swift_module_tokens(
        &rust_ident,
        item_struct.ident.span(),
        &swift_source,
        &defaults,
    )
}

/// The `FieldDef` of the field a brand is the value of, its type parameters already erased.
fn brand_value_field(
    held: &Field,
    type_parameters: &[String],
    args: &SwiftTypeArgs<'_>,
) -> FieldDef {
    let mut field_def = member_def(args, "value", held);
    field_def.erase_type_parameters(type_parameters);
    field_def
}

/// The Swift tokens for a value that carries no shape of its own on the wire beyond one wrapped
/// value: a branded newtype, or a non-branded tuple struct. Both publish a struct with one
/// `public let value` and a single-value-container codec. `rules` is what the wrapper's
/// `validate()` runs.
fn value_wrapper_tokens(
    ident: &Ident,
    generics: &syn::Generics,
    args: &SwiftTypeArgs<'_>,
    value_field: &FieldDef,
    wrapped: &Wrapped,
    rules: &[String],
) -> TokenStream {
    let rust_ident = ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_swift_name(&rust_ident, &export_name);
    let mut validate = validate::method(rules);
    match wrapped {
        Wrapped::Brand {
            bounds,
            text_member,
        } => {
            validate::note(
                &rust_ident,
                text_member.as_ref().map(|_| validate::Text::Member),
                !rules.is_empty(),
            );
            if let Some(member) = text_member {
                write!(validate, "; {member}").unwrap();
            }
            if !bounds.is_empty() {
                validate::note_bounded(&rust_ident);
                write!(validate, "; {}", validate::bound_member(bounds)).unwrap();
            }
        }
        Wrapped::Slot | Wrapped::Tuple => validate::note(&rust_ident, None, !rules.is_empty()),
    }

    let generic_params = swift_generic_params(generics);
    let mut aux = Vec::new();
    let (real_type, shaped) = if matches!(wrapped, Wrapped::Slot) {
        slot_resolve(value_field, &export_name, &mut aux)
    } else {
        let (real_type, shape) = swift_resolve(value_field, &export_name, &mut aux);
        (
            real_type,
            Shaped {
                def: value_field.clone(),
                shape,
            },
        )
    };

    let decode = if shaped.converts() {
        format!(
            "try {{ let wireValue = try container.decode({}.self); return {} }}()",
            wire_type(&shaped.def, &shaped.shape),
            shaped.converted("wireValue")
        )
    } else {
        format!("try container.decode({real_type}.self)")
    };
    let encode = format!("try container.encode({})", shaped.written("value"));
    let helpers = datetime_helpers_for(&export_name, &generic_params, &[&shaped.shape]);
    let mut twins = Vec::new();
    let own_read = if matches!(wrapped, Wrapped::Slot) {
        lone_slot_read(&shaped, &real_type, &export_name, &decode, &mut twins)
    } else {
        wrapper_read(wrapped, value_field, &real_type, &decode)
    };
    let reads = read::both(
        &own_read,
        &preprocess::Written {
            named: &export_name,
            own: args.preprocess,
            twins,
            ..preprocess::Written::default()
        },
    );
    preprocess::note(&rust_ident, read::takes_keys(&reads));
    let body = format!(
        "public struct {export_name}{generic_params}: Codable, Sendable {{ \
         public let value: {real_type}; \
         public init(value: {real_type}) {{ self.value = value }}; \
         {reads}\
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.singleValueContainer(); {encode} }}; {validate} }}; {helpers}"
    );
    let typealias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let swift_source = if aux.is_empty() {
        format!("{body}{typealias}")
    } else {
        format!("{} {body}{typealias}", aux.join(" "))
    };

    swift_module_tokens(
        &rust_ident,
        ident.span(),
        &swift_source,
        &SerdeDefaults::none(),
    )
}

/// The statements of the own read of a tuple struct's one slot, read from the decoder itself as
/// a slot is: unjudged where `validate()` walks what it holds, through its functions where it
/// names some, a type written for it joining `twins`, and as `decode` reads it otherwise.
fn lone_slot_read(
    shaped: &Shaped,
    real_type: &str,
    hint: &str,
    decode: &str,
    twins: &mut Vec<String>,
) -> String {
    let def = &shaped.def;
    let handed_keys = !def.is_optional() && preprocess::beside(def);
    let source = if handed_keys {
        read::Source::Beside(preprocess::taken_with(iter::empty()))
    } else {
        read::Source::Decoder
    };
    let held = |read: &str| format!("self.value = {read}");
    let walked = validate::walks(def, false) || handed_keys;
    let functions = preprocess::functions(def);
    if !functions.is_empty() {
        corrected_member(
            &preprocess::Member {
                def,
                functions,
                hint,
                keys: "",
                plain: preprocess::Plain::Declared,
                real_type,
                stem: "value",
                walked,
            },
            Some(shaped),
            &source,
            &held,
            twins,
        )
    } else if walked {
        read::unjudged(def, hint, &source, &held, &mut read::Locals::default())
    } else {
        format!("let container = try decoder.singleValueContainer(); self.value = {decode}")
    }
}

/// The statements of the own read of a struct of one `value`: a tuple struct reads its tuple
/// alone, and a brand reads its value as `decode` reads it. A value that takes the keys the types
/// around it took is handed them, and judged there as its own read judges it.
fn wrapper_read(
    wrapped: &Wrapped,
    value_field: &FieldDef,
    real_type: &str,
    decode: &str,
) -> String {
    if matches!(wrapped, Wrapped::Tuple) {
        return format!("self.value = try {real_type}(swiftSchemaUnjudged: decoder)");
    }
    if !value_field.is_optional() && preprocess::beside(value_field) {
        let taken = preprocess::taken_with(iter::empty());
        return format!(
            "self.value = try {real_type}(swiftSchemaUnjudged: decoder, swiftSchemaWithout: {taken}); \
             let held = value.validate(); if !held.isEmpty {{ throw DecodingError.dataCorrupted(\
             DecodingError.Context(codingPath: decoder.codingPath, \
             debugDescription: held.joined(separator: \"; \"))) }}"
        );
    }
    format!("let container = try decoder.singleValueContainer(); self.value = {decode}")
}

/// The Swift tokens for a non-branded tuple struct. One that declares a single slot is that slot's
/// value, which serde writes whatever the slot's attributes say; a wider one wraps the tuple of the
/// slots serde reads.
fn tuple_struct_swift_tokens(item_struct: &ItemStruct, args: &SwiftTypeArgs<'_>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let Fields::Unnamed(unnamed) = &item_struct.fields else {
        return TokenStream::new();
    };
    let slot_def = |slot: &Field| {
        let mut field_def = member_def(args, "slot", slot);
        field_def.erase_type_parameters(&type_parameters);
        field_def
    };
    let export_name = compute_item_export_name(&item_struct.ident.to_string(), args.name_override);
    if let Some(only) = unnamed.unnamed.first()
        && unnamed.unnamed.len() == 1
    {
        let value_field = slot_def(only);
        let mut rules = validate::lone_slot_rules(&export_name, &value_field);
        rules.extend(validate::own_checks(args.check));
        return value_wrapper_tokens(
            &item_struct.ident,
            &item_struct.generics,
            args,
            &value_field,
            &Wrapped::Slot,
            &rules,
        );
    }
    let slots: Vec<FieldDef> = unnamed
        .unnamed
        .iter()
        .filter(|slot| !parse_serde_key_omission(&slot.attrs).absent_from_wire())
        .map(slot_def)
        .collect();
    let mut rules = validate::tuple_struct_rules(&export_name, &slots);
    rules.extend(validate::own_checks(args.check));
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
        #[cfg(any(feature = "zod", feature = "jsonschema"))]
        serde_default: None,
        #[cfg(feature = "jsonschema")]
        type_span: proc_macro2::Span::call_site(),
    };
    value_wrapper_tokens(
        &item_struct.ident,
        &item_struct.generics,
        args,
        &tuple_field,
        &Wrapped::Tuple,
        &rules,
    )
}

/// The Swift tokens a `type X = ...;` alias earns: `public typealias {export_name} = {target};`.
fn alias_swift_tokens(item_type: &ItemType, name_override: Option<&str>) -> TokenStream {
    let export_name = compute_alias_export_name(&item_type.ident.to_string(), name_override);
    register_swift_name(&item_type.ident.to_string(), &export_name);
    let type_parameters = type_parameters_in_scope(&item_type.generics);
    let mut target = arrayed(get_field_def(&export_name, &item_type.ty, ""));
    target.erase_type_parameters(&type_parameters);
    validate::note(
        &item_type.ident.to_string(),
        validate::text_of(&target),
        validate::names_a_validated_type(&target),
    );
    if validate::names_a_bounded_type(&target) {
        validate::note_bounded(&item_type.ident.to_string());
    }
    preprocess::note(
        &item_type.ident.to_string(),
        !target.is_optional() && preprocess::beside(&target),
    );
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
    swift_module_tokens(
        &rust_ident,
        item_type.ident.span(),
        &swift_source,
        &SerdeDefaults::none(),
    )
}

/// One variant's own payload, resolved to at most one associated value: none for a `Unit`
/// variant, an existing type for a `TupleSingle` slot, or a freshly generated struct (pushed
/// into `aux`) for a `Named` payload or a folded `TupleMultiple` tuple. `step` is what serde
/// writes on the way to the payload: the variant's name, the content key, or nothing.
fn resolve_variant_payload(
    shape: &EnumShape<'_>,
    variant: &Variant,
    field_rule: RenameRule,
    step: Option<&str>,
    aux: &mut Vec<String>,
) -> VariantPayload {
    match classify_variant(variant) {
        VariantKind::Unit => VariantPayload {
            bound: Vec::new(),
            converted: None,
            corrected: None,
            held: None,
            rules: Vec::new(),
            takes_keys: false,
            walked: None,
            written: false,
        },
        VariantKind::Named => named_payload(shape, variant, field_rule, step, aux),
        VariantKind::TupleSingle => slot_payload(shape, variant, step, aux),
        VariantKind::TupleMultiple => slots_payload(shape, variant, step, aux),
    }
}

/// The payload of a variant with named members: the struct written for them, each member read as
/// the default the enum's defaults hold for it in this variant. The struct declares the enum's
/// parameters its members name, and the case holds it at those parameters.
fn named_payload(
    shape: &EnumShape<'_>,
    variant: &Variant,
    field_rule: RenameRule,
    step: Option<&str>,
    aux: &mut Vec<String>,
) -> VariantPayload {
    let export_name = shape.export_name;
    let struct_name = format!("{export_name}{}Payload", variant.ident);
    let variant_name = variant.ident.to_string();
    let fields = collect_swift_fields(
        &variant.fields,
        field_rule,
        shape.type_parameters,
        &struct_name,
        shape.args,
        aux,
        &|field| shape.defaults.at(Some(&variant_name), field),
    );
    let members = field_rules(&struct_name, &fields);
    let reached = parameters_reached(
        &fields
            .iter()
            .map(|field| &field.field_def)
            .collect::<Vec<_>>(),
    );
    let named_parameters: Vec<String> = shape
        .type_parameters
        .iter()
        .filter(|parameter| reached.contains(parameter))
        .cloned()
        .collect();
    let generic_params = bounded_generic_params(&named_parameters);
    let declared = struct_declaration(&struct_name, &generic_params, &fields, None, &members, &[]);
    let takes_keys = read::takes_keys(&declared);
    aux.push(declared);
    let bound = fields
        .iter()
        .flat_map(|field| {
            validate::bound_alone(
                export_name,
                &validate::Checked {
                    access: format!("payload.{}", field.swift_name),
                    def: &field.field_def,
                    runs_both: false,
                    step: (!field.flatten).then(|| field.wire_name.clone()),
                },
            )
        })
        .collect();
    VariantPayload {
        bound,
        converted: None,
        corrected: None,
        held: Some(held_at(&struct_name, &generic_params)),
        rules: validate::payload_rules(export_name, step, !members.is_empty()),
        takes_keys,
        walked: None,
        written: true,
    }
}

/// The payload of a variant of one slot: the slot's own type.
fn slot_payload(
    shape: &EnumShape<'_>,
    variant: &Variant,
    step: Option<&str>,
    aux: &mut Vec<String>,
) -> VariantPayload {
    let export_name = shape.export_name;
    let mut field_def = variant.fields.iter().next().map_or_else(
        || get_field_def("value", &syn::parse_quote!(()), ""),
        |field| member_def(shape.args, "value", field),
    );
    field_def.erase_type_parameters(shape.type_parameters);
    let hint = format!("{export_name}{}Value", variant.ident);
    let (held, slot) = slot_resolve(&field_def, &hint, aux);
    let runs_both = shape.tag_attrs.untagged;
    let checked = validate::Checked {
        access: "payload".to_owned(),
        def: &field_def,
        runs_both,
        step: step.map(ToOwned::to_owned),
    };
    let rules = validate::member_rules(export_name, &checked);
    let bound = validate::bound_alone(export_name, &checked);
    // Under a tag alone and under none, the slot is read from the decoder of the enum.
    let handed_keys = shape.tag_attrs.content.is_none()
        && (shape.tag_attrs.untagged || shape.tag_attrs.tag.is_some())
        && preprocess::beside(&field_def);
    let walked = (validate::walks(&field_def, runs_both) || handed_keys)
        .then(|| (field_def.clone(), hint.clone()));
    let corrected =
        (!preprocess::functions(&field_def).is_empty()).then(|| (field_def.clone(), hint));
    VariantPayload {
        bound,
        converted: slot.converts().then_some(slot),
        corrected,
        held: Some(held),
        rules,
        takes_keys: false,
        walked,
        written: false,
    }
}

/// The payload of a variant of several slots: the struct written for the tuple.
fn slots_payload(
    shape: &EnumShape<'_>,
    variant: &Variant,
    step: Option<&str>,
    aux: &mut Vec<String>,
) -> VariantPayload {
    let export_name = shape.export_name;
    let slots: Vec<FieldDef> = variant
        .fields
        .iter()
        .map(|field| {
            let mut field_def = member_def(shape.args, "slot", field);
            field_def.erase_type_parameters(shape.type_parameters);
            field_def
        })
        .collect();
    let checked = !validate::slot_rules(export_name, &slots).is_empty();
    let bound = slots
        .iter()
        .enumerate()
        .flat_map(|(position, slot)| {
            validate::bound_alone(
                export_name,
                &validate::Checked {
                    access: format!("payload.slot{position}"),
                    def: slot,
                    runs_both: false,
                    step: Some(position.to_string()),
                },
            )
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
        #[cfg(any(feature = "zod", feature = "jsonschema"))]
        serde_default: None,
        #[cfg(feature = "jsonschema")]
        type_span: proc_macro2::Span::call_site(),
    };
    let hint = format!("{export_name}{}", variant.ident);
    VariantPayload {
        bound,
        converted: None,
        corrected: None,
        held: Some(swift_full_real_type(&tuple_field, &hint, aux)),
        rules: validate::payload_rules(export_name, step, checked),
        takes_keys: false,
        walked: None,
        written: true,
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
fn plain_enum_swift_source(
    item_enum: &ItemEnum,
    rust_ident: &str,
    export_name: &str,
    args: &SwiftTypeArgs<'_>,
) -> String {
    let checks = args.check;
    let rule = container_rename_rule(&item_enum.attrs);
    let cases = item_enum
        .variants
        .iter()
        .fold(String::new(), |mut acc, variant| {
            let rust_name = variant.ident.to_string();
            let case_name = swift_member(&RenameRule::CamelCase.apply_to_variant(&rust_name));
            let wire = rename_override(&variant.attrs)
                .unwrap_or_else(|| rule.apply_to_variant(&rust_name));
            write!(acc, "case {case_name} = \"{wire}\"; ").unwrap();
            acc
        });
    validate::note(rust_ident, None, !checks.is_empty());
    let validate = validate::method(&validate::own_checks(checks));
    // With no check and no function there is nothing to judge or to call, and the read stays the
    // one Swift synthesizes.
    let reads = if checks.is_empty() && args.preprocess.is_empty() {
        format!(
            "public {} {{ try self.init(from: decoder) }}; ",
            read::UNJUDGED
        )
    } else {
        read::both(
            &format!(
                "let container = try decoder.singleValueContainer(); \
                 let raw = try container.decode(String.self); \
                 guard let read = {export_name}(rawValue: raw) else {{ \
                 throw DecodingError.dataCorruptedError(in: container, debugDescription: \
                 \"Cannot initialize {export_name} from invalid String value \\(raw)\") }}; \
                 self = read"
            ),
            &preprocess::Written {
                named: export_name,
                own: args.preprocess,
                ..preprocess::Written::default()
            },
        )
    };
    let body = format!(
        "public enum {export_name}: String, Codable, Sendable {{ {cases}{reads}{validate} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, "");
    format!("{body}{typealias}")
}

/// The Swift tokens the externally tagged shape earns — serde's default once a variant carries
/// data. Swift's own associated-value synthesis wraps a payload under an auto-generated `_0` key
/// rather than serde's bare `{"Foo": payload}`, so this writes the codec by hand instead.
fn external_tagged_enum_swift_source(shape: &EnumShape<'_>, aux: &mut Vec<String>) -> String {
    let &EnumShape {
        args,
        defaults: _defaults,
        export_name,
        generic_params,
        item_enum,
        rust_ident,
        tag_attrs,
        type_parameters: _type_parameters,
    } = shape;
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let mut cases = String::new();
    let mut unit_decode_arms = String::new();
    let mut object_decode_arms = String::new();
    let mut encode_arms = String::new();
    let mut ruled: Vec<(String, Vec<String>)> = Vec::new();
    let mut slots: Vec<SwiftFieldShape> = Vec::new();
    let mut twins = Vec::new();
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = swift_member(&RenameRule::CamelCase.apply_to_variant(&rust_name));
        let wire_tag = rename_override(&variant.attrs)
            .unwrap_or_else(|| variant_rule.apply_to_variant(&rust_name));
        let payload =
            resolve_variant_payload(shape, variant, field_rule, Some(wire_tag.as_str()), aux);
        ruled.push((case_name.clone(), payload.rules.clone()));
        cases.push_str(&variant_case(&case_name, payload.held.as_deref()));
        slots.extend(payload.converted.as_ref().map(|slot| slot.shape.clone()));
        if payload.held.is_some() {
            let source = read::Source::Keyed {
                container: "container".to_owned(),
                key: "key".to_owned(),
                presence: read::Presence::Known,
            };
            let from = VariantRead {
                keys: "",
                source: &source,
                stem: swift_bare(&case_name),
            };
            let read = payload.read(
                &from,
                &|held| format!("self = .{case_name}({held})"),
                &mut twins,
            );
            write!(object_decode_arms, "case \"{wire_tag}\": {read}; ").unwrap();
            write!(
                encode_arms,
                "case .{case_name}(let payload): var container = encoder.container(keyedBy: SwiftSchemaExternalCodingKey.self); \
                 try container.encode({}, forKey: SwiftSchemaExternalCodingKey(stringValue: \"{wire_tag}\")!); ",
                payload.written("payload")
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
    let reads = read::both(
        &format!(
            "if let single = try? decoder.singleValueContainer(), let tag = try? single.decode(String.self) {{ \
             switch tag {{ {unit_decode_arms} default: break }} }}; \
             let container = try decoder.container(keyedBy: SwiftSchemaExternalCodingKey.self); \
             guard let key = container.allKeys.first else {{ \
             throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"empty object for {export_name}\")) }}; \
             switch key.stringValue {{ {object_decode_arms} \
             default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(key.stringValue)\")) }}"
        ),
        &preprocess::Written {
            beside: "",
            named: export_name,
            own: args.preprocess,
            twins,
        },
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ switch self {{ {encode_arms} }} }};"
    );
    let validate = enum_validate(shape, &ruled);
    let body = format!(
        "public enum {export_name}{generic_params}: Codable, Sendable {{ {cases} {key_type} {reads}{encode_method} {validate} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{}{typealias}", enum_date_helpers(shape, &slots))
}

/// The Swift tokens the internally tagged shape earns (`tag = "..."`, no `content`): the tag
/// merges into the same object a `Named` payload's own fields sit in, so a variant's own payload
/// struct decodes/encodes through the *same* `decoder`/`encoder` the enum's own tag read from.
fn internal_tagged_enum_swift_source(shape: &EnumShape<'_>, aux: &mut Vec<String>) -> String {
    let &EnumShape {
        args,
        defaults: _defaults,
        export_name,
        generic_params,
        item_enum,
        rust_ident,
        tag_attrs,
        type_parameters: _type_parameters,
    } = shape;
    let tag_key = tag_attrs.tag.as_deref().unwrap_or("type");
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let mut cases = String::new();
    let mut decode_arms = String::new();
    let mut encode_arms = String::new();
    let mut ruled: Vec<(String, Vec<String>)> = Vec::new();
    let mut slots: Vec<SwiftFieldShape> = Vec::new();
    let mut twins = Vec::new();
    // A payload is read from the object the tag is in, and is not handed the tag.
    let source = read::Source::Beside(preprocess::taken_with(iter::once(tag_key)));
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = swift_member(&RenameRule::CamelCase.apply_to_variant(&rust_name));
        let wire_tag = rename_override(&variant.attrs)
            .unwrap_or_else(|| variant_rule.apply_to_variant(&rust_name));
        let payload = resolve_variant_payload(shape, variant, field_rule, None, aux);
        ruled.push((case_name.clone(), payload.rules.clone()));
        cases.push_str(&variant_case(&case_name, payload.held.as_deref()));
        slots.extend(payload.converted.as_ref().map(|slot| slot.shape.clone()));
        if payload.held.is_some() {
            let from = VariantRead {
                keys: "",
                source: &source,
                stem: swift_bare(&case_name),
            };
            let read = payload.read(
                &from,
                &|held| format!("self = .{case_name}({held})"),
                &mut twins,
            );
            write!(decode_arms, "case \"{wire_tag}\": {read}; ").unwrap();
            write!(
                encode_arms,
                "case .{case_name}(let payload): try container.encode(\"{wire_tag}\", forKey: .swiftSchemaTag); \
                 try {}.encode(to: encoder); ",
                payload.written("payload")
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
    let reads = read::both(
        &format!(
            "let container = try decoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); \
             let tag = try container.decode(String.self, forKey: .swiftSchemaTag); \
             switch tag {{ {decode_arms} default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(tag)\")) }}"
        ),
        &preprocess::Written {
            beside: "",
            named: export_name,
            own: args.preprocess,
            twins,
        },
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); \
         switch self {{ {encode_arms} }} }};"
    );
    let validate = enum_validate(shape, &ruled);
    let body = format!(
        "public enum {export_name}{generic_params}: Codable, Sendable {{ {cases} {tag_coding_key} {reads}{encode_method} {validate} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{}{typealias}", enum_date_helpers(shape, &slots))
}

/// The Swift tokens the adjacently tagged shape earns (`tag = "...", content = "..."`): the tag
/// and the payload sit at two keys of the same object, so a variant's own payload decodes and
/// encodes through the `content` key directly, needing no shared-decoder trick.
fn adjacent_tagged_enum_swift_source(shape: &EnumShape<'_>, aux: &mut Vec<String>) -> String {
    let &EnumShape {
        args,
        defaults: _defaults,
        export_name,
        generic_params,
        item_enum,
        rust_ident,
        tag_attrs,
        type_parameters: _type_parameters,
    } = shape;
    let tag_key = tag_attrs.tag.as_deref().unwrap_or("type");
    let content_key = tag_attrs.content.as_deref().unwrap_or("content");
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let mut cases = String::new();
    let mut decode_arms = String::new();
    let mut encode_arms = String::new();
    let mut ruled: Vec<(String, Vec<String>)> = Vec::new();
    let mut slots: Vec<SwiftFieldShape> = Vec::new();
    let mut twins = Vec::new();
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = swift_member(&RenameRule::CamelCase.apply_to_variant(&rust_name));
        let wire_tag = rename_override(&variant.attrs)
            .unwrap_or_else(|| variant_rule.apply_to_variant(&rust_name));
        let payload = resolve_variant_payload(shape, variant, field_rule, Some(content_key), aux);
        ruled.push((case_name.clone(), payload.rules.clone()));
        cases.push_str(&variant_case(&case_name, payload.held.as_deref()));
        slots.extend(payload.converted.as_ref().map(|slot| slot.shape.clone()));
        if payload.held.is_some() {
            let source = read::Source::Keyed {
                container: "container".to_owned(),
                key: ".swiftSchemaContent".to_owned(),
                presence: read::Presence::Refused,
            };
            let from = VariantRead {
                keys: "SwiftSchemaAdjacentCodingKeys",
                source: &source,
                stem: swift_bare(&case_name),
            };
            let read = payload.read(
                &from,
                &|held| format!("self = .{case_name}({held})"),
                &mut twins,
            );
            write!(decode_arms, "case \"{wire_tag}\": {read}; ").unwrap();
            write!(
                encode_arms,
                "case .{case_name}(let payload): try container.encode(\"{wire_tag}\", forKey: .swiftSchemaTag); \
                 try container.encode({}, forKey: .swiftSchemaContent); ",
                payload.written("payload")
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
    let reads = read::both(
        &format!(
            "let container = try decoder.container(keyedBy: SwiftSchemaAdjacentCodingKeys.self); \
             let tag = try container.decode(String.self, forKey: .swiftSchemaTag); \
             switch tag {{ {decode_arms} default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(tag)\")) }}"
        ),
        &preprocess::Written {
            beside: "",
            named: export_name,
            own: args.preprocess,
            twins,
        },
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: SwiftSchemaAdjacentCodingKeys.self); \
         switch self {{ {encode_arms} }} }};"
    );
    let validate = enum_validate(shape, &ruled);
    let body = format!(
        "public enum {export_name}{generic_params}: Codable, Sendable {{ {cases} {tagging_keys} {reads}{encode_method} {validate} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{}{typealias}", enum_date_helpers(shape, &slots))
}

/// The Swift tokens a `#[serde(untagged)]` enum earns: its own read takes the first variant whose
/// payload reads and whose bounds hold, the bound of a member and of a brand it holds. A check and
/// a model a variant holds choose nothing: `init(from:)` judges the variant taken.
fn untagged_enum_swift_source(shape: &EnumShape<'_>, aux: &mut Vec<String>) -> String {
    let &EnumShape {
        args,
        defaults: _defaults,
        export_name,
        generic_params,
        item_enum,
        rust_ident,
        tag_attrs,
        type_parameters: _type_parameters,
    } = shape;
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let mut cases = String::new();
    let mut choosers = String::new();
    let mut decode_attempts = String::new();
    let mut encode_arms = String::new();
    let mut ruled: Vec<(String, Vec<String>)> = Vec::new();
    let mut slots: Vec<SwiftFieldShape> = Vec::new();
    let mut twins = Vec::new();
    // A payload is read from the decoder of the enum, and is handed the keys the enum is.
    let source = read::Source::Beside(preprocess::taken_with(iter::empty()));
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = swift_member(&RenameRule::CamelCase.apply_to_variant(&rust_name));
        let payload = resolve_variant_payload(shape, variant, field_rule, None, aux);
        ruled.push((case_name.clone(), payload.rules.clone()));
        cases.push_str(&variant_case(&case_name, payload.held.as_deref()));
        slots.extend(payload.converted.as_ref().map(|slot| slot.shape.clone()));
        if let Some(ty) = &payload.held {
            let named = variant.ident.unraw();
            let from = VariantRead {
                keys: "",
                source: &source,
                stem: swift_bare(&case_name),
            };
            let returned = payload.read(&from, &|held| format!("return {held}"), &mut twins);
            let reader = format!("swiftSchemaRead{named}");
            let tried = tried_read(&returned, export_name, &reader, ty, &mut choosers);
            let holds = if payload.bound.is_empty() {
                String::new()
            } else {
                write!(
                    choosers,
                    "private static func swiftSchemaBound{named}(_ payload: {ty}) -> [String] \
                     {{ var errors: [String] = []; {}; return errors }}; ",
                    payload.bound.join("; ")
                )
                .unwrap();
                format!(", {export_name}.swiftSchemaBound{named}(value).isEmpty")
            };
            write!(
                decode_attempts,
                "if let value = try? {tried}{holds} {{ self = .{case_name}(value); return }}; "
            )
            .unwrap();
            write!(
                encode_arms,
                "case .{case_name}(let value): try {}.encode(to: encoder); ",
                payload.written("value")
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
    let reads = read::both(
        &format!(
            "{decode_attempts}throw DecodingError.typeMismatch({export_name}.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"no untagged member matched\"))"
        ),
        &preprocess::Written {
            beside: &choosers,
            named: export_name,
            own: args.preprocess,
            twins,
        },
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ switch self {{ {encode_arms} }} }};"
    );
    let validate = enum_validate(shape, &ruled);
    let key_type = read::any_key(&choosers);
    let helpers = validate::helpers(&choosers, &validate);
    let body = format!(
        "public enum {export_name}{generic_params}: Codable, Sendable {{ {cases} {key_type}{choosers}{reads}{encode_method} {validate}{helpers} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{}{typealias}", enum_date_helpers(shape, &slots))
}

/// The expression the untagged enum `owner` tries a variant's payload with, of type `read_as`,
/// where `returned` is the statements that read it: one expression is tried where it is written,
/// and a read of several statements in the function `reader`, added to `choosers` and handed the
/// keys taken where its statements name them.
fn tried_read(
    returned: &str,
    owner: &str,
    reader: &str,
    read_as: &str,
    choosers: &mut String,
) -> String {
    if let Some(read) = returned.strip_prefix("return try ")
        && !returned.contains("; ")
    {
        return read.to_owned();
    }
    let (taking, taken) = if read::names_taken(returned) {
        (", _ taken: [String]", ", taken")
    } else {
        ("", "")
    };
    write!(
        choosers,
        "private static func {reader}(_ decoder: Decoder{taking}) throws -> {read_as} \
         {{ {returned} }}; "
    )
    .unwrap();
    format!("{owner}.{reader}(decoder{taken})")
}

/// The date helpers the read and the write of an enum call where the lone slot of a variant reads a
/// date, of `slots`, after the separator that ends the enum. Empty where none does.
fn enum_date_helpers(shape: &EnumShape<'_>, slots: &[SwiftFieldShape]) -> String {
    let shapes: Vec<&SwiftFieldShape> = slots.iter().collect();
    let helpers = datetime_helpers_for(shape.export_name, shape.generic_params, &shapes);
    if helpers.is_empty() {
        helpers
    } else {
        format!("; {helpers}")
    }
}

/// An enum's `validate()`: the rules of the case the value holds, then the enum's own checks.
fn enum_validate(shape: &EnumShape<'_>, variants: &[(String, Vec<String>)]) -> String {
    let mut rules: Vec<String> = validate::held_case(variants).into_iter().collect();
    rules.extend(validate::own_checks(shape.args.check));
    validate::note(shape.rust_ident, None, !rules.is_empty());
    validate::method(&rules)
}

/// The Swift tokens an enum earns: a `String`-backed enum for an all-unit, untagged-by-default
/// declaration; otherwise the tagging shape `process_enum` itself dispatches the declaration to.
fn enum_swift_tokens(item_enum: &ItemEnum, args: &SwiftTypeArgs<'_>) -> TokenStream {
    let rust_ident = item_enum.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_swift_name(&rust_ident, &export_name);
    validate::note(&rust_ident, None, true);
    let tag_attrs = enum_tag_attrs(&item_enum.attrs);
    let writes_bare_variant_names =
        tag_attrs.tag.is_none() && tag_attrs.content.is_none() && !tag_attrs.untagged;
    let type_parameters = type_parameters_in_scope(&item_enum.generics);
    let generic_params = swift_generic_params(&item_enum.generics);
    let defaults = SerdeDefaults::of_enum(item_enum);
    let shape = EnumShape {
        args,
        defaults: &defaults,
        export_name: &export_name,
        generic_params: &generic_params,
        item_enum,
        rust_ident: &rust_ident,
        tag_attrs: &tag_attrs,
        type_parameters: &type_parameters,
    };
    let mut aux = Vec::new();
    let swift_source = if is_plain_enum(item_enum) && writes_bare_variant_names {
        plain_enum_swift_source(item_enum, &rust_ident, &export_name, args)
    } else if tag_attrs.untagged {
        untagged_enum_swift_source(&shape, &mut aux)
    } else if tag_attrs.tag.is_some() && tag_attrs.content.is_some() {
        adjacent_tagged_enum_swift_source(&shape, &mut aux)
    } else if tag_attrs.tag.is_some() {
        internal_tagged_enum_swift_source(&shape, &mut aux)
    } else {
        external_tagged_enum_swift_source(&shape, &mut aux)
    };
    preprocess::note(&rust_ident, read::takes_keys(&swift_source));
    let full_source = if aux.is_empty() {
        swift_source
    } else {
        format!("{} {swift_source}", aux.join(" "))
    };
    swift_module_tokens(&rust_ident, item_enum.ident.span(), &full_source, &defaults)
}
