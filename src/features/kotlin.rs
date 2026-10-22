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
mod arrays;
mod preprocess;
mod read;
mod texts;
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
use self::texts::TextForm;
use self::validate::{Member, Rules};
use crate::features::model_schema_prop::{FunctionName, ModelSchemaPropMeta};
use crate::features::serde::defaults::{SerdeDefaults, substituted};
use crate::features::serde::{
    brand_field, has_serde_skip_serializing, keys_taken_when_flattened, parse_serde_key_omission,
    struct_tag, tag_and_content_when_flattened, variant_names_when_flattened, variants_on_the_wire,
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
    /// What the members read off the object, as the Kotlin terms subtracted from it, or `None`
    /// where a member may read any key: a flattened map, and a flattened type whose keys are not
    /// known.
    read: Option<String>,
    serialize_stmts: Vec<String>,
}

/// The tag the object a serializer [`flatten_merging_serializer`] writes reads holds beside its
/// members.
#[derive(Clone, Copy)]
enum BesideTag<'tag> {
    /// The tag of a variant under a tag alone, which its enum reads and writes.
    Enum(&'tag str),
    /// No tag.
    None,
    /// A struct's own tag key and the struct's name, which the serializer checks and writes.
    Own(&'tag (String, String)),
}

/// What one flattened member adds to the serializer of its holder.
struct FlattenedMember {
    deserialize: String,
    /// Whether the member is read under `lenient`, which reads past a key no member reads.
    lenient: bool,
    /// The merge of what it writes, and nothing for a member serde never writes.
    serialize: Option<String>,
    /// The Kotlin term of the keys it takes, which the members flattened after it are not handed.
    taken: String,
}

impl FlattenPlan {
    /// Adds the statements of a flattened `member`.
    fn push(&mut self, member: FlattenedMember) {
        self.needs_lenient |= member.lenient;
        self.serialize_stmts.extend(member.serialize);
        self.deserialize_stmts.push(member.deserialize);
    }
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

/// A kind of JSON value a serializer this module writes reads a payload as.
#[derive(Clone, Copy)]
enum JsonKind {
    Array,
    Object,
    /// A tag, which serde writes as a string.
    Text,
}

impl JsonKind {
    /// The `kotlinx.serialization.json` class a value of the kind is.
    const fn class(self) -> &'static str {
        match self {
            Self::Array => "JsonArray",
            Self::Object => "JsonObject",
            Self::Text => "JsonPrimitive",
        }
    }

    /// What a refusal names the kind by.
    const fn named(self) -> &'static str {
        match self {
            Self::Array => "an array",
            Self::Object => "an object",
            Self::Text => "a string",
        }
    }
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
    /// The names serde reads each enum the emitter wrote under when it is flattened, one list per
    /// variant, by Rust ident: the first entry of the holder's object under one of them is the
    /// enum's.
    static KOTLIN_VARIANT_NAMES: RefCell<HashMap<String, Vec<Vec<String>>>> =
        RefCell::new(HashMap::new());
    /// Each externally tagged enum the emitter wrote that writes a unit variant as its bare name,
    /// by Rust ident: serde writes that variant flattened as an entry, its name over `null`.
    static KOTLIN_BARE_NAMED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// The keys serde reads each struct the emitter wrote from when it is flattened, by Rust
    /// ident: the descriptor of a serializer written by hand declares no element, and the
    /// plugin's names the struct's own tag and a member serde never reads, and no alias.
    static KOTLIN_TAKEN_KEYS: RefCell<HashMap<String, Vec<String>>> =
        RefCell::new(HashMap::new());
    /// Every key each struct the emitter wrote reads, by Rust ident: the name and each alias of
    /// its members, a member serde never reads among them, and its own tag. A struct with a
    /// flattened member of its own, or that names a function on itself, is not noted.
    static KOTLIN_READ_KEYS: RefCell<HashMap<String, Vec<String>>> =
        RefCell::new(HashMap::new());
    /// The tag, then the content key where it has one, of each enum under a tag key the emitter
    /// wrote, by Rust ident: what a flattened `Option` of it is looked for by.
    static KOTLIN_TAG_KEYS: RefCell<HashMap<String, Vec<String>>> =
        RefCell::new(HashMap::new());
    /// The tag and the content key each enum under both the emitter wrote takes off its holder's
    /// object when it is flattened, by Rust ident: serde reads it as a struct of the two.
    static KOTLIN_TAKEN_TAG_KEYS: RefCell<HashMap<String, Vec<String>>> =
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
/// first entry under, one list per variant, as serde reads an externally tagged enum: `None` for
/// any other type, one the emitter has not written, and a member that names a function, which is
/// handed the whole object.
fn flattened_variant_names(def: &FieldDef) -> Option<(&str, Vec<Vec<String>>)> {
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

/// The keys serde reads the struct a flattened member typed `def` holds from, where the emitter
/// wrote that struct: `None` for any other type.
fn recorded_keys(def: &FieldDef) -> Option<Vec<String>> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return None;
    };
    KOTLIN_TAKEN_KEYS.with(|noted| noted.borrow().get(name).cloned())
}

/// Every key the struct a flattened member typed `def` holds reads, where the emitter noted them:
/// `None` for any other type, and for a member that may hand its object to a function.
fn read_keys(def: &FieldDef) -> Option<Vec<String>> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return None;
    };
    if preprocess::takes_no_key(def) {
        return None;
    }
    KOTLIN_READ_KEYS.with(|noted| noted.borrow().get(name).cloned())
}

/// Notes every key the struct written under `rust_ident` reads, for a holder written after it: the
/// name and each alias of each of its `fields`, and its own tag. A struct with a flattened member,
/// or that names a function on itself, `own`, reads keys not known here, and is not noted.
fn note_read_keys(
    rust_ident: &str,
    fields: &[KotlinField],
    own_tag: Option<&(String, String)>,
    own: &[FunctionName],
) {
    let known = own.is_empty() && !fields.iter().any(|field| field.flatten);
    KOTLIN_READ_KEYS.with(|noted| {
        if known {
            let every = fields
                .iter()
                .flat_map(|field| once(&field.wire_name).chain(&field.aliases))
                .chain(own_tag.map(|(key, _)| key))
                .cloned()
                .collect();
            noted.borrow_mut().insert(rust_ident.to_owned(), every);
        } else {
            noted.borrow_mut().remove(rust_ident);
        }
    });
}

/// The tag, then the content key where it has one, of an enum under a tag key: what a flattened
/// `Option` of it is looked for by. None for an untagged enum, and for one that names a function
/// on itself, which is read from what the function answers.
fn tag_keys(tag_attrs: &EnumTagAttrs, preprocess: &[FunctionName]) -> Vec<String> {
    if tag_attrs.untagged || !preprocess.is_empty() {
        return Vec::new();
    }
    tag_attrs
        .tag
        .iter()
        .chain(&tag_attrs.content)
        .cloned()
        .collect()
}

/// Notes `keys`, the tag and the content key of the enum written under `rust_ident`, for a holder
/// written after it: none for an enum under no tag key, or one that names a function on itself.
fn note_tag_keys(rust_ident: &str, keys: Vec<String>) {
    KOTLIN_TAG_KEYS.with(|noted| {
        if keys.is_empty() {
            noted.borrow_mut().remove(rust_ident);
        } else {
            noted.borrow_mut().insert(rust_ident.to_owned(), keys);
        }
    });
}

/// The tag and the content key of the enum under a tag key a flattened member typed `def` holds,
/// where the emitter wrote that enum: `None` for any other type.
fn recorded_tag_keys(def: &FieldDef) -> Option<Vec<String>> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return None;
    };
    KOTLIN_TAG_KEYS.with(|noted| noted.borrow().get(name).cloned())
}

/// The tag and the content key the enum under both a flattened member typed `def` holds takes off
/// its holder's object, where the emitter wrote that enum and the member names no function: `None`
/// for any other type.
fn taken_tag_keys(def: &FieldDef) -> Option<Vec<String>> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return None;
    };
    if !preprocess::of(def).is_empty() {
        return None;
    }
    KOTLIN_TAKEN_TAG_KEYS.with(|noted| noted.borrow().get(name).cloned())
}

/// Whether a flattened member typed `def` is read from the whole object its holder reads, as
/// TypeScript and Zod read it: a struct, and an enum under a tag key, the emitter wrote that hands
/// its object to no function.
fn reads_the_whole(def: &FieldDef) -> bool {
    (recorded_keys(def).is_some() || recorded_tag_keys(def).is_some())
        && !preprocess::takes_no_key(def)
}

/// `keys` as a Kotlin set of strings.
fn kotlin_set_of(keys: &[String]) -> String {
    if keys.is_empty() {
        // Kotlin infers no element type from an empty `setOf()`.
        "setOf<String>()".to_owned()
    } else {
        let listed = keys
            .iter()
            .map(|key| kotlin_quoted(key))
            .collect::<Vec<_>>()
            .join(", ");
        format!("setOf({listed})")
    }
}

/// The keys a flattened struct typed `def` takes off its holder's object, as a Kotlin set: those
/// serde reads where the emitter wrote the struct, the tag and the content key of an enum under
/// both it wrote, and the `elementNames` of the descriptor `serializer` answers for any type it
/// recorded no keys for.
fn flattened_struct_keys(def: &FieldDef, serializer: &str) -> String {
    recorded_keys(def)
        .or_else(|| taken_tag_keys(def))
        .map_or_else(
            || format!("({serializer}).descriptor.elementNames.toSet()"),
            |keys| kotlin_set_of(&keys),
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

/// `name`, a key or a name serde writes, as a Kotlin string literal, which reads `$` as a template,
/// `\` as an escape and `"` as its end.
pub fn kotlin_quoted(name: &str) -> String {
    format!("\"{}\"", kotlin_template_text(name))
}

/// `held`, the Kotlin variable of a `JsonElement`, read as `kind`: any other kind is refused with
/// `SerializationException`, which every refusal of a serializer this module writes throws.
fn read_as(held: &str, kind: JsonKind) -> String {
    format!(
        "{held} as? {} ?: throw SerializationException(\"expected {}, found \" + {held})",
        kind.class(),
        kind.named()
    )
}

/// `payload`, the Kotlin expression of a `JsonElement`, read as `kind` as [`read_as`] reads it.
fn payload_read_as(payload: &str, kind: JsonKind) -> String {
    format!("{payload}.let {{ {} }}", read_as("it", kind))
}

/// `read`, the Kotlin expression of what an object holds under `key`, refused where it holds
/// nothing, in serde's words.
fn present(read: &str, key: &str) -> String {
    format!(
        "({read} ?: throw SerializationException(\"missing field `{}`\"))",
        kotlin_template_text(key)
    )
}

/// The statement that refuses `object`, the Kotlin variable of a `JsonObject`, where it holds no
/// `key` or holds another value than `named` under it: the tag of a struct under its own tag, which
/// TypeScript and Zod refuse in the same places, where serde reads past it.
fn own_tag_held(object: &str, key: &str, named: &str) -> String {
    let key_text = kotlin_template_text(key);
    let name_text = kotlin_template_text(named);
    format!(
        "when ({object}[\"{key_text}\"]) {{ JsonPrimitive(\"{name_text}\") -> {{}}; null -> throw \
         SerializationException(\"missing field `{key_text}`\"); else -> throw \
         SerializationException(\"expected tag \\\"{name_text}\\\", found \" + \
         {object}[\"{key_text}\"]) }}"
    )
}

/// The property a struct under its own tag holds its name `named` in, written under `key`.
fn own_tag_property(key: &str, named: &str) -> String {
    format!(
        "@SerialName({}) @EncodeDefault val kotlinSchemaTag: String = {}; ",
        kotlin_quoted(key),
        kotlin_quoted(named)
    )
}

/// The statement that refuses the first of `left`, the Kotlin expression of the keys of an object
/// no member read, in the words of the plugin's serializer, which refuses it under the same `Json`:
/// one whose configuration does not ignore unknown keys, as TypeScript and Zod refuse it.
fn unknown_key_refused(left: &str) -> String {
    format!(
        "if (!input.json.configuration.ignoreUnknownKeys) {left}.firstOrNull()?.let {{ throw \
         SerializationException(\"Encountered an unknown key '\" + it + \"'\") }}"
    )
}

/// The statements that read `payload` as the array of a tuple of `length` positions, `array`, and
/// refuse an array of another length, in serde's words.
fn tuple_array(payload: &str, length: usize) -> String {
    format!(
        "val array = {}; if (array.size != {length}) throw SerializationException(\"invalid \
         length \" + array.size + \", expected {length}\")",
        payload_read_as(payload, JsonKind::Array)
    )
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
/// from serialization only is pushed a nullable level, since kotlinx has no "absent key" spelling,
/// but for a flattened one, which writes no key.
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
        let flatten = field_is_flatten(&field.attrs);
        // A flattened member has no key to leave out, and serde reads it whatever it writes.
        if omission.omits_key && !field_def.is_optional() && !flatten {
            field_def.nullable_levels.push(field_def.array_depth);
        }
        let defaulted = default_at(&rust_name);
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
            sibling_typename(name, generics, kotlin_typename)
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

/// The Kotlin type of the type written `name` with `arguments`, each typed by `typed`.
fn sibling_typename(
    name: &str,
    arguments: &[FieldDef],
    typed: impl Fn(&FieldDef) -> String,
) -> String {
    let class_name = lookup_kotlin_name(name).unwrap_or_else(|| name.to_owned());
    if arguments.is_empty() {
        class_name
    } else {
        let written = arguments.iter().map(typed).collect::<Vec<_>>().join(", ");
        format!("{class_name}<{written}>")
    }
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

/// A reference to `field`'s own `KSerializer`. A field reaching none of `type_parameters`, no text
/// of one form ([`texts::reaches`]) and no array of one length ([`arrays::reaches`]) resolves
/// through the reified `serializer<T>()` as before; one that does composes
/// `ListSerializer`/`MapSerializer`/`.nullable` instead: a parameter is never reifiable, and the
/// reified serializer reads no annotation that names the serializer of a form or of a length.
fn kotlin_serializer_expr(field: &FieldDef, type_parameters: &[String]) -> String {
    let composed = |def: &FieldDef| {
        field_reaches_type_parameter(def, type_parameters)
            || texts::reaches(def)
            || arrays::reaches(def)
    };
    if !composed(field) {
        return format!("serializer<{}>()", kotlin_typename(field));
    }
    let held = innermost(field);
    if !composed(&held) {
        // An array of one length over a value the reified serializer reads.
        return listed_serializer(field, format!("serializer<{}>()", kotlin_typename(&held)));
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
            texts::named_by(name)
                .filter(|_| generics.is_empty())
                .map_or_else(
                    || {
                        let class_name = lookup_kotlin_name(name).unwrap_or_else(|| name.clone());
                        let arguments = generics
                            .iter()
                            .map(|generic| kotlin_serializer_expr(generic, type_parameters))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{class_name}.serializer({arguments})")
                    },
                    |target| kotlin_serializer_expr(&target, &[]),
                )
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
        FieldDefType::Char => texts::serializer(TextForm::Char),
        // None of these ever reaches a type parameter, a text of one form or an array of one
        // length at its innermost level (see `composed`), so this arm is unreached in practice; it
        // stays total rather than a wildcard.
        FieldDefType::Tuple(_)
        | FieldDefType::Unknown
        | FieldDefType::StringLiteral(_)
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Boolean
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
        | FieldDefType::F64 => format!("serializer<{}>()", kotlin_typename(&held)),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => format!("serializer<{}>()", kotlin_typename(&held)),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate => texts::serializer(TextForm::Date),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDateTime => texts::serializer(TextForm::DateTime),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveTime => texts::serializer(TextForm::Time),
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => format!("serializer<{}>()", kotlin_typename(&held)),
    };
    listed_serializer(field, scalar)
}

/// `scalar`, the serializer of what `field` holds at its innermost level, under each level of its
/// list and the `Option` it is written in: a level of one length is read through the class that
/// refuses an array of any other.
fn listed_serializer(field: &FieldDef, scalar: String) -> String {
    let listed = (0..field.array_depth).fold(scalar, |inner, level| {
        let item = if field.is_nullable_at(level) {
            format!("{inner}.nullable")
        } else {
            inner
        };
        let class = field
            .fixed_length_at(level)
            .map_or_else(|| "ListSerializer".to_owned(), arrays::serializer);
        format!("{class}({item})")
    });
    if field.is_optional() {
        format!("{listed}.nullable")
    } else {
        listed
    }
}

/// `field` under its list levels, their lengths and its nullable ones: the value at the innermost
/// level.
fn innermost(field: &FieldDef) -> FieldDef {
    let mut bare = field.clone();
    bare.array_depth = 0;
    bare.array_lengths.clear();
    bare.nullable_levels.clear();
    bare
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
/// in one is read unjudged, which makes it a shape of its own. A tuple of no element is `()`,
/// which [`unit_typename`] writes.
fn tuple_wrapper_typename(elements: &[FieldDef], walked: bool) -> String {
    if elements.is_empty() {
        return unit_typename();
    }
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
         val input = decoder as JsonDecoder; {}; \
         return {wrapper_name}({decode_entries}) }} }}",
        tuple_array("input.decodeJsonElement()", elements.len())
    );
    KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(class_text));
    KOTLIN_TUPLES.with(|tuples| {
        tuples.borrow_mut().push((shape, wrapper_name.clone()));
    });
    wrapper_name
}

/// The Kotlin type of `()`: an object, declared once per item beside the serializer that writes
/// it as the `null` serde writes for `()` and reads that `null` alone. It is noted among the tuple
/// shapes under the empty shape, which no tuple of an element writes.
fn unit_typename() -> String {
    if let Some(named) = KOTLIN_TUPLES.with(|tuples| {
        tuples
            .borrow()
            .iter()
            .find(|(declared, _)| declared.is_empty())
            .map(|(_, named)| named.clone())
    }) {
        return named;
    }
    let index = KOTLIN_TUPLE_COUNTER.with(|counter| {
        let next = counter.get() + 1;
        counter.set(next);
        next
    });
    let named = format!("KotlinUnit{index}");
    let serializer = format!("{named}Serializer");
    let declared = format!(
        "@Serializable(with = {serializer}::class) object {named} \
         object {serializer} : KSerializer<{named}> {{ \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{named}\"); \
         override fun serialize(encoder: Encoder, value: {named}) {{ \
         (encoder as JsonEncoder).encodeJsonElement(JsonNull) }}; \
         override fun deserialize(decoder: Decoder): {named} {{ \
         val payload = (decoder as JsonDecoder).decodeJsonElement(); if (payload !is JsonNull) \
         throw SerializationException(\"invalid type: \" + payload + \", expected unit\"); \
         return {named} }} }}"
    );
    KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(declared));
    KOTLIN_TUPLES.with(|tuples| tuples.borrow_mut().push((String::new(), named.clone())));
    named
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

/// The declarations the types named since the last item was written need, a tuple's wrapper, a
/// range's class, a `Result`'s interface and the serializer of a text of one form and of an array
/// of one length among them, each once: the next item numbers its own.
pub fn kotlin_declarations() -> Vec<String> {
    KOTLIN_RANGES.with(|ranges| ranges.borrow_mut().clear());
    KOTLIN_RESULTS.with(|results| results.borrow_mut().clear());
    KOTLIN_TUPLES.with(|tuples| tuples.borrow_mut().clear());
    texts::clear();
    arrays::clear();
    KOTLIN_TUPLE_AUX.with(RefCell::take)
}

/// A reference to the `KSerializer` a value typed `field`, held by no type the emitter writes, is
/// read with: `serializer<T>()` of its Kotlin type, or one composed over the serializer of each
/// text of one form and the class of each array of one length it holds, which
/// [`kotlin_declarations`] declares.
pub fn kotlin_read_serializer(field: &FieldDef) -> String {
    kotlin_serializer_expr(field, &[])
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
    let full_source = kotlin_declarations()
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
        format!("@SerialName({}) ", kotlin_quoted(&field.wire_name))
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
/// payload under the field's key is read as the field's own `preprocess` corrects it, and a key
/// serde requires is refused where it is absent. An optional member's key is left out where it
/// holds no value, as serde leaves it out, but under `nullable`, and a member serde never writes
/// has no `serialize` statement.
fn own_field_plan(
    field: &KotlinField,
    prop: &str,
    type_parameters: &[String],
) -> (Option<String>, String) {
    let field_serializer = read::member_serializer(&field.field_def, type_parameters);
    let key = kotlin_quoted(&field.wire_name);
    let put =
        format!("put({key}, output.json.encodeToJsonElement({field_serializer}, value.{prop}))");
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
            "obj[{key}]?.let {{ input.json.decodeFromJsonElement({field_serializer}, {}) }}",
            preprocess::corrected(functions, "it")
        )
    } else if let Some(at) = &field.default_at {
        format!(
            "obj[{key}]?.let {{ input.json.decodeFromJsonElement({field_serializer}, {}) }} \
             ?: input.json.decodeFromString({field_serializer}, \"{at}\")",
            preprocess::corrected(functions, "it")
        )
    } else {
        format!(
            "input.json.decodeFromJsonElement({field_serializer}, {})",
            preprocess::corrected(
                functions,
                &present(&format!("obj[{key}]"), &field.wire_name)
            )
        )
    };
    (serialize, format!("val {prop} = {decode_expr}"))
}

/// One flattened *struct* (or `Option<Struct>`) field's own statements: the `serialize` merge, the
/// `deserialize` read (off `rest`, the object serde hands the member, as the member's own
/// `preprocess` then corrects it), and the keys it takes, which the members flattened after it are
/// handed the object without. A struct whose keys [`read_keys`] knows is read under the caller's
/// `Json`, from `rest` kept to those keys, so a model it holds at any depth refuses a key no member
/// reads as it does held under a key; any other member is read leniently. An optional field is
/// `null` where its own keys, or the tag and the content key of an enum under a tag key the emitter
/// wrote, are all absent from `rest`, and, naming no function and having no key to look for,
/// wherever its read throws: a struct the emitter wrote that takes no key, and a type it recorded
/// no keys for whose serializer's descriptor names no element, which is refused where it reads with
/// no judgement and breaks a rule. One that names a function is read from what the function
/// answers, and where that read throws is `null` where `rest` holds none of its keys, or no key at
/// all where it has none to look for, and refused where it holds one.
fn flatten_struct_field_plan(
    field: &KotlinField,
    prop: &str,
    type_parameters: &[String],
    rest: &str,
) -> FlattenedMember {
    let base = flatten_base_field(&field.field_def);
    let inner_serializer = read::member_serializer(&base, type_parameters);
    let serialize = flattened_merge(field, prop, &inner_serializer);
    let keys_ident = format!("{}Keys", kotlin_bare(prop));
    let mut statements = vec![format!(
        "val {keys_ident} = {}",
        flattened_struct_keys(&base, &inner_serializer)
    )];
    let functions = preprocess::of(&field.field_def);
    let corrected = preprocess::corrected(functions, rest);
    let known = read_keys(&base);
    let decoded = known.as_ref().map_or_else(
        || format!("lenient.decodeFromJsonElement({inner_serializer}, {corrected})"),
        |keys| {
            format!(
                "input.json.decodeFromJsonElement({inner_serializer}, \
                 JsonObject({corrected}.filterKeys {{ it in {} }}))",
                kotlin_set_of(keys)
            )
        },
    );
    // With no key to look for, the type is held wherever it reads, as serde holds it.
    let tried = format!("runCatching {{ {decoded} }}.getOrNull()");
    let looked_for =
        |keys: &str| format!("if ({rest}.keys.any {{ it in {keys} }}) {decoded} else null");
    let read = if !field.field_def.is_optional() {
        decoded
    } else if !functions.is_empty() {
        // Zod's union tries the type over what the function answers ahead of the branch without
        // it, which takes an object that holds none of the type's keys.
        let corrected_ident = format!("{}Corrected", kotlin_bare(prop));
        statements.push(format!("val {corrected_ident} = {corrected}"));
        let any_key = format!("{rest}.keys.any({keys_ident}::contains)");
        let held = if recorded_keys(&base).is_some_and(|keys| !keys.is_empty()) {
            any_key
        } else {
            format!("if ({keys_ident}.isEmpty()) {rest}.isNotEmpty() else {any_key}")
        };
        format!(
            "runCatching {{ lenient.decodeFromJsonElement({inner_serializer}, {corrected_ident}) \
             }}.getOrElse {{ if ({held}) throw it else null }}"
        )
    } else {
        match (recorded_keys(&base), recorded_tag_keys(&base)) {
            (Some(keys), _) if keys.is_empty() => tried,
            (Some(_), _) => looked_for(&keys_ident),
            // The enum's serializer is written by hand, so its descriptor names neither key.
            (None, Some(tags)) => looked_for(&kotlin_set_of(&tags)),
            // A serializer written by hand declares no element; the plugin's names the keys.
            (None, None) => {
                let without_keys = if read::member_read_unjudged(&base) {
                    tried
                } else {
                    read_or_judged(&inner_serializer, &corrected)
                };
                format!(
                    "if ({keys_ident}.isEmpty()) {without_keys} else {}",
                    looked_for(&keys_ident)
                )
            }
        }
    };
    statements.push(format!("val {prop} = {read}"));
    FlattenedMember {
        deserialize: statements.join("; "),
        lenient: known.is_none(),
        serialize,
        taken: keys_ident,
    }
}

/// The read of `payload` by `serializer`, which judges, `null` where it throws. Where the
/// serializer answers the one it reads through with no judgement when it is called, as the
/// serializer of each type the emitter writes does, a payload that reads with none and breaks a
/// rule is refused with what the judgement throws.
fn read_or_judged(serializer: &str, payload: &str) -> String {
    format!(
        "runCatching {{ lenient.decodeFromJsonElement({serializer}, {payload}) }}.getOrElse {{ \
         refused -> if (runCatching {{ lenient.decodeFromJsonElement(({serializer} as? \
         Function0<*>)?.invoke() as? DeserializationStrategy<*> ?: {serializer}, {payload}) \
         }}.isSuccess) throw refused else null }}"
    )
}

/// The `serialize` statement that merges a flattened member's own keys into its holder's object,
/// read by `inner_serializer`; an optional member that holds no value merges none, and a member
/// serde never writes has no statement. A unit variant an enum writes as its bare name is merged
/// as serde merges it: its name over `null`.
fn flattened_merge(field: &KotlinField, prop: &str, inner_serializer: &str) -> Option<String> {
    if field.never_written {
        return None;
    }
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
    Some(if field.field_def.is_optional() {
        let held = merged("flattened");
        format!("value.{prop}?.let {{ flattened -> {held} }}")
    } else {
        merged(&format!("value.{prop}"))
    })
}

/// One flattened externally tagged enum's own statements, as [`flatten_struct_field_plan`] writes
/// a struct's: the enum `read_under` names is read from the one entry of `rest` under one of the
/// names it holds, one list per variant, which serde refuses the holder without and an optional
/// enum reads no value without, and the key of that entry, which the members flattened after it
/// are handed the object without. Entries under two names are refused, as TypeScript and Zod
/// refuse them, naming each variant they belong to once, in the order the enum declares them.
fn flatten_enum_field_plan(
    field: &KotlinField,
    prop: &str,
    type_parameters: &[String],
    rest: &str,
    (enum_name, names): (&str, &[Vec<String>]),
) -> FlattenedMember {
    let base = flatten_base_field(&field.field_def);
    let inner_serializer = read::member_serializer(&base, type_parameters);
    let serialize = flattened_merge(field, prop, &inner_serializer);
    let entry_ident = format!("{}Entry", kotlin_bare(prop));
    let entries_ident = format!("{}Entries", kotlin_bare(prop));
    let listed = kotlin_set_of(&names.concat());
    let variants = names
        .iter()
        .filter_map(|variant| {
            Some(format!(
                "if ({entries_ident}.any {{ it.key in {} }}) {} else null",
                kotlin_set_of(variant),
                kotlin_quoted(variant.first()?)
            ))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let entry_read = format!(
        "{rest}.entries.filter {{ it.key in {listed} }}.also {{ {entries_ident} -> if \
         ({entries_ident}.size > 1) throw SerializationException(\"more than one variant of enum \
         {enum_name} found in flattened data: \" + listOfNotNull({variants}).joinToString(\", \")) \
         }}.firstOrNull()"
    );
    let (decode, entry_taken) = if field.field_def.is_optional() {
        (
            format!(
                "val {entry_ident} = {entry_read}; val {prop} = {entry_ident}?.let {{ input.json.decodeFromJsonElement({inner_serializer}, JsonObject(mapOf(it.toPair()))) }}"
            ),
            format!("setOfNotNull({entry_ident}?.key)"),
        )
    } else {
        (
            format!(
                "val {entry_ident} = {entry_read} ?: throw SerializationException(\"no variant of enum {enum_name} found in flattened data\"); val {prop} = input.json.decodeFromJsonElement({inner_serializer}, JsonObject(mapOf({entry_ident}.toPair())))"
            ),
            format!("{entry_ident}.key"),
        )
    };
    FlattenedMember {
        deserialize: decode,
        lenient: false,
        serialize,
        taken: entry_taken,
    }
}

/// One flattened map field's own statements: the `serialize` merge and the `deserialize` read,
/// over every key of the object, as `functions`, the field's own `preprocess`, correct it. A map
/// flattened beside another member is refused, so serde hands the map every key of the object.
/// A key Kotlin holds as no `String` is written as the text `key_serializer` writes for it, and
/// read back through it, so a key it refuses refuses the entry. An optional map writes no entry
/// where it holds no value, and is read as `null`, as serde reads it, wherever an entry does not
/// read as the map's key and value; under a function, where the function answers `null`.
fn flatten_map_field_plan(
    prop: &str,
    optional: bool,
    (key_serializer, value_serializer): (Option<&str>, &str),
    functions: &[FunctionName],
) -> (String, String) {
    let held = if optional { "?" } else { "" };
    let written_key = key_serializer.map_or_else(
        || "k".to_owned(),
        |key| format!("output.json.encodeToJsonElement({key}, k).jsonPrimitive.content"),
    );
    let serialize = format!(
        "value.{prop}{held}.forEach {{ (k, v) -> put({written_key}, output.json.encodeToJsonElement({value_serializer}, v)) }}"
    );
    let entries = |object: &str| {
        key_serializer.map_or_else(
            || {
                format!(
                    "{object}.mapValues {{ (_, v) -> input.json.decodeFromJsonElement({value_serializer}, v) }}"
                )
            },
            |key| {
                format!(
                    "{object}.entries.associate {{ (k, v) -> input.json.decodeFromJsonElement({key}, JsonPrimitive(k)) to input.json.decodeFromJsonElement({value_serializer}, v) }}"
                )
            },
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
                entries(&format!("({})", read_as("it", JsonKind::Object)))
            )
        } else {
            entries(&payload_read_as(&corrected, JsonKind::Object))
        }
    };
    (serialize, format!("val {prop} = {read}"))
}

/// The [`FlattenPlan`] of `fields`, read from an object that holds `beside`, a key no member
/// reads, as well: the tag of a variant under a tag alone.
fn flatten_plan(
    fields: &[KotlinField],
    type_parameters: &[String],
    beside: Option<&str>,
) -> FlattenPlan {
    let mut plan = FlattenPlan {
        ctor_args: Vec::new(),
        deserialize_stmts: Vec::new(),
        needs_lenient: false,
        read: None,
        serialize_stmts: Vec::new(),
    };
    let own_wire_keys: Vec<String> = fields
        .iter()
        .filter(|field| !field.flatten)
        .map(|field| kotlin_quoted(&field.wire_name))
        .chain(beside.map(kotlin_quoted))
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
    let mut members_read = Some(taken.clone());

    for field in fields {
        let prop = kotlin_property_name(&field.rust_name);
        if !field.flatten {
            let (serialize, decode) = own_field_plan(field, &prop, type_parameters);
            plan.serialize_stmts.extend(serialize);
            plan.deserialize_stmts.push(decode);
        } else if let FieldDefType::Map(map_key, map_value) = &field.field_def.field_type {
            let key_serializer = (kotlin_typename(map_key) != "String" || texts::reaches(map_key))
                .then(|| kotlin_serializer_expr(map_key, type_parameters));
            let value_serializer =
                read::held_serializer(&field.field_def, map_value, type_parameters);
            let (serialize, decode) = flatten_map_field_plan(
                &prop,
                field.field_def.is_optional(),
                (key_serializer.as_deref(), &value_serializer),
                preprocess::of(&field.field_def),
            );
            if !field.never_written {
                plan.serialize_stmts.push(serialize);
            }
            plan.deserialize_stmts.push(decode);
            members_read = None;
        } else {
            let rest = if taken.is_empty() {
                "obj".to_owned()
            } else {
                format!("JsonObject(obj{taken})")
            };
            if let Some((enum_name, names)) = flattened_variant_names(&field.field_def) {
                let read_under = (enum_name, names.as_slice());
                let member =
                    flatten_enum_field_plan(field, &prop, type_parameters, &rest, read_under);
                let _ = write!(taken, " - {}", member.taken);
                if let Some(so_far) = &mut members_read {
                    let _ = write!(so_far, " - {}", member.taken);
                }
                plan.push(member);
            } else {
                let handed = if reads_the_whole(&flatten_base_field(&field.field_def)) {
                    "obj"
                } else {
                    &rest
                };
                let member = flatten_struct_field_plan(field, &prop, type_parameters, handed);
                if !preprocess::takes_no_key(&field.field_def) {
                    let _ = write!(taken, " - {}", member.taken);
                }
                plan.push(member);
                members_read = members_read
                    .zip(read_keys(&field.field_def).or_else(|| taken_tag_keys(&field.field_def)))
                    .map(|(mut so_far, keys)| {
                        let _ = write!(so_far, " - {}", kotlin_set_of(&keys));
                        so_far
                    });
            }
        }
        plan.ctor_args.push(format!("{prop} = {prop}"));
    }

    plan.read = members_read;
    plan
}

/// The unjudged `KSerializer` a struct with one or more `#[serde(flatten)]` fields earns, and so
/// does a variant with one, since `kotlinx.serialization` has no annotation for flatten: it merges
/// each flattened value's own keys into the enclosing object, tolerant of the other fields' own
/// keys on decode. A struct or a variant with a member serde never writes earns it too:
/// `kotlinx.serialization` has no annotation for a member read and never written. `own` is the
/// type's `preprocess`, which corrects the object every member is read from, and each alias of
/// `renamed` is read in that object as the name it stands for. A key no member reads is refused
/// wherever [`FlattenPlan::read`] knows every key the members read, but the tag of a variant under
/// a tag alone, which its enum hands it with the object.
fn flatten_merging_serializer(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    fields: &[KotlinField],
    beside_tag: BesideTag<'_>,
    own: &[FunctionName],
    renamed: &[(String, String)],
) -> String {
    let (own_tag, beside) = match beside_tag {
        BesideTag::None => (None, None),
        BesideTag::Own(own_tag) => (Some(own_tag), None),
        BesideTag::Enum(key) => (None, Some(key)),
    };
    let self_type = format!("{export_name}{generic_params}");
    let serializer_head = read::written_head(export_name, type_parameters);
    let mut plan = flatten_plan(fields, type_parameters, beside);
    // A struct's own tag is written ahead of everything merged into the object.
    if let Some((key, named)) = own_tag {
        plan.serialize_stmts.insert(
            0,
            format!("put({}, {})", kotlin_quoted(key), kotlin_quoted(named)),
        );
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
    let tag_held = own_tag.map_or_else(String::new, |(key, named)| {
        format!("{}; ", own_tag_held("obj", key, named))
    });
    let unknown = plan.read.map_or_else(String::new, |read| {
        let tag = own_tag.map_or_else(String::new, |(key, _)| format!(" - {}", kotlin_quoted(key)));
        format!(
            "{}; ",
            unknown_key_refused(&format!("(obj{read}{tag}).keys"))
        )
    });
    let deserialize_body = format!(
        "override fun deserialize(decoder: Decoder): {self_type} {{ \
         val input = decoder as JsonDecoder; val obj = {}; {tag_held}{}; {unknown}\
         return {export_name}({}) }}",
        payload_read_as(
            &aliases::keys_named(
                &preprocess::corrected(own, "input.decodeJsonElement()"),
                renamed
            ),
            JsonKind::Object
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
/// itself. Handed `tag`, a struct's own tag key and the struct's name, it refuses a corrected
/// object that holds any other value under the key, or none, and writes the tag ahead of the
/// struct's members, as serde writes it.
fn kept_unjudged(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    correction: &str,
    tag: Option<&(String, String)>,
) -> String {
    let read = tag.map_or_else(
        || correction.to_owned(),
        |(key, named)| {
            format!(
                "{correction}.also {{ whole: JsonElement -> if (whole is JsonObject) {} }}",
                own_tag_held("whole", key, named)
            )
        },
    );
    let overrides: Vec<String> = (read != ELEMENT)
        .then(|| read::transform(&read))
        .into_iter()
        .chain(tag.map(|(key, _)| read::tag_first(key)))
        .collect();
    if overrides.is_empty() {
        read::generated(export_name, generic_params, type_parameters)
    } else {
        read::transforming(
            export_name,
            generic_params,
            type_parameters,
            &overrides.join("; "),
        )
    }
}

/// Notes the keys the struct written under `rust_ident` is read from when it is flattened.
fn note_taken_keys(rust_ident: &str, keys: Vec<String>) {
    KOTLIN_TAKEN_KEYS.with(|noted| {
        noted.borrow_mut().insert(rust_ident.to_owned(), keys);
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
    // A struct that names a function takes no key (`preprocess::note`), and an optional holder
    // still looks for it by the keys its read reads.
    let taken = keys_taken_when_flattened(&item_struct.fields, rule, &[]);
    note_taken_keys(&rust_ident, taken);
    let own_tag = struct_tag(item_struct);
    note_read_keys(&rust_ident, &fields, own_tag.as_ref(), args.preprocess);
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
    let kotlin_source = if written_by_hand {
        let unjudged = flatten_merging_serializer(
            &export_name,
            &generic_params,
            &type_parameters,
            &fields,
            own_tag.as_ref().map_or(BesideTag::None, BesideTag::Own),
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
        // struct's name. The unjudged serializer refuses a payload that holds any other value
        // under the key, or none, and moves the tag ahead of the members, where serde writes it.
        let tag = own_tag
            .as_ref()
            .map_or_else(String::new, |(key, named)| own_tag_property(key, named));
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
                ),
                own_tag.as_ref()
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
    let value_type = read::with_text_forms(&value_field);
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
            None,
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
    // kotlinc refuses a data class of no property: a struct of no slot is a plain class.
    let class = if slots.is_empty() {
        "class"
    } else {
        "data class"
    };
    let kotlin_source = format!(
        "@Serializable(with = {serializer_name}::class) \
         {class} {export_name}{generic_params}({params}) {{ {validated} }} \
         {serializer_head} : KSerializer<{export_name}{generic_params}> {{ \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{export_name}\"); \
         override fun serialize(encoder: Encoder, value: {export_name}{generic_params}) {{ \
         val output = encoder as JsonEncoder; \
         output.encodeJsonElement(buildJsonArray {{ {encode_entries} }}) }}; \
         override fun deserialize(decoder: Decoder): {export_name}{generic_params} {{ \
         val input = decoder as JsonDecoder; {}; \
         return {export_name}({decode_entries}) }} }} {}{alias}",
        tuple_array(&payload, slots.len()),
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
    if type_parameters.is_empty() {
        texts::note_alias(&rust_ident, &target);
    }
    let generic_params = kotlin_generic_params(&item_type.generics);
    let target_type = read::with_text_forms(&target);
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
fn enum_kotlin_tokens(declared: &ItemEnum, args: &KotlinArgs<'_>) -> TokenStream {
    // Plain or not as every variant declared reads, as `process_enum` dispatches it.
    let plain = is_plain_enum(declared);
    let item_enum = &variants_on_the_wire(declared);
    let rust_ident = item_enum.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_kotlin_name(&rust_ident, &export_name);
    let tag_attrs = enum_tag_attrs(&item_enum.attrs);
    note_tag_keys(&rust_ident, tag_keys(&tag_attrs, args.preprocess));
    KOTLIN_TAKEN_TAG_KEYS.with(|noted| {
        let taken = tag_and_content_when_flattened(item_enum, args.preprocess);
        if taken.is_empty() {
            noted.borrow_mut().remove(&rust_ident);
        } else {
            noted.borrow_mut().insert(rust_ident.clone(), taken);
        }
    });
    let writes_bare_variant_names =
        tag_attrs.tag.is_none() && tag_attrs.content.is_none() && !tag_attrs.untagged;
    let defaults = SerdeDefaults::of_enum(item_enum);
    let kotlin_source = if plain && writes_bare_variant_names {
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

/// Whether a variant's `payload` is read and written by [the merging
/// serializer](flatten_merging_serializer): it holds a `#[serde(flatten)]` member, which serde
/// reads and writes at the variant's own level, or a member serde never writes, which the plugin
/// would write.
fn written_by_hand(payload: &VariantPayload) -> bool {
    matches!(
        payload,
        VariantPayload::Named(fields)
            if fields.iter().any(|field| field.flatten || field.never_written)
    )
}

/// `{Subclass}Unjudged`, the serializer the emitter writes for the subclass `plugin` names, where
/// [`written_by_hand`] says the variant is read by one. The enum judges the variant, so the
/// subclass has no judged serializer.
fn variant_serializer(
    plugin: &Plugin<'_>,
    generic_params: &str,
    payload: &VariantPayload,
    tag: Option<&str>,
) -> Option<String> {
    let VariantPayload::Named(fields) = payload else {
        return None;
    };
    written_by_hand(payload).then(|| {
        flatten_merging_serializer(
            plugin.owner,
            generic_params,
            plugin.type_parameters,
            fields,
            tag.map_or(BesideTag::None, BesideTag::Enum),
            &[],
            &aliases::of_members(fields),
        )
    })
}

/// One variant's own subclass declaration, implementing `export_name`. `bare_value` selects a
/// `@JvmInline value class` for a scalar `Value` payload — the bare wire form an untagged member
/// needs — while a tagged form wraps it in an ordinary `data class` property instead. The plugin
/// writes the subclass's serializer, `plugin`, which reads a model the variant holds unjudged, but
/// for a variant [`variant_serializer`] reads. Beside the declaration, the type a `Value`
/// payload's property is declared with, which the enum's serializer reads the value by: a walked
/// and an unwalked wrapper of one tuple are two classes.
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
    let annotation = serial_name.map_or_else(String::new, |wire| {
        format!("@SerialName({}) ", kotlin_quoted(wire))
    });
    match payload {
        VariantPayload::Unit => (
            format!(
                "{annotation}@Serializable data object {subclass_name} : {export_name}{nothing_generic_args}"
            ),
            None,
        ),
        VariantPayload::Named(fields) if written_by_hand(payload) => (
            format!(
                "{annotation}@Serializable(with = {subclass_name}Unjudged::class) data class {subclass_name}{generic_params}({}) : {export_name}{generic_params}",
                data_class_params(fields, None)
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
/// lone slot, which that object holds under `value`. A variant [`written_by_hand`] calls its
/// members' functions itself.
fn variant_keyed(
    payload: &VariantPayload,
    plugin: &Plugin<'_>,
) -> Vec<(String, Vec<FunctionName>)> {
    match payload {
        VariantPayload::Named(_) if written_by_hand(payload) => Vec::new(),
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

/// Each alias of a member of a variant's `payload`, beside the name the member is written under,
/// which the enum's serializer reads as that name. A variant [`written_by_hand`] reads its own.
fn members_renamed(payload: &VariantPayload) -> Vec<(String, String)> {
    if written_by_hand(payload) {
        Vec::new()
    } else if let VariantPayload::Named(fields) = payload {
        aliases::of_members(fields)
    } else {
        Vec::new()
    }
}

/// The read of a unit variant: from an entry over `null` alone with no tag attribute, as serde
/// reads it, from an object with no content key under a tag and a content key, as Zod reads it,
/// and from an object of its tag `tag_key` alone under a tag alone, as Zod reads it, refusing any
/// other key under a `Json` that does not ignore unknown keys.
fn unit_variant_read(
    tagging: Tagging<'_>,
    subclass_name: &str,
    export_name: &str,
    tag_key: &str,
) -> String {
    let empty = match tagging {
        Tagging::Adjacent(_) => "data == null",
        Tagging::External => "data is JsonNull",
        Tagging::Unwrapped => {
            let left = format!("(obj - {}).keys", kotlin_quoted(tag_key));
            return format!("{{ {}; {subclass_name} }}", unknown_key_refused(&left));
        }
    };
    format!(
        "if ({empty}) {subclass_name} else throw SerializationException(\"{export_name}: \" + tag \
         + \" takes no value\")"
    )
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
    let mut subclass = format!(
        "{declaration} {}",
        variant_body(&payload, ctx.tagging, &wire_tag, ctx.serializable, ctx.args)
    );
    let tag_beside = beside_the_tag.then_some(ctx.tag_key);
    if let Some(serializer) = variant_serializer(&plugin, ctx.generic_params, &payload, tag_beside)
    {
        let _ = write!(subclass, " {serializer}");
    }

    let tag_key = ctx.tag_key;
    // Under a tag and a content key the content is read out of a `Map` index, which is nullable
    // in Kotlin, and refused where it is absent; under a tag alone it is the object without the
    // tag, or the whole object for a variant read by hand, whose flattened struct reads the tag
    // as TypeScript and Zod read it; with no tag attribute the object's one entry is destructured,
    // already non-null.
    let data = match ctx.tagging {
        Tagging::Adjacent(content) => present("data", content),
        Tagging::External => "data".to_owned(),
        Tagging::Unwrapped if written_by_hand(&payload) => "obj".to_owned(),
        Tagging::Unwrapped => format!("JsonObject(obj - {})", kotlin_quoted(tag_key)),
    };
    let own = || kotlin_subclass_serializer_expr(&subclass_name, ctx.type_parameters);
    let (content_expr, decode_expr) = match (&payload, &declared) {
        (VariantPayload::Unit, _) => (
            None,
            unit_variant_read(ctx.tagging, &subclass_name, ctx.export_name, ctx.tag_key),
        ),
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

    let (key, tag) = (kotlin_quoted(tag_key), kotlin_quoted(&wire_tag));
    let serialize_arm = match (ctx.tagging, &content_expr) {
        (Tagging::Adjacent(content), Some(expr)) => format!(
            "is {subclass_name} -> buildJsonObject {{ put({key}, {tag}); put({}, {expr}) }}",
            kotlin_quoted(content)
        ),
        (Tagging::External, Some(expr)) => {
            format!("is {subclass_name} -> buildJsonObject {{ put({tag}, {expr}) }}")
        }
        (Tagging::Unwrapped, Some(expr)) => format!(
            "is {subclass_name} -> buildJsonObject {{ put({key}, {tag}); \
             {expr}.jsonObject.forEach {{ (k, v) -> put(k, v) }} }}"
        ),
        (Tagging::Adjacent(_) | Tagging::Unwrapped, None) => {
            format!("is {subclass_name} -> buildJsonObject {{ put({key}, {tag}) }}")
        }
        (Tagging::External, None) => format!("is {subclass_name} -> JsonPrimitive({tag})"),
    };
    let mut read_under = tag;
    for alias in aliases::written_on(&variant.attrs) {
        let _ = write!(read_under, ", {}", kotlin_quoted(&alias));
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
    let obj = payload_read_as(&whole, JsonKind::Object);
    let unknown = "throw SerializationException(\"unknown tag \" + tag)";
    let key = kotlin_quoted(tag_key);
    let deserialize_body = match tagging {
        Tagging::Adjacent(content) => format!(
            "val obj = {obj}; val tag = {}.content; val data = obj[{}]; {}; \
             return when (tag) {{ {arms} else -> {unknown} }}",
            payload_read_as(&present(&format!("obj[{key}]"), tag_key), JsonKind::Text),
            kotlin_quoted(content),
            unknown_key_refused(&format!("(obj - {key} - {}).keys", kotlin_quoted(content)))
        ),
        Tagging::External => format!(
            "val element = {whole}; \
             val (tag, data) = if (element is JsonPrimitive) element.content to JsonNull \
             else ({}).entries.singleOrNull()?.let {{ it.key to it.value }} ?: throw \
             SerializationException(\"expected an object of one key, found \" + element); \
             return when (tag) {{ {arms} else -> {unknown} }}",
            read_as("element", JsonKind::Object)
        ),
        Tagging::Unwrapped => format!(
            "val obj = {obj}; val tag = (obj[{key}] as? JsonPrimitive)?.content; \
             return when (tag) {{ {arms} else -> {unknown} }}"
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
        subclasses.extend(variant_serializer(&plugin, &generic_params, &payload, None));
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
         return {deserialize_chain}.getOrElse {{ throw SerializationException(\"No variant of \
         {export_name} matched\") }} }} }}",
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
                format!("@SerialName({}) {name}", kotlin_quoted(wire))
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
            kept_unjudged(export_name, "", &[], &correction, None)
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
