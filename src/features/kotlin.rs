//! Kotlin type generation with `kotlinx.serialization` annotations: one `kotlin_definition()` per
//! `#[model_schema]` item in a `{snake}_kotlin` module, dispatched from `exec_model_schema` the way
//! the Dart backend is. Unlike Dart, Kotlin has no built-in JSON codec — the compiler plugin
//! `kotlinx.serialization` reads the annotations this module writes and generates the encoder and
//! decoder of a class itself. The shapes the plugin cannot express declaratively (an enum under
//! any tagging, a struct with a flattened member, and a tuple) carry a `KSerializer` this module
//! writes instead.
//!
//! Every type but a plain enum has two serializers, which `read` writes: `{Type}Unjudged`, the
//! plugin's or the written one, and `{Type}Serializer`, which reads through it and judges. The
//! unjudged one calls each function a `preprocess` names, as `preprocess` writes the call.

mod aliases;
mod preprocess;
mod read;
mod validate;

use core::cell::{Cell, RefCell};
use core::fmt::Write as _;
use core::iter::once;
use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt as _;
use syn::punctuated::Punctuated;
use syn::{
    Field, Fields, Ident, Item, ItemEnum, ItemStruct, ItemType, Token, Type, TypeParamBound,
    Variant, WherePredicate,
};

use self::preprocess::Plugin;
use self::validate::{Member, Rules};
use crate::features::model_schema_prop::{FunctionName, ModelSchemaPropMeta};
use crate::features::serde::defaults::{SerdeDefaults, substituted};
use crate::features::serde::{
    brand_field, has_serde_skip_serializing, keys_taken_when_flattened, parse_serde_key_omission,
    struct_tag, variant_names_when_flattened,
};
use crate::field_type::{
    FieldDef, FieldDefType, RESULT_ARMS, RangeKind, VariantKind, classify_variant, get_field_def,
    is_plain_enum, is_sequence_wrapper,
};
use crate::rename_rule::{RenameRule, resolve_rename_rule};
use crate::utils::{
    compute_alias_export_name, compute_item_export_name, to_snake_case, type_parameters_in_scope,
};

#[cfg(feature = "serde")]
use crate::features::serde::{parse_serde_field_attributes, parse_serde_type_attributes};

/// The words Kotlin reserves everywhere, which name a member only between backticks. A soft keyword
/// or a modifier such as `value` or `data` is not among them.
const KOTLIN_HARD_KEYWORDS: [&str; 28] = [
    "as",
    "break",
    "class",
    "continue",
    "do",
    "else",
    "false",
    "for",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "null",
    "object",
    "package",
    "return",
    "super",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "typeof",
    "val",
    "var",
    "when",
    "while",
];

/// What a sealed interface declares the method each of its variants overrides as.
const VALIDATE_DECLARED: &str = "{ fun validate(): List<String> }";

/// The payload a serializer that corrects the plugin's reads, `transformDeserialize`'s parameter.
const ELEMENT: &str = "element";

/// What `#[model_schema(...)]` wrote on an item, as this module reads it.
pub struct KotlinArgs<'args> {
    /// The functions `check` names on the type, each taking the whole value.
    pub check: &'args [FunctionName],
    /// The type declared per type parameter, which a brand's bounds are run at.
    pub default_types: &'args [(Ident, Type)],
    /// A brand's `maxLength`.
    pub max_length: Option<usize>,
    /// A member's own `model_schema_prop`, held to the string check of the enum around it.
    pub member_meta: &'args dyn Fn(&Field) -> ModelSchemaPropMeta,
    /// A brand's `minLength`.
    pub min_length: Option<usize>,
    pub name_override: Option<&'args str>,
    /// A brand's `pattern`.
    pub pattern: Option<&'args str>,
    /// The functions `preprocess` names on the type, each taking the whole payload.
    pub preprocess: &'args [FunctionName],
}

/// One field this module has decided belongs on the wire: its Rust name (camel-cased into the
/// Kotlin property spelling by [`kotlin_property_name`]), its wire name, whether it is a
/// `#[serde(flatten)]` source, and the [`FieldDef`] describing its type.
struct KotlinField {
    /// The names serde reads the member under beside its wire name, its aliases.
    aliases: Vec<String>,
    /// Where the JSON of the default serde reads for an absent key is written.
    default_at: Option<String>,
    /// Whether that default is of a member whose type names a parameter of the class: the empty
    /// collection, the one default `SerdeDefaults` keeps for such a member.
    empty_default: bool,
    field_def: FieldDef,
    flatten: bool,
    /// Whether serde never writes the member: it is under `skip_serializing`.
    never_written: bool,
    rust_name: String,
    wire_name: String,
}

/// The container attributes read off any enum, in the shape `process_enum` itself dispatches on —
/// `tag`/`content`/`untagged` default to "written none of them" without the `serde` feature, which
/// is what leaves an all-unit enum publishing the plain enum-class shape regardless.
struct EnumTagAttrs {
    content: Option<String>,
    rename_all: Option<String>,
    rename_all_fields: Option<String>,
    tag: Option<String>,
    untagged: bool,
}

/// One tagged/untagged variant's payload, classified once so every builder below reads it the same
/// way. A `TupleMultiple` payload is folded into one `Tuple` [`FieldDef`], matching how a slot list
/// renders everywhere else in this module.
enum VariantPayload {
    Named(Vec<KotlinField>),
    Unit,
    Value(Box<FieldDef>),
}

/// How serde tags the variants of an enum, which is what names a variant's member in a message.
#[derive(Clone, Copy)]
enum Tagging<'tag> {
    /// A tag and a content key: a member is written under the content key.
    Adjacent(&'tag str),
    /// No tag attribute: a member is written under the variant's name.
    External,
    /// A tag alone, or no tag at all: a member is written beside the tag, or as the value.
    Unwrapped,
}

/// What [`flatten_plan`] builds from a struct's fields: one `serialize_stmts` entry per field
/// serde writes (own or flattened), one `deserialize_stmts` entry per field (own reads and
/// flattened reads interleaved, in field order, each flattened read handed the object without the
/// keys taken ahead of it), and the constructor argument list in field order.
struct FlattenPlan {
    ctor_args: Vec<String>,
    deserialize_stmts: Vec<String>,
    needs_lenient: bool,
    serialize_stmts: Vec<String>,
}

/// Everything [`dispatched_tagged_variant`] needs that stays the same across every variant of one
/// enum, read once by [`dispatched_tagged_enum_kotlin_source`] and passed through — so the loop
/// over variants is the only thing that changes per call.
struct DispatchedTaggedContext<'ctx> {
    args: &'ctx KotlinArgs<'ctx>,
    /// The defaults of the members of the enum's variants.
    defaults: &'ctx SerdeDefaults,
    export_name: &'ctx str,
    field_rule: RenameRule,
    generic_params: &'ctx str,
    nothing_generic_args: &'ctx str,
    serializable: &'ctx [String],
    tag_key: &'ctx str,
    /// [`Tagging::Unwrapped`] is a tag alone here: an untagged enum is written elsewhere.
    tagging: Tagging<'ctx>,
    type_parameters: &'ctx [String],
    variant_rule: RenameRule,
}

thread_local! {
    /// The Kotlin class/enum name each Rust ident publishes, read back by a sibling reference.
    static KOTLIN_NAMES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    /// The auxiliary top-level declarations a Tuple-shaped field earns — a wrapper `data class` and
    /// its `KSerializer` (see [`tuple_wrapper_typename`]). Drained by [`kotlin_module_tokens`].
    static KOTLIN_TUPLE_AUX: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Monotonic across the whole compilation, matching how `KOTLIN_NAMES` persists across
    /// invocations — never reset, since every tuple wrapper this module ever writes lands in the
    /// same consuming file and must not collide with any other item's.
    static KOTLIN_TUPLE_COUNTER: Cell<u32> = const { Cell::new(0) };
    /// The class each kind of range the item being expanded writes is declared as, by the members
    /// it declares, so every range of one kind names one class. Cleared with
    /// [`KOTLIN_TUPLE_AUX`].
    static KOTLIN_RANGES: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    /// The wrapper each shape of tuple the item being expanded writes is declared as, by the body
    /// its class writes, so every ask of one shape names one wrapper. Cleared with
    /// [`KOTLIN_TUPLE_AUX`].
    static KOTLIN_TUPLES: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    /// The sealed interface each shape of `Result` the item being expanded writes is declared as,
    /// by which of its arms holds `()`, so every `Result` of one shape names one interface. Cleared
    /// with [`KOTLIN_TUPLE_AUX`].
    static KOTLIN_RESULTS: RefCell<Vec<([bool; 2], String)>> = const { RefCell::new(Vec::new()) };
    /// The names serde reads each enum the emitter wrote under when it is flattened, by Rust
    /// ident: the first entry of the holder's object under one of them is the enum's.
    static KOTLIN_VARIANT_NAMES: RefCell<HashMap<String, Vec<String>>> =
        RefCell::new(HashMap::new());
    /// Each externally tagged enum the emitter wrote that writes a unit variant as its bare name,
    /// by Rust ident: serde writes that variant flattened as an entry, its name over `null`.
    static KOTLIN_BARE_NAMED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// The keys serde takes off its holder's object for each struct the emitter wrote a serializer
    /// for by hand, by Rust ident: the descriptor of that serializer declares no element.
    static KOTLIN_TAKEN_KEYS: RefCell<HashMap<String, Vec<String>>> =
        RefCell::new(HashMap::new());
}

/// The Kotlin tokens `item` earns, given what `#[model_schema(...)]` wrote on it. Dispatches on
/// the item's own shape; an item this module has nothing to say about earns nothing.
pub fn kotlin_schema_dispatch(item: &Item, args: &KotlinArgs<'_>) -> TokenStream {
    if let Item::Struct(item_struct) = item {
        struct_kotlin_tokens(item_struct, args)
    } else if let Item::Enum(item_enum) = item {
        enum_kotlin_tokens(item_enum, args)
    } else if let Item::Type(item_type) = item {
        alias_kotlin_tokens(item_type, args.name_override)
    } else {
        TokenStream::new()
    }
}

/// The `u64`/`usize` width `field` is, or at any depth reaches, or `None`. Kotlin has a `ULong`
/// that could carry `u64`; the refusal exists for parity with the Swift target instead.
pub fn kotlin_refused_width(field: &FieldDef) -> Option<&'static str> {
    match &field.field_type {
        FieldDefType::U64 => Some("u64"),
        FieldDefType::Usize => Some("usize"),
        FieldDefType::SiblingType(_, generics) => generics.iter().find_map(kotlin_refused_width),
        FieldDefType::Map(key, value) => {
            kotlin_refused_width(key).or_else(|| kotlin_refused_width(value))
        }
        FieldDefType::Tuple(elements) => elements.iter().find_map(kotlin_refused_width),
        FieldDefType::Range(_, bound) => kotlin_refused_width(bound),
        FieldDefType::Result(ok, err) => {
            kotlin_refused_width(ok).or_else(|| kotlin_refused_width(err))
        }
        FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::StringLiteral(_)
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Boolean
        | FieldDefType::Char
        | FieldDefType::String
        | FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64 => None,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => None,
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => None,
    }
}

fn register_kotlin_name(rust_ident: &str, export_name: &str) {
    KOTLIN_NAMES.with(|names| {
        names
            .borrow_mut()
            .insert(rust_ident.to_owned(), export_name.to_owned());
    });
}

/// The Rust ident of the enum a flattened member typed `def` holds, and the names it takes the
/// first entry under, as serde reads an externally tagged enum: `None` for any other type, one the
/// emitter has not written, and a member that names a function, which is handed the whole object.
fn flattened_variant_names(def: &FieldDef) -> Option<(&str, Vec<String>)> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return None;
    };
    if !preprocess::of(def).is_empty() {
        return None;
    }
    let names = KOTLIN_VARIANT_NAMES.with(|noted| noted.borrow().get(name).cloned())?;
    (!names.is_empty()).then_some((name.as_str(), names))
}

/// Whether a flattened member typed `def` may write a variant as its bare name: an externally
/// tagged enum the emitter wrote with a unit variant, whatever function it names.
fn writes_a_bare_name(def: &FieldDef) -> bool {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return false;
    };
    KOTLIN_BARE_NAMED.with(|noted| noted.borrow().contains(name))
}

/// The keys a flattened struct typed `def` takes off its holder's object, as a Kotlin set: those
/// serde reads where the emitter wrote the struct's serializer by hand, and the `elementNames` of
/// the descriptor `serializer` answers for any other struct.
fn flattened_struct_keys(def: &FieldDef, serializer: &str) -> String {
    let recorded = if let FieldDefType::SiblingType(name, _) = &def.field_type {
        KOTLIN_TAKEN_KEYS.with(|noted| noted.borrow().get(name).cloned())
    } else {
        None
    };
    recorded.map_or_else(
        || format!("({serializer}).descriptor.elementNames.toSet()"),
        |keys| {
            if keys.is_empty() {
                // Kotlin infers no element type from an empty `setOf()`.
                "setOf<String>()".to_owned()
            } else {
                let listed = keys
                    .iter()
                    .map(|key| format!("\"{}\"", kotlin_template_text(key)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("setOf({listed})")
            }
        },
    )
}

/// The Kotlin name registered for `rust_ident`, or `None` for a type declared below the one
/// asking — which falls back to its own Rust ident, harmless since a renamed item always
/// re-publishes that ident too, as a `typealias`.
fn lookup_kotlin_name(rust_ident: &str) -> Option<String> {
    KOTLIN_NAMES.with(|names| names.borrow().get(rust_ident).cloned())
}

/// The `FieldDef` of the field a brand is the value of, its type parameters already erased, with
/// the bounds the brand writes on itself as the value's own.
fn brand_value_field(held: &Field, type_parameters: &[String], args: &KotlinArgs<'_>) -> FieldDef {
    let mut field_def = get_field_def("value", &held.ty, "");
    field_def.model_schema_prop_meta = Some(ModelSchemaPropMeta {
        max_length: args.max_length,
        min_length: args.min_length,
        pattern: args.pattern.map(ToOwned::to_owned),
        ..ModelSchemaPropMeta::default()
    });
    field_def.erase_type_parameters(type_parameters);
    field_def
}

/// One field's `#[model_schema_prop(...)]` metadata, folded into the `FieldDef` `get_field_def`
/// built for it — `get_field_def` reads the Rust type alone, so `nullable`/`as_number` and the
/// bounds need this filled in separately, as `process_field` does for the other three surfaces.
fn field_def_with_prop_meta(name: &str, field: &Field, args: &KotlinArgs<'_>) -> FieldDef {
    let mut field_def = get_field_def(name, &field.ty, "");
    field_def.model_schema_prop_meta = Some((args.member_meta)(field));
    field_def
}

/// The `#[serde(rename = "...")]` a field or variant earns, honored only where the `serde` feature
/// reads serde attributes at all.
#[cfg(feature = "serde")]
fn rename_override(attrs: &[syn::Attribute]) -> Option<String> {
    parse_serde_field_attributes(attrs).rename
}

#[cfg(not(feature = "serde"))]
const fn rename_override(_attrs: &[syn::Attribute]) -> Option<String> {
    None
}

/// Whether `attrs` carries `#[serde(flatten)]` — always `false` without the `serde` feature, since
/// no field can carry an attribute the container itself never reads.
#[cfg(feature = "serde")]
fn field_is_flatten(attrs: &[syn::Attribute]) -> bool {
    parse_serde_field_attributes(attrs).flatten
}

#[cfg(not(feature = "serde"))]
const fn field_is_flatten(_attrs: &[syn::Attribute]) -> bool {
    false
}

/// The wire name a field with Rust name `rust_name` and its own `rename` writes under, once
/// `rule` — the container's own `rename_all`, [`RenameRule::None`] without the `serde` feature —
/// has had its say. An explicit rename always wins over the container's rule, matching serde.
fn wire_field_name(rust_name: &str, rename: Option<&str>, rule: RenameRule) -> String {
    rename.map_or_else(|| rule.apply_to_field(rust_name), ToOwned::to_owned)
}

/// A container's own `rename_all`, or [`RenameRule::None`] without the `serde` feature to read it
/// with — shared by a struct and an enum, whose variant names read the very same attribute.
#[cfg(feature = "serde")]
fn container_rename_rule(attrs: &[syn::Attribute]) -> RenameRule {
    let meta = parse_serde_type_attributes(attrs);
    resolve_rename_rule(meta.rename_all.as_deref())
}

#[cfg(not(feature = "serde"))]
fn container_rename_rule(_attrs: &[syn::Attribute]) -> RenameRule {
    resolve_rename_rule(None)
}

/// The container attributes an enum's own dispatch reads, or every field left at its default
/// without the `serde` feature to read one with.
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

/// A property's name without the backticks [`kotlin_property_name`] may have put around it: what
/// a longer identifier is built from, and what the wire name is compared with.
pub fn kotlin_bare(property: &str) -> &str {
    property.trim_matches('`')
}

/// `text` as it is written inside a Kotlin string: a backslash, a quote and a `$`, which opens a
/// template, each behind a backslash, and a line break or a tab as its escape.
pub fn kotlin_template_text(text: &str) -> String {
    let mut written = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => written.push_str("\\\\"),
            '"' => written.push_str("\\\""),
            '$' => written.push_str("\\$"),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            other => written.push(other),
        }
    }
    written
}

/// `cased`, a name already cased as Kotlin writes one, between backticks where it is a hard
/// keyword, which is the name itself to Kotlin and to `kotlinx.serialization`.
pub fn kotlin_name(cased: &str) -> String {
    if KOTLIN_HARD_KEYWORDS.contains(&cased) {
        format!("`{cased}`")
    } else {
        cased.to_owned()
    }
}

/// `rust_name` cased the way a Kotlin property is: `conversation_id` -> `conversationId`. Reuses
/// serde's own `camelCase` rule (`rename_rule.rs`) rather than a second implementation, since the
/// two rules coincide exactly on a `snake_case` Rust identifier.
pub fn kotlin_property_name(rust_name: &str) -> String {
    kotlin_name(&RenameRule::CamelCase.apply_to_field(rust_name))
}

/// Whether `field` carries `#[model_schema_prop(nullable)]` — the flag that keeps an `Option<T>`
/// property's key always written, so it earns no `= null` default.
fn is_nullable_flag(field: &FieldDef) -> bool {
    field
        .model_schema_prop_meta
        .as_ref()
        .is_some_and(|meta| meta.nullable)
}

#[cfg(feature = "chrono")]
fn has_as_number(field: &FieldDef) -> bool {
    field
        .model_schema_prop_meta
        .as_ref()
        .is_some_and(|meta| meta.as_number)
}

/// Walks a named-field struct's or a struct-shaped enum variant's fields into [`KotlinField`]s,
/// dropping any field a serde attribute takes off the wire in both directions. A field dropped
/// from serialization only is pushed a nullable level, since kotlinx has no "absent key" spelling.
fn collect_kotlin_fields(
    fields: &Fields,
    rule: RenameRule,
    type_parameters: &[String],
    args: &KotlinArgs<'_>,
    default_at: &dyn Fn(&str) -> Option<String>,
) -> Vec<KotlinField> {
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
        let mut field_def = field_def_with_prop_meta(&rust_name, field, args);
        field_def.erase_type_parameters(type_parameters);
        if omission.omits_key && !field_def.is_optional() {
            field_def.nullable_levels.push(field_def.array_depth);
        }
        let defaulted = default_at(&rust_name);
        let flatten = field_is_flatten(&field.attrs);
        collected.push(KotlinField {
            aliases: if flatten || omission.skips_deserializing {
                Vec::new()
            } else {
                aliases::written_on(&field.attrs)
            },
            empty_default: defaulted.is_some()
                && field_reaches_type_parameter(&field_def, type_parameters),
            default_at: defaulted,
            field_def,
            flatten,
            never_written: has_serde_skip_serializing(&field.attrs),
            rust_name,
            wire_name,
        });
    }
    collected
}

/// The Kotlin type before the outer `?` an [`FieldDef::is_optional`] field carries: the scalar
/// match, then one `List<…>` per array level, an inner level written `?` where
/// [`FieldDef::is_nullable_at`] says so. Mirrors `dart_base`.
fn kotlin_base(field: &FieldDef) -> String {
    let scalar = match &field.field_type {
        // A type written as no path, a function pointer or a macro among them.
        FieldDefType::Unknown => "JsonElement".to_owned(),
        FieldDefType::TypeParam(name) => name.clone(),
        FieldDefType::Tuple(elements) => tuple_wrapper_typename(elements, false),
        FieldDefType::Range(kind, bound) => range_typename(*kind, &kotlin_typename(bound)),
        FieldDefType::Result(ok, err) => result_typename(ok, err, kotlin_typename),
        FieldDefType::SiblingType(name, generics) => {
            if let [element] = generics.as_slice()
                && is_sequence_wrapper(name)
            {
                return kotlin_base(&field.collection_element_field(element));
            }
            let class_name = lookup_kotlin_name(name).unwrap_or_else(|| name.clone());
            if generics.is_empty() {
                class_name
            } else {
                let arguments = generics
                    .iter()
                    .map(kotlin_typename)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{class_name}<{arguments}>")
            }
        }
        FieldDefType::Map(key, value) => {
            format!("Map<{}, {}>", kotlin_typename(key), kotlin_typename(value))
        }
        FieldDefType::Boolean => "Boolean".to_owned(),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            "String".to_owned()
        }
        FieldDefType::BooleanLiteral(_) => "Boolean".to_owned(),
        FieldDefType::NumberLiteral(_) => "Double".to_owned(),
        FieldDefType::U8 => "UByte".to_owned(),
        FieldDefType::U16 => "UShort".to_owned(),
        FieldDefType::U32 => "UInt".to_owned(),
        // Refused under `check_kotlin_width_field` in `model_schema.rs`; a filler rendering keeps
        // this dispatch total for the compile_error! tokens emitted alongside it to stand on.
        FieldDefType::U64 | FieldDefType::Usize => "Long".to_owned(),
        FieldDefType::I8 => "Byte".to_owned(),
        FieldDefType::I16 => "Short".to_owned(),
        FieldDefType::I32 => "Int".to_owned(),
        FieldDefType::I64 | FieldDefType::Isize => "Long".to_owned(),
        FieldDefType::F32 => "Float".to_owned(),
        FieldDefType::F64 => "Double".to_owned(),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => "ObjectId".to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            "String".to_owned()
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => {
            if has_as_number(field) {
                "Long".to_owned()
            } else {
                "String".to_owned()
            }
        }
    };
    (0..field.array_depth).fold(scalar, |wrapped, level| {
        let item = if field.is_nullable_at(level) {
            format!("{wrapped}?")
        } else {
            wrapped
        };
        format!("List<{item}>")
    })
}

/// The Kotlin type `field` renders as: [`kotlin_base`] plus the `?` an [`FieldDef::is_optional`]
/// field carries — one nullable spelling for both a bare `Option<T>` and
/// `#[model_schema_prop(nullable)]` (see [`property_declaration`] for the `= null` default).
pub fn kotlin_typename(field: &FieldDef) -> String {
    let base = kotlin_base(field);
    if field.is_optional() {
        format!("{base}?")
    } else {
        base
    }
}

/// Whether `field` reaches one of `type_parameters` at any depth. Decides whether
/// [`kotlin_serializer_expr`] must compose `field`'s `KSerializer` from the enclosing item's own
/// constructor serializers instead of the reified `serializer<T>()`, which needs a concrete type.
fn field_reaches_type_parameter(field: &FieldDef, type_parameters: &[String]) -> bool {
    match &field.field_type {
        FieldDefType::TypeParam(name) => type_parameters.iter().any(|parameter| parameter == name),
        FieldDefType::SiblingType(_, generics) => generics
            .iter()
            .any(|generic| field_reaches_type_parameter(generic, type_parameters)),
        FieldDefType::Map(key, value) => {
            field_reaches_type_parameter(key, type_parameters)
                || field_reaches_type_parameter(value, type_parameters)
        }
        FieldDefType::Range(_, bound) => field_reaches_type_parameter(bound, type_parameters),
        FieldDefType::Result(ok, err) => {
            field_reaches_type_parameter(ok, type_parameters)
                || field_reaches_type_parameter(err, type_parameters)
        }
        FieldDefType::Tuple(_)
        | FieldDefType::Unknown
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
        | FieldDefType::Isize
        | FieldDefType::Usize
        | FieldDefType::F32
        | FieldDefType::F64 => false,
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => false,
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => false,
    }
}

/// `{lowerCamel(parameter)}Serializer` — the constructor parameter a serializer with type
/// parameters binds one parameter's own `KSerializer` to, and the identifier every reference to
/// that parameter's serializer reads back.
fn kotlin_serializer_param_name(parameter: &str) -> String {
    format!(
        "{}Serializer",
        RenameRule::CamelCase.apply_to_variant(parameter)
    )
}

/// A reference to `field`'s own `KSerializer`. A field reaching none of `type_parameters`
/// resolves through the reified `serializer<T>()` as before; one that does reach a parameter
/// composes `ListSerializer`/`MapSerializer`/`.nullable` instead, since a parameter is never reifiable.
fn kotlin_serializer_expr(field: &FieldDef, type_parameters: &[String]) -> String {
    if !field_reaches_type_parameter(field, type_parameters) {
        return format!("serializer<{}>()", kotlin_typename(field));
    }
    let scalar = match &field.field_type {
        FieldDefType::TypeParam(name) => kotlin_serializer_param_name(name),
        FieldDefType::SiblingType(name, generics) => {
            if let [element] = generics.as_slice()
                && is_sequence_wrapper(name)
            {
                return kotlin_serializer_expr(
                    &field.collection_element_field(element),
                    type_parameters,
                );
            }
            let class_name = lookup_kotlin_name(name).unwrap_or_else(|| name.clone());
            let arguments = generics
                .iter()
                .map(|generic| kotlin_serializer_expr(generic, type_parameters))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{class_name}.serializer({arguments})")
        }
        FieldDefType::Map(key, value) => format!(
            "MapSerializer({}, {})",
            kotlin_serializer_expr(key, type_parameters),
            kotlin_serializer_expr(value, type_parameters)
        ),
        FieldDefType::Range(kind, bound) => {
            range_serializer(*kind, &kotlin_serializer_expr(bound, type_parameters))
        }
        FieldDefType::Result(ok, err) => {
            result_serializer(ok, err, |arm| kotlin_serializer_expr(arm, type_parameters))
        }
        // None of these ever reaches a type parameter (see `field_reaches_type_parameter`), so
        // this arm is unreached in practice; it stays total rather than a wildcard.
        FieldDefType::Tuple(_)
        | FieldDefType::Unknown
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
        | FieldDefType::Isize
        | FieldDefType::Usize
        | FieldDefType::F32
        | FieldDefType::F64 => format!("serializer<{}>()", kotlin_typename(field)),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => format!("serializer<{}>()", kotlin_typename(field)),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => format!("serializer<{}>()", kotlin_typename(field)),
    };
    let wrapped = (0..field.array_depth).fold(scalar, |inner, level| {
        let item = if field.is_nullable_at(level) {
            format!("{inner}.nullable")
        } else {
            inner
        };
        format!("ListSerializer({item})")
    });
    if field.is_optional() {
        format!("{wrapped}.nullable")
    } else {
        wrapped
    }
}

/// The `KSerializer` for a variant's own subclass, which always carries the enclosing item's full
/// type parameter list (see [`variant_subclass`]): the reified `serializer<{subclass}>()` for a
/// non-generic item; `{subclass}.serializer(...)`, the plugin's own companion method, for a generic one.
fn kotlin_subclass_serializer_expr(subclass_name: &str, type_parameters: &[String]) -> String {
    if type_parameters.is_empty() {
        format!("serializer<{subclass_name}>()")
    } else {
        let arguments = type_parameters
            .iter()
            .map(|parameter| kotlin_serializer_param_name(parameter))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{subclass_name}.serializer({arguments})")
    }
}

/// The Kotlin type name for a Tuple-shaped field: queues a wrapper `data class` plus the
/// `KSerializer` that reads and writes it as a JSON array, once per shape in the item being
/// expanded. Monotonically numbered so two fields named alike in two different items never
/// collide. `walked` says the tuple is held where a `validate()` walks its positions, so a model
/// in one is read unjudged, which makes it a shape of its own.
fn tuple_wrapper_typename(elements: &[FieldDef], walked: bool) -> String {
    let declared_by = |element: &FieldDef| {
        if walked {
            read::position_typename(element)
        } else {
            kotlin_typename(element)
        }
    };
    let params = elements
        .iter()
        .enumerate()
        .map(|(slot, element)| format!("val slot{slot}: {}", declared_by(element)))
        .collect::<Vec<_>>()
        .join(", ");
    let read_by = |element: &FieldDef| {
        if walked {
            read::position_serializer(element)
        } else {
            kotlin_serializer_expr(element, &[])
        }
    };
    let encode_entries = elements
        .iter()
        .enumerate()
        .map(|(slot, element)| {
            format!(
                "add(output.json.encodeToJsonElement({}, value.slot{slot}))",
                read_by(element)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let decode_entries = elements
        .iter()
        .enumerate()
        .map(|(slot, element)| {
            format!(
                "input.json.decodeFromJsonElement({}, {})",
                read_by(element),
                preprocess::corrected(preprocess::of(element), &format!("array[{slot}]"))
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let shape = format!("{params}|{encode_entries}|{decode_entries}");
    if let Some(named) = KOTLIN_TUPLES.with(|tuples| {
        tuples
            .borrow()
            .iter()
            .find(|(declared, _)| *declared == shape)
            .map(|(_, named)| named.clone())
    }) {
        return named;
    }
    let index = KOTLIN_TUPLE_COUNTER.with(|counter| {
        let next = counter.get() + 1;
        counter.set(next);
        next
    });
    let wrapper_name = format!("KotlinTuple{index}");
    let serializer_name = format!("{wrapper_name}Serializer");
    let class_text = format!(
        "@Serializable(with = {serializer_name}::class) data class {wrapper_name}({params}) \
         object {serializer_name} : KSerializer<{wrapper_name}> {{ \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{wrapper_name}\"); \
         override fun serialize(encoder: Encoder, value: {wrapper_name}) {{ \
         val output = encoder as JsonEncoder; \
         output.encodeJsonElement(buildJsonArray {{ {encode_entries} }}) }}; \
         override fun deserialize(decoder: Decoder): {wrapper_name} {{ \
         val input = decoder as JsonDecoder; val array = input.decodeJsonElement().jsonArray; \
         return {wrapper_name}({decode_entries}) }} }}"
    );
    KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(class_text));
    KOTLIN_TUPLES.with(|tuples| {
        tuples.borrow_mut().push((shape, wrapper_name.clone()));
    });
    wrapper_name
}

/// The `@Serializable` data class a std range of `kind` is read and written through, declared
/// once per item over the type of its bounds: its members are the bounds serde writes, under their
/// keys, which the plugin reads and writes as that object.
fn range_class(kind: RangeKind) -> String {
    let members = kind
        .members()
        .iter()
        .map(|member| format!("val {member}: Bound"))
        .collect::<Vec<_>>()
        .join(", ");
    KOTLIN_RANGES.with(|ranges| {
        let mut written = ranges.borrow_mut();
        if let Some((_, named)) = written.iter().find(|(declared, _)| *declared == members) {
            return named.clone();
        }
        let index = KOTLIN_TUPLE_COUNTER.with(|counter| {
            let next = counter.get() + 1;
            counter.set(next);
            next
        });
        let named = format!("KotlinRange{index}");
        KOTLIN_TUPLE_AUX.with(|aux| {
            aux.borrow_mut().push(format!(
                "@Serializable data class {named}<Bound>({members})"
            ));
        });
        written.push((members, named.clone()));
        named
    })
}

/// The Kotlin type of a std range of `kind` whose bounds are typed `held`.
fn range_typename(kind: RangeKind, held: &str) -> String {
    format!("{}<{held}>", range_class(kind))
}

/// The `KSerializer` of a std range of `kind`: the plugin's for its class, handed `held`, the
/// serializer of its bounds.
fn range_serializer(kind: RangeKind, held: &str) -> String {
    format!("{}.serializer({held})", range_class(kind))
}

/// The sealed interface a `Result` of the arms `ok` and `err` is read and written through, declared
/// once per item for each shape, beside a class per arm and the serializer that reads and writes
/// the object whose one key names the arm. It is generic over the type of each arm that holds a
/// value; an arm of `()` is an object, written as `null`. Any other object is refused, as serde
/// refuses it.
fn result_class(ok: &FieldDef, err: &FieldDef) -> String {
    let shape = [ok.is_unit(), err.is_unit()];
    KOTLIN_RESULTS.with(|results| {
        let mut written = results.borrow_mut();
        if let Some((_, named)) = written.iter().find(|(declared, _)| *declared == shape) {
            return named.clone();
        }
        let index = KOTLIN_TUPLE_COUNTER.with(|counter| {
            let next = counter.get() + 1;
            counter.set(next);
            next
        });
        let named = format!("KotlinResult{index}");
        KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(result_declarations(&named, shape)));
        written.push((shape, named.clone()));
        named
    })
}

/// The declarations of the interface `named` of a `Result` whose arms hold `()` where `shape`
/// says so: the interface, a class per arm, and its serializer. The serializer reads the object
/// as the plugin reads a class, key by key, so a key no arm writes is refused as serde refuses it.
fn result_declarations(named: &str, shape: [bool; 2]) -> String {
    let held: Vec<&str> = RESULT_ARMS
        .into_iter()
        .zip(shape)
        .filter(|(_, unit)| !unit)
        .map(|(arm, _)| arm)
        .collect();
    let listed = |each: &dyn Fn(&str) -> String| {
        if held.is_empty() {
            String::new()
        } else {
            let written: Vec<String> = held.iter().map(|arm| each(arm)).collect();
            format!("<{}>", written.join(", "))
        }
    };
    let covariant = listed(&|arm| format!("out {arm}Value"));
    let invariant = listed(&|arm| format!("{arm}Value"));
    let serializer = format!("{named}Serializer");
    let refusal = "throw SerializationException(\"Not one key, Ok or Err, of a Result\")";
    let (mut arms, mut elements, mut writes, mut reads) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (index, (arm, unit)) in RESULT_ARMS.into_iter().zip(shape).enumerate() {
        // Each arm is an instance of the interface whatever the other arm holds.
        let arguments: Vec<String> = held
            .iter()
            .map(|other| {
                if *other == arm {
                    format!("{arm}Value")
                } else {
                    "Nothing".to_owned()
                }
            })
            .collect();
        let implemented = if arguments.is_empty() {
            named.to_owned()
        } else {
            format!("{named}<{}>", arguments.join(", "))
        };
        let read_by = if unit {
            "JsonNull.serializer()".to_owned()
        } else {
            format!("{}Serializer", arm.to_lowercase())
        };
        elements.push(format!(
            "element(\"{arm}\", {read_by}.descriptor, isOptional = true)"
        ));
        if unit {
            arms.push(format!("data object {named}{arm} : {implemented}"));
            writes.push(format!(
                "is {named}{arm} -> encodeSerializableElement(descriptor, {index}, {read_by}, JsonNull)"
            ));
            reads.push(format!(
                "{index} -> {{ decodeSerializableElement(descriptor, {index}, {read_by}); {named}{arm} }}"
            ));
        } else {
            arms.push(format!(
                "data class {named}{arm}<out {arm}Value>(val value: {arm}Value) : {implemented}"
            ));
            writes.push(format!(
                "is {named}{arm} -> encodeSerializableElement(descriptor, {index}, {read_by}, value.value)"
            ));
            reads.push(format!(
                "{index} -> {named}{arm}(decodeSerializableElement(descriptor, {index}, {read_by}))"
            ));
        }
    }
    let opened = if held.is_empty() {
        format!("object {serializer}")
    } else {
        let handed: Vec<String> = held
            .iter()
            .map(|arm| {
                format!(
                    "private val {}Serializer: KSerializer<{arm}Value>",
                    arm.to_lowercase()
                )
            })
            .collect();
        format!("class {serializer}{invariant}({})", handed.join(", "))
    };
    let own = format!("{named}{invariant}");
    format!(
        "@Serializable(with = {serializer}::class) sealed interface {named}{covariant} {} \
         {opened} : KSerializer<{own}> {{ \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{named}\") {{ {} }}; \
         override fun serialize(encoder: Encoder, value: {own}) = encoder.encodeStructure(descriptor) {{ \
         when (value) {{ {} }} }}; \
         override fun deserialize(decoder: Decoder): {own} = decoder.decodeStructure(descriptor) {{ \
         var read: {own}? = null; \
         while (true) {{ val index = decodeElementIndex(descriptor); \
         if (index == CompositeDecoder.DECODE_DONE) break; if (read != null) {refusal}; \
         read = when (index) {{ {}; else -> {refusal} }} }}; \
         read ?: {refusal} }} }}",
        arms.join(" "),
        elements.join("; "),
        writes.join("; "),
        reads.join("; "),
    )
}

/// The Kotlin type of a `Result` of the arms `ok` and `err`, each arm that holds a value typed as
/// `typed` answers.
fn result_typename(ok: &FieldDef, err: &FieldDef, typed: impl Fn(&FieldDef) -> String) -> String {
    let held: Vec<String> = [ok, err]
        .into_iter()
        .filter(|arm| !arm.is_unit())
        .map(typed)
        .collect();
    let class = result_class(ok, err);
    if held.is_empty() {
        class
    } else {
        format!("{class}<{}>", held.join(", "))
    }
}

/// The `KSerializer` of a `Result` of the arms `ok` and `err`, handed the serializer `read_by`
/// answers for each arm that holds a value.
fn result_serializer(
    ok: &FieldDef,
    err: &FieldDef,
    read_by: impl Fn(&FieldDef) -> String,
) -> String {
    let held: Vec<String> = [ok, err]
        .into_iter()
        .filter(|arm| !arm.is_unit())
        .map(read_by)
        .collect();
    let serializer = format!("{}Serializer", result_class(ok, err));
    if held.is_empty() {
        serializer
    } else {
        format!("{serializer}({})", held.join(", "))
    }
}

/// The `{ident}_kotlin` module ident an item's own `kotlin_definition()` publishes from.
fn kotlin_module_ident(rust_ident: &str, span: proc_macro2::Span) -> Ident {
    Ident::new(&format!("{}_kotlin", to_snake_case(rust_ident)), span)
}

/// The module `{ident}_kotlin` publishes `kotlin_definition()` from — never a direct inherent
/// `impl {ident}`, since a Rust type alias resolves to its target under the orphan/coherence rules.
/// The one choke point every dispatch path returns through, so it also drains [`KOTLIN_TUPLE_AUX`]
/// ahead of the source, and after it the serializers its preprocessed members are read through.
///
/// Where a member is read as a default, the module also declares what is handed the JSON of the
/// defaults, which `kotlin_definition()` writes in place of each placeholder, between the quotes
/// of a Kotlin string.
fn kotlin_module_tokens(
    rust_ident: &str,
    span: proc_macro2::Span,
    kotlin_source: &str,
    defaults: &SerdeDefaults,
) -> TokenStream {
    KOTLIN_RANGES.with(|ranges| ranges.borrow_mut().clear());
    KOTLIN_RESULTS.with(|results| results.borrow_mut().clear());
    KOTLIN_TUPLES.with(|tuples| tuples.borrow_mut().clear());
    let full_source = KOTLIN_TUPLE_AUX
        .with(RefCell::take)
        .into_iter()
        .chain(once(kotlin_source.to_owned()))
        .chain(preprocess::serializers())
        .collect::<Vec<_>>()
        .join(" ");
    let module_ident = kotlin_module_ident(rust_ident, span);
    let holder = defaults.holder();
    if holder.is_empty() {
        return quote! {
            pub mod #module_ident {
                pub fn kotlin_definition() -> ::std::string::String {
                    #full_source.to_owned()
                }
            }
        };
    }
    let written = proc_macro2::Ident::new("written", proc_macro2::Span::call_site());
    let replaced = substituted(
        &quote! { SerdeDefaults },
        &written,
        &[('\\', "\\\\"), ('"', "\\\""), ('$', "\\$")],
    );
    let values = defaults.values_impl(&module_ident);
    quote! {
        pub mod #module_ident {
            #holder

            pub fn kotlin_definition() -> ::std::string::String {
                let mut #written = ::std::string::String::from(#full_source);
                #replaced
                #written
            }
        }

        #values
    }
}

/// Whether `bounds` names `Serialize`.
fn names_serialize(bounds: &Punctuated<TypeParamBound, Token![+]>) -> bool {
    bounds.iter().any(|bound| {
        matches!(
            bound,
            TypeParamBound::Trait(held)
                if held.path.segments.last().is_some_and(|last| last.ident == "Serialize")
        )
    })
}

/// The type parameters `generics` declares `Serialize` for, inline or in its `where` clause.
fn serialized_parameters(generics: &syn::Generics) -> Vec<String> {
    let mut declared: Vec<String> = generics
        .type_params()
        .filter(|parameter| names_serialize(&parameter.bounds))
        .map(|parameter| parameter.ident.to_string())
        .collect();
    let predicates = generics
        .where_clause
        .iter()
        .flat_map(|clause| &clause.predicates);
    for predicate in predicates {
        if let WherePredicate::Type(held) = predicate
            && let Type::Path(named) = &held.bounded_ty
            && let Some(parameter) = named.path.get_ident()
            && names_serialize(&held.bounds)
        {
            declared.push(parameter.to_string());
        }
    }
    declared
}

/// The Kotlin expression `validate()` reads the property `rust_name` from: the property itself,
/// through `this` where it carries the name of the list the method fills.
fn property_read(rust_name: &str) -> String {
    let property = kotlin_property_name(rust_name);
    if property == "violations" {
        format!("this.{property}")
    } else {
        property
    }
}

/// `place` with `step` written after it, as the text of a Kotlin string.
fn place_under(place: Option<&str>, step: &str) -> String {
    let written = kotlin_template_text(step);
    place.map_or_else(
        || written.clone(),
        |under| format!("{}.{written}", kotlin_template_text(under)),
    )
}

/// What each of `fields` adds to `rules`: a member under the key serde writes it at, after
/// `under`, and a flattened one, which writes no key, under `under` alone.
fn named_member_rules(rules: &mut Rules<'_>, fields: &[KotlinField], under: Option<&str>) {
    for field in fields {
        let place = if field.flatten {
            under.map(kotlin_template_text)
        } else {
            Some(place_under(under, &field.wire_name))
        };
        rules.member(&Member {
            def: &field.field_def,
            place,
            read: property_read(&field.rust_name),
        });
    }
}

/// What each of `slots` adds to `rules`, read from `holder` and under its position after `under`:
/// a lone slot is the value serde writes, and adds no position.
fn slot_rules(rules: &mut Rules<'_>, slots: &[FieldDef], holder: &str, under: Option<&str>) {
    for (at, slot) in slots.iter().enumerate() {
        let place = if slots.len() == 1 {
            under.map(kotlin_template_text)
        } else {
            Some(place_under(under, &at.to_string()))
        };
        rules.member(&Member {
            def: slot,
            place,
            read: format!("{holder}slot{at}"),
        });
    }
}

/// The `typealias {rust_ident}{generic_params} = {export_name}{generic_params};` a renamed item
/// re-publishes under its own Rust ident — the alias a reference declared above the rename still
/// resolves through. Empty for an item that already publishes under its own ident.
fn ident_typealias(rust_ident: &str, export_name: &str, generic_params: &str) -> String {
    if rust_ident == export_name {
        String::new()
    } else {
        format!(" typealias {rust_ident}{generic_params} = {export_name}{generic_params};")
    }
}

/// `<T, U>` for the type parameters `generics` declares, or the empty string for none.
fn kotlin_generic_params(generics: &syn::Generics) -> String {
    let parameters = type_parameters_in_scope(generics);
    if parameters.is_empty() {
        String::new()
    } else {
        format!("<{}>", parameters.join(", "))
    }
}

/// `<out T, out U>` for a sealed base's own declaration — never for a use site, where Kotlin
/// refuses a variance annotation. Covariant so a unit variant's `data object` may implement the
/// base at [`kotlin_nothing_generic_args`] regardless of what fills `T`.
fn kotlin_out_generic_params(generics: &syn::Generics) -> String {
    let parameters = type_parameters_in_scope(generics);
    if parameters.is_empty() {
        String::new()
    } else {
        let out = parameters
            .iter()
            .map(|parameter| format!("out {parameter}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("<{out}>")
    }
}

/// `<Nothing, Nothing>` for the type parameters `generics` declares, or the empty string for none —
/// the filling a unit variant's `data object` implements the sealed base at, one `Nothing` per
/// parameter, since the object binds none of its own.
fn kotlin_nothing_generic_args(generics: &syn::Generics) -> String {
    let parameters = type_parameters_in_scope(generics);
    if parameters.is_empty() {
        String::new()
    } else {
        format!("<{}>", vec!["Nothing"; parameters.len()].join(", "))
    }
}

/// One property declaration inside a constructor's parameter list: `@SerialName("...")` only where
/// the wire spelling differs from the Kotlin property, then `val name: Type`, then `= null` for a
/// field whose key may be absent — a bare `Option<T>` without `#[model_schema_prop(nullable)]`.
/// `plugin` is the class where the plugin writes its serializer, which reads the member by the
/// serializer the property names for it: a model's unjudged one, or the one that calls the
/// member's own `preprocess`. Without it, the member is declared as the emitter's own serializer
/// reads it ([`read::declared_typename`]).
fn property_declaration(field: &KotlinField, plugin: Option<&Plugin<'_>>) -> String {
    let prop_name = kotlin_property_name(&field.rust_name);
    let annotation = if kotlin_bare(&prop_name) == field.wire_name {
        String::new()
    } else {
        format!("@SerialName(\"{}\") ", field.wire_name)
    };
    let (unjudged, ty) = plugin.map_or_else(
        || (String::new(), read::declared_typename(&field.field_def)),
        |reader| reader.declared(&field.rust_name, &field.field_def),
    );
    let default = field.default_at.as_ref().map_or_else(
        || {
            if field.field_def.is_optional() && !is_nullable_flag(&field.field_def) {
                " = null".to_owned()
            } else {
                String::new()
            }
        },
        // The plugin calls this for an absent key: the default, read as a payload is.
        |at| {
            if field.empty_default {
                // An initializer reaches no serializer of the class's parameters to read `[]` with.
                let empty = if field.field_def.array_depth == 0
                    && matches!(field.field_def.field_type, FieldDefType::Map(..))
                {
                    "emptyMap()"
                } else {
                    "emptyList()"
                };
                format!(" = {empty}")
            } else {
                format!(
                    " = Json.decodeFromString({}, \"{at}\")",
                    read::member_serializer(&field.field_def, &[])
                )
            }
        },
    );
    format!("{annotation}{unjudged}val {prop_name}: {ty}{default}")
}

/// The constructor parameter list a set of [`KotlinField`]s earns, joined by `, `.
fn data_class_params(fields: &[KotlinField], plugin: Option<&Plugin<'_>>) -> String {
    fields
        .iter()
        .map(|field| property_declaration(field, plugin))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `field`'s own type with any outer `Option` stripped — the inner type a flatten field's plain
/// serializer and descriptor read regardless of whether the field itself is `Option<...>`.
fn flatten_base_field(field_def: &FieldDef) -> FieldDef {
    let mut base = field_def.clone();
    base.nullable_levels.clear();
    base
}

/// One non-flattened field's own `serialize`/`deserialize` statements and constructor argument —
/// the branch [`flatten_plan`] takes for every field that is not itself `#[serde(flatten)]`. The
/// payload under the field's key is read as the field's own `preprocess` corrects it. An optional
/// member's key is left out where it holds no value, as serde leaves it out, but under `nullable`,
/// and a member serde never writes has no `serialize` statement.
fn own_field_plan(
    field: &KotlinField,
    prop: &str,
    type_parameters: &[String],
) -> (Option<String>, String) {
    let field_serializer = read::member_serializer(&field.field_def, type_parameters);
    let put = format!(
        "put(\"{}\", output.json.encodeToJsonElement({field_serializer}, value.{prop}))",
        field.wire_name
    );
    let serialize = if field.never_written {
        None
    } else if field.field_def.is_optional() && !is_nullable_flag(&field.field_def) {
        Some(format!("if (value.{prop} != null) {put}"))
    } else {
        Some(put)
    };
    let functions = preprocess::of(&field.field_def);
    let decode_expr = if field.field_def.is_optional() {
        format!(
            "obj[\"{}\"]?.let {{ input.json.decodeFromJsonElement({field_serializer}, {}) }}",
            field.wire_name,
            preprocess::corrected(functions, "it")
        )
    } else if let Some(at) = &field.default_at {
        format!(
            "obj[\"{}\"]?.let {{ input.json.decodeFromJsonElement({field_serializer}, {}) }} \
             ?: input.json.decodeFromString({field_serializer}, \"{at}\")",
            field.wire_name,
            preprocess::corrected(functions, "it")
        )
    } else {
        format!(
            "input.json.decodeFromJsonElement({field_serializer}, {})",
            preprocess::corrected(functions, &format!("obj.getValue(\"{}\")", field.wire_name))
        )
    };
    (serialize, format!("val {prop} = {decode_expr}"))
}

/// One flattened *struct* (or `Option<Struct>`) field's own statements: the `serialize` merge, the
/// `deserialize` read (leniently, off `rest`, the object serde hands the member, as the member's
/// own `preprocess` then corrects it, and `null` where an optional field's own keys are all absent
/// from `rest`), and the keys it takes, which the members flattened after it are handed the object
/// without.
fn flatten_struct_field_plan(
    field: &KotlinField,
    prop: &str,
    type_parameters: &[String],
    rest: &str,
) -> (String, String, String) {
    let base = flatten_base_field(&field.field_def);
    let inner_serializer = read::member_serializer(&base, type_parameters);
    let serialize = flattened_merge(field, prop, &inner_serializer);
    let keys_ident = format!("{}Keys", kotlin_bare(prop));
    let key_read = format!(
        "val {keys_ident} = {}",
        flattened_struct_keys(&base, &inner_serializer)
    );
    let handed = preprocess::corrected(preprocess::of(&field.field_def), rest);
    let decode = if field.field_def.is_optional() {
        format!(
            "val {prop} = if ({rest}.keys.any {{ it in {keys_ident} }}) lenient.decodeFromJsonElement({inner_serializer}, {handed}) else null"
        )
    } else {
        format!("val {prop} = lenient.decodeFromJsonElement({inner_serializer}, {handed})")
    };
    (serialize, format!("{key_read}; {decode}"), keys_ident)
}

/// The `serialize` statement that merges a flattened member's own keys into its holder's object,
/// read by `inner_serializer`; an optional member that holds no value merges none. A unit variant
/// an enum writes as its bare name is merged as serde merges it: its name over `null`.
fn flattened_merge(field: &KotlinField, prop: &str, inner_serializer: &str) -> String {
    let merged = |held: &str| {
        let written = format!("output.json.encodeToJsonElement({inner_serializer}, {held})");
        if writes_a_bare_name(&field.field_def) {
            format!(
                "when (val written = {written}) {{ is JsonPrimitive -> put(written.content, JsonNull); else -> written.jsonObject.forEach {{ (k, v) -> put(k, v) }} }}"
            )
        } else {
            format!("{written}.jsonObject.forEach {{ (k, v) -> put(k, v) }}")
        }
    };
    if field.field_def.is_optional() {
        let held = merged("flattened");
        format!("value.{prop}?.let {{ flattened -> {held} }}")
    } else {
        merged(&format!("value.{prop}"))
    }
}

/// One flattened externally tagged enum's own statements, as [`flatten_struct_field_plan`] writes
/// a struct's: the enum `read_under` names is read from the first entry of `rest` under one of the
/// names it holds, which serde refuses the holder without and an optional enum reads no value
/// without, and the key of that entry, which the members flattened after it are handed the object
/// without.
fn flatten_enum_field_plan(
    field: &KotlinField,
    prop: &str,
    type_parameters: &[String],
    rest: &str,
    (enum_name, names): (&str, &[String]),
) -> (String, String, String) {
    let base = flatten_base_field(&field.field_def);
    let inner_serializer = read::member_serializer(&base, type_parameters);
    let serialize = flattened_merge(field, prop, &inner_serializer);
    let entry_ident = format!("{}Entry", kotlin_bare(prop));
    let listed = names
        .iter()
        .map(|name| format!("\"{}\"", kotlin_template_text(name)))
        .collect::<Vec<_>>()
        .join(", ");
    let entry_read = format!("{rest}.entries.firstOrNull {{ it.key in setOf({listed}) }}");
    let (decode, entry_taken) = if field.field_def.is_optional() {
        (
            format!(
                "val {entry_ident} = {entry_read}; val {prop} = {entry_ident}?.let {{ lenient.decodeFromJsonElement({inner_serializer}, JsonObject(mapOf(it.toPair()))) }}"
            ),
            format!("setOfNotNull({entry_ident}?.key)"),
        )
    } else {
        (
            format!(
                "val {entry_ident} = {entry_read} ?: throw SerializationException(\"no variant of enum {enum_name} found in flattened data\"); val {prop} = lenient.decodeFromJsonElement({inner_serializer}, JsonObject(mapOf({entry_ident}.toPair())))"
            ),
            format!("{entry_ident}.key"),
        )
    };
    (serialize, decode, entry_taken)
}

/// One flattened map field's own statements: the `serialize` merge and the `deserialize` read,
/// over every key of the object, as `functions`, the field's own `preprocess`, correct it. A map
/// flattened beside another member is refused, so serde hands the map every key of the object.
/// An optional map writes no entry where it holds no value, and is read as `null`, as serde reads
/// it, wherever an entry does not read as the map's value; under a function, where the function
/// answers `null`.
fn flatten_map_field_plan(
    prop: &str,
    optional: bool,
    value_serializer: &str,
    functions: &[FunctionName],
) -> (String, String) {
    let held = if optional { "?" } else { "" };
    let serialize = format!(
        "value.{prop}{held}.forEach {{ (k, v) -> put(k, output.json.encodeToJsonElement({value_serializer}, v)) }}"
    );
    let entries = |object: &str| {
        format!(
            "{object}.mapValues {{ (_, v) -> input.json.decodeFromJsonElement({value_serializer}, v) }}"
        )
    };
    let read = if functions.is_empty() {
        let every = entries("obj");
        if optional {
            format!("runCatching {{ {every} }}.getOrNull()")
        } else {
            every
        }
    } else {
        let corrected = preprocess::corrected(functions, "obj");
        if optional {
            format!(
                "{corrected}.let {{ if (it is JsonNull) null else {} }}",
                entries("it.jsonObject")
            )
        } else {
            entries(&format!("{corrected}.jsonObject"))
        }
    };
    (serialize, format!("val {prop} = {read}"))
}

fn flatten_plan(fields: &[KotlinField], type_parameters: &[String]) -> FlattenPlan {
    let mut plan = FlattenPlan {
        ctor_args: Vec::new(),
        deserialize_stmts: Vec::new(),
        needs_lenient: false,
        serialize_stmts: Vec::new(),
    };
    let own_wire_keys: Vec<String> = fields
        .iter()
        .filter(|field| !field.flatten)
        .map(|field| format!("\"{}\"", field.wire_name))
        .collect();
    // Kotlin infers no element type from an empty `setOf()`.
    let own_literal = if own_wire_keys.is_empty() {
        "setOf<String>()".to_owned()
    } else {
        format!("setOf({})", own_wire_keys.join(", "))
    };
    // What serde hands a flattened member: the object without the keys the holder's own members
    // take, and without those each member flattened ahead of it took as it was read.
    let mut taken = if own_wire_keys.is_empty() {
        String::new()
    } else {
        format!(" - {own_literal}")
    };

    for field in fields {
        let prop = kotlin_property_name(&field.rust_name);
        if !field.flatten {
            let (serialize, decode) = own_field_plan(field, &prop, type_parameters);
            plan.serialize_stmts.extend(serialize);
            plan.deserialize_stmts.push(decode);
        } else if let FieldDefType::Map(_, map_value) = &field.field_def.field_type {
            let value_serializer =
                read::held_serializer(&field.field_def, map_value, type_parameters);
            let (serialize, decode) = flatten_map_field_plan(
                &prop,
                field.field_def.is_optional(),
                &value_serializer,
                preprocess::of(&field.field_def),
            );
            plan.serialize_stmts.push(serialize);
            plan.deserialize_stmts.push(decode);
        } else {
            plan.needs_lenient = true;
            let rest = if taken.is_empty() {
                "obj".to_owned()
            } else {
                format!("JsonObject(obj{taken})")
            };
            if let Some((enum_name, names)) = flattened_variant_names(&field.field_def) {
                let read_under = (enum_name, names.as_slice());
                let (serialize, decode, entry_taken) =
                    flatten_enum_field_plan(field, &prop, type_parameters, &rest, read_under);
                plan.serialize_stmts.push(serialize);
                plan.deserialize_stmts.push(decode);
                let _ = write!(taken, " - {entry_taken}");
            } else {
                let (serialize, decode, keys_ident) =
                    flatten_struct_field_plan(field, &prop, type_parameters, &rest);
                plan.serialize_stmts.push(serialize);
                plan.deserialize_stmts.push(decode);
                if !preprocess::takes_no_key(&field.field_def) {
                    let _ = write!(taken, " - {keys_ident}");
                }
            }
        }
        plan.ctor_args.push(format!("{prop} = {prop}"));
    }

    plan
}

/// The unjudged `KSerializer` a struct with one or more `#[serde(flatten)]` fields earns, since
/// `kotlinx.serialization` has no annotation for flatten: it merges each flattened value's own
/// keys into the enclosing object, tolerant of the other fields' own keys on decode. A struct with
/// a member serde never writes earns it too: `kotlinx.serialization` has no annotation for a
/// member read and never written. `own` is the type's `preprocess`, which corrects the object
/// every member is read from, and each alias of `renamed` is read in that object as the name it
/// stands for.
fn flatten_merging_serializer(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    fields: &[KotlinField],
    own_tag: Option<&(String, String)>,
    own: &[FunctionName],
    renamed: &[(String, String)],
) -> String {
    let self_type = format!("{export_name}{generic_params}");
    let serializer_head = read::written_head(export_name, type_parameters);
    let mut plan = flatten_plan(fields, type_parameters);
    // A struct's own tag is written ahead of everything merged into the object.
    if let Some((key, named)) = own_tag {
        plan.serialize_stmts
            .insert(0, format!("put(\"{key}\", \"{named}\")"));
    }

    let lenient_field = if plan.needs_lenient {
        "private val lenient = Json { ignoreUnknownKeys = true }; "
    } else {
        ""
    };
    let serialize_body = format!(
        "override fun serialize(encoder: Encoder, value: {self_type}) {{ \
         val output = encoder as JsonEncoder; \
         output.encodeJsonElement(buildJsonObject {{ {} }}) }}",
        plan.serialize_stmts.join("; ")
    );
    let deserialize_body = format!(
        "override fun deserialize(decoder: Decoder): {self_type} {{ \
         val input = decoder as JsonDecoder; val obj = {}.jsonObject; {}; \
         return {export_name}({}) }}",
        aliases::keys_named(
            &preprocess::corrected(own, "input.decodeJsonElement()"),
            renamed
        ),
        plan.deserialize_stmts.join("; "),
        plan.ctor_args.join(", "),
    );
    format!(
        "{serializer_head} : KSerializer<{self_type}> {{ \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{export_name}\"); \
         {lenient_field}{serialize_body}; {deserialize_body} }}"
    )
}

/// The unjudged serializer of a type the plugin writes a serializer for: the plugin's own, behind
/// `correction`, the Kotlin expression [`ELEMENT`] is read as, where that is not [`ELEMENT`]
/// itself.
fn kept_unjudged(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    correction: &str,
) -> String {
    if correction == ELEMENT {
        read::generated(export_name, generic_params, type_parameters)
    } else {
        read::transforming(export_name, generic_params, type_parameters, correction)
    }
}

/// Notes the keys the struct written under `rust_ident` takes off its holder's object when it is
/// flattened, `None` where the `elementNames` of its serializer's descriptor are those keys.
fn note_taken_keys(rust_ident: &str, taken: Option<Vec<String>>) {
    KOTLIN_TAKEN_KEYS.with(|noted| {
        let mut recorded = noted.borrow_mut();
        if let Some(keys) = taken {
            recorded.insert(rust_ident.to_owned(), keys);
        } else {
            recorded.remove(rust_ident);
        }
    });
}

/// The Kotlin tokens a named-field struct earns: a `data class` with its `validate()` and its two
/// serializers, plus a `typealias` under its own Rust ident when `name = "..."` moved its
/// published name elsewhere. The plugin's serializer is the unjudged one, and a struct carrying
/// one or more `#[serde(flatten)]` fields, or a member serde never writes and the plugin would,
/// earns [a merging one](flatten_merging_serializer) in its place.
fn struct_kotlin_tokens(item_struct: &ItemStruct, args: &KotlinArgs<'_>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    if let Some(held) = brand_field(item_struct) {
        return value_class_tokens(item_struct, held, args);
    }
    if matches!(item_struct.fields, Fields::Unnamed(_)) {
        return tuple_struct_kotlin_tokens(item_struct, args);
    }

    let rust_ident = item_struct.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_kotlin_name(&rust_ident, &export_name);
    validate::note_model(&rust_ident);

    let rule = container_rename_rule(&item_struct.attrs);
    let defaults = SerdeDefaults::of(item_struct);
    let fields = collect_kotlin_fields(
        &item_struct.fields,
        rule,
        &type_parameters,
        args,
        &|field| defaults.at(None, field),
    );
    let generic_params = kotlin_generic_params(&item_struct.generics);
    let alias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let written_by_hand = fields
        .iter()
        .any(|field| field.flatten || field.never_written);
    let renamed = aliases::of_members(&fields);
    // The descriptor of the plugin's serializer names no alias.
    note_taken_keys(
        &rust_ident,
        (written_by_hand || !renamed.is_empty())
            .then(|| keys_taken_when_flattened(&item_struct.fields, rule, args.preprocess)),
    );
    let handed_on = fields
        .iter()
        .any(|field| field.flatten && preprocess::takes_no_key(&field.field_def));
    preprocess::note(&rust_ident, handed_on || !args.preprocess.is_empty());
    let plugin = Plugin {
        owner: &export_name,
        type_parameters: &type_parameters,
    };
    let body = if matches!(item_struct.fields, Fields::Unit) {
        // A unit struct is Kotlin's own no-data type: `object`, not `class` below.
        format!("object {export_name}{generic_params}")
    } else if fields.is_empty() {
        format!("class {export_name}{generic_params}")
    } else {
        format!(
            "data class {export_name}{generic_params}({})",
            data_class_params(&fields, (!written_by_hand).then_some(&plugin))
        )
    };
    let serializable = serialized_parameters(&item_struct.generics);
    let mut rules = Rules::of(&serializable);
    named_member_rules(&mut rules, &fields, None);
    rules.own(args.check);
    let validated = rules.written("");
    let own_tag = struct_tag(item_struct);
    let kotlin_source = if written_by_hand {
        let unjudged = flatten_merging_serializer(
            &export_name,
            &generic_params,
            &type_parameters,
            &fields,
            own_tag.as_ref(),
            args.preprocess,
            &renamed,
        );
        format!(
            "@Serializable(with = {export_name}Serializer::class) {body} {{ {validated} }} \
             {unjudged} {}{alias}",
            read::judged(&export_name, &generic_params, &type_parameters, true)
        )
    } else {
        // A struct's own tag: a property outside the constructor, always written and holding the
        // struct's name unless a payload says otherwise, as serde writes it and reads past it.
        let tag = own_tag.map_or_else(String::new, |(key, named)| {
            format!(
                "@SerialName(\"{key}\") @EncodeDefault val kotlinSchemaTag: String = \"{named}\"; "
            )
        });
        format!(
            "{} {body} {{ {tag}{validated} }} {} {}{alias}",
            read::kept(&export_name),
            kept_unjudged(
                &export_name,
                &generic_params,
                &type_parameters,
                &preprocess::keyed(
                    &aliases::keys_named(
                        &preprocess::corrected(args.preprocess, ELEMENT),
                        &renamed
                    ),
                    &plugin.keyed(&fields)
                )
            ),
            read::judged(&export_name, &generic_params, &type_parameters, false)
        )
    };

    kotlin_module_tokens(
        &rust_ident,
        item_struct.ident.span(),
        &kotlin_source,
        &defaults,
    )
}

/// The type a brand that holds one of its own parameters bare declares for it, as the bounds of
/// the brand are run at: the `default_types` entry, and a `String` where none is written.
fn declared_default(value_field: &FieldDef, args: &KotlinArgs<'_>) -> Option<FieldDef> {
    let FieldDefType::TypeParam(parameter) = &value_field.field_type else {
        return None;
    };
    if value_field.is_list() || value_field.is_optional() {
        return None;
    }
    let declared = args
        .default_types
        .iter()
        .find(|(named, _)| named == parameter)
        .map_or_else(
            || get_field_def("value", &syn::parse_quote!(String), ""),
            |(_, ty)| get_field_def("value", ty, ""),
        );
    Some(declared)
}

/// The Kotlin tokens for a brand, which carries no shape of its own on the wire beyond the one
/// value it holds: a `@JvmInline value class`, serialized identically to that value alone, whose
/// `validate()` runs the brand's own bounds and checks, and its two serializers.
fn value_class_tokens(
    item_struct: &ItemStruct,
    held: &Field,
    args: &KotlinArgs<'_>,
) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let value_field = brand_value_field(held, &type_parameters, args);
    let rust_ident = item_struct.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_kotlin_name(&rust_ident, &export_name);

    let generic_params = kotlin_generic_params(&item_struct.generics);
    let value_type = kotlin_typename(&value_field);
    let alias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let serializable = serialized_parameters(&item_struct.generics);
    let mut rules = Rules::of(&serializable);
    rules.brand(
        &Member {
            def: &value_field,
            place: None,
            read: "value".to_owned(),
        },
        declared_default(&value_field, args).as_ref(),
    );
    rules.own(args.check);
    let text = rules.text_member(&value_field);
    validate::note_brand(&rust_ident, text.is_some(), rules.bounded());
    let answered = text.map_or_else(String::new, |member| format!("{member}; "));
    let validated = rules.written("");
    let unjudged = if args.preprocess.is_empty() || type_parameters.is_empty() {
        kept_unjudged(
            &export_name,
            &generic_params,
            &type_parameters,
            &preprocess::corrected(args.preprocess, ELEMENT),
        )
    } else {
        let correction = preprocess::corrected(args.preprocess, "input.decodeJsonElement()");
        read::transforming_inline(&export_name, &generic_params, &type_parameters, &correction)
    };
    let kotlin_source = format!(
        "{} value class {export_name}{generic_params}(val value: {value_type}) {{ \
         {answered}{validated} }} {unjudged} {}{alias}",
        read::kept_inline(&export_name),
        read::judged(&export_name, &generic_params, &type_parameters, false)
    );

    kotlin_module_tokens(
        &rust_ident,
        item_struct.ident.span(),
        &kotlin_source,
        &SerdeDefaults::none(),
    )
}

/// The Kotlin tokens for a non-branded tuple struct: a `data class` over one property per slot,
/// read unjudged by the array `KSerializer` a Tuple-shaped field earns — the struct's own name
/// stands in for what [`tuple_wrapper_typename`] would otherwise synthesize — and judged by its
/// own.
fn tuple_struct_kotlin_tokens(item_struct: &ItemStruct, args: &KotlinArgs<'_>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let Fields::Unnamed(unnamed) = &item_struct.fields else {
        return TokenStream::new();
    };
    let rust_ident = item_struct.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_kotlin_name(&rust_ident, &export_name);
    validate::note_model(&rust_ident);

    let slots: Vec<FieldDef> = unnamed
        .unnamed
        .iter()
        .filter(|slot| !parse_serde_key_omission(&slot.attrs).absent_from_wire())
        .map(|slot| {
            let mut field_def = field_def_with_prop_meta("slot", slot, args);
            field_def.erase_type_parameters(&type_parameters);
            field_def
        })
        .collect();

    let generic_params = kotlin_generic_params(&item_struct.generics);
    let serializer_name = format!("{export_name}Serializer");
    let serializer_head = read::written_head(&export_name, &type_parameters);
    let params = slots
        .iter()
        .enumerate()
        .map(|(slot, element)| format!("val slot{slot}: {}", read::declared_typename(element)))
        .collect::<Vec<_>>()
        .join(", ");
    let encode_entries = slots
        .iter()
        .enumerate()
        .map(|(slot, element)| {
            format!(
                "add(output.json.encodeToJsonElement({}, value.slot{slot}))",
                read::member_serializer(element, &type_parameters)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let decode_entries = slots
        .iter()
        .enumerate()
        .map(|(slot, element)| {
            format!(
                "input.json.decodeFromJsonElement({}, {})",
                read::member_serializer(element, &type_parameters),
                preprocess::corrected(preprocess::of(element), &format!("array[{slot}]"))
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let payload = preprocess::corrected(args.preprocess, "input.decodeJsonElement()");
    let serializable = serialized_parameters(&item_struct.generics);
    let mut rules = Rules::of(&serializable);
    slot_rules(&mut rules, &slots, "", None);
    rules.own(args.check);
    let validated = rules.written("");
    let alias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let kotlin_source = format!(
        "@Serializable(with = {serializer_name}::class) \
         data class {export_name}{generic_params}({params}) {{ {validated} }} \
         {serializer_head} : KSerializer<{export_name}{generic_params}> {{ \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{export_name}\"); \
         override fun serialize(encoder: Encoder, value: {export_name}{generic_params}) {{ \
         val output = encoder as JsonEncoder; \
         output.encodeJsonElement(buildJsonArray {{ {encode_entries} }}) }}; \
         override fun deserialize(decoder: Decoder): {export_name}{generic_params} {{ \
         val input = decoder as JsonDecoder; val array = {payload}.jsonArray; \
         return {export_name}({decode_entries}) }} }} {}{alias}",
        read::judged(&export_name, &generic_params, &type_parameters, true)
    );

    kotlin_module_tokens(
        &rust_ident,
        item_struct.ident.span(),
        &kotlin_source,
        &SerdeDefaults::none(),
    )
}

/// The Kotlin tokens a `type X = ...;` alias earns: a `typealias` pointing at the target's own
/// rendering — never a wrapper of its own, unlike Dart (which has no bare structural alias to
/// point at): Kotlin's `typealias` is exactly that spelling.
fn alias_kotlin_tokens(item_type: &ItemType, name_override: Option<&str>) -> TokenStream {
    let rust_ident = item_type.ident.to_string();
    let export_name = compute_alias_export_name(&rust_ident, name_override);
    register_kotlin_name(&rust_ident, &export_name);

    let type_parameters = type_parameters_in_scope(&item_type.generics);
    let mut target = get_field_def(&export_name, &item_type.ty, "");
    target.erase_type_parameters(&type_parameters);
    validate::note_alias(&rust_ident, &target);
    let generic_params = kotlin_generic_params(&item_type.generics);
    let target_type = kotlin_typename(&target);
    let alias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let kotlin_source = format!("typealias {export_name}{generic_params} = {target_type};{alias}");

    kotlin_module_tokens(
        &rust_ident,
        item_type.ident.span(),
        &kotlin_source,
        &SerdeDefaults::none(),
    )
}

/// The Kotlin tokens an enum earns: an `enum class` for an all-unit, untagged-by-default
/// declaration; a `sealed interface` for every tagged or untagged shape otherwise — mirroring
/// exactly the shape `process_enum` itself dispatches a declaration to.
fn enum_kotlin_tokens(item_enum: &ItemEnum, args: &KotlinArgs<'_>) -> TokenStream {
    let rust_ident = item_enum.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_kotlin_name(&rust_ident, &export_name);
    let tag_attrs = enum_tag_attrs(&item_enum.attrs);
    let writes_bare_variant_names =
        tag_attrs.tag.is_none() && tag_attrs.content.is_none() && !tag_attrs.untagged;
    let defaults = SerdeDefaults::of_enum(item_enum);
    let kotlin_source = if is_plain_enum(item_enum) && writes_bare_variant_names {
        validate::note_plain_enum(&rust_ident);
        plain_enum_kotlin_source(item_enum, &export_name, args)
    } else {
        validate::note_model(&rust_ident);
        preprocess::note(&rust_ident, !args.preprocess.is_empty());
        let names = variant_names_when_flattened(item_enum, args.preprocess);
        KOTLIN_VARIANT_NAMES.with(|noted| {
            noted.borrow_mut().insert(rust_ident.clone(), names);
        });
        let bare_named = writes_bare_variant_names
            && item_enum
                .variants
                .iter()
                .any(|variant| classify_variant(variant) == VariantKind::Unit);
        if bare_named {
            KOTLIN_BARE_NAMED.with(|noted| {
                noted.borrow_mut().insert(rust_ident.clone());
            });
        }
        if tag_attrs.untagged {
            untagged_enum_kotlin_source(item_enum, &export_name, &tag_attrs, args, &defaults)
        } else {
            dispatched_tagged_enum_kotlin_source(
                item_enum,
                &export_name,
                &tag_attrs,
                args,
                &defaults,
            )
        }
    };
    kotlin_module_tokens(
        &rust_ident,
        item_enum.ident.span(),
        &kotlin_source,
        &defaults,
    )
}

/// The body of a variant's own subclass: what [`variant_rules`] writes for it.
fn variant_body(
    payload: &VariantPayload,
    tagging: Tagging<'_>,
    wire_tag: &str,
    serializable: &[String],
    args: &KotlinArgs<'_>,
) -> String {
    let mut rules = Rules::of(serializable);
    variant_rules(&mut rules, payload, tagging, wire_tag, args);
    format!("{{ {} }}", rules.written("override "))
}

/// What a variant adds to `rules`: the members of its payload, each under the steps serde writes
/// on the way to it, then the enum's own checks.
fn variant_rules(
    rules: &mut Rules<'_>,
    payload: &VariantPayload,
    tagging: Tagging<'_>,
    wire_tag: &str,
    args: &KotlinArgs<'_>,
) {
    let under = match tagging {
        Tagging::Adjacent(content) => Some(content),
        Tagging::External => Some(wire_tag),
        Tagging::Unwrapped => None,
    };
    match payload {
        VariantPayload::Unit => {}
        VariantPayload::Named(fields) => named_member_rules(rules, fields, under),
        VariantPayload::Value(field_def) => {
            if let FieldDefType::Tuple(slots) = &field_def.field_type {
                slot_rules(rules, slots, "value.", under);
            } else {
                rules.member(&Member {
                    def: field_def,
                    place: under.map(kotlin_template_text),
                    read: "value".to_owned(),
                });
            }
        }
    }
    rules.own(args.check);
}

/// One tagged/untagged variant's payload — see [`VariantPayload`]. A `TupleMultiple` payload is
/// folded into one `Tuple` [`FieldDef`], matching how a slot list renders everywhere else in this
/// module. A member is read as the default `defaults` holds for it in this variant.
fn variant_payload(
    variant: &Variant,
    field_rule: RenameRule,
    type_parameters: &[String],
    args: &KotlinArgs<'_>,
    defaults: &SerdeDefaults,
) -> VariantPayload {
    let variant_name = variant.ident.to_string();
    match classify_variant(variant) {
        VariantKind::Unit => VariantPayload::Unit,
        VariantKind::Named => VariantPayload::Named(collect_kotlin_fields(
            &variant.fields,
            field_rule,
            type_parameters,
            args,
            &|field| defaults.at(Some(&variant_name), field),
        )),
        VariantKind::TupleSingle => {
            let slot = variant.fields.iter().next();
            VariantPayload::Value(Box::new(slot.map_or_else(
                || get_field_def("value", &syn::parse_quote!(()), ""),
                |field| {
                    let mut field_def = field_def_with_prop_meta("value", field, args);
                    field_def.erase_type_parameters(type_parameters);
                    field_def
                },
            )))
        }
        VariantKind::TupleMultiple => {
            let slots: Vec<FieldDef> = variant
                .fields
                .iter()
                .map(|field| {
                    let mut field_def = field_def_with_prop_meta("slot", field, args);
                    field_def.erase_type_parameters(type_parameters);
                    field_def
                })
                .collect();
            VariantPayload::Value(Box::new(FieldDef {
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
                set_levels: Vec::new(),
                #[cfg(feature = "jsonschema")]
                type_span: proc_macro2::Span::call_site(),
            }))
        }
    }
}

/// One variant's own subclass declaration, implementing `export_name`. `bare_value` selects a
/// `@JvmInline value class` for a scalar `Value` payload — the bare wire form an untagged member
/// needs — while a tagged form wraps it in an ordinary `data class` property instead. The plugin
/// writes the subclass's serializer, `plugin`, which reads a model the variant holds unjudged.
/// Beside the declaration, the type a `Value` payload's property is declared with, which the
/// enum's serializer reads the value by: a walked and an unwalked wrapper of one tuple are two
/// classes.
fn variant_subclass(
    export_name: &str,
    plugin: &Plugin<'_>,
    generic_params: &str,
    nothing_generic_args: &str,
    serial_name: Option<&str>,
    bare_value: bool,
    payload: &VariantPayload,
) -> (String, Option<String>) {
    let subclass_name = plugin.owner;
    let annotation =
        serial_name.map_or_else(String::new, |wire| format!("@SerialName(\"{wire}\") "));
    match payload {
        VariantPayload::Unit => (
            format!(
                "{annotation}@Serializable data object {subclass_name} : {export_name}{nothing_generic_args}"
            ),
            None,
        ),
        VariantPayload::Named(fields) => (
            format!(
                "{annotation}@Serializable data class {subclass_name}{generic_params}({}) : {export_name}{generic_params}",
                data_class_params(fields, Some(plugin))
            ),
            None,
        ),
        VariantPayload::Value(field_def) => {
            let (unjudged, held) = read::plugin_read(field_def);
            let class = if bare_value {
                "@JvmInline @Serializable value class"
            } else {
                "@Serializable data class"
            };
            (
                format!(
                    "{annotation}{class} {subclass_name}{generic_params}({unjudged}val value: {held}) : {export_name}{generic_params}"
                ),
                Some(held),
            )
        }
    }
}

/// The members of a variant whose functions are called at their key by the enum's serializer,
/// which hands the plugin the variant's object: each member `plugin` reads no other way, and a
/// lone slot, which that object holds under `value`.
fn variant_keyed(
    payload: &VariantPayload,
    plugin: &Plugin<'_>,
) -> Vec<(String, Vec<FunctionName>)> {
    match payload {
        VariantPayload::Named(fields) => plugin.keyed(fields),
        VariantPayload::Unit => Vec::new(),
        VariantPayload::Value(field_def) => {
            let functions = preprocess::of(field_def);
            if functions.is_empty() {
                Vec::new()
            } else {
                vec![("value".to_owned(), functions.to_vec())]
            }
        }
    }
}

/// Each alias of a member of a variant's `payload`, beside the name the member is written under.
fn members_renamed(payload: &VariantPayload) -> Vec<(String, String)> {
    if let VariantPayload::Named(fields) = payload {
        aliases::of_members(fields)
    } else {
        Vec::new()
    }
}

/// One variant's own subclass, `serialize` arm and `deserialize` arm, for
/// [`dispatched_tagged_enum_kotlin_source`] — pulled out of that function's own loop to keep it
/// under this crate's line budget per function.
fn dispatched_tagged_variant(
    variant: &Variant,
    ctx: &DispatchedTaggedContext<'_>,
) -> (String, String, String) {
    let variant_rust_name = variant.ident.to_string();
    let wire_tag = rename_override(&variant.attrs)
        .unwrap_or_else(|| ctx.variant_rule.apply_to_variant(&variant_rust_name));
    let subclass_name = format!("{}{variant_rust_name}", ctx.export_name);
    let payload = variant_payload(
        variant,
        ctx.field_rule,
        ctx.type_parameters,
        ctx.args,
        ctx.defaults,
    );
    let beside_the_tag = matches!(ctx.tagging, Tagging::Unwrapped);
    let plugin = Plugin {
        owner: &subclass_name,
        type_parameters: ctx.type_parameters,
    };
    let (declaration, declared) = variant_subclass(
        ctx.export_name,
        &plugin,
        ctx.generic_params,
        ctx.nothing_generic_args,
        // Under a tag alone the subclass is named by its tag.
        beside_the_tag.then_some(wire_tag.as_str()),
        false,
        &payload,
    );
    let subclass = format!(
        "{declaration} {}",
        variant_body(&payload, ctx.tagging, &wire_tag, ctx.serializable, ctx.args)
    );

    let tag_key = ctx.tag_key;
    // Under a tag and a content key the content is read out of a `Map` index, which is nullable
    // in Kotlin; under a tag alone it is the object without the tag; with no tag attribute the
    // object's one entry is destructured, already non-null.
    let data = match ctx.tagging {
        Tagging::Adjacent(_) => "data!!".to_owned(),
        Tagging::External => "data".to_owned(),
        Tagging::Unwrapped => format!("JsonObject(obj - \"{tag_key}\")"),
    };
    let own = || kotlin_subclass_serializer_expr(&subclass_name, ctx.type_parameters);
    let (content_expr, decode_expr) = match (&payload, &declared) {
        (VariantPayload::Unit, _) => (None, subclass_name.clone()),
        (VariantPayload::Value(field_def), Some(held)) if !beside_the_tag => {
            let read_by = read::declared_serializer(field_def, held, ctx.type_parameters);
            (
                Some(format!(
                    "output.json.encodeToJsonElement({read_by}, value.value)"
                )),
                format!(
                    "{subclass_name}(input.json.decodeFromJsonElement({read_by}, {}))",
                    preprocess::corrected(preprocess::of(field_def), &data)
                ),
            )
        }
        // Under a tag alone a variant is its subclass's own object, a lone slot included.
        (VariantPayload::Named(_) | VariantPayload::Value(_), _) => (
            Some(format!("output.json.encodeToJsonElement({}, value)", own())),
            format!(
                "input.json.decodeFromJsonElement({}, {})",
                own(),
                preprocess::keyed(
                    &aliases::keys_named(&data, &members_renamed(&payload)),
                    &variant_keyed(&payload, &plugin)
                )
            ),
        ),
    };

    let serialize_arm = match (ctx.tagging, &content_expr) {
        (Tagging::Adjacent(content), Some(expr)) => format!(
            "is {subclass_name} -> buildJsonObject {{ put(\"{tag_key}\", \"{wire_tag}\"); put(\"{content}\", {expr}) }}"
        ),
        (Tagging::External, Some(expr)) => {
            format!("is {subclass_name} -> buildJsonObject {{ put(\"{wire_tag}\", {expr}) }}")
        }
        (Tagging::Unwrapped, Some(expr)) => format!(
            "is {subclass_name} -> buildJsonObject {{ put(\"{tag_key}\", \"{wire_tag}\"); \
             {expr}.jsonObject.forEach {{ (k, v) -> put(k, v) }} }}"
        ),
        (Tagging::Adjacent(_) | Tagging::Unwrapped, None) => {
            format!(
                "is {subclass_name} -> buildJsonObject {{ put(\"{tag_key}\", \"{wire_tag}\") }}"
            )
        }
        (Tagging::External, None) => format!("is {subclass_name} -> JsonPrimitive(\"{wire_tag}\")"),
    };
    let mut read_under = format!("\"{wire_tag}\"");
    for alias in aliases::written_on(&variant.attrs) {
        let _ = write!(read_under, ", \"{}\"", kotlin_template_text(&alias));
    }
    let deserialize_arm = format!("{read_under} -> {decode_expr}");

    (subclass, serialize_arm, deserialize_arm)
}

/// The Kotlin tokens a tagged enum earns, under a tag alone (`tag = "..."`), under a tag and a
/// content key (`tag = "...", content = "..."`) and under serde's own default: a plain `sealed
/// interface`, one subclass per variant, the unjudged `KSerializer` that reads and writes the tag
/// by hand over the variants' own serializers, and the enum's own, which judges.
fn dispatched_tagged_enum_kotlin_source(
    item_enum: &ItemEnum,
    export_name: &str,
    tag_attrs: &EnumTagAttrs,
    args: &KotlinArgs<'_>,
    defaults: &SerdeDefaults,
) -> String {
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let type_parameters = type_parameters_in_scope(&item_enum.generics);
    let generic_params = kotlin_generic_params(&item_enum.generics);
    let nothing_generic_args = kotlin_nothing_generic_args(&item_enum.generics);
    let tag_key = tag_attrs.tag.as_deref().unwrap_or("type");
    let tagging = match (tag_attrs.content.as_deref(), tag_attrs.tag.as_deref()) {
        (Some(content), _) => Tagging::Adjacent(content),
        (None, Some(_)) => Tagging::Unwrapped,
        (None, None) => Tagging::External,
    };
    let serializable = serialized_parameters(&item_enum.generics);

    let ctx = DispatchedTaggedContext {
        args,
        defaults,
        export_name,
        field_rule,
        generic_params: &generic_params,
        nothing_generic_args: &nothing_generic_args,
        serializable: &serializable,
        tag_key,
        tagging,
        type_parameters: &type_parameters,
        variant_rule,
    };
    let (subclasses, serialize_arms, deserialize_arms): (Vec<_>, Vec<_>, Vec<_>) = item_enum
        .variants
        .iter()
        .map(|variant| dispatched_tagged_variant(variant, &ctx))
        .fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut subclasses, mut serializes, mut deserializes),
             (subclass, serialize, deserialize)| {
                subclasses.push(subclass);
                serializes.push(serialize);
                deserializes.push(deserialize);
                (subclasses, serializes, deserializes)
            },
        );

    let arms = deserialize_arms.join("; ");
    let whole = preprocess::corrected(args.preprocess, "input.decodeJsonElement()");
    let deserialize_body = match tagging {
        Tagging::Adjacent(content) => format!(
            "val obj = {whole}.jsonObject; \
             val tag = obj.getValue(\"{tag_key}\").jsonPrimitive.content; \
             val data = obj[\"{content}\"]; \
             return when (tag) {{ {arms} else -> error(\"unknown tag \" + tag) }}"
        ),
        Tagging::External => format!(
            "val element = {whole}; \
             val (tag, data) = if (element is JsonPrimitive) element.content to JsonNull \
             else element.jsonObject.entries.single().let {{ it.key to it.value }}; \
             return when (tag) {{ {arms} else -> error(\"unknown tag \" + tag) }}"
        ),
        // A tag no variant has is refused with the decoder's own exception.
        Tagging::Unwrapped => format!(
            "val obj = {whole}.jsonObject; \
             val tag = (obj[\"{tag_key}\"] as? JsonPrimitive)?.content; \
             return when (tag) {{ {arms} else -> throw SerializationException(\"unknown tag \" + \
             tag) }}"
        ),
    };

    let serializer_name = format!("{export_name}Serializer");
    let base = format!(
        "@Serializable(with = {serializer_name}::class) sealed interface {export_name}{} \
         {VALIDATE_DECLARED}",
        kotlin_out_generic_params(&item_enum.generics)
    );
    let serializer_head = read::written_head(export_name, &type_parameters);
    let serializer_object = format!(
        "{serializer_head} : KSerializer<{export_name}{generic_params}> {{ \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{export_name}\"); \
         override fun serialize(encoder: Encoder, value: {export_name}{generic_params}) {{ \
         val output = encoder as JsonEncoder; \
         output.encodeJsonElement(when (value) {{ {} }}) }}; \
         override fun deserialize(decoder: Decoder): {export_name}{generic_params} {{ \
         val input = decoder as JsonDecoder; {deserialize_body} }} }}",
        serialize_arms.join("; "),
    );

    format!(
        "{base} {} {serializer_object} {}",
        subclasses.join(" "),
        read::judged(export_name, &generic_params, &type_parameters, true)
    )
}

/// The Kotlin tokens a `#[serde(untagged)]` enum earns: a plain `sealed interface`, the unjudged
/// `KSerializer` that tries each variant's own serializer in turn and takes the first that decodes
/// and whose bounds hold, as serde's read of the enum does, and the enum's own, which judges.
fn untagged_enum_kotlin_source(
    item_enum: &ItemEnum,
    export_name: &str,
    tag_attrs: &EnumTagAttrs,
    args: &KotlinArgs<'_>,
    defaults: &SerdeDefaults,
) -> String {
    let field_rule = resolve_rename_rule(tag_attrs.rename_all_fields.as_deref());
    let type_parameters = type_parameters_in_scope(&item_enum.generics);
    let generic_params = kotlin_generic_params(&item_enum.generics);
    let nothing_generic_args = kotlin_nothing_generic_args(&item_enum.generics);
    let serializable = serialized_parameters(&item_enum.generics);

    let mut subclasses = Vec::new();
    let mut serialize_arms = Vec::new();
    let mut deserialize_chain = String::from("runCatching { error(\"unreachable\") as Nothing }");
    for variant in &item_enum.variants {
        let variant_rust_name = variant.ident.to_string();
        let subclass_name = format!("{export_name}{variant_rust_name}");
        let payload = variant_payload(variant, field_rule, &type_parameters, args, defaults);
        let bare_value = matches!(payload, VariantPayload::Value(_));
        let mut rules = Rules::choosing(&serializable);
        variant_rules(&mut rules, &payload, Tagging::Unwrapped, "", args);
        // A check and what a held model reports choose no variant: the enum's own read judges.
        let held_to_its_bounds = if rules.bounded() {
            format!(
                ".also {{ chosen -> if (chosen.kotlinSchemaBound().isNotEmpty()) error(\"a bound \
                 of {subclass_name} is broken\") }}"
            )
        } else {
            String::new()
        };
        let plugin = Plugin {
            owner: &subclass_name,
            type_parameters: &type_parameters,
        };
        let (declaration, _) = variant_subclass(
            export_name,
            &plugin,
            &generic_params,
            &nothing_generic_args,
            None,
            bare_value,
            &payload,
        );
        subclasses.push(format!(
            "{declaration} {{ {} }}",
            rules.written("override ")
        ));
        let subclass_serializer = kotlin_subclass_serializer_expr(&subclass_name, &type_parameters);
        serialize_arms.push(format!(
            "is {subclass_name} -> output.json.encodeToJsonElement({subclass_serializer}, value)"
        ));
        // A slot is the payload itself, and a member is under its key in it.
        let handed = if let VariantPayload::Value(field_def) = &payload {
            preprocess::corrected(preprocess::of(field_def), "element")
        } else {
            preprocess::keyed(
                &aliases::keys_named("element", &members_renamed(&payload)),
                &variant_keyed(&payload, &plugin),
            )
        };
        deserialize_chain = format!(
            "{deserialize_chain}.recoverCatching {{ input.json.decodeFromJsonElement({subclass_serializer}, {handed}){held_to_its_bounds} as {export_name}{generic_params} }}"
        );
    }

    let serializer_name = format!("{export_name}Serializer");
    let base = format!(
        "@Serializable(with = {serializer_name}::class) sealed interface {export_name}{} \
         {VALIDATE_DECLARED}",
        kotlin_out_generic_params(&item_enum.generics)
    );
    let serializer_head = read::written_head(export_name, &type_parameters);
    let serializer_object = format!(
        "{serializer_head} : KSerializer<{export_name}{generic_params}> {{ \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{export_name}\"); \
         override fun serialize(encoder: Encoder, value: {export_name}{generic_params}) {{ \
         val output = encoder as JsonEncoder; \
         output.encodeJsonElement(when (value) {{ {} }}) }}; \
         override fun deserialize(decoder: Decoder): {export_name}{generic_params} {{ \
         val input = decoder as JsonDecoder; val element = {}; \
         return {deserialize_chain}.getOrElse {{ error(\"No variant of {export_name} matched\") }} }} }}",
        serialize_arms.join("; "),
        preprocess::corrected(args.preprocess, "input.decodeJsonElement()"),
    );

    format!(
        "{base} {} {serializer_object} {}",
        subclasses.join(" "),
        read::judged(export_name, &generic_params, &type_parameters, true)
    )
}

/// One plain-enum variant's Kotlin member name (its Rust ident, verbatim — Kotlin enum constants
/// read naturally in `PascalCase` too) and wire value.
fn plain_enum_member(variant: &Variant, rule: RenameRule) -> (String, String) {
    let rust_name = variant.ident.to_string();
    let wire = rename_override(&variant.attrs).unwrap_or_else(|| rule.apply_to_variant(&rust_name));
    (rust_name, wire)
}

/// The Kotlin tokens a plain (all-unit, string-wire) enum earns: a serializable `enum class`,
/// `@SerialName` only where the wire spelling differs from the Rust ident. One that names a
/// `preprocess` is read by `{Type}Unjudged`, its one serializer, and so is one that reads a
/// variant under an alias, whose serializer is written by hand.
fn plain_enum_kotlin_source(
    item_enum: &ItemEnum,
    export_name: &str,
    args: &KotlinArgs<'_>,
) -> String {
    let rule = container_rename_rule(&item_enum.attrs);
    let variants: Vec<_> = item_enum
        .variants
        .iter()
        .map(|variant| {
            let (name, wire) = plain_enum_member(variant, rule);
            (name, wire, aliases::written_on(&variant.attrs))
        })
        .collect();
    let members = variants
        .iter()
        .map(|(name, wire, _)| {
            if name == wire {
                name.clone()
            } else {
                format!("@SerialName(\"{wire}\") {name}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut rules = Rules::of(&[]);
    rules.own(args.check);
    let body = format!(
        "enum class {export_name} {{ {members}; {} }}",
        rules.written("")
    );
    let renamed: Vec<_> = variants
        .iter()
        .flat_map(|(_, wire, read_under)| {
            read_under.iter().map(|alias| (alias.clone(), wire.clone()))
        })
        .collect();
    if !args.preprocess.is_empty() {
        // The enum's one serializer is the plugin's behind its functions.
        let correction =
            aliases::text_named(&preprocess::corrected(args.preprocess, ELEMENT), &renamed);
        format!(
            "{} {body} {}",
            read::kept_plain(export_name),
            kept_unjudged(export_name, "", &[], &correction)
        )
    } else if renamed.is_empty() {
        format!("@Serializable {body}")
    } else {
        format!(
            "@Serializable(with = {export_name}Unjudged::class) {body} {}",
            aliases::plain_enum_serializer(export_name, &variants)
        )
    }
}

#[cfg(test)]
mod tests;
