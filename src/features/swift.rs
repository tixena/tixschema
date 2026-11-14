//! Swift type and `Codable`-codec generation.

mod aliases;
pub mod forms;
mod forwarded;
mod keys;
mod preprocess;
mod read;
mod validate;

use crate::utils::is_rule_less;
use core::cell::RefCell;
use core::fmt::Write as _;
use core::iter;
use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt as _;
use syn::{Field, Fields, Ident, Item, ItemEnum, ItemStruct, ItemType, Variant};

use crate::bound_message::swift_literal_text;
#[cfg(feature = "serde")]
use crate::features::member_keys;
use crate::features::model_schema_prop::{FunctionName, ModelSchemaPropMeta};
use crate::features::serde::defaults::{SerdeDefaults, substituted};
use crate::features::serde::{
    brand_field, derives_hash, derives_partial_eq, has_serde_skip_serializing,
    keys_taken_when_flattened, parse_serde_key_omission, struct_tag,
    tag_and_content_when_flattened, variant_names_when_flattened, variants_on_the_wire,
    written_slots,
};
use crate::features::text_forms::TextForm;
use crate::features::unmet_keys_filled;
use crate::field_type::{
    EXPECTED_NUMERIC_KEY, EXPECTED_QUOTE, EXPECTED_VALUE, FieldDef, FieldDefType, INVALID_NUMBER,
    MissingKey, NUMBER_OUT_OF_RANGE, RESULT_ARMS, RangeKind, Scope, VariantKind, get_field_def,
    is_plain_enum, is_sequence_wrapper,
};
#[cfg(feature = "serde")]
use crate::model_schema::flatten_order::flattened_keys;
use crate::model_schema::{variant_field_rule, variant_wire_kind};
use crate::rename_rule::{RenameRule, resolve_rename_rule};
#[cfg(feature = "serde")]
use crate::utils::is_unconsumed;
use crate::utils::{
    MapKeyWire, NumberKey, compute_alias_export_name, compute_item_export_name, is_nonstrict,
    to_snake_case, type_parameters_in_scope, within_variant,
};

#[cfg(feature = "serde")]
use crate::features::serde::{parse_serde_field_attributes, parse_serde_type_attributes};

/// Appended to a formatted date: chrono writes a date of whole seconds with no fraction.
#[cfg(feature = "chrono")]
pub const CHRONO_WHOLE_SECONDS: &str = ".replacingOccurrences(of: \".000Z\", with: \"Z\")";

/// The options of the formatter that reads and writes a fraction of a second.
#[cfg(feature = "chrono")]
pub const FRACTIONAL_OPTIONS: &str = ".withInternetDateTime, .withFractionalSeconds";

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

/// The options of the formatter that reads a date of whole seconds.
#[cfg(feature = "chrono")]
pub const WHOLE_OPTIONS: &str = ".withInternetDateTime";

/// The read an untagged enum hands a variant's struct to where the enum is handed keys: it refuses a
/// key of the object that is neither the struct's nor taken, then reads the struct.
const ONLY_ITS_KEYS: &str = "private static func swiftSchemaOnly<Read>(_ decoder: Decoder, _ keys: \
     [String], _ taken: [String], _ read: @autoclosure () throws -> Read) throws -> Read { for \
     unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys where \
     !keys.contains(unknown.stringValue) && !taken.contains(unknown.stringValue) { throw \
     DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath + [unknown], \
     debugDescription: \"unknown field `\\(unknown.stringValue)`\")) }; return try read() }; ";

/// What the own read of a struct is written around: whether the types around it hand it keys it
/// reads none of, the keys it refuses a key beside, and the functions it names on itself.
struct OwnRead<'own> {
    around: bool,
    closed: Option<Vec<String>>,
    corrects: &'own [FunctionName],
    /// The valid type, at its parameters, where the struct is the draft of one.
    valid: Option<&'own str>,
}

/// What a valid struct declares beyond what every one does.
#[derive(Default)]
struct ValidForm<'form> {
    /// The conformances written after `Codable, Sendable`, each led by a comma.
    conformances: &'form str,
    /// The type of the value of a struct of one slot, which a map keyed by it builds it from.
    keyed: Option<&'form str>,
    /// Whether the draft is a plain enum, whose raw value a map keyed by the type builds it from.
    raw: bool,
    /// The constants of the valid type: one for each unit case, where nothing can refuse one.
    statics: &'form str,
}

/// Whether a member is a stored property of its struct or sits in a box.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Storage {
    Boxed,
    Direct,
}

/// One field this module has decided belongs on the wire, resolved to the Swift property it
/// earns: its Rust name, its lower-camel Swift name, its wire name, whether the key always
/// reaches the wire, whether it is a `#[serde(flatten)]` source, and the shape that drives its
/// decode/encode statements.
struct SwiftField {
    /// The names serde reads the member under beside its wire name, its aliases: none for a
    /// flattened member or one serde never reads.
    aliases: Vec<String>,
    /// Where the JSON of the default serde reads for an absent key is written.
    default_at: Option<String>,
    field_def: FieldDef,
    flatten: bool,
    /// How the member is stored: an `Option` of the type that holds it cannot be a stored
    /// property of that type, so it is held behind a box.
    held: Storage,
    /// What the types written for the field are named from.
    hint: String,
    /// Whether serde never writes the member: it is under `skip_serializing`.
    never_written: bool,
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
    /// The Rust name of the item, which a member written `Self` is named as.
    pub type_name: &'args str,
    /// Whether the item is a type a service writes for itself, which has no draft.
    pub undrafted: bool,
}

/// What every tagging of an enum is written from.
struct EnumShape<'item> {
    args: &'item SwiftTypeArgs<'item>,
    /// The name the enum is declared under: its draft's where it has one.
    declared: &'item str,
    /// The defaults of the members of the enum's variants.
    defaults: &'item SerdeDefaults,
    export_name: &'item str,
    generic_params: &'item str,
    item_enum: &'item ItemEnum,
    rust_ident: &'item str,
    tag_attrs: &'item EnumTagAttrs,
    type_parameters: &'item [String],
    /// The valid type, at its parameters, where the enum is the draft of one.
    valid: Option<&'item str>,
}

/// What a struct of one `value` wraps.
enum Wrapped {
    /// The value of a brand, with the bounds the brand carries and the member it answers its text
    /// with.
    Brand {
        bounds: Vec<String>,
        length_member: Option<String>,
        text_member: Option<String>,
    },
    /// The one slot of a tuple struct that declares one, which serde writes as the struct.
    Slot,
    /// The tuple of a tuple struct of several slots, or of none, which serde writes as `[]`.
    Tuple,
}

/// One variant's payload: the type its case holds, what the case adds to `validate()`, and how
/// the payload is read.
struct VariantPayload {
    /// The externally tagged enum the one slot of a variant under a tag alone holds, which serde
    /// writes beside the tag as it writes it flattened.
    beside: SlotEntries,
    /// What chooses the variant in an untagged enum: the bounds of its own members, on `payload`.
    bound: Vec<String>,
    /// The one slot of the variant, where it is read and written as the type it is written as, or
    /// read item by item to hold an array to its length.
    converted: Option<Shaped>,
    /// The one slot of the variant, with what its types are named from, where it is read from a
    /// payload: one that names a function, and under a tag alone one whose type takes no keys.
    corrected: Option<(FieldDef, String)>,
    held: Option<String>,
    /// The keys the payload refuses a key beside, where its read refuses one.
    keys: Option<Vec<String>>,
    /// Whether Swift holds the payload as an optional, which `try?` reads as `nil` where its read
    /// answered `nil` and where it threw alike.
    optional: bool,
    /// The type parameters of the enum `held` names.
    parameters: Vec<String>,
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

/// The unit variants of an externally tagged enum written above, held by the one slot of a
/// variant under a tag alone: serde writes such a variant beside the tag as an entry of its name
/// over `null`. Empty for any other slot.
#[derive(Default)]
struct SlotEntries {
    /// The unit variants the enum's own read takes from an entry over `null` in the object
    /// without the tag, or in what the slot's functions answer of it.
    units: Vec<preprocess::Entry>,
    /// The `value` of a brand or a struct of one slot over the enum, which the variants are told
    /// by, and nothing where the slot holds the enum itself.
    value: &'static str,
    /// The unit variants written as an entry of their names over `null`.
    written: Vec<UnitEntry>,
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
    /// A `DateTime<Tz>` field: the wire is an ISO 8601 string with fractional seconds, which
    /// `JSONDecoder`'s default strategy cannot parse into `Date` without a formatter.
    #[cfg(feature = "chrono")]
    DateTimePlain,
    /// A map whose key is not written as a JSON object key can decode directly: `wrapper` names
    /// the per-field struct that carries the string round trip.
    MapWrapper(String),
    /// The leaf reads and writes exactly as its own Swift type spells it.
    None,
    /// A text of one form: read as the Zod schema of the form reads it, and written in the form
    /// serde writes.
    Text(TextForm),
    /// A map Swift keys natively whose values convert: each value is turned through its own shape.
    Values(Box<Shaped>),
}

/// A value read and written through its shape: the value as the emitter declares it, the shape
/// that says what turns the type it is written as into its own, and what the types written for it
/// are named from.
#[derive(Clone)]
struct Shaped {
    def: FieldDef,
    hint: String,
    shape: SwiftFieldShape,
}

/// How Swift reads the keys of a map.
enum MapKeys {
    /// As the `String` a `[String: Value]` is keyed by.
    Text,
    /// Through a wrapper written for the field.
    Wrapped,
}

/// How a map's wrapper reads the key of one entry from `wireKey`.
enum KeyRead {
    /// The expression answers the key, or `nil` for a key that does not parse.
    Parsed(String),
    /// The expression answers the key, or throws.
    Whole(String),
}

/// A field's fully resolved Swift shape: the property type callers see, the type a `container`
/// call actually decodes, and what (if anything) turns one into the other.
#[derive(Clone)]
struct SwiftFieldShape {
    leaf_conversion: LeafConversion,
    real_leaf: String,
    wire_leaf: String,
}

/// What a flattened member is handed of its holder's object.
#[derive(Default)]
struct Handed {
    /// The keys it is read without, as a Swift expression.
    taken: String,
    /// Whether it can be handed fewer keys than the object holds: its holder has a member of its
    /// own, a member flattened ahead of it took some, or the types around its holder took some.
    withheld: bool,
}

impl SwiftField {
    /// The container the member is read from under its key, and that key, as Swift expressions:
    /// `container` under `.{name}`, or, for a member serde reads under an alias too, the container
    /// of every key under the name the payload holds it under.
    fn keyed_at(&self, container: &str) -> (String, String) {
        if self.aliases.is_empty() {
            (container.to_owned(), format!(".{}", self.swift_name))
        } else {
            (
                "aliased".to_owned(),
                aliases::key(swift_bare(&self.swift_name)),
            )
        }
    }

    /// The member's value, read and written through its shape.
    fn shaped(&self) -> Shaped {
        Shaped {
            def: self.field_def.clone(),
            hint: self.hint.clone(),
            shape: self.shape.clone(),
        }
    }
}

impl EnumTagAttrs {
    /// Whether the enum is written under a tag with no content key: serde takes the tag off the
    /// object and hands each variant the rest.
    const fn tag_alone(&self) -> bool {
        self.tag.is_some() && self.content.is_none() && !self.untagged
    }
}

impl VariantPayload {
    /// The statements that read the payload and hand it to `sink`: a type written for the variant
    /// and a value `validate()` walks unjudged, a slot that names a function from what the
    /// function answers, a slot under a tag alone whose type takes no keys from the object
    /// without the tag, and any other through its shape. A type written for such a slot joins
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
                    flat: false,
                    functions: preprocess::functions(def),
                    hint,
                    keys: from.keys,
                    plain: preprocess::Plain::Declared,
                    real_type: held,
                    refusing: false,
                    stem: from.stem,
                    units: &self.beside.units,
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
            (None, Some(slot)) => {
                slot.read(held, from.stem, source, sink, &mut read::Locals::default())
            }
            (None, None) => sink(&format!("try {}", source.plain(held, &self.parameters))),
        }
    }

    /// What the enum writes for the payload `payload` names: itself, or the value of the type its
    /// slot is written as.
    fn written(&self, payload: &str) -> String {
        self.converted
            .as_ref()
            .map_or_else(|| payload.to_owned(), |slot| slot.written(payload))
    }

    /// The statement that writes the payload `payload` names beside the tag, through the enum's
    /// encoder: a unit variant of the enum [`Self::beside`] holds as an entry of its name over
    /// `null`.
    fn written_beside(&self, payload: &str) -> String {
        unit_entries_write(
            &self.beside.written,
            &format!("{payload}{}", self.beside.value),
            &self.written(payload),
        )
    }
}

impl Shaped {
    /// `def`, a value at a level of this one, read through the same leaf.
    fn at(&self, def: FieldDef) -> Self {
        Self {
            def,
            hint: self.hint.clone(),
            shape: self.shape.clone(),
        }
    }

    /// What `local`, the value read as the type it is written as, is turned into.
    fn converted(&self, local: &str) -> String {
        converted_from(&self.def, &self.shape, local)
    }

    /// Whether the value is read and written as a type other than its own.
    const fn converts(&self) -> bool {
        !matches!(self.shape.leaf_conversion, LeafConversion::None)
    }

    /// The statements that read the value from `source` and hand it to `sink`: item by item where
    /// it holds an array of a fixed length ([`read::counted`]), and otherwise
    /// [`Self::read_whole`].
    fn read(
        &self,
        real_type: &str,
        stem: &str,
        source: &read::Source,
        sink: &dyn Fn(&str) -> String,
        locals: &mut read::Locals,
    ) -> String {
        if read::holds_a_length(&self.def) {
            return read::counted(self, stem, source, sink, locals);
        }
        self.read_whole(real_type, stem, source, sink)
    }

    /// The statements that read the value from `source` in one call and hand it to `sink`: as
    /// `real_type`, its own, or as the type it is written as, held in `wire{Stem}` and turned into
    /// its own.
    fn read_whole(
        &self,
        real_type: &str,
        stem: &str,
        source: &read::Source,
        sink: &dyn Fn(&str) -> String,
    ) -> String {
        let parameters = parameters_reached(&[&self.def]);
        if !self.converts() {
            return sink(&format!("try {}", source.plain(real_type, &parameters)));
        }
        let local = format!("wire{}", swift_upper_camel(stem));
        format!(
            "let {local} = try {}; {}",
            source.plain(&wire_type(&self.def, &self.shape), &parameters),
            sink(&self.converted(&local))
        )
    }

    /// The Swift type the emitter declares the value as.
    fn real_type(&self) -> String {
        let wrapped = wrap_array_levels(&self.def, &self.shape.real_leaf);
        if self.def.is_optional() {
            format!("{wrapped}?")
        } else {
            wrapped
        }
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

    /// The unit variants of each externally tagged enum the emitter wrote, by Rust ident: serde
    /// writes such a variant flattened as an entry of its holder's object, its name over `null`.
    static UNIT_ENTRIES: RefCell<HashMap<String, Vec<UnitEntry>>> = RefCell::new(HashMap::new());

    /// The Rust ident of each brand and struct of one slot the emitter wrote whose `value` is such
    /// an enum, which [`UNIT_ENTRIES`] holds the unit variants of: a holder tells them by `value`.
    static VALUE_ENTRIES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

    /// The target of each alias the emitter wrote that declares no parameter and holds an array to
    /// its length, by Rust ident, and what the types written for it are named from: a holder reads
    /// a value of the alias item by item, as it reads one of the target.
    static COUNTED_ALIASES: RefCell<HashMap<String, (FieldDef, String)>> =
        RefCell::new(HashMap::new());

    /// The Rust ident of each alias the emitter wrote whose target Swift holds as an optional.
    static OPTIONAL_ALIASES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// A unit variant of an externally tagged enum: the case Swift declares it as, and the name serde
/// writes it under.
#[derive(Clone)]
struct UnitEntry {
    case_name: String,
    /// Whether the enum is the draft of a valid type, which a value of the enum is read through.
    drafted: bool,
    name: String,
}

/// Records `entries`, the unit variants of the enum `rust_ident`, for a holder written after it
/// that flattens it: none for an enum of any other shape.
fn note_unit_entries(rust_ident: &str, entries: Vec<UnitEntry>) {
    UNIT_ENTRIES.with(|recorded| {
        if entries.is_empty() {
            recorded.borrow_mut().remove(rust_ident);
        } else {
            recorded.borrow_mut().insert(rust_ident.to_owned(), entries);
        }
    });
}

/// The case Swift declares `variant` as, and the name serde writes it under: its `rename`, or the
/// enum's `rule`.
fn variant_names(variant: &Variant, rule: RenameRule) -> (String, String) {
    let rust_name = variant.ident.to_string();
    let named = swift_member(&RenameRule::CamelCase.apply_to_variant(&rust_name));
    let case_name = if named == "valid" {
        "valid_".to_owned()
    } else {
        named
    };
    let wire_name =
        rename_override(&variant.attrs).unwrap_or_else(|| rule.apply_to_variant(&rust_name));
    (case_name, wire_name)
}

/// The unit variants of `item_enum`, an externally tagged enum, which it writes as their names.
fn unit_entries(item_enum: &ItemEnum, tag_attrs: &EnumTagAttrs) -> Vec<UnitEntry> {
    let rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    item_enum
        .variants
        .iter()
        .filter(|variant| variant_wire_kind(variant) == VariantKind::Unit)
        .map(|variant| {
            let (case_name, name) = variant_names(variant, rule);
            UnitEntry {
                case_name,
                drafted: false,
                name,
            }
        })
        .collect()
}

/// The unit variants of the externally tagged enum `def` describes, a flattened member or the slot
/// of a variant under a tag alone, under an `Option` or not, where the emitter wrote it above: none
/// for any other type.
fn unit_entries_of(def: &FieldDef) -> Vec<UnitEntry> {
    let FieldDefType::SiblingType(name, _) = &def.field_type else {
        return Vec::new();
    };
    UNIT_ENTRIES.with(|recorded| recorded.borrow().get(name).cloned().unwrap_or_default())
}

/// Whether `def` names a brand or a struct of one slot whose `value` is an enum that writes a unit
/// variant as an entry of its name.
fn wraps_entries(def: &FieldDef) -> bool {
    matches!(&def.field_type, FieldDefType::SiblingType(name, _)
        if VALUE_ENTRIES.with(|recorded| recorded.borrow().contains(name)))
}

/// What the unit variants of `held`, a value of `def`, are told by: the value itself, and the
/// `value` of a brand or a struct of one slot over such an enum.
fn entries_held(def: &FieldDef, held: &str) -> String {
    if wraps_entries(def) {
        format!("{held}.value")
    } else {
        held.to_owned()
    }
}

/// Records the unit variants and the entries of `value`, the value of the brand or the struct of
/// one slot `rust_ident`, where it is an externally tagged enum written above: serde writes and
/// reads the wrapper flattened, and beside a tag, as that value.
fn note_value_entries(rust_ident: &str, value: &FieldDef) {
    let (units, entries) = if value.is_list() || value.is_optional() {
        (Vec::new(), Vec::new())
    } else {
        (
            unit_entries_of(value),
            preprocess::entries_of(value).map_or_else(Vec::new, |(_, entries)| entries),
        )
    };
    VALUE_ENTRIES.with(|recorded| {
        if units.is_empty() {
            recorded.borrow_mut().remove(rust_ident);
        } else {
            recorded.borrow_mut().insert(rust_ident.to_owned());
        }
    });
    note_unit_entries(rust_ident, units);
    preprocess::note_entries(rust_ident, entries);
}

/// Records `target`, the target of the alias `rust_ident` and what its types are named from, for a
/// holder written after it: `None` for an alias that holds no array to its length.
fn note_counted_alias(rust_ident: &str, target: Option<(FieldDef, String)>) {
    COUNTED_ALIASES.with(|recorded| {
        if let Some(noted) = target {
            recorded.borrow_mut().insert(rust_ident.to_owned(), noted);
        } else {
            recorded.borrow_mut().remove(rust_ident);
        }
    });
}

/// Records whether Swift holds the target of the alias `rust_ident` as an optional.
fn note_optional_alias(rust_ident: &str, optional: bool) {
    OPTIONAL_ALIASES.with(|recorded| {
        if optional {
            recorded.borrow_mut().insert(rust_ident.to_owned());
        } else {
            recorded.borrow_mut().remove(rust_ident);
        }
    });
}

/// Whether Swift holds a value of `def` as an optional: an `Option`, or an alias the emitter wrote
/// above of one.
fn held_as_optional(def: &FieldDef) -> bool {
    def.is_optional()
        || def.array_depth == 0
            && matches!(&def.field_type, FieldDefType::SiblingType(name, _)
                if OPTIONAL_ALIASES.with(|recorded| recorded.borrow().contains(name)))
}

/// `def` with the alias its leaf names written out as the alias's target, the levels `def` writes
/// around the alias outermost, read through the target's leaf: `None` where its leaf names no alias
/// the emitter noted as holding an array to its length.
fn through_alias(def: &FieldDef) -> Option<Shaped> {
    let FieldDefType::SiblingType(name, arguments) = &def.field_type else {
        return None;
    };
    if !arguments.is_empty() {
        return None;
    }
    let (target, hint) = COUNTED_ALIASES.with(|recorded| recorded.borrow().get(name).cloned())?;
    let below = target.array_depth;
    let mut composed = target;
    composed.array_depth = below.saturating_add(def.array_depth);
    composed.nullable_levels.extend(
        def.nullable_levels
            .iter()
            .map(|level| level.saturating_add(below)),
    );
    composed.array_lengths.extend(
        def.array_lengths
            .iter()
            .map(|&(level, length)| (level.saturating_add(below), length)),
    );
    composed.set_levels.extend(
        def.set_levels
            .iter()
            .map(|level| level.saturating_add(below)),
    );
    composed.name.clone_from(&def.name);
    let (_, shape) = swift_resolve(&composed, &hint, &mut Vec::new());
    Some(Shaped {
        def: composed,
        hint,
        shape,
    })
}

/// The Swift tokens `item` earns, given what `#[model_schema(...)]` says of it. Dispatches on the
/// item's own shape; an item this module has nothing to say about earns nothing.
pub fn swift_schema_dispatch(item: &Item, args: &SwiftTypeArgs<'_>) -> TokenStream {
    forwarded::record(item, args);
    let tokens = if let Item::Struct(item_struct) = item {
        struct_swift_tokens(item_struct, args)
    } else if let Item::Enum(item_enum) = item {
        enum_swift_tokens(item_enum, args)
    } else if let Item::Type(item_type) = item {
        alias_swift_tokens(item_type, args.name_override)
    } else {
        TokenStream::new()
    };
    forwarded::clear();
    tokens
}

/// Whether `field`'s own declared type has no Swift mapping: a `u64`.
pub const fn refuses_swift(field: &FieldDef) -> bool {
    matches!(field.field_type, FieldDefType::U64)
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
    let full_source = format!("{}{swift_source}", forwarded::take());
    let written = Ident::new("written", proc_macro2::Span::call_site());
    let escaped = [('\\', "\\\\"), ('"', "\\\"")];
    let (beside, keys) = unmet_keys_filled(
        &written,
        &module_ident,
        |text| format!("\"{}\"", swift_literal_text(text)),
        '"',
        &escaped,
    );
    if holder.is_empty() && keys.is_empty() {
        return quote! {
            pub mod #module_ident {
                pub fn swift_definition() -> ::std::string::String {
                    #full_source.to_owned()
                }
            }
        };
    }
    let mut replaced = if holder.is_empty() {
        TokenStream::new()
    } else {
        substituted(&quote! { SerdeDefaults }, &written, &escaped)
    };
    replaced.extend(keys);
    let values = defaults.values_impl(&module_ident);
    quote! {
        pub mod #module_ident {
            #holder

            pub fn swift_definition() -> ::std::string::String {
                let mut #written = ::std::string::String::from(#full_source);
                #replaced
                #written
            }
        }

        #values
        #beside
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

/// `name`, a key or a name serde writes, as a Swift string literal, which reads `\` as an escape
/// and `"` as its end.
pub fn swift_quoted(name: &str) -> String {
    format!("\"{}\"", swift_literal_text(name))
}

/// `name` as a property of an emitted struct: [`swift_member`], and a property named `validate` or
/// `validationIssues` moved off the methods an emitted type has, and one named `draft` off the
/// valid type's own property. The key on the wire stays the one
/// serde writes.
fn swift_property(name: &str) -> String {
    if matches!(name, "validate" | "validationIssues" | "draft" | "valid") {
        format!("{name}_")
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
        def.reach_type_parameters(&mut reached);
    }
    reached
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
    match &keys::peeled(key).field_type {
        FieldDefType::Boolean => "Bool".to_owned(),
        FieldDefType::Char | FieldDefType::String | FieldDefType::StringLiteral(_) => {
            "String".to_owned()
        }
        FieldDefType::U8 => "UInt8".to_owned(),
        FieldDefType::U16 => "UInt16".to_owned(),
        FieldDefType::U32 => "UInt32".to_owned(),
        FieldDefType::Usize => "UInt64".to_owned(),
        FieldDefType::I8 => "Int8".to_owned(),
        FieldDefType::I16 => "Int16".to_owned(),
        FieldDefType::I32 => "Int32".to_owned(),
        FieldDefType::I64 => "Int64".to_owned(),
        FieldDefType::Isize => "Int".to_owned(),
        FieldDefType::F32 => "Float".to_owned(),
        FieldDefType::F64 => "Double".to_owned(),
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
        | FieldDefType::Range(_, _)
        | FieldDefType::Result(_, _)
        | FieldDefType::Tuple(_)
        | FieldDefType::U64 => "String".to_owned(),
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
/// back into that `String` from `held` — the pair mirrors
/// `dart::dart_map_key_decode`/`dart_map_key_encode`. A brand is read by [`read_map_key`] and
/// written by [`write_map_key`].
fn swift_map_key_codec(key: &FieldDef, held: &str) -> (Option<String>, String) {
    match &key.field_type {
        FieldDefType::Boolean => (
            Some("(wireKey == \"true\" ? true : wireKey == \"false\" ? false : nil)".to_owned()),
            format!("({held} ? \"true\" : \"false\")"),
        ),
        FieldDefType::SiblingType(name, _) => {
            let type_name = lookup_swift_name(name).unwrap_or_else(|| name.clone());
            (
                Some(format!("{type_name}(rawValue: wireKey)")),
                format!("{held}.rawValue"),
            )
        }
        // A number is parsed from `number`, the text of the number serde reads off the key, which
        // the wrapper binds ahead of the parse.
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::Usize
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Isize => {
            let swift_type = swift_map_key_scalar(key);
            (
                Some(format!("{swift_type}(number)")),
                format!("String({held})"),
            )
        }
        FieldDefType::F32 | FieldDefType::F64 => {
            let swift_type = swift_map_key_scalar(key);
            (
                Some(format!("{swift_type}(number)")),
                format!("\"\\({held})\""),
            )
        }
        FieldDefType::Char => (None, forms::sent_call(TextForm::Char, held)),
        FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Map(_, _)
        | FieldDefType::Range(_, _)
        | FieldDefType::Result(_, _)
        | FieldDefType::Tuple(_)
        | FieldDefType::U64 => (None, held.to_owned()),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => (None, held.to_owned()),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => (
            None,
            forms::held_as_text(key)
                .map_or_else(|| held.to_owned(), |form| forms::sent_call(form, held)),
        ),
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => (
            Some(
                "Self.isoFormatterForKeys.date(from: wireKey) ?? \
                 Self.isoWholeFormatterForKeys.date(from: wireKey)"
                    .to_owned(),
            ),
            format!("Self.isoFormatterForKeys.string(from: {held}){CHRONO_WHOLE_SECONDS}"),
        ),
    }
}

/// The read of `key` from `wireKey`: through the alias it names, and through the `value` of the
/// brand it names.
fn read_map_key(written: &FieldDef) -> KeyRead {
    let key = keys::peeled(written);
    if let Some((rust_ident, value)) = keys::brand_of(&key) {
        let name = lookup_swift_name(&rust_ident).unwrap_or(rust_ident);
        return match read_map_key(&value) {
            KeyRead::Whole(read) => KeyRead::Whole(format!("{name}(value: {read})")),
            KeyRead::Parsed(read) => {
                KeyRead::Parsed(format!("(({read}).map {{ {name}(value: $0) }})"))
            }
        };
    }
    if let Some(form) = forms::held_as_text(&key) {
        return KeyRead::Whole(forms::read_call(
            form,
            "wireKey",
            "container.codingPath + [codingKey]",
        ));
    }
    swift_map_key_codec(&key, "key")
        .0
        .map_or_else(|| KeyRead::Whole("wireKey".to_owned()), KeyRead::Parsed)
}

/// The text of `held`, a key of type `key`, which a map's wrapper writes as the key of an entry.
fn write_map_key(written: &FieldDef, held: &str) -> String {
    let key = keys::peeled(written);
    if let Some((_, value)) = keys::brand_of(&key) {
        return write_map_key(&value, &format!("{held}.value"));
    }
    swift_map_key_codec(&key, held).1
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
    let encode_key = write_map_key(key, "key");
    let bound_key = wrapper_bound_key(key);
    let own_read = unjudged.map_or_else(String::new, |entry| {
        format!(
            "{}{} {{ {} }}; ",
            read::any_key(entry),
            read::UNJUDGED,
            wrapper_entries(key_type, value_type, &bound_key, entry, ""),
        )
    });
    let entries = wrapper_entries(
        key_type,
        value_type,
        &bound_key,
        &wrapper_plain_entry(value, value_type),
        "",
    );
    let written = value.written("value");
    // Static, as the read uses it before `storage` is set, and computed, as a generic type stores
    // no static property and a stored `ISO8601DateFormatter` is not `Sendable`.
    #[cfg(feature = "chrono")]
    let formatter_decl = if matches!(keys::leaf(key).field_type, FieldDefType::DateTime) {
        format!(
            "private static var isoFormatterForKeys: ISO8601DateFormatter {{ let f = ISO8601DateFormatter(); f.formatOptions = [{FRACTIONAL_OPTIONS}]; return f }}; \
             private static var isoWholeFormatterForKeys: ISO8601DateFormatter {{ let f = ISO8601DateFormatter(); f.formatOptions = [{WHOLE_OPTIONS}]; return f }};"
        )
    } else {
        String::new()
    };
    #[cfg(not(feature = "chrono"))]
    let formatter_decl = String::new();
    let mut helpers = read::helpers_for(&format!("{own_read}{entries}"));
    if bound_key.contains("swiftSchemaKeyNumber(") {
        helpers.push_str(&key_number_read());
    }
    format!(
        "public struct {wrapper_name}{generic_params}: Codable, Sendable {{ \
         public var storage: [{key_type}: {value_type}]; \
         {formatter_decl} {helpers}\
         private struct DynamicCodingKey: CodingKey {{ \
         var stringValue: String; init?(stringValue: String) {{ self.stringValue = stringValue }}; \
         var intValue: Int? {{ nil }}; init?(intValue: Int) {{ nil }} }}; \
         public init(storage: [{key_type}: {value_type}]) {{ self.storage = storage }}; \
         {own_read}\
         public init(from decoder: Decoder) throws {{ {entries} }}; \
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: DynamicCodingKey.self); \
         for (key, value) in storage {{ \
         let wireKey = {encode_key}; \
         guard let codingKey = DynamicCodingKey(stringValue: wireKey) else {{ continue }}; \
         try container.encode({written}, forKey: codingKey) }} }} }};"
    )
}

/// What a map's wrapper binds the key of one entry with, read from `wireKey` as `key` is.
fn wrapper_bound_key(key: &FieldDef) -> String {
    let inner = judged_inner_brands(key);
    match read_map_key(key) {
        KeyRead::Whole(read) => format!("let key = {read}; {inner}"),
        KeyRead::Parsed(parsed) => keys::leaf(key).field_type.number_key().map_or_else(
            || {
                format!(
                    "guard let key = {parsed} else {{ \
                     throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, debugDescription: \"bad key \\(wireKey)\") }}; {}{inner}",
                    refused_repeat_of_a_date_key(key)
                )
            },
            |number| format!("{}{inner}", number_bound_key(number, &parsed)),
        ),
    }
}

/// What judges each brand the brand of `key` holds, the innermost first, refusing the entry: serde
/// reads each through its own judged read, and the key's own `validate()`, which the holder's
/// walk runs, runs its own check alone, as the Rust brand's does.
fn judged_inner_brands(key: &FieldDef) -> String {
    let mut judged = Vec::new();
    let mut held = "key".to_owned();
    let mut below = keys::brand_of(key).map(|(_, value)| value);
    while let Some((rust_ident, value)) = below.as_ref().and_then(keys::brand_of) {
        held.push_str(".value");
        if validate::validates(&rust_ident) {
            judged.push(format!(
                "do {{ let violations = {held}.validate(); if !violations.isEmpty {{ \
                 throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
                 debugDescription: violations.joined(separator: \"; \")) }} }}; "
            ));
        }
        below = Some(value);
    }
    judged.reverse();
    judged.concat()
}

/// The static function a map's wrapper reads the number of a key's text with, as serde reads the
/// text between a key's quotes: the text of the number and whether serde reads it as a float. A text
/// that is no number, and a float beyond a `Double`, are refused in serde's words for text that is
/// not the document; what follows the number is the caller's to refuse, after the type refuses the
/// number.
fn key_number_read() -> String {
    let refused = |said: &str| format!("throw refused({})", swift_quoted(said));
    format!(
        "private static func swiftSchemaKeyNumber(_ text: String, at codingPath: [CodingKey]) \
         throws -> (String, Bool) {{ let bytes = Array(text.utf8); func refused(_ said: String) -> \
         DecodingError {{ DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         codingPath, debugDescription: said)) }}; func digit(_ at: Int) -> Bool {{ at < \
         bytes.count && bytes[at] >= 48 && bytes[at] <= 57 }}; func digits(_ from: Int) throws -> \
         Int {{ guard digit(from) else {{ {invalid} }}; var at = from; while digit(at) {{ at += 1 \
         }}; return at }}; guard let first = bytes.first, first == 45 || (48...57).contains(first) \
         else {{ {numeric} }}; let sign = first == 45 ? 1 : 0; let zero = sign < bytes.count && \
         bytes[sign] == 48; var at = try (zero ? sign + 1 : digits(sign)); if zero && digit(at) {{ \
         {invalid} }}; var float = false; if at < bytes.count && bytes[at] == 46 {{ at = try \
         digits(at + 1); float = true }}; if at < bytes.count && (bytes[at] == 101 || bytes[at] == \
         69) {{ at = try digits(at + 1 < bytes.count && (bytes[at + 1] == 43 || bytes[at + 1] == \
         45) ? at + 2 : at + 1); float = true }}; let number = String(decoding: bytes[..<at], as: \
         UTF8.self); let wide = float || (sign == 1 ? number == \"-0\" || Int64(number) == nil : \
         UInt64(number) == nil); if wide && !(Double(number)?.isFinite ?? false) {{ {beyond} }}; \
         return (number, wide) }}; ",
        invalid = refused(INVALID_NUMBER),
        numeric = refused(EXPECTED_NUMERIC_KEY),
        beyond = refused(NUMBER_OUT_OF_RANGE),
    )
}

/// What a map's wrapper binds a key read as `number` with, from the text serde reads between the
/// key's quotes: the number, then the type's refusal of it, then the text after it. A refusal serde
/// classifies as text that is not the document is in its words; a float for an integer key is
/// refused at the map, and an integer beyond the type under the number. The wrapper declares
/// [`key_number_read`].
fn number_bound_key(number: NumberKey, parsed: &str) -> String {
    let refused = |path: &str, said: &str| {
        format!(
            "throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: {path}, \
             debugDescription: {said}))"
        )
    };
    let read = |wide: &str| {
        format!(
            "let (number, {wide}) = try Self.swiftSchemaKeyNumber(wireKey, at: \
             container.codingPath); "
        )
    };
    let trailing = format!(
        "if number.utf8.count < wireKey.utf8.count {{ {} }}; ",
        refused("container.codingPath", &swift_quoted(EXPECTED_QUOTE))
    );
    let expected = number.expected;
    if number.range.is_none() {
        return format!(
            "{}{trailing}guard let key = {parsed} else {{ throw \
             DecodingError.dataCorruptedError(forKey: codingKey, in: container, debugDescription: \
             \"bad key \\(wireKey)\") }}; ",
            read("_")
        );
    }
    format!(
        "{}if wide {{ {} }}; guard let key = {parsed} else {{ {} }}; {trailing}",
        read("wide"),
        refused(
            "container.codingPath",
            &format!("\"invalid type: floating point `\\(number)`, expected {expected}\"")
        ),
        refused(
            "container.codingPath + [DynamicCodingKey(stringValue: number)! as CodingKey]",
            &format!("\"invalid value: integer `\\(number)`, expected {expected}\"")
        ),
    )
}

/// A date key Swift holds to its millisecond: two keys finer than that read as one, so the read
/// refuses the second instead of dropping an entry.
#[cfg(feature = "chrono")]
fn refused_repeat_of_a_date_key(key: &FieldDef) -> &'static str {
    if matches!(keys::leaf(key).field_type, FieldDefType::DateTime) {
        "if result.keys.contains(key) { \
         throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, debugDescription: \"repeated key \\(wireKey)\") }; "
    } else {
        ""
    }
}

#[cfg(not(feature = "chrono"))]
const fn refused_repeat_of_a_date_key(_key: &FieldDef) -> &'static str {
    ""
}

/// Where a map's wrapper reads the value of one entry from.
fn wrapper_entry_source() -> read::Source {
    read::Source::Keyed {
        container: "container".to_owned(),
        key: "codingKey".to_owned(),
        presence: read::Presence::Known,
    }
}

/// The read of one entry's `value` by a map's wrapper, as the `value_type` it is declared as.
fn wrapper_plain_entry(value: &Shaped, value_type: &str) -> String {
    value.read(
        value_type,
        "value",
        &wrapper_entry_source(),
        &|held| read::store_entry("result", "key", held, &value.def),
        &mut read::Locals::default(),
    )
}

/// The read of one entry's value by a map's wrapper where `validate()` walks the `value` the map
/// holds: unjudged, its types named from `value_hint`. `None` where the walk reaches nothing.
fn wrapper_unjudged_entry(value: &FieldDef, value_hint: &str) -> Option<String> {
    validate::reaches(value).then(|| {
        read::unjudged(
            value,
            value_hint,
            &wrapper_entry_source(),
            &|held| read::store_entry("result", "key", held, value),
            &mut read::Locals::default(),
        )
    })
}

/// The statements a map's wrapper reads its entries into `storage` with: each key bound by
/// `bound_key` and each value read by `entry`, under every key but those `skipped`, a `where`
/// clause, leaves out.
fn wrapper_entries(
    key_type: &str,
    value_type: &str,
    bound_key: &str,
    entry: &str,
    skipped: &str,
) -> String {
    format!(
        "let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
         var result: [{key_type}: {value_type}] = [:]; \
         for codingKey in container.allKeys{skipped} {{ \
         let wireKey = codingKey.stringValue; \
         {bound_key}{entry} }}; \
         storage = result"
    )
}

/// The read the wrapper of a map under a key Swift wraps is given where the member `def` describes
/// holds the map flattened, the wrapper named from `hint`: an initializer, in an extension, that
/// reads no entry under a key the types around it took. `None` for a member that holds no such
/// map.
fn flattened_wrapper_read(def: &FieldDef, hint: &str) -> Option<String> {
    let held = held_map(def)?;
    let FieldDefType::Map(key, value) = &held.field_type else {
        return None;
    };
    if !matches!(map_keys(key), MapKeys::Wrapped) {
        return None;
    }
    let value_hint = format!("{hint}Value");
    let entry_def = arrayed((**value).clone());
    let (value_type, shape) = swift_resolve(&entry_def, &value_hint, &mut Vec::new());
    let entry = wrapper_unjudged_entry(value, &value_hint).unwrap_or_else(|| {
        wrapper_plain_entry(
            &Shaped {
                def: entry_def,
                hint: value_hint.clone(),
                shape,
            },
            &value_type,
        )
    });
    let (wrapper, _) = keyed_map_wrapper(hint, key, value);
    Some(format!(
        "extension {wrapper} {{ {} {{ {} }} }};",
        read::TAKING,
        wrapper_entries(
            &swift_map_key_scalar(key),
            &value_type,
            &wrapper_bound_key(key),
            &entry,
            " where !taken.contains(codingKey.stringValue)",
        ),
    ))
}

/// The Swift tuple-slot struct a `Tuple` field or a `TupleMultiple` variant payload earns: no
/// bare Swift tuple conforms to `Codable`, so each occurrence gets its own unkeyed-container
/// struct, named after the field it belongs to so two occurrences never collide. `elements` is
/// each slot's type, and what the slot is written as. Its read refuses an array of another length,
/// a longer one among them, as serde and Zod refuse it.
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
    let length = elements.len();
    // The struct of a tuple struct of no slot reads and writes nothing through its container.
    let (read_through, written) = if elements.is_empty() {
        ("let", "_ = encoder.unkeyedContainer();".to_owned())
    } else {
        (
            "var",
            format!("var container = encoder.unkeyedContainer(); {encode_calls}"),
        )
    };
    let both = read::both(
        &format!(
            "{read_through} container = try decoder.unkeyedContainer(); if let count = \
             container.count, count != {length} {{ throw DecodingError.dataCorrupted(\
             DecodingError.Context(codingPath: container.codingPath, debugDescription: \"invalid \
             length \\(count), expected {length}\")) }}; {reads}"
        ),
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
         {written} }}; {validate} }};"
    )
}

/// The struct `{name_hint}Tuple` a `()` leaf is held as: read from the `null` serde writes for
/// `()` and from nothing else, and written as `null`.
fn unit_shape(name_hint: &str, aux: &mut Vec<String>) -> SwiftFieldShape {
    let struct_name = format!("{name_hint}Tuple");
    let both = read::both(
        "let unit = try decoder.singleValueContainer(); guard unit.decodeNil() else { let found \
         = try JSONSerialization.data(withJSONObject: SwiftSchemaPayload(from: decoder).value, \
         options: [.fragmentsAllowed, .sortedKeys, .withoutEscapingSlashes]); throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: \"invalid type: \\(String(decoding: found, as: UTF8.self)), expected \
         unit\")) }",
        &preprocess::Written {
            named: &struct_name,
            ..preprocess::Written::default()
        },
    );
    aux.push(format!(
        "public struct {struct_name}: Codable, Sendable {{ public init() {{}}; {both}\
         public func encode(to encoder: Encoder) throws {{ var container = \
         encoder.singleValueContainer(); try container.encodeNil() }}; {} }};",
        validate::method(&[])
    ));
    direct_shape(struct_name)
}

/// `field`'s shape before array/optional wrapping: the property type, the type a `container` call
/// decodes, and what turns the second into the first, generating an auxiliary struct into `aux`
/// when the leaf needs one of its own.
fn swift_field_shape(field: &FieldDef, name_hint: &str, aux: &mut Vec<String>) -> SwiftFieldShape {
    match &field.field_type {
        // A type written as no path, a function pointer or a macro among them.
        FieldDefType::Unknown => direct_shape("String".to_owned()),
        FieldDefType::TypeParam(name) => direct_shape(name.clone()),
        FieldDefType::Tuple(elements) if elements.is_empty() => unit_shape(name_hint, aux),
        FieldDefType::Tuple(elements) => tuple_shape(elements, name_hint, aux),
        FieldDefType::Range(kind, bound) => range_shape(*kind, bound, name_hint, aux),
        FieldDefType::Result(ok, err) => result_shape(ok, err, name_hint, aux),
        FieldDefType::SiblingType(name, generics) => sibling_shape(name, generics, name_hint, aux),
        FieldDefType::Map(key, value) => map_shape(key, value, name_hint, aux),
        FieldDefType::Boolean | FieldDefType::BooleanLiteral(_) => direct_shape("Bool".to_owned()),
        FieldDefType::String | FieldDefType::StringLiteral(_) => direct_shape("String".to_owned()),
        FieldDefType::Char => text_shape(TextForm::Char),
        FieldDefType::NumberLiteral(_) | FieldDefType::F64 => direct_shape("Double".to_owned()),
        FieldDefType::F32 => direct_shape("Float".to_owned()),
        FieldDefType::U8 => direct_shape("UInt8".to_owned()),
        FieldDefType::U16 => direct_shape("UInt16".to_owned()),
        FieldDefType::U32 => direct_shape("UInt32".to_owned()),
        FieldDefType::U64 | FieldDefType::Usize => direct_shape("UInt64".to_owned()),
        FieldDefType::I8 => direct_shape("Int8".to_owned()),
        FieldDefType::I16 => direct_shape("Int16".to_owned()),
        FieldDefType::I32 => direct_shape("Int32".to_owned()),
        FieldDefType::I64 => direct_shape("Int64".to_owned()),
        FieldDefType::Isize => direct_shape("Int".to_owned()),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => direct_shape("ObjectId".to_owned()),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            forms::held_as_text(field).map_or_else(|| direct_shape("String".to_owned()), text_shape)
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => datetime_shape(field),
    }
}

/// A text of one form, held as a `String` and read and written through the functions of its form.
fn text_shape(form: TextForm) -> SwiftFieldShape {
    SwiftFieldShape {
        leaf_conversion: LeafConversion::Text(form),
        real_leaf: "String".to_owned(),
        wire_leaf: "String".to_owned(),
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

/// The `DateTime<Tz>` leaf: `Date` is the real type, the ISO 8601 string chrono writes is the wire
/// type. Under `as_number` both are the `Int64` of epoch milliseconds serde writes.
#[cfg(feature = "chrono")]
fn datetime_shape(field: &FieldDef) -> SwiftFieldShape {
    if has_as_number(field) {
        direct_shape("Int64".to_owned())
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
            let walked = validate::walks(element);
            let functions = preprocess::functions(element);
            let read = if !functions.is_empty() {
                corrected_member(
                    &preprocess::Member {
                        def: element,
                        flat: false,
                        functions,
                        hint: &hint,
                        keys: "",
                        plain: preprocess::Plain::Declared,
                        real_type: &element_type,
                        refusing: false,
                        stem: &stem,
                        units: &[],
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
                slot.read(&element_type, &stem, &source, &held, &mut locals)
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
    let helpers = leaf_helpers_for(&struct_name, &generic_params, &shapes, None);
    if !helpers.is_empty() {
        aux.push(helpers);
    }
    direct_shape(held_at(&struct_name, &generic_params))
}

/// A std range leaf: a struct of its own, `{hint}Range`, holds each bound serde writes under its
/// key, read and written through the bound's shape, and walked by its `validate()` under that key.
/// Every key is required, and any other key refused, as serde reads them. The struct declares the
/// parameters its bound reaches.
fn range_shape(
    kind: RangeKind,
    bound: &FieldDef,
    name_hint: &str,
    aux: &mut Vec<String>,
) -> SwiftFieldShape {
    let struct_name = format!("{name_hint}Range");
    let hint = format!("{struct_name}Bound");
    let (real_type, shape) = swift_resolve(bound, &hint, aux);
    let fields: Vec<SwiftField> = kind
        .members()
        .iter()
        .map(|member| {
            let mut field_def = arrayed(bound.clone());
            (*member).clone_into(&mut field_def.name);
            SwiftField {
                aliases: Vec::new(),
                default_at: None,
                field_def,
                flatten: false,
                hint: hint.clone(),
                held: Storage::Direct,
                never_written: false,
                real_type: real_type.clone(),
                required: true,
                shape: shape.clone(),
                swift_name: (*member).to_owned(),
                wire_name: (*member).to_owned(),
            }
        })
        .collect();
    let generic_params = bounded_generic_params(&parameters_reached(&[bound]));
    let props = fields.iter().fold(String::new(), |mut acc, field| {
        write!(
            acc,
            "public let {}: {}; ",
            field.swift_name, field.real_type
        )
        .unwrap();
        acc
    });
    let mut locals = read::Locals::default();
    let mut twins = Vec::new();
    let members = fields
        .iter()
        .map(|field| {
            decode_statement(
                field,
                "container",
                &Handed::default(),
                &mut locals,
                &mut twins,
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let written = kind
        .members()
        .iter()
        .map(|member| swift_quoted(member))
        .collect::<Vec<_>>()
        .join(", ");
    // Swift's own read of an enum refuses an object of the wrong keys as a type mismatch.
    let only_written = format!(
        "let wireKeys = try decoder.container(keyedBy: SwiftSchemaAnyKey.self); \
         if wireKeys.allKeys.contains(where: {{ ![{written}].contains($0.stringValue) }}) {{ \
         throw DecodingError.typeMismatch(Self.self, DecodingError.Context(codingPath: \
         decoder.codingPath, debugDescription: \"Unknown key of a {}: \\(wireKeys.allKeys.map \
         {{ $0.stringValue }})\")) }}",
        kind.name()
    );
    let reads = read::both(
        &format!(
            "{only_written}; let container = try decoder.container(keyedBy: CodingKeys.self); \
             {members}"
        ),
        &preprocess::Written {
            named: &struct_name,
            twins,
            ..preprocess::Written::default()
        },
    );
    let encodes = fields
        .iter()
        .map(|field| encode_statement(field, "container"))
        .collect::<Vec<_>>()
        .join("; ");
    aux.push(format!(
        "public struct {struct_name}{generic_params}: Codable, Sendable {{ {props}{} {}{reads}\
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: CodingKeys.self); {encodes} }}; {} }};",
        swift_coding_keys(&fields, &[]),
        memberwise_initializer(&fields),
        validate::method(&validate::range_rules(&struct_name, kind, bound)),
    ));
    let helpers = leaf_helpers_for(&struct_name, &generic_params, &[&shape], None);
    if !helpers.is_empty() {
        aux.push(helpers);
    }
    direct_shape(held_at(&struct_name, &generic_params))
}

/// The case of the enum written for a `Result` that holds the arm serde writes under `arm`.
fn result_case(arm: &str) -> String {
    arm.to_lowercase()
}

/// The statements that bind `container`, the keyed container of `key_type` the decoder holds, and
/// `key`, its one key, as serde reads an externally tagged enum and a `Result`: a value that is
/// neither a string nor an object of one key is refused in serde's words for text that is not the
/// document, and a string as Foundation refuses it.
fn one_key_container(key_type: &str) -> String {
    format!(
        "guard let container = try? decoder.container(keyedBy: {key_type}.self), \
         container.allKeys.count == 1, let key = container.allKeys.first else {{ if (try? \
         decoder.singleValueContainer().decode(String.self)) != nil {{ _ = try \
         decoder.container(keyedBy: {key_type}.self) }}; throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: \"{EXPECTED_VALUE}\")) }}; "
    )
}

/// The statement that refuses `tag`, a string, as a unit variant where the variant it names carries
/// a value, as serde refuses it: under the variant's own step, `expected` being what the variant is.
fn unit_for_data(key_type: &str, expected: &str) -> String {
    format!(
        "throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath + \
         [{key_type}(stringValue: tag)!], debugDescription: \"invalid type: unit variant, expected \
         {expected}\"))"
    )
}

/// The arm of the string read of an externally tagged enum that refuses a string naming `variant`
/// where it carries a value, and none for a unit variant.
fn carried_unit_arm(variant: &Variant, wire_tag: &str) -> String {
    variant_wire_kind(variant)
        .carried_as()
        .map_or_else(String::new, |expected| {
            format!(
                "case {}: {}; ",
                aliases::case_names(wire_tag, &aliases::written_on(&variant.attrs)),
                unit_for_data("SwiftSchemaExternalCodingKey", expected)
            )
        })
}

/// A `Result` leaf: an enum of its own, `{hint}Result`, of one case per arm, read from and written
/// as the object whose one key names the arm, and walked by its `validate()` under that key. An
/// arm of `()` is a case with no value, written as `null`. Any other value is refused, as serde
/// refuses it: one that is neither a string nor an object of one key in serde's words for text that
/// is not the document. The enum declares the parameters its arms reach.
fn result_shape(
    ok: &FieldDef,
    err: &FieldDef,
    name_hint: &str,
    aux: &mut Vec<String>,
) -> SwiftFieldShape {
    let enum_name = format!("{name_hint}Result");
    let mut cases = String::new();
    let mut reads = String::new();
    let mut encodes = String::new();
    let mut shapes = Vec::new();
    let mut locals = read::Locals::default();
    let refused = |said: &str| {
        format!(
            "throw DecodingError.typeMismatch(Self.self, DecodingError.Context(codingPath: \
             decoder.codingPath, debugDescription: \"{said}\"))"
        )
    };
    for (arm, def) in RESULT_ARMS.into_iter().zip([ok, err]) {
        let case = result_case(arm);
        let key = format!("SwiftSchemaAnyKey(stringValue: \"{arm}\")!");
        if def.is_unit() {
            write!(cases, "case {case}; ").unwrap();
            write!(
                reads,
                "case \"{arm}\": guard try container.decodeNil(forKey: key) else {{ {} }}; \
                 self = .{case}; ",
                refused(&format!("Expected null under {arm}"))
            )
            .unwrap();
            write!(
                encodes,
                "case .{case}: try container.encodeNil(forKey: {key}); "
            )
            .unwrap();
            continue;
        }
        let hint = format!("{enum_name}{arm}");
        let (held, slot) = slot_resolve(def, &hint, aux);
        write!(cases, "case {case}({held}); ").unwrap();
        let source = read::Source::Keyed {
            container: "container".to_owned(),
            key: "key".to_owned(),
            presence: read::Presence::Known,
        };
        let sink = |read: &str| format!("self = .{case}({read})");
        let read = if validate::walks(def) {
            read::unjudged(def, &hint, &source, &sink, &mut locals)
        } else {
            slot.read(&held, &case, &source, &sink, &mut locals)
        };
        write!(reads, "case \"{arm}\": {read}; ").unwrap();
        write!(
            encodes,
            "case .{case}(let payload): try container.encode({}, forKey: {key}); ",
            slot.written("payload")
        )
        .unwrap();
        shapes.push(slot.shape);
    }
    let generic_params = bounded_generic_params(&parameters_reached(&[ok, err]));
    let carried = RESULT_ARMS
        .into_iter()
        .zip([ok, err])
        .filter(|(_, def)| !def.is_unit())
        .fold(String::new(), |mut carried, (arm, _)| {
            let _ = write!(
                carried,
                "case \"{arm}\": {}; ",
                unit_for_data("SwiftSchemaAnyKey", "newtype variant")
            );
            carried
        });
    let both = read::both(
        &format!(
            "{}{}switch key.stringValue {{ {reads}default: {} }}",
            if carried.is_empty() {
                String::new()
            } else {
                format!(
                    "if let single = try? decoder.singleValueContainer(), let tag = try? \
                     single.decode(String.self) {{ switch tag {{ {carried}default: break }} }}; "
                )
            },
            one_key_container("SwiftSchemaAnyKey"),
            refused("Not one key, Ok or Err, of a Result")
        ),
        &preprocess::Written {
            named: &enum_name,
            ..preprocess::Written::default()
        },
    );
    aux.push(format!(
        "public enum {enum_name}{generic_params}: Codable, Sendable {{ {cases}{both}\
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: SwiftSchemaAnyKey.self); \
         switch self {{ {encodes}}} }}; {} }};",
        validate::method(&validate::result_rules(&enum_name, ok, err)),
    ));
    let held_shapes: Vec<&SwiftFieldShape> = shapes.iter().collect();
    let helpers = leaf_helpers_for(&enum_name, &generic_params, &held_shapes, None);
    if !helpers.is_empty() {
        aux.push(helpers);
    }
    direct_shape(held_at(&enum_name, &generic_params))
}

/// A slot, a value serde writes with no key, as it is declared and read: its Swift type, and its
/// shape.
fn slot_resolve(written: &FieldDef, name_hint: &str, aux: &mut Vec<String>) -> (String, Shaped) {
    let def = arrayed(written.clone());
    let (real_type, resolved) = swift_resolve(&def, name_hint, aux);
    let hint = name_hint.to_owned();
    (
        real_type,
        Shaped {
            def,
            hint,
            shape: resolved,
        },
    )
}

/// A `SiblingType` leaf: the referenced item's own published name, with generic arguments rendered
/// the same way a field's own type is. An argument whose read calls the date helpers, or holds an
/// array to its length, is held in a wrapper that does so, as the type it fills reads it through
/// `Decodable`. Every helper an argument needs is named from `name_hint`, the holder's field, so no
/// two holders declare one.
fn sibling_shape(
    name: &str,
    generics: &[FieldDef],
    name_hint: &str,
    aux: &mut Vec<String>,
) -> SwiftFieldShape {
    let class_name = lookup_swift_name(name).unwrap_or_else(|| name.to_owned());
    if generics.is_empty() {
        return direct_shape(class_name);
    }
    let arguments = generics
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            let def = arrayed(argument.clone());
            let hint = format!("{name_hint}Argument{index}");
            let (real_type, shape) = swift_resolve(&def, &hint, aux);
            converted_type(&real_type, &Shaped { def, hint, shape }, aux)
        })
        .collect::<Vec<_>>()
        .join(", ");
    direct_shape(format!("{class_name}<{arguments}>"))
}

/// Whether `def` is, or holds at any depth, a map Swift reads through a wrapper.
fn holds_wrapped_map(def: &FieldDef) -> bool {
    if let FieldDefType::Map(key, value) = &def.field_type {
        matches!(map_keys(key), MapKeys::Wrapped) || holds_wrapped_map(value)
    } else {
        false
    }
}

/// The Swift type a type argument or an alias of `held` names, `real_type` being its own: the
/// wrapper that reads the wire form where the value converts, and `real_type` where it does not.
fn converted_type(real_type: &str, held: &Shaped, aux: &mut Vec<String>) -> String {
    if read::holds_a_length(&held.def) || reads_a_date(&held.shape) {
        held_argument(real_type, held, aux)
    } else if holds_wrapped_map(&held.def) {
        wire_type(&held.def, &held.shape)
    } else {
        real_type.to_owned()
    }
}

/// The wrapper a type argument of the Swift type `real_type` is held in, pushed into `aux` with the
/// date helpers where its read and its write call them: a single value, read and written as the
/// type `argument` is written as. `{hint}Date` reads a date, and `{hint}Fixed` an array held to its
/// length, item by item. The wrapper declares the parameters `argument` reaches.
fn held_argument(real_type: &str, argument: &Shaped, aux: &mut Vec<String>) -> String {
    let counted = read::holds_a_length(&argument.def);
    let name = format!(
        "{}{}",
        argument.hint,
        if reads_a_date(&argument.shape) {
            "Date"
        } else {
            "Fixed"
        }
    );
    let parameters = parameters_reached(&[&argument.def]);
    let generic_params = bounded_generic_params(&parameters);
    let read = if counted {
        read::counted(
            argument,
            "value",
            &read::Source::Decoder,
            &|held| format!("self.value = {held}"),
            &mut read::Locals::default(),
        )
    } else {
        format!(
            "let container = try decoder.singleValueContainer(); \
             let wireValue = try {}; self.value = {}",
            read::fitted(
                "container.decode(",
                &wire_type(&argument.def, &argument.shape),
                ".self)",
                &parameters
            ),
            argument.converted("wireValue")
        )
    };
    let reads = read::both(
        &read,
        &preprocess::Written {
            named: &name,
            ..preprocess::Written::default()
        },
    );
    aux.push(format!(
        "public struct {name}{generic_params}: Codable, Sendable {{ \
         public let value: {real_type}; \
         public init(value: {real_type}) {{ self.value = value }}; \
         {reads}\
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.singleValueContainer(); try container.encode({}) }}; {} }};",
        argument.written("value"),
        validate::method(&[])
    ));
    let helpers = leaf_helpers_for(&name, &generic_params, &[&argument.shape], None);
    if !helpers.is_empty() {
        aux.push(helpers);
    }
    held_at(&name, &generic_params)
}

/// How Swift reads a map under `key`: a string-shaped key is the key of a native dictionary, and
/// any other is read through a wrapper.
fn map_keys(written: &FieldDef) -> MapKeys {
    let key = &keys::peeled(written);
    if !matches!(
        key.map_key_wire(),
        MapKeyWire::Named | MapKeyWire::Signed64 | MapKeyWire::Unsigned64
    ) {
        return MapKeys::Wrapped;
    }
    match key.field_type {
        FieldDefType::String | FieldDefType::StringLiteral(_) => MapKeys::Text,
        // Read and written through the functions of its form, which a native dictionary calls none
        // of.
        FieldDefType::Char => MapKeys::Wrapped,
        FieldDefType::SiblingType(_, _)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Map(_, _)
        | FieldDefType::Range(_, _)
        | FieldDefType::Result(_, _)
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
        | FieldDefType::Isize
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

/// A `Map` leaf: a string-shaped key stays a plain `[String: Value]`, natively `Codable`, read as
/// the map of what its values are written as where they convert; any other key earns a wrapper
/// struct.
fn map_shape(
    key: &FieldDef,
    value: &FieldDef,
    name_hint: &str,
    aux: &mut Vec<String>,
) -> SwiftFieldShape {
    let value_hint = format!("{name_hint}Value");
    let def = arrayed(value.clone());
    let (value_type, shape) = swift_resolve(&def, &value_hint, aux);
    let entry = Shaped {
        def,
        hint: value_hint.clone(),
        shape,
    };
    let native = match map_keys(key) {
        MapKeys::Text => Some("String"),
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
    let unjudged = wrapper_unjudged_entry(value, &value_hint);
    aux.push(map_key_wrapper_struct(
        &wrapper_name,
        &generic_params,
        &key_type,
        &value_type,
        key,
        &entry,
        unjudged.as_deref(),
    ));
    let helpers = leaf_helpers_for(
        &wrapper_name,
        &generic_params,
        &[&entry.shape],
        forms::held_as_text(&keys::leaf(key)),
    );
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
        LeafConversion::Text(form) => forms::read_call(*form, expr, "decoder.codingPath"),
        LeafConversion::Values(value) => format!(
            "{}{expr}.mapValues {{ {} }}",
            tried(shape),
            converted_from(&value.def, &value.shape, "$0")
        ),
    }
}

/// Whether turning a value through `conversion` throws: the parse of a date does, at any depth of
/// the values of a map Swift keys natively, and the storage of a map's wrapper does not.
fn throws(conversion: &LeafConversion) -> bool {
    match conversion {
        #[cfg(feature = "chrono")]
        LeafConversion::DateTimePlain => true,
        LeafConversion::Text(_) => true,
        LeafConversion::Values(value) => throws(&value.shape.leaf_conversion),
        LeafConversion::MapWrapper(_) | LeafConversion::None => false,
    }
}

/// Whether writing a value through `conversion` throws: the write of a text of one form does, at
/// any depth of the values of a map Swift keys natively.
fn writes_throw(conversion: &LeafConversion) -> bool {
    match conversion {
        LeafConversion::Text(_) => true,
        LeafConversion::Values(value) => writes_throw(&value.shape.leaf_conversion),
        #[cfg(feature = "chrono")]
        LeafConversion::DateTimePlain => false,
        LeafConversion::MapWrapper(_) | LeafConversion::None => false,
    }
}

/// `written`, the write of a value of `shape` over a closure, behind the `try` it needs where the
/// write throws, in parentheses, so that a `try` written around it reaches nothing it covers.
fn write_tried(shape: &SwiftFieldShape, written: String) -> String {
    if writes_throw(&shape.leaf_conversion) {
        format!("(try {written})")
    } else {
        written
    }
}

/// `try ` ahead of a conversion of a value of `shape` that throws, and nothing ahead of one that
/// does not: Swift warns of a `try` over a call that throws nothing.
fn tried(shape: &SwiftFieldShape) -> &'static str {
    if throws(&shape.leaf_conversion) {
        "try "
    } else {
        ""
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
        LeafConversion::Text(form) => forms::sent_call(*form, expr),
        LeafConversion::Values(value) => write_tried(
            shape,
            format!(
                "{expr}.mapValues {{ {} }}",
                written_from(&value.def, &value.shape, "$0")
            ),
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
    let marker = tried(shape);
    if field.is_nullable_at(inner_level) {
        format!("{marker}{expr}.map {{ {marker}$0.map {{ {inner} }} }}")
    } else {
        format!("{marker}{expr}.map {{ {inner} }}")
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
        write_tried(
            shape,
            format!(
                "{expr}.map {{ {} }}",
                write_tried(shape, format!("$0.map {{ {inner} }}"))
            ),
        )
    } else {
        write_tried(shape, format!("{expr}.map {{ {inner} }}"))
    }
}

/// What `local`, a value of `def` read as [`wire_type`], is turned into: a value of its own type.
fn converted_from(def: &FieldDef, shape: &SwiftFieldShape, local: &str) -> String {
    if matches!(shape.leaf_conversion, LeafConversion::None) {
        return local.to_owned();
    }
    if def.is_optional() {
        let inner = decode_conversion(def, shape, def.array_depth, "$0");
        format!("{}{local}.map {{ {inner} }}", tried(shape))
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
        write_tried(shape, format!("{held}.map {{ {inner} }}"))
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
/// field `validate()` walks is read unjudged, one that names a function from what it answers, and
/// one serde reads under an alias too under the name the payload holds it under. An optional
/// member takes an absent key as `nil`, as serde reads it, whether its key is required or not.
fn decode_statement(
    field: &SwiftField,
    container: &str,
    handed: &Handed,
    locals: &mut read::Locals,
    twins: &mut Vec<String>,
) -> String {
    let absent_is_nil = field.field_def.is_optional();
    let (keyed, key) = field.keyed_at(container);
    let source = if field.flatten {
        read::Source::Beside(handed.taken.clone())
    } else {
        read::Source::Keyed {
            container: keyed.clone(),
            key: key.clone(),
            presence: if absent_is_nil {
                read::Presence::Nil
            } else {
                read::Presence::Refused
            },
        }
    };
    let read_present = present_statement(
        field,
        &source,
        handed.withheld,
        absent_is_nil,
        container,
        locals,
        twins,
    );
    // A flattened enum places the read of its entry itself, past the locals the members after it
    // read.
    let present = if field.flatten && !read_present.contains("swiftSchemaFlattened(") {
        read::flattened(&read_present)
    } else {
        read_present
    };
    let read = match &field.default_at {
        Some(at) => {
            let path_key = key
                .strip_prefix('.')
                .map_or_else(|| key.clone(), |name| format!("CodingKeys.{name}"));
            format!(
                "if {keyed}.contains({key}) {{ {present} }} else {{ {} }}",
                default_statement(
                    field,
                    at,
                    &format!("{keyed}.codingPath + [{path_key}]"),
                    twins
                ),
            )
        }
        None => present,
    };
    if field.aliases.is_empty() {
        read
    } else {
        format!(
            "{}{read}",
            aliases::lookup(
                swift_bare(&field.swift_name),
                &field.wire_name,
                &field.aliases
            )
        )
    }
}

/// The statements that read `field` from `source`, where the payload holds its key or it has no
/// default. `withheld` says a flattened `field` is handed fewer keys than its holder's object holds.
fn present_statement(
    field: &SwiftField,
    source: &read::Source,
    withheld: bool,
    absent_is_nil: bool,
    container: &str,
    locals: &mut read::Locals,
    twins: &mut Vec<String>,
) -> String {
    let member = |held: &str| format!("self.{} = {held}", field.swift_name);
    let walked = validate::walks(&field.field_def);
    let functions = preprocess::functions(&field.field_def);
    if !functions.is_empty() {
        if field.flatten && field.field_def.is_optional() && held_map(&field.field_def).is_none() {
            return corrected_option(field, functions, source, twins);
        }
        return corrected_field(field, functions, walked, source, None, twins);
    }
    if field.flatten
        && let read::Source::Beside(taken) = source
        && let Some((named, entries)) = preprocess::entries_of(&field.field_def)
    {
        return flattened_enum(field, (named, &entries), taken, twins);
    }
    if field.flatten
        && let Some(read) = flattened_map(field, walked, source, locals)
    {
        return read;
    }
    if field.flatten && field.field_def.is_optional() {
        return flattened_option(field, source, withheld, container, locals, twins);
    }
    let only = preprocess::only_keys(&field.field_def);
    if field.flatten && !only.is_empty() {
        // Its read refuses any other key, which serde reads past where it is flattened.
        let kept = read::Source::Keeping(preprocess::listed(&only));
        return corrected_field(field, &[], walked, &kept, None, twins);
    }
    let beside = field.flatten && preprocess::beside(&field.field_def);
    // What fills a type parameter may refuse a key beside its own, which serde reads past here.
    let parameter = matches!(field.field_def.field_type, FieldDefType::TypeParam(_))
        && field.field_def.array_depth == 0;
    let read = if field.flatten
        && (withheld || parameter)
        && !beside
        && !preprocess::reads_the_whole(&field.field_def)
    {
        // A type that reads its holder's decoder sees every key, so it is read from the payload
        // without the keys taken.
        corrected_field(field, &[], walked, source, None, twins)
    } else if walked || beside {
        read::unjudged(&field.field_def, &field.hint, source, &member, locals)
    } else if field.flatten {
        format!(
            "self.{swift} = try {real_type}(from: decoder)",
            swift = field.swift_name,
            real_type = field.real_type,
        )
    } else if read::holds_a_length(&field.field_def) {
        let stem = swift_bare(&field.swift_name);
        read::counted(&field.shaped(), stem, source, &member, locals)
    } else {
        keyed_read(field, container, absent_is_nil)
    };
    if field.flatten
        && let Some((filled, keys)) = preprocess::filled_alone(&field.field_def, &field.real_type)
    {
        // A type parameter filled at a type that refuses a key beside its own, which serde reads
        // past here.
        let kept = read::Source::Keeping(keys);
        let alone = corrected_field(field, &[], walked, &kept, None, twins);
        return format!("if {filled} {{ {alone} }} else {{ {read} }}");
    }
    read
}

/// The statements that read the flattened externally tagged enum `field` holds, under an `Option`
/// or not, from the one entry under one of `entries`' names of the object without `taken`.
fn flattened_enum(
    field: &SwiftField,
    entries: (&str, &[preprocess::Entry]),
    taken: &str,
    twins: &mut Vec<String>,
) -> String {
    let optional = field.field_def.is_optional();
    let def = if optional {
        validate::under_option(&field.field_def)
    } else {
        field.field_def.clone()
    };
    let real_type = if optional {
        field
            .real_type
            .strip_suffix('?')
            .unwrap_or(&field.real_type)
    } else {
        &field.real_type
    };
    preprocess::entry(
        &preprocess::Member {
            def: &def,
            flat: false,
            functions: &[],
            hint: &field.hint,
            keys: "CodingKeys",
            plain: preprocess::Plain::Declared,
            real_type,
            refusing: false,
            stem: swift_bare(&field.swift_name),
            units: &[],
            walked: validate::walks(&def),
        },
        entries,
        taken,
        optional,
        &|held| format!("self.{} = {held}", field.swift_name),
        twins,
    )
}

/// The names serde reads a variant of `item_enum` under where it is flattened into a holder, each
/// with the name the enum's own read takes the variant under and whether the variant holds no
/// value, in the order [`variant_names_when_flattened`] lists them.
fn flattened_entries(item_enum: &ItemEnum, preprocess: &[FunctionName]) -> Vec<preprocess::Entry> {
    let read = variant_names_when_flattened(item_enum, preprocess);
    let rule = container_rename_rule(&item_enum.attrs);
    item_enum
        .variants
        .iter()
        .flat_map(|variant| {
            let variant_name = rename_override(&variant.attrs)
                .unwrap_or_else(|| rule.apply_to_variant(&variant.ident.to_string()));
            let unit = variant_wire_kind(variant) == VariantKind::Unit;
            iter::once(variant_name.clone())
                .chain(aliases::written_on(&variant.attrs))
                .map(move |name| preprocess::Entry {
                    name,
                    unit,
                    variant: variant_name.clone(),
                })
        })
        .filter(|entry| read.iter().flatten().any(|name| *name == entry.name))
        .collect()
}

/// The tag, then the content key where it has one, of an enum under a tag key: what a flattened
/// `Option` of it is tested by. None for an untagged enum, and for one that names a function on
/// itself, which is read from what the function answers.
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

/// The statements that read from `source` the flattened `Option` `field`, which names no function
/// and holds no map: `nil` where the object it is handed holds none of its type's keys, a struct's
/// or the tag and the content key of an enum under a tag key, and the type otherwise, read as a
/// flattened member of that type is, a read that throws refusing the payload. A type parameter is
/// tested by the keys of the type that fills it, where they are known as it is read. A type whose
/// keys are not known is read as `nil` wherever its read throws, and so is every type under a
/// `nonstrict` holder where one of its keys is there, as its Zod schema and serde read it.
fn flattened_option(
    field: &SwiftField,
    source: &read::Source,
    withheld: bool,
    container: &str,
    locals: &mut read::Locals,
    twins: &mut Vec<String>,
) -> String {
    let held = flattened_under_option(field);
    let read = present_statement(&held, source, withheld, false, container, locals, twins);
    let absent = format!("self.{} = nil", field.swift_name);
    let caught = format!("do {{ {read} }} catch {{ {absent} }}");
    let where_held = if is_nonstrict() { &caught } else { &read };
    // A type read from its own keys alone is looked for in the whole object it is read from.
    let without = if withheld
        && !preprocess::reads_the_whole(&field.field_def)
        && preprocess::only_keys(&field.field_def).is_empty()
        && let read::Source::Beside(taken) = source
    {
        Some(taken.as_str())
    } else {
        None
    };
    if let Some(filled) = preprocess::filled_keys(&held.field_def, &held.real_type) {
        let standing = format!("{}Standing", swift_bare(&field.swift_name));
        return format!(
            "let {standing}: [String]? = {filled}; if let {standing} {{ if {} {{ {where_held} }} else \
             {{ {absent} }} }} else {{ {caught} }}",
            preprocess::holds_one_listed(&standing, without)
        );
    }
    let keys = preprocess::keys_standing_for(&field.field_def);
    if keys.is_empty() {
        return caught;
    }
    format!(
        "if {} {{ {where_held} }} else {{ {absent} }}",
        preprocess::holds_one_of(&keys, without)
    )
}

/// The member the flattened `Option` `field` holds, under the same name: what its read reads.
fn flattened_under_option(field: &SwiftField) -> SwiftField {
    SwiftField {
        aliases: Vec::new(),
        default_at: None,
        field_def: validate::under_option(&field.field_def),
        flatten: true,
        hint: field.hint.clone(),
        held: Storage::Direct,
        never_written: field.never_written,
        real_type: field
            .real_type
            .strip_suffix('?')
            .unwrap_or(&field.real_type)
            .to_owned(),
        required: field.required,
        shape: field.shape.clone(),
        swift_name: field.swift_name.clone(),
        wire_name: field.wire_name.clone(),
    }
}

/// The statements that read from `source` the flattened `Option` `field`, which names `functions`
/// and holds no map, as its holder's Zod schema reads it: the type it holds, read from what the
/// functions answer, and where that read throws, `nil` where the object they are handed holds none
/// of a struct's keys, or no key at all for a type whose keys are not known, and refused otherwise.
/// An enum's tag stands for none of its variants' members, so its keys are not known. Under a
/// `nonstrict` holder it is `nil` exactly where the functions answer `null`
/// ([`preprocess::optional`]).
fn corrected_option(
    field: &SwiftField,
    functions: &[FunctionName],
    source: &read::Source,
    twins: &mut Vec<String>,
) -> String {
    let held = flattened_under_option(field);
    let keys = preprocess::struct_keys(&held.field_def);
    let walked = validate::walks(&held.field_def);
    corrected_field(&held, functions, walked, source, Some(&keys), twins)
}

/// The map a member `def` describes holds, read through an `Option`, or `None` for a member that
/// holds no map.
fn held_map(def: &FieldDef) -> Option<FieldDef> {
    let held = if def.is_optional() {
        validate::under_option(def)
    } else {
        def.clone()
    };
    (held.array_depth == 0 && matches!(held.field_type, FieldDefType::Map(..))).then_some(held)
}

/// The statements that read from `source` the map the flattened `field` holds, entry by entry and
/// without the keys the types around its holder took, the tag of a variant under a tag key among
/// them: `None` for a field that holds no map. A map under a key Swift wraps is read by its
/// wrapper, through the read [`flattened_wrapper_read`] gives it. An optional map is `nil`
/// wherever an entry does not read, as serde reads it.
fn flattened_map(
    field: &SwiftField,
    walked: bool,
    source: &read::Source,
    locals: &mut read::Locals,
) -> Option<String> {
    if !matches!(source, read::Source::Beside(_)) {
        return None;
    }
    let def = held_map(&field.field_def)?;
    let FieldDefType::Map(key, value) = &def.field_type else {
        return None;
    };
    let member = |held: &str| format!("self.{} = {held}", field.swift_name);
    let read = if matches!(map_keys(key), MapKeys::Wrapped) {
        let (wrapper, generic_params) = keyed_map_wrapper(&field.hint, key, value);
        read::beside(source, &held_at(&wrapper, &generic_params), &|read| {
            member(&format!("{read}.storage"))
        })?
    } else {
        let value_hint = format!("{}Value", field.hint);
        let held = arrayed((**value).clone());
        let (value_type, shape) = swift_resolve(&held, &value_hint, &mut Vec::new());
        let entry = Shaped {
            def: held,
            hint: value_hint.clone(),
            shape,
        };
        let stem = format!("{}Entry", swift_bare(&field.swift_name));
        read::read_entries(
            value,
            &swift_full_real_type(&def, &field.hint, &mut Vec::new()),
            source,
            true,
            &|under, put, nested| {
                if walked {
                    read::unjudged(value, &value_hint, under, put, nested)
                } else {
                    entry.read(&value_type, &stem, under, put, nested)
                }
            },
            &member,
            locals,
        )
    };
    Some(if field.field_def.is_optional() {
        format!("do {{ {read} }} catch {{ {} }}", member("nil"))
    } else {
        read
    })
}

/// The statements that read `field` from the JSON of its default, written at `at`, at the coding
/// path `path`: as it is read from a payload under its key, with no function called for it.
fn default_statement(field: &SwiftField, at: &str, path: &str, twins: &mut Vec<String>) -> String {
    let wire = swift_wire_array_type(&field.field_def, &field.shape);
    let convert = |local: &str| converted(field, local);
    let shaped = field.shaped();
    let plain = plain_read(&shaped, &wire, &convert);
    let stem = format!("{}Default", swift_bare(&field.swift_name));
    preprocess::defaulted(
        &preprocess::Member {
            def: &field.field_def,
            flat: false,
            functions: &[],
            hint: &field.hint,
            keys: "CodingKeys",
            plain,
            real_type: &field.real_type,
            refusing: false,
            stem: &stem,
            units: &[],
            walked: validate::walks(&field.field_def),
        },
        at,
        path,
        &|held| format!("self.{} = {held}", field.swift_name),
        twins,
    )
}

/// The statements that read from `source` a field that names `functions`, from what they answer:
/// as its own type, or as the type it is written as, turned into its own. `absent` holds, for the
/// type a flattened `Option` holds, the keys [`preprocess::optional`] tests where its read throws.
fn corrected_field(
    field: &SwiftField,
    functions: &[FunctionName],
    walked: bool,
    source: &read::Source,
    absent: Option<&[String]>,
    twins: &mut Vec<String>,
) -> String {
    let wire = wire_type(&field.field_def, &field.shape);
    let convert = |local: &str| converted(field, local);
    let shaped = field.shaped();
    let plain = plain_read(&shaped, &wire, &convert);
    let units = if field.flatten {
        preprocess::units_of(&field.field_def)
    } else {
        Vec::new()
    };
    let member = preprocess::Member {
        def: &field.field_def,
        flat: field.flatten && preprocess::is_unseen(&field.field_def),
        functions,
        hint: &field.hint,
        keys: "CodingKeys",
        plain,
        real_type: &field.real_type,
        refusing: field.flatten && is_nonstrict() && !functions.is_empty(),
        stem: swift_bare(&field.swift_name),
        units: &units,
        walked,
    };
    let sink = |held: &str| format!("self.{} = {held}", field.swift_name);
    match absent {
        Some(keys) => preprocess::optional(&member, source, keys, &sink, twins),
        None => preprocess::member(&member, source, &sink, twins),
    }
}

/// [`preprocess::member`] for `member`, a slot: read as it is declared, item by item where `shaped`
/// holds an array of a fixed length, or as the type it is written as where `shaped` turns that
/// into its own.
fn corrected_member(
    member: &preprocess::Member<'_>,
    shaped: Option<&Shaped>,
    source: &read::Source,
    sink: &dyn Fn(&str) -> String,
    twins: &mut Vec<String>,
) -> String {
    let Some(slot) = shaped.filter(|slot| slot.converts() || read::holds_a_length(&slot.def))
    else {
        return preprocess::member(member, source, sink, twins);
    };
    let wire = wire_type(&slot.def, &slot.shape);
    let convert = |local: &str| slot.converted(local);
    preprocess::member(
        &preprocess::Member {
            plain: plain_read(slot, &wire, &convert),
            ..*member
        },
        source,
        sink,
        twins,
    )
}

/// How `shaped`, a value `validate()` walks nothing of, is read from a payload: item by item where
/// it holds an array of a fixed length, as `wire`, the type it is written as, which `convert` turns
/// into its own where its shape converts, and as the type it is declared as otherwise.
fn plain_read<'read>(
    shaped: &'read Shaped,
    wire: &'read str,
    convert: &'read dyn Fn(&str) -> String,
) -> preprocess::Plain<'read> {
    if read::holds_a_length(&shaped.def) {
        preprocess::Plain::Counted(shaped)
    } else if shaped.converts() {
        preprocess::Plain::Wire { convert, wire }
    } else {
        preprocess::Plain::Declared
    }
}

/// What `local` holds, a field read as the type it is written as, turned into the field's own.
fn converted(field: &SwiftField, local: &str) -> String {
    converted_from(&field.field_def, &field.shape, local)
}

/// The statements that read under its key, from `own`, the container of the type's own keys, or
/// from the container a member under an alias is read from, a field `validate()` walks nothing of:
/// as its own type, or as the type it is written as, turned into its own.
fn keyed_read(field: &SwiftField, own: &str, absent_is_nil: bool) -> String {
    let (container, key) = field.keyed_at(own);
    let keyed = format!(".self, forKey: {key})");
    let decode = format!("{container}.decode(");
    let decode_if_present = format!("{container}.decodeIfPresent(");
    let parameters = parameters_reached(&[&field.field_def]);
    if matches!(field.shape.leaf_conversion, LeafConversion::None) {
        return if absent_is_nil {
            let inner = field
                .real_type
                .strip_suffix('?')
                .unwrap_or(&field.real_type);
            format!(
                "self.{} = try {}",
                field.swift_name,
                read::fitted(&decode_if_present, inner, &keyed, &parameters)
            )
        } else {
            format!(
                "self.{} = try {}",
                field.swift_name,
                read::fitted(&decode, &field.real_type, &keyed, &parameters)
            )
        };
    }
    let wire_array = swift_wire_array_type(&field.field_def, &field.shape);
    let read = if field.required {
        if field.field_def.is_optional() {
            format!(
                "try {}",
                read::fitted(&decode, &format!("{wire_array}?"), &keyed, &parameters)
            )
        } else {
            format!(
                "try {}",
                read::fitted(&decode, &wire_array, &keyed, &parameters)
            )
        }
    } else {
        format!(
            "try {}",
            read::fitted(&decode_if_present, &wire_array, &keyed, &parameters)
        )
    };
    let local = format!("wire{}", swift_upper_camel(swift_bare(&field.swift_name)));
    format!(
        "let {local} = {read}; self.{} = {}",
        field.swift_name,
        converted(field, &local)
    )
}

/// The `try container.encode...` statement a field earns inside a hand-written `encode(to:)`. The
/// property is read through `self.`, so a member named after a local of the method (`container`,
/// `tagged`, `held`, `entry`, a `wire{Member}`) or its parameter, `encoder`, is never read as
/// either.
fn encode_statement(field: &SwiftField, container: &str) -> String {
    let property = format!("self.{}", field.swift_name);
    if field.flatten && field.field_def.is_optional() {
        // `Optional`'s own `encode(to:)` writes a single value over the holder's keys, and `null`
        // for `nil`, where serde writes the held value's keys beside them, and nothing for `None`.
        let written = encode_conversion(
            &field.field_def,
            &field.shape,
            field.field_def.array_depth,
            "held",
        );
        return format!(
            "if let held = {property} {{ {} }}",
            flattened_write(&field.field_def, "held", &written)
        );
    }
    if field.flatten {
        let written = written_from(&field.field_def, &field.shape, &property);
        return flattened_write(&field.field_def, &property, &written);
    }
    let source = if matches!(field.shape.leaf_conversion, LeafConversion::None) {
        property
    } else {
        let convert = written_from(&field.field_def, &field.shape, &property);
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

/// The statement that writes `held`, a flattened value of `def`, as `written` through the holder's
/// encoder. A unit variant of an externally tagged enum written above is an entry of its name over
/// `null`, as serde writes it: the bare name the enum writes is dropped by the holder's object.
fn flattened_write(def: &FieldDef, held: &str, written: &str) -> String {
    let entries = unit_entries_of(def);
    if entries.is_empty() && preprocess::is_unseen(def) {
        // A plain enum is the name of an entry over `null`, and any other type writes its members.
        return format!(
            "if let plain = ({written} as Any) as? any RawRepresentable, let name = plain.rawValue \
             as? String {{ var entry = encoder.container(keyedBy: SwiftSchemaAnyKey.self); \
             try entry.encodeNil(forKey: SwiftSchemaAnyKey(stringValue: name)!) }} else {{ \
             try {written}.encode(to: encoder) }}"
        );
    }
    unit_entries_write(&entries, &entries_held(def, held), written)
}

/// The statement that writes `held`, a value of an enum whose unit variants are `entries`, as
/// `written` through the encoder of the object it joins: a unit variant as an entry of its name
/// over `null`.
fn unit_entries_write(entries: &[UnitEntry], held: &str, written: &str) -> String {
    let whole = format!("try {written}.encode(to: encoder)");
    if entries.is_empty() {
        return whole;
    }
    let cases = entries.iter().fold(String::new(), |mut cases, entry| {
        write!(
            cases,
            "case .{}: var entry = encoder.container(keyedBy: SwiftSchemaAnyKey.self); \
             try entry.encodeNil(forKey: SwiftSchemaAnyKey(stringValue: \"{}\")!); ",
            entry.case_name,
            swift_literal_text(&entry.name)
        )
        .unwrap();
        cases
    });
    let through = if entries.iter().any(|entry| entry.drafted) {
        ".draft"
    } else {
        ""
    };
    format!("switch {held}{through} {{ {cases}default: {whole} }}")
}

/// The per-struct static helpers a `DateTime` field's decode/encode calls into — a formatter with
/// fractional seconds, so the emitted round trip is byte-for-byte the ISO 8601 text chrono wrote,
/// and one without them, which reads the text chrono writes for a date of whole seconds.
/// Swift refuses a static stored property in a generic type, so a type that declares a parameter
/// builds its formatters on each call, and any other shares them.
#[cfg(feature = "chrono")]
fn datetime_helpers(export_name: &str, declares_a_parameter: bool) -> String {
    let formatter = |named: &str, options: &str| {
        if declares_a_parameter {
            format!(
                "private static var {named}: ISO8601DateFormatter {{ let f = \
                 ISO8601DateFormatter(); f.formatOptions = [{options}]; return f }};"
            )
        } else {
            format!(
                "private static let {named}: ISO8601DateFormatter = {{ let f = \
                 ISO8601DateFormatter(); f.formatOptions = [{options}]; return f }}();"
            )
        }
    };
    format!(
        "extension {export_name} {{ {} {} \
         private static func swiftSchemaParseIso8601Date(_ text: String, codingPath: [CodingKey]) throws -> Date {{ \
         guard let date = swiftSchemaIso8601Formatter.date(from: text) ?? \
         swiftSchemaWholeIso8601Formatter.date(from: text) else {{ \
         throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: codingPath, debugDescription: \"bad ISO8601 date: \\(text)\")) }}; \
         return date }}; \
         private static func swiftSchemaFormatIso8601Date(_ date: Date) -> String {{ swiftSchemaIso8601Formatter.string(from: date){CHRONO_WHOLE_SECONDS} }} }};",
        formatter("swiftSchemaIso8601Formatter", FRACTIONAL_OPTIONS),
        formatter("swiftSchemaWholeIso8601Formatter", WHOLE_OPTIONS),
    )
}

/// Whether reading a value of `shape` calls [`datetime_helpers`]: a `DateTime<Tz>` leaf under any
/// list or optional, or one held as the value of a map Swift keys natively.
#[cfg(feature = "chrono")]
fn reads_a_date(shape: &SwiftFieldShape) -> bool {
    match &shape.leaf_conversion {
        LeafConversion::DateTimePlain => true,
        LeafConversion::Values(value) => reads_a_date(&value.shape),
        LeafConversion::MapWrapper(_) | LeafConversion::None | LeafConversion::Text(_) => false,
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

/// The forms of the texts a value of `shape` is read and written through, gathered into `found`
/// once each.
fn text_forms_of(shape: &SwiftFieldShape, found: &mut Vec<TextForm>) {
    match &shape.leaf_conversion {
        LeafConversion::Text(form) => {
            if !found.contains(form) {
                found.push(*form);
            }
        }
        LeafConversion::Values(value) => text_forms_of(&value.shape, found),
        #[cfg(feature = "chrono")]
        LeafConversion::DateTimePlain => {}
        LeafConversion::MapWrapper(_) | LeafConversion::None => {}
    }
}

/// The helpers the read and the write of one of `shapes` call, written into `export_name`: the
/// date helpers ([`datetime_helpers_for`]) and an extension with those of each text of one form,
/// `keyed` the form of the keys of a map's wrapper. Nothing where none is called.
fn leaf_helpers_for(
    export_name: &str,
    generic_params: &str,
    shapes: &[&SwiftFieldShape],
    keyed: Option<TextForm>,
) -> String {
    let mut written = datetime_helpers_for(export_name, generic_params, shapes);
    let mut found: Vec<TextForm> = keyed.into_iter().collect();
    for shape in shapes {
        text_forms_of(shape, &mut found);
    }
    let texts = forms::helpers(&found);
    if !texts.is_empty() {
        write!(written, "extension {export_name} {{ {texts}}};").unwrap();
    }
    written
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
        field_def.named_within(args.type_name, type_parameters);
        let flatten = field_is_flatten(&field.attrs);
        if flatten
            && field_def.array_depth == 0
            && !field_def.is_optional()
            && let Some(held) = field_def.written_in_place()
        {
            // An alias of an `Option` is a `typealias` of it, which a flattened read takes apart.
            field_def = held;
        }
        // A flattened member has no key to leave out, and serde reads it whatever it writes.
        let defaulted = default_at(&rust_name);
        let leaves_key_out = omission.omits_key && defaulted.is_none();
        if leaves_key_out && !field_def.is_optional() && !flatten {
            field_def.nullable_levels.push(field_def.array_depth);
        }
        let swift_name = RenameRule::CamelCase.apply_to_field(&rust_name);
        let field_hint = format!("{name_hint}{}", swift_upper_camel(&swift_name));
        let (real_type, shape) = swift_resolve(&field_def, &field_hint, aux);
        if flatten {
            aux.extend(flattened_wrapper_read(&field_def, &field_hint));
        }
        let held = if !flatten
            && field_def.array_depth == 0
            && field_def.is_optional()
            && matches!(&field_def.field_type, FieldDefType::SiblingType(name, _) if name == args.type_name)
        {
            Storage::Boxed
        } else {
            Storage::Direct
        };
        collected.push(SwiftField {
            aliases: if flatten || omission.skips_deserializing {
                Vec::new()
            } else {
                aliases::written_on(&field.attrs)
            },
            default_at: defaulted,
            field_def,
            flatten,
            hint: field_hint,
            held,
            never_written: has_serde_skip_serializing(&field.attrs),
            real_type,
            required: !leaves_key_out,
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
        write!(cases, "case {name} = {}; ", swift_quoted(wire)).unwrap();
    }
    for field in fields {
        if swift_bare(&field.swift_name) == field.wire_name {
            write!(cases, "case {}; ", field.swift_name).unwrap();
        } else {
            write!(
                cases,
                "case {} = {}; ",
                field.swift_name,
                swift_quoted(&field.wire_name)
            )
            .unwrap();
        }
    }
    format!("enum CodingKeys: String, CodingKey {{ {cases} }};")
}

/// The properties of a struct over `fields`: `var` on a draft, which code changes, and `let` on any
/// other struct, the member that holds an `Option` of its own type under `owner`'s wrapper.
fn member_properties(fields: &[SwiftField], owner: &str, draft: bool) -> String {
    fields.iter().fold(String::new(), |mut acc, field| {
        let (kind, mutable) = if draft {
            ("var", "var")
        } else {
            ("let", "private(set) var")
        };
        if field.held == Storage::Boxed {
            write!(
                acc,
                "@{} public {mutable} {}: {}; ",
                indirection_name(owner),
                field.swift_name,
                field.real_type
            )
        } else {
            write!(
                acc,
                "public {kind} {}: {}; ",
                field.swift_name, field.real_type
            )
        }
        .unwrap();
        acc
    })
}

/// The body of a set of [`SwiftField`]s: properties, `CodingKeys`, the memberwise initializer,
/// the reads, `encode(to:)` only where synthesis cannot reach, then `validate`, the method every
/// emitted type has. `extra_coding_keys` lists cases with no matching field (an
/// internally-tagged variant's tag), `corrects` the functions the struct `owner` names, and
/// `around` says the types around the struct hand it keys it reads none of.
fn struct_body_content(
    fields: &[SwiftField],
    extra_coding_keys: &[(String, String)],
    own_tag: Option<&(String, String)>,
    validate: &str,
    owner: &str,
    read: OwnRead<'_>,
) -> String {
    let OwnRead {
        around,
        closed,
        corrects,
        valid,
    } = read;
    let props = member_properties(fields, owner, valid.is_some());
    let coding_keys = swift_coding_keys(fields, extra_coding_keys);
    // A struct's own tag has no property to synthesize a codec from, so the tag is encoded and
    // read under a key set of its own.
    let unsynthesized = own_tag.is_some()
        || fields.iter().any(|field| {
            field.flatten || !matches!(field.shape.leaf_conversion, LeafConversion::None)
        });
    let needs_encode = unsynthesized
        || fields
            .iter()
            .any(|field| field_is_nullable_flag(field) || field.never_written);

    let memberwise_init = memberwise_initializer(fields);
    let mut twins = Vec::new();
    let members = member_reads(fields, around, &mut twins);
    let keyed = if names_container(&members) {
        "let container = try decoder.container(keyedBy: CodingKeys.self); "
    } else {
        ""
    };
    let aliased = if fields.iter().any(|field| !field.aliases.is_empty()) {
        aliases::CONTAINER
    } else {
        ""
    };
    let tag_held = own_tag.map_or_else(String::new, |(key, named)| own_tag_held(key, named));
    let reads = read::both(
        &format!("{tag_held}{keyed}{aliased}{members}"),
        &preprocess::Written {
            beside: "",
            draft: valid.is_some(),
            named: owner,
            own: corrects,
            sealed: false,
            twins,
            closed,
        },
    );
    let encode_method = if needs_encode {
        let statements = fields
            .iter()
            .filter(|field| !field.never_written)
            .map(|field| encode_statement(field, "container"))
            .collect::<Vec<_>>()
            .join("; ");
        let container = if names_container(&statements) {
            "var container = encoder.container(keyedBy: CodingKeys.self); "
        } else if fields
            .iter()
            .any(|field| field.never_written || (field.flatten && field.field_def.is_optional()))
        {
            // No member is written under a key, and a flattened `Option` may write nothing: the
            // object is opened all the same.
            "_ = encoder.container(keyedBy: CodingKeys.self); "
        } else {
            ""
        };
        let tag = own_tag.map_or_else(String::new, |(_, named)| {
            format!(
                "var tagged = encoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); \
                 try tagged.encode({}, forKey: .swiftSchemaTag); ",
                swift_quoted(named)
            )
        });
        let key = read::any_key_written(&reads, &statements);
        format!(
            "{key}public func encode(to encoder: Encoder) throws {{ \
             {tag}{container}{statements} }}; "
        )
    } else {
        String::new()
    };
    let tag_keys = own_tag.map_or_else(String::new, |(key, _)| {
        format!(
            "private enum SwiftSchemaTagCodingKeys: String, CodingKey {{ \
             case swiftSchemaTag = {} }}; ",
            swift_quoted(key)
        )
    });
    format!("{props}{coding_keys} {tag_keys}{memberwise_init}{reads}{encode_method}{validate}")
}

/// The statements that refuse the object the decoder holds where it holds no `key`, or another
/// value than `named` under it: the tag of a struct under its own tag, which TypeScript and Zod
/// refuse in the same places, where serde reads past it. A value refused is quoted as JSON.
fn own_tag_held(key: &str, named: &str) -> String {
    let key_text = swift_literal_text(key);
    let name_text = swift_literal_text(named);
    format!(
        "let tagged = try decoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); guard \
         tagged.contains(.swiftSchemaTag) else {{ throw DecodingError.dataCorrupted(\
         DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"missing field \
         `{key_text}`\")) }}; guard (try? tagged.decode(String.self, forKey: .swiftSchemaTag)) == \
         \"{name_text}\" else {{ let found = try JSONSerialization.data(withJSONObject: \
         tagged.decode(SwiftSchemaPayload.self, forKey: .swiftSchemaTag).value, options: \
         [.fragmentsAllowed, .sortedKeys, .withoutEscapingSlashes]); throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: \"expected tag \\\"{name_text}\\\", found \\(String(decoding: found, \
         as: UTF8.self))\")) }}; "
    )
}

/// Whether `statements` read or write through `container`, the keyed container of a struct's
/// members: a member of it, never `decoder.container(...)`, `self.container` or the key
/// `.container`.
fn names_container(statements: &str) -> bool {
    statements
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
        })
        .any(|path| path.starts_with("container."))
}

/// The statements that read each of `fields` in the type's own read, in the order declared, a
/// type written for one joining `twins`. `around` says the types around the holder hand it keys it
/// reads none of.
/// Each flattened member is handed the object without the keys its holder's own members, the
/// members flattened ahead of it and the types around its holder took, as serde reads it.
fn member_reads(fields: &[SwiftField], around: bool, twins: &mut Vec<String>) -> String {
    let alone = handed_reads(fields, false);
    // A holder of no member of its own is handed keys by the types around it where they hand it
    // some, or where one of its reads takes them.
    let (statements, written) =
        if fields.iter().all(|field| field.flatten) && (around || read::names_taken(&alone.0)) {
            handed_reads(fields, true)
        } else {
            alone
        };
    twins.extend(written);
    statements
}

/// [`member_reads`] with the types written for the members, where `around` says the types around
/// the holder hand it keys it takes.
fn handed_reads(fields: &[SwiftField], around: bool) -> (String, Vec<String>) {
    let mut locals = read::Locals::default();
    let mut twins = Vec::new();
    let mut taken_ahead: Vec<String> = fields
        .iter()
        .filter(|field| !field.flatten)
        .flat_map(|field| iter::once(&field.wire_name).chain(&field.aliases).cloned())
        .collect();
    // The local of each flattened enum ahead, which holds the name of the entry it took.
    let mut found_ahead: Vec<String> = Vec::new();
    let statements = fields
        .iter()
        .map(|field| {
            let handed = Handed {
                taken: iter::once(preprocess::taken_with(
                    taken_ahead.iter().map(String::as_str),
                ))
                .chain(found_ahead.iter().cloned())
                .collect::<Vec<_>>()
                .join(" + "),
                withheld: around || !taken_ahead.is_empty() || !found_ahead.is_empty(),
            };
            if field.flatten {
                for key in preprocess::keys_taken_by(&field.field_def) {
                    if !taken_ahead.contains(&key) {
                        taken_ahead.push(key);
                    }
                }
                if preprocess::entries_of(&field.field_def).is_some() {
                    found_ahead.push(preprocess::found(swift_bare(&field.swift_name)));
                }
            }
            decode_statement(field, "container", &handed, &mut locals, &mut twins)
        })
        .collect::<Vec<_>>()
        .join("; ");
    (statements, twins)
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
                    real_type: Some(field.real_type.as_str()),
                    access: validate::property_read(&field.swift_name),
                    def: &field.field_def,
                    step: (!field.flatten).then(|| field.wire_name.clone()),
                },
            )
        })
        .collect()
}

/// The property wrapper a struct names for its members that hold an `Option` of its own type.
fn indirection_name(owner: &str) -> String {
    format!("{owner}SwiftSchemaIndirect")
}

/// The property wrapper `name`, which keeps its value in a class so that a struct can hold an
/// `Option` of itself, and reads and writes the `Option` as the plain one is: an absent key is
/// `nil`, and `nil` is written as no key.
fn indirection(name: &str) -> String {
    format!(
        "@propertyWrapper public struct {name}<Wrapped: Codable & Sendable>: Codable, Sendable {{ \
         private final class Box: Sendable {{ let value: Wrapped?; \
         init(_ value: Wrapped?) {{ self.value = value }} }}; \
         private var box: Box; \
         public init(wrappedValue: Wrapped?) {{ box = Box(wrappedValue) }}; \
         public var wrappedValue: Wrapped? {{ get {{ box.value }} set {{ box = Box(newValue) }} }}; \
         public init(from decoder: Decoder) throws {{ \
         let container = try decoder.singleValueContainer(); \
         box = Box(container.decodeNil() ? nil : try container.decode(Wrapped.self)) }}; \
         public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.singleValueContainer(); \
         if let value = box.value {{ try container.encode(value) }} else {{ try container.encodeNil() }} }} }}; \
         extension KeyedDecodingContainer {{ \
         func decode<Wrapped>(_ type: {name}<Wrapped>.Type, forKey key: Key) throws -> {name}<Wrapped> {{ \
         {name}(wrappedValue: try decodeIfPresent(Wrapped.self, forKey: key)) }} }}; \
         extension KeyedEncodingContainer {{ \
         mutating func encode<Wrapped>(_ value: {name}<Wrapped>, forKey key: Key) throws {{ \
         try encodeIfPresent(value.wrappedValue, forKey: key) }} }};"
    )
}

/// `every` with the keys of a payload added, or `None` where the payload's are not known.
fn with_keys(every: Option<Vec<String>>, payload: Option<&Vec<String>>) -> Option<Vec<String>> {
    every
        .zip(payload)
        .map(|(held, more)| held.into_iter().chain(more.iter().cloned()).collect())
}

/// `keys` unless the type is `nonstrict` or names a function on itself, which refuse no key.
fn closed_when_strict(keys: Option<Vec<String>>, corrects: &[FunctionName]) -> Option<Vec<String>> {
    if is_nonstrict() || !corrects.is_empty() {
        None
    } else {
        keys
    }
}

/// The key type an enum under a tag and a content key reads them through, and the keys it refuses
/// a key beside.
fn adjacent_keys(
    tag_key: &str,
    content_key: &str,
    corrects: &[FunctionName],
) -> (String, Option<Vec<String>>) {
    let tagging_keys = format!(
        "private enum SwiftSchemaAdjacentCodingKeys: String, CodingKey {{ \
         case swiftSchemaContent = {}, swiftSchemaTag = {} }};",
        swift_quoted(content_key),
        swift_quoted(tag_key)
    );
    let closed = closed_when_strict(
        Some(vec![tag_key.to_owned(), content_key.to_owned()]),
        corrects,
    );
    (tagging_keys, closed)
}

/// The attempt that reads, and the arm that writes, an untagged variant that holds nothing: the
/// `null` serde writes for it.
fn null_variant(case_name: &str, decode_attempts: &mut String, encode_arms: &mut String) {
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

/// The call that holds a variant's payload to its bounds, with the function that runs them added
/// to `choosers`, or `None` where the payload has none. `held` is the enum, the variant and the
/// payload type.
fn bound_check(
    bound: &[String],
    held: (&str, &str, &str),
    choosers: &mut String,
) -> Option<String> {
    let (export_name, named, ty) = held;
    (!bound.is_empty()).then(|| {
        write!(
            choosers,
            "private static func swiftSchemaBound{named}(_ payload: {ty}) -> [String] \
             {{ var errors: [String] = []; {}; return errors }}; ",
            bound.join("; ")
        )
        .unwrap();
        format!("{export_name}.swiftSchemaBound{named}(value).isEmpty")
    })
}

/// The expression an untagged enum `owner` tries a variant's struct with, `read`: through the
/// struct's own read alone, which refuses a key it does not read, where the enum is handed no keys,
/// and otherwise through the read that takes them, after refusing a key that is neither one of the
/// struct's `keys` nor one taken, as serde tries a strict variant on the keys its holder leaves.
fn checked_alone(
    keys: Option<&Vec<String>>,
    read: &str,
    owner: &str,
    choosers: &mut String,
) -> String {
    let Some((own, alone)) = keys
        .filter(|_| !is_nonstrict())
        .and_then(|held| Some((held, read.strip_suffix(", swiftSchemaWithout: taken)")?)))
    else {
        return read.to_owned();
    };
    if !choosers.contains(ONLY_ITS_KEYS) {
        choosers.push_str(ONLY_ITS_KEYS);
    }
    format!(
        "(taken.isEmpty ? {alone}) : {owner}.swiftSchemaOnly(decoder, {}, taken, {read}))",
        preprocess::listed(own)
    )
}
/// The keys a struct over `fields` refuses a key beside: those of its own members and of those it
/// flattens. `None` where it refuses none: a `nonstrict` type, a type that names a function, one
/// under a tag of its own and a type that flattens a member whose keys are not known.
fn closed_keys(
    fields: &[SwiftField],
    own_tag: Option<&(String, String)>,
    corrects: &[FunctionName],
) -> Option<Vec<String>> {
    if is_nonstrict() || !corrects.is_empty() || own_tag.is_some() {
        return None;
    }
    let mut keys: Vec<String> = Vec::new();
    for field in fields {
        if field.flatten {
            let held = if field.field_def.is_optional() {
                validate::under_option(&field.field_def)
            } else {
                field.field_def.clone()
            };
            keys.extend(flattened_contribution(&held)?);
        } else {
            keys.push(field.wire_name.clone());
            keys.extend(field.aliases.iter().cloned());
        }
    }
    let mut seen = HashSet::new();
    keys.retain(|key| seen.insert(key.clone()));
    Some(keys)
}

/// Every key a holder takes of the flattened member typed `def`, where they are all known: the keys
/// its type refuses a key beside, else those it takes. A member serde hands every key left counts
/// only where it refuses a key beside its own.
fn flattened_contribution(def: &FieldDef) -> Option<Vec<String>> {
    if !preprocess::functions(def).is_empty() || !preprocess::only_keys(def).is_empty() {
        return None;
    }
    if leaves_its_keys(def) {
        // serde hands such a member every key its holder leaves. The macro's classifier answers its
        // keys, which the Swift registries hold for no enum and no struct of one slot.
        return flattened_keys(def);
    }
    preprocess::closed_keys_of(def)
        .or_else(|| {
            let taken = preprocess::keys_taken_by(def);
            (!taken.is_empty()).then_some(taken)
        })
        .or_else(|| flattened_keys(def))
        .or_else(|| unmet_keys(def))
}

/// The keys of a flattened member whose type is declared below its holder or in another crate, as
/// the placeholder `swift_definition()` fills.
#[cfg(feature = "serde")]
fn unmet_keys(def: &FieldDef) -> Option<Vec<String>> {
    member_keys::of(def)
}

/// With no `serde`, nothing is flattened.
#[cfg(not(feature = "serde"))]
const fn unmet_keys(_def: &FieldDef) -> Option<Vec<String>> {
    None
}

/// With no `serde`, nothing is flattened.
#[cfg(not(feature = "serde"))]
const fn flattened_keys(_def: &FieldDef) -> Option<Vec<String>> {
    None
}

/// Whether serde leaves the keys of the type `def` holds in the object it reads them from, which
/// its holder then cannot name.
#[cfg(feature = "serde")]
fn leaves_its_keys(def: &FieldDef) -> bool {
    matches!(&def.field_type, FieldDefType::SiblingType(name, _) if is_unconsumed(name))
}

/// Whether serde leaves the keys of the type `def` holds in the object it reads them from, which
/// its holder then cannot name.
#[cfg(not(feature = "serde"))]
const fn leaves_its_keys(_def: &FieldDef) -> bool {
    false
}

/// The `public struct {export_name}{generics}: Codable, Sendable { ... }` a named-field struct or
/// a struct-shaped variant payload earns, with the `DateTime` helpers appended when a field needs
/// them. `rules` is what its `validate()` runs, `corrects` the functions it names on itself, and
/// `around` says the types around it hand it keys it reads none of.
fn struct_declaration(
    export_name: &str,
    generic_params: &str,
    fields: &[SwiftField],
    own_tag: Option<&(String, String)>,
    rules: &[String],
    read: OwnRead<'_>,
) -> String {
    let validate = read.valid.map_or_else(
        || validate::method(rules),
        |valid| validate::draft_method(rules, valid),
    );
    let content = struct_body_content(fields, &[], own_tag, &validate, export_name, read);
    let shapes: Vec<&SwiftFieldShape> = fields.iter().map(|field| &field.shape).collect();
    let mut helpers = leaf_helpers_for(export_name, generic_params, &shapes, None);
    if fields.iter().any(|field| field.held == Storage::Boxed) {
        helpers.push_str(&indirection(&indirection_name(export_name)));
    }
    format!(
        "public struct {export_name}{generic_params}: Codable, Sendable {{ {content} }}; {helpers}"
    )
}

/// The Swift tokens of a branded newtype: the sealed struct of its one value, under the bounds and
/// the checks it names.
fn brand_swift_tokens(
    item_struct: &ItemStruct,
    held: &Field,
    args: &SwiftTypeArgs<'_>,
    export_name: &str,
) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let rust_ident = item_struct.ident.to_string();
    let value_field = brand_value_field(held, &type_parameters, args);
    let bounds = validate::brand_rules(export_name, &value_field, args);
    let mut rules = validate::own_repeat_rules(
        export_name,
        &validate::Checked {
            real_type: None,
            access: "value".to_owned(),
            def: &value_field,
            step: None,
        },
    );
    rules.extend(bounds.iter().cloned());
    rules.extend(validate::own_checks(args.check));
    let bare = if let FieldDefType::TypeParam(parameter) = &value_field.field_type
        && !value_field.is_list()
    {
        type_parameters.iter().position(|named| named == parameter)
    } else {
        None
    };
    let tokens = value_wrapper_tokens(
        &item_struct.ident,
        &item_struct.generics,
        args,
        &value_field,
        &Wrapped::Brand {
            bounds,
            length_member: validate::brand_length_member(&value_field),
            text_member: validate::brand_text_member(&value_field),
        },
        &rules,
        hashes_as_key(&item_struct.attrs, &value_field),
    );
    validate::note_bare(&rust_ident, bare);
    tokens
}

/// The Swift tokens a named-field struct earns: a struct, plus a `typealias` under its own Rust
/// ident when `name = "..."` moved its published name elsewhere.
fn struct_swift_tokens(item_struct: &ItemStruct, args: &SwiftTypeArgs<'_>) -> TokenStream {
    let type_parameters = type_parameters_in_scope(&item_struct.generics);
    let rust_ident = item_struct.ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    if let Some(held) = brand_field(item_struct) {
        return brand_swift_tokens(item_struct, held, args, &export_name);
    }
    if matches!(item_struct.fields, Fields::Unnamed(_)) {
        return tuple_struct_swift_tokens(item_struct, args);
    }

    register_swift_name(&rust_ident, &export_name);
    validate::note(&rust_ident, None, true);

    let rule = container_rename_rule(&item_struct.attrs);
    preprocess::note_keys(
        &rust_ident,
        keys_taken_when_flattened(&item_struct.fields, rule, args.preprocess),
    );
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
    let valid = held_at(&export_name, &generic_params);
    let drafted = !args.undrafted;
    let declared = declared_name(&export_name, args.undrafted);
    let mut rules = field_rules(&declared, &fields);
    rules.extend(validate::own_checks(args.check));
    validate::note(&rust_ident, None, !rules.is_empty());
    let closed = closed_keys(&fields, own_tag.as_ref(), args.preprocess);
    preprocess::note_closed(&rust_ident, closed.as_deref());
    let declaration = struct_declaration(
        &declared,
        &generic_params,
        &fields,
        own_tag.as_ref(),
        &rules,
        OwnRead {
            around: false,
            closed,
            corrects: args.preprocess,
            valid: drafted.then_some(valid.as_str()),
        },
    );
    preprocess::note(&rust_ident, read::takes_keys(&declaration));
    let statics = if matches!(item_struct.fields, Fields::Unit)
        && generic_params.is_empty()
        && is_rule_less()
    {
        format!("public static let instance = {export_name}(draft: {export_name}Draft()); ")
    } else {
        String::new()
    };
    let body = if drafted {
        with_valid_struct(
            &declaration,
            &export_name,
            &generic_params,
            &ValidForm {
                statics: &statics,
                ..ValidForm::default()
            },
        )
    } else {
        declaration
    };
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

/// The valid struct `export_name` that wraps the draft written as `body`: it is built only through
/// the draft's `validate()` and the read, reads each member through the draft, and writes what the
/// draft writes. A holder in the generated file reads and judges it through the forwards.
fn with_valid_struct(
    body: &str,
    export_name: &str,
    generic_params: &str,
    form: &ValidForm<'_>,
) -> String {
    let draft = held_at(&format!("{export_name}Draft"), generic_params);
    let (conformances, statics) = (form.conformances, form.statics);
    let built_from = if form.raw {
        format!(
            "fileprivate init?(rawValue: String) {{ guard let draft = {draft}(rawValue: rawValue) \
             else {{ return nil }}; self.draft = draft }}; "
        )
    } else {
        form.keyed.map_or_else(String::new, |value| {
            format!("fileprivate init(value: {value}) {{ draft = {draft}(value: value) }}; ")
        })
    };
    let taking = if read::takes_keys(body) {
        format!(
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: \
             [String]) throws {{ draft = try {draft}(swiftSchemaUnjudged: decoder, \
             swiftSchemaWithout: taken) }}; "
        )
    } else {
        String::new()
    };
    format!(
        "{body}\n\n@dynamicMemberLookup public struct {export_name}{generic_params}: Codable, Sendable{conformances} {{ \
         public let draft: {draft}; \
         fileprivate init(draft: {draft}) {{ self.draft = draft }}; {built_from}{statics}\
         public subscript<SwiftSchemaHeld>(dynamicMember path: KeyPath<{draft}, SwiftSchemaHeld>) -> SwiftSchemaHeld {{ \
         draft[keyPath: path] }}; \
         public init(from decoder: Decoder) throws {{ draft = try {draft}(from: decoder) }}; \
         fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws {{ \
         draft = try {draft}(swiftSchemaUnjudged: decoder) }}; {taking}\
         public func encode(to encoder: Encoder) throws {{ try draft.encode(to: encoder) }}; \
         fileprivate func validate() -> [String] {{ draft.validationIssues() }}; \
         fileprivate func swiftSchemaValidateBound() -> [String] {{ \
         draft.swiftSchemaValidateBound() }} }};"
    )
}

/// The valid struct a struct of one `value` over the draft written as `body` has: `Hashable`
/// where its draft is, and built from a value where a map keyed by a struct of one slot asks.
fn wrapper_form<'form>(
    wrapped: &Wrapped,
    hashable: bool,
    real_type: &'form str,
) -> ValidForm<'form> {
    ValidForm {
        conformances: if hashable { ", Hashable" } else { "" },
        keyed: matches!(wrapped, Wrapped::Slot).then_some(real_type),
        raw: false,
        statics: "",
    }
}

/// The name a model type is declared under: its draft's, unless a service wrote the type.
fn declared_name(export_name: &str, undrafted: bool) -> String {
    if undrafted {
        export_name.to_owned()
    } else {
        format!("{export_name}Draft")
    }
}

/// The `FieldDef` of the field a brand is the value of, its type parameters already erased.
fn brand_value_field(
    held: &Field,
    type_parameters: &[String],
    args: &SwiftTypeArgs<'_>,
) -> FieldDef {
    let mut field_def = member_def(args, "value", held);
    field_def.named_within(args.type_name, type_parameters);
    field_def
}

/// The `validate()` of the struct of one `value` named `rust_ident`, which records what its
/// checks and bounds are for the types written below.
fn wrapper_validate(
    rust_ident: &str,
    wrapped: &Wrapped,
    rules: &[String],
    valid: Option<&str>,
) -> String {
    let mut validate = validate::rules_method(rules);
    match wrapped {
        Wrapped::Brand {
            bounds,
            length_member,
            text_member,
        } => {
            validate::note(
                rust_ident,
                text_member.as_ref().map(|_| validate::Text::Member),
                !rules.is_empty(),
            );
            if let Some(member) = length_member {
                validate::note_length(rust_ident, Some(validate::Length::Member));
                write!(validate, "; {member}").unwrap();
            }
            if let Some(member) = text_member {
                write!(validate, "; {member}").unwrap();
            }
            if bounds.is_empty() {
                write!(validate, "; {}", validate::EMPTY_BOUND).unwrap();
            } else {
                validate::note_bounded(rust_ident);
                write!(validate, "; {}", validate::bound_member(bounds)).unwrap();
            }
        }
        Wrapped::Slot | Wrapped::Tuple => {
            validate::note(rust_ident, None, !rules.is_empty());
            if let Some(draft_of) = valid {
                validate = validate::draft_method(rules, draft_of);
            } else {
                write!(validate, "; {}", validate::EMPTY_BOUND).unwrap();
            }
        }
    }
    validate
}

/// Whether the Rust item can key a map by `value_field`: it derives `Hash`, or it holds a float,
/// which has no `Hash`, and derives `PartialEq`.
fn hashes_as_key(attrs: &[syn::Attribute], value_field: &FieldDef) -> bool {
    derives_hash(attrs)
        || (derives_partial_eq(attrs)
            && matches!(
                keys::leaf(value_field).field_type,
                FieldDefType::F32 | FieldDefType::F64
            ))
}

/// Records the brand `rust_ident` for the maps keyed by it below, and answers whether its Swift
/// struct declares `Hashable`: where the Rust item can key a map (`hashes`), declares no parameter
/// and holds a value Swift hashes by itself.
fn note_brand_key(
    rust_ident: &str,
    wrapped: &Wrapped,
    value_field: &FieldDef,
    generic_params: &str,
    hashes: bool,
) -> bool {
    let declared = generic_params.is_empty();
    let hashable = hashes && declared && keys::hashable(value_field);
    keys::note_brand(
        rust_ident,
        (!matches!(wrapped, Wrapped::Tuple) && declared).then_some((value_field, hashable)),
    );
    hashable
}

/// The Swift type the one value of a wrapper is held as, and how it is read and written, with the
/// types written for it joining `aux`.
fn wrapper_shaped(
    wrapped: &Wrapped,
    value_field: &FieldDef,
    export_name: &str,
    aux: &mut Vec<String>,
) -> (String, Shaped) {
    if matches!(wrapped, Wrapped::Slot) {
        return slot_resolve(value_field, export_name, aux);
    }
    let (real_type, shape) = match (wrapped, &value_field.field_type) {
        // The slots of a tuple struct are read from an array, an empty one where there are
        // none, which is not the `null` of `()`.
        (Wrapped::Tuple, FieldDefType::Tuple(slots)) => {
            let shape = tuple_shape(slots, export_name, aux);
            (shape.real_leaf.clone(), shape)
        }
        _ => swift_resolve(value_field, export_name, aux),
    };
    (
        real_type,
        Shaped {
            def: value_field.clone(),
            hint: export_name.to_owned(),
            shape,
        },
    )
}

/// The Swift tokens for a value that carries no shape of its own on the wire beyond one wrapped
/// value: a branded newtype, or a non-branded tuple struct. Both publish a struct with one
/// `public let value`, read from a single value and written by the value's own `encode(to:)`.
/// `rules` is what the wrapper's `validate()` runs.
fn value_wrapper_tokens(
    ident: &Ident,
    generics: &syn::Generics,
    args: &SwiftTypeArgs<'_>,
    value_field: &FieldDef,
    wrapped: &Wrapped,
    rules: &[String],
    hashes: bool,
) -> TokenStream {
    let sealed = !matches!(wrapped, Wrapped::Slot | Wrapped::Tuple);
    let drafted = !sealed && !args.undrafted;
    let rust_ident = ident.to_string();
    let export_name = compute_item_export_name(&rust_ident, args.name_override);
    register_swift_name(&rust_ident, &export_name);
    let declared = declared_name(&export_name, !drafted);
    let valid_at = held_at(
        &export_name,
        &bounded_generic_params(&type_parameters_in_scope(generics)),
    );
    let validate = wrapper_validate(
        &rust_ident,
        wrapped,
        rules,
        drafted.then_some(valid_at.as_str()),
    );
    if !matches!(wrapped, Wrapped::Tuple) {
        note_value_entries(&rust_ident, value_field);
    }

    let type_parameters = type_parameters_in_scope(generics);
    let generic_params = bounded_generic_params(&type_parameters);
    let hashable = note_brand_key(&rust_ident, wrapped, value_field, &generic_params, hashes);
    let mut aux = Vec::new();
    let (real_type, shaped) = wrapper_shaped(wrapped, value_field, &export_name, &mut aux);

    let decode = wrapper_decode(&shaped, &real_type, &type_parameters);
    let encode = wrapper_write(&shaped);
    let helpers = leaf_helpers_for(&declared, &generic_params, &[&shaped.shape], None);
    let mut twins = Vec::new();
    let own_read = if matches!(wrapped, Wrapped::Slot) {
        lone_slot_read(&shaped, &real_type, &export_name, &decode, &mut twins)
    } else {
        wrapper_read(wrapped, &shaped, &real_type, &decode)
    };
    let reads = read::both(
        &own_read,
        &preprocess::Written {
            draft: drafted,
            named: &declared,
            own: args.preprocess,
            sealed,
            twins,
            closed: (matches!(wrapped, Wrapped::Slot)
                && !is_nonstrict()
                && !value_field.is_optional())
            .then(|| preprocess::closed_keys_of(value_field))
            .flatten(),
            ..preprocess::Written::default()
        },
    );
    preprocess::note(&rust_ident, read::takes_keys(&reads));
    let lone_value = !matches!(wrapped, Wrapped::Tuple) && args.preprocess.is_empty();
    preprocess::note_only_keys(
        &rust_ident,
        generic_params.is_empty().then_some(export_name.as_str()),
        lone_value.then_some(value_field),
    );
    let conformances = if hashable {
        "Codable, Sendable, Hashable"
    } else {
        "Codable, Sendable"
    };
    let init = value_init(sealed, &real_type);
    let property = if drafted { "var" } else { "let" };
    let declaration = format!(
        "public struct {declared}{generic_params}: {conformances} {{ \
         public {property} value: {real_type}; \
         {init}\
         {reads}\
         public func encode(to encoder: Encoder) throws {{ {encode} }}; {validate} }}; {helpers}"
    );
    let body = if drafted {
        let form = wrapper_form(wrapped, hashable, &real_type);
        with_valid_struct(&declaration, &export_name, &generic_params, &form)
    } else {
        declaration
    };
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

/// The statement that reads the one value of a wrapper from its container.
fn wrapper_decode(shaped: &Shaped, real_type: &str, type_parameters: &[String]) -> String {
    if shaped.converts() {
        format!(
            "try {{ let wireValue = try {}; return {} }}()",
            read::fitted(
                "container.decode(",
                &wire_type(&shaped.def, &shaped.shape),
                ".self)",
                type_parameters
            ),
            shaped.converted("wireValue")
        )
    } else {
        format!(
            "try {}",
            read::fitted("container.decode(", real_type, ".self)", type_parameters)
        )
    }
}

/// The initializers of a struct of one `value`: a brand's is built through `init(validating:)`
/// alone, and the `init(value:)` it keeps is `fileprivate`.
fn value_init(sealed: bool, real_type: &str) -> String {
    if sealed {
        format!(
            "fileprivate init(value: {real_type}) {{ self.value = value }}; \
             public init(validating value: {real_type}) throws {{ self.init(value: value); \
             let violations = self.validate(); if !violations.isEmpty {{ throw \
             DecodingError.dataCorrupted(DecodingError.Context(codingPath: [], \
             debugDescription: violations.joined(separator: \"; \"))) }} }}; "
        )
    } else {
        format!("public init(value: {real_type}) {{ self.value = value }}; ")
    }
}

/// The statements of the `encode(to:)` of a struct of one `value`, `shaped`: the value writes
/// itself through the wrapper's encoder, as serde writes a newtype, so a flattened one joins its
/// keys to the holder's object, which a single value written over it would replace.
fn wrapper_write(shaped: &Shaped) -> String {
    let written = shaped.written("value");
    if written == "value" {
        "try value.encode(to: encoder)".to_owned()
    } else {
        format!("let written = {written}; try written.encode(to: encoder)")
    }
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
    let walked = validate::walks(def) || handed_keys;
    let functions = preprocess::functions(def);
    if !functions.is_empty() {
        corrected_member(
            &preprocess::Member {
                def,
                flat: false,
                functions,
                hint,
                keys: "",
                plain: preprocess::Plain::Declared,
                real_type,
                refusing: false,
                stem: "value",
                units: &[],
                walked,
            },
            Some(shaped),
            &source,
            &held,
            twins,
        )
    } else if walked {
        read::unjudged(def, hint, &source, &held, &mut read::Locals::default())
    } else if read::holds_a_length(def) {
        read::counted(
            shaped,
            "value",
            &source,
            &held,
            &mut read::Locals::default(),
        )
    } else {
        format!("let container = try decoder.singleValueContainer(); self.value = {decode}")
    }
}

/// The statements of the own read of a struct of one `value`, `shaped`: a tuple struct reads its
/// tuple alone, and a brand reads its value item by item where it holds an array of a fixed length
/// and as `decode` reads it otherwise. A value that takes the keys the types around it took is
/// handed them, and judged there as its own read judges it.
fn wrapper_read(wrapped: &Wrapped, shaped: &Shaped, real_type: &str, decode: &str) -> String {
    if matches!(wrapped, Wrapped::Tuple) {
        return format!("self.value = try {real_type}(swiftSchemaUnjudged: decoder)");
    }
    let value_field = &shaped.def;
    if read::holds_a_length(value_field) {
        return read::counted(
            shaped,
            "value",
            &read::Source::Decoder,
            &|held| format!("self.value = {held}"),
            &mut read::Locals::default(),
        );
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
        field_def.named_within(args.type_name, &type_parameters);
        field_def
    };
    let export_name = compute_item_export_name(&item_struct.ident.to_string(), args.name_override);
    let owner = declared_name(&export_name, args.undrafted);
    if let Some(only) = unnamed.unnamed.first()
        && unnamed.unnamed.len() == 1
    {
        let value_field = slot_def(only);
        let mut rules = validate::lone_slot_rules(&owner, &value_field);
        rules.extend(validate::own_checks(args.check));
        return value_wrapper_tokens(
            &item_struct.ident,
            &item_struct.generics,
            args,
            &value_field,
            &Wrapped::Slot,
            &rules,
            hashes_as_key(&item_struct.attrs, &value_field),
        );
    }
    let slots: Vec<FieldDef> = unnamed
        .unnamed
        .iter()
        .filter(|slot| !parse_serde_key_omission(&slot.attrs).absent_from_wire())
        .map(slot_def)
        .collect();
    let mut rules = validate::tuple_struct_rules(&owner, &slots);
    rules.extend(validate::own_checks(args.check));
    let tuple_field = FieldDef {
        absent_from_wire: false,
        array_depth: 0,
        array_lengths: Vec::new(),
        docs: String::new(),
        description: None,
        field_type: FieldDefType::Tuple(slots),
        model_schema_prop_meta: None,
        name: "value".to_owned(),
        nullable_levels: Vec::new(),
        omits_value: false,
        missing_key: MissingKey::Required,
        serde_default: None,
        bytes: false,
        set_levels: Vec::new(),
        scope: Scope::default(),
        type_span: proc_macro2::Span::call_site(),
    };
    value_wrapper_tokens(
        &item_struct.ident,
        &item_struct.generics,
        args,
        &tuple_field,
        &Wrapped::Tuple,
        &rules,
        false,
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
    validate::note_length(&item_type.ident.to_string(), validate::length_of(&target));
    if validate::names_a_bounded_type(&target) {
        validate::note_bounded(&item_type.ident.to_string());
    }
    preprocess::note(
        &item_type.ident.to_string(),
        !target.is_optional() && preprocess::beside(&target),
    );
    preprocess::note_only_keys(&item_type.ident.to_string(), None, Some(&target));
    // The alias of an externally tagged enum is read and written by the enum's entries.
    if !target.is_list() && !target.is_optional() {
        note_unit_entries(&item_type.ident.to_string(), unit_entries_of(&target));
        preprocess::note_entries(
            &item_type.ident.to_string(),
            preprocess::entries_of(&target).map_or_else(Vec::new, |(_, entries)| entries),
        );
    }
    let counted = (type_parameters.is_empty() && read::holds_a_length(&target)).then(|| {
        through_alias(&target).map_or_else(
            || (target.clone(), export_name.clone()),
            |held| (held.def, held.hint),
        )
    });
    note_counted_alias(&item_type.ident.to_string(), counted);
    note_optional_alias(&item_type.ident.to_string(), held_as_optional(&target));
    keys::note_alias(
        &item_type.ident.to_string(),
        type_parameters.is_empty().then_some(&target),
    );
    let generic_params = swift_generic_params(&item_type.generics);
    let mut aux = Vec::new();
    let (real_type, shape) = swift_resolve(&target, &export_name, &mut aux);
    let target_type = if target.array_depth == 0 && !target.is_optional() {
        let hint = export_name.clone();
        converted_type(
            &real_type,
            &Shaped {
                def: target.clone(),
                hint,
                shape,
            },
            &mut aux,
        )
    } else {
        real_type
    };
    read::note_alias(&export_name, &type_parameters, &target_type);
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
    rename_all_fields: Option<&str>,
    step: Option<&str>,
    aux: &mut Vec<String>,
) -> VariantPayload {
    within_variant(&variant.ident, || match variant_wire_kind(variant) {
        VariantKind::Unit => VariantPayload {
            beside: SlotEntries::default(),
            bound: Vec::new(),
            converted: None,
            corrected: None,
            held: None,
            keys: Some(Vec::new()),
            optional: false,
            parameters: Vec::new(),
            rules: Vec::new(),
            takes_keys: false,
            walked: None,
            written: false,
        },
        VariantKind::Named => named_payload(shape, variant, rename_all_fields, step, aux),
        VariantKind::TupleSingle => slot_payload(shape, variant, step, aux),
        VariantKind::TupleMultiple => slots_payload(shape, variant, step, aux),
    })
}

/// The payload of a variant with named members: the struct written for them, each member read as
/// the default the enum's defaults hold for it in this variant. The struct declares the enum's
/// parameters its members name, and the case holds it at those parameters.
fn named_payload(
    shape: &EnumShape<'_>,
    variant: &Variant,
    rename_all_fields: Option<&str>,
    step: Option<&str>,
    aux: &mut Vec<String>,
) -> VariantPayload {
    let export_name = shape.export_name;
    let struct_name = format!("{export_name}{}Payload", variant.ident);
    let variant_name = variant.ident.to_string();
    let fields = collect_swift_fields(
        &variant.fields,
        variant_field_rule(variant, rename_all_fields),
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
    // Under a tag alone the payload is read from the object the tag is in, so it takes the keys
    // the enum hands it, the tag among them, where no member of its own reads one.
    let keys = closed_keys(&fields, None, &[]);
    let declared = struct_declaration(
        &struct_name,
        &generic_params,
        &fields,
        None,
        &members,
        OwnRead {
            around: shape.tag_attrs.tag_alone(),
            closed: keys.clone(),
            corrects: &[],
            valid: None,
        },
    );
    let takes_keys = read::takes_keys(&declared);
    aux.push(declared);
    let bound = fields
        .iter()
        .flat_map(|field| {
            validate::bound_alone(
                shape.declared,
                &validate::Checked {
                    real_type: None,
                    access: format!("payload.{}", field.swift_name),
                    def: &field.field_def,
                    step: (!field.flatten).then(|| field.wire_name.clone()),
                },
            )
        })
        .collect();
    VariantPayload {
        beside: SlotEntries::default(),
        bound,
        converted: None,
        corrected: None,
        held: Some(held_at(&struct_name, &generic_params)),
        keys,
        optional: false,
        parameters: named_parameters,
        rules: validate::payload_rules(shape.declared, step, !members.is_empty()),
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
    field_def.named_within(shape.args.type_name, shape.type_parameters);
    let hint = format!("{export_name}{}Value", variant.ident);
    let (held, slot) = slot_resolve(&field_def, &hint, aux);
    let checked = validate::Checked {
        real_type: None,
        access: "payload".to_owned(),
        def: &field_def,
        step: step.map(ToOwned::to_owned),
    };
    let rules = validate::member_rules(shape.declared, &checked);
    let bound = validate::bound_alone(shape.declared, &checked);
    // Under a tag alone and under none, the slot is read from the decoder of the enum.
    let handed_keys = shape.tag_attrs.content.is_none()
        && (shape.tag_attrs.untagged || shape.tag_attrs.tag.is_some())
        && preprocess::beside(&field_def);
    let walked =
        (validate::walks(&field_def) || handed_keys).then(|| (field_def.clone(), hint.clone()));
    // Under a tag alone a type that takes no keys is read from the object without the tag, but a
    // struct or an enum under a tag key written above, which reads the tag as TypeScript and Zod
    // read it.
    let corrected = (!preprocess::functions(&field_def).is_empty()
        || (shape.tag_attrs.tag_alone()
            && !handed_keys
            && !preprocess::reads_the_whole(&field_def)))
    .then(|| (field_def.clone(), hint));
    let beside = if shape.tag_attrs.tag_alone() {
        SlotEntries {
            units: preprocess::units_of(&field_def),
            value: if wraps_entries(&field_def) {
                ".value"
            } else {
                ""
            },
            written: unit_entries_of(&field_def),
        }
    } else {
        SlotEntries::default()
    };
    VariantPayload {
        beside,
        bound,
        converted: (slot.converts() || read::holds_a_length(&slot.def)).then_some(slot),
        corrected,
        held: Some(held),
        keys: flattened_contribution(&field_def),
        optional: held_as_optional(&field_def),
        parameters: parameters_reached(&[&field_def]),
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
    let tuple_field = written_slots(variant, |field| {
        let mut field_def = member_def(shape.args, "slot", field);
        field_def.named_within(shape.args.type_name, shape.type_parameters);
        field_def
    });
    let slots: &[FieldDef] = if let FieldDefType::Tuple(slots) = &tuple_field.field_type {
        slots
    } else {
        &[]
    };
    let checked = !validate::slot_rules(shape.declared, slots).is_empty();
    let bound = slots
        .iter()
        .enumerate()
        .flat_map(|(position, slot)| {
            validate::bound_alone(
                shape.declared,
                &validate::Checked {
                    real_type: None,
                    access: format!("payload.slot{position}"),
                    def: slot,
                    step: Some(position.to_string()),
                },
            )
        })
        .collect();
    let hint = format!("{export_name}{}", variant.ident);
    // The struct of a tuple of no slot reads and writes the `[]` serde writes for the variant.
    let held = if slots.is_empty() {
        tuple_shape(&[], &hint, aux).real_leaf
    } else {
        swift_full_real_type(&tuple_field, &hint, aux)
    };
    VariantPayload {
        beside: SlotEntries::default(),
        bound,
        converted: None,
        corrected: None,
        held: Some(held),
        keys: None,
        optional: false,
        parameters: parameters_reached(&[&tuple_field]),
        rules: validate::payload_rules(shape.declared, step, checked),
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
fn plain_enum_swift_source(shape: &EnumShape<'_>) -> String {
    let &EnumShape {
        args,
        declared,
        defaults: _defaults,
        export_name,
        generic_params: _generic_params,
        item_enum,
        rust_ident,
        tag_attrs: _tag_attrs,
        type_parameters: _type_parameters,
        valid: _valid,
    } = shape;
    let checks = args.check;
    let rule = container_rename_rule(&item_enum.attrs);
    let variants: Vec<(String, String, Vec<String>)> = item_enum
        .variants
        .iter()
        .map(|variant| {
            let (case_name, wire) = variant_names(variant, rule);
            (case_name, wire, aliases::written_on(&variant.attrs))
        })
        .collect();
    let mut cases = variants
        .iter()
        .fold(String::new(), |mut acc, (case_name, wire, _)| {
            write!(acc, "case {case_name} = {}; ", swift_quoted(wire)).unwrap();
            acc
        });
    cases.push_str(&aliases::raw_value_init(&variants));
    validate::note(rust_ident, None, !checks.is_empty());
    let rules = validate::own_checks(checks);
    let validate = shape.valid.map_or_else(
        || validate::method(&rules),
        |valid| validate::draft_method(&rules, valid),
    );
    let reads = read::both(
        &format!(
            "let raw: String; var entry: SwiftSchemaAnyKey? = nil; if let single = try? \
             decoder.singleValueContainer(), let text = try? single.decode(String.self) {{ raw = \
             text }} else {{ guard let container = try? decoder.container(keyedBy: \
             SwiftSchemaAnyKey.self), container.allKeys.count == 1, let key = \
             container.allKeys.first else {{ throw \
             DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
             debugDescription: \"{EXPECTED_VALUE}\")) }}; entry = key; raw = key.stringValue }}; \
             guard let read = {declared}(rawValue: raw) else {{ throw \
             DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
             debugDescription: \"unknown variant `\\(raw)`\")) }}; if let entry, let container = try? \
             decoder.container(keyedBy: SwiftSchemaAnyKey.self), !((try? \
             container.decodeNil(forKey: entry)) ?? false) {{ throw \
             DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath + \
             [entry], debugDescription: \"invalid type: expected unit\")) }}; self = read"
        ),
        &preprocess::Written {
            draft: shape.valid.is_some(),
            named: declared,
            own: args.preprocess,
            ..preprocess::Written::default()
        },
    );
    let body =
        format!("public enum {declared}: String, Codable, Sendable {{ {cases}{reads}{validate} }}");
    let typealias = ident_typealias(rust_ident, export_name, "");
    format!("{body}{typealias}")
}

/// The Swift tokens the externally tagged shape earns — serde's default once a variant carries
/// data. Swift's own associated-value synthesis wraps a payload under an auto-generated `_0` key
/// rather than serde's bare `{"Foo": payload}`, so this writes the codec by hand instead. A value
/// that is neither a string nor an object of one key is refused in serde's words for text that is
/// not the document: `allKeys` keeps no order to pick by.
fn external_tagged_enum_swift_source(shape: &EnumShape<'_>, aux: &mut Vec<String>) -> String {
    let &EnumShape {
        args: _args,
        defaults: _defaults,
        export_name,
        declared,
        generic_params,
        item_enum,
        rust_ident,
        tag_attrs,
        type_parameters: _type_parameters,
        valid: _valid,
    } = shape;
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let mut cases = String::new();
    let mut unit_decode_arms = String::new();
    let mut object_decode_arms = String::new();
    let mut encode_arms = String::new();
    let mut ruled: Vec<(String, Vec<String>)> = Vec::new();
    let mut slots: Vec<SwiftFieldShape> = Vec::new();
    let mut twins = Vec::new();
    for variant in &item_enum.variants {
        let (case_name, wire_tag) = variant_names(variant, variant_rule);
        let payload = resolve_variant_payload(
            shape,
            variant,
            tag_attrs.rename_all_fields.as_deref(),
            Some(wire_tag.as_str()),
            aux,
        );
        unit_decode_arms.push_str(&carried_unit_arm(variant, &wire_tag));
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
            write!(
                object_decode_arms,
                "case {}: {read}; ",
                aliases::case_names(&wire_tag, &aliases::written_on(&variant.attrs))
            )
            .unwrap();
            encode_arms.push_str(&external_encode_arm(
                &case_name,
                &wire_tag,
                Some(&payload.written("payload")),
            ));
        } else {
            write!(
                unit_decode_arms,
                "case {}: self = .{case_name}; return; ",
                aliases::case_names(&wire_tag, &aliases::written_on(&variant.attrs))
            )
            .unwrap();
            encode_arms.push_str(&external_encode_arm(&case_name, &wire_tag, None));
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
             {}switch key.stringValue {{ {object_decode_arms} \
             default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(key.stringValue)\")) }}",
            one_key_container("SwiftSchemaExternalCodingKey")
        ),
        &enum_written(shape, "", twins, None),
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ switch self {{ {encode_arms} }} }};"
    );
    let validate = enum_validate(shape, &ruled);
    let body = format!(
        "public enum {declared}{generic_params}: Codable, Sendable {{ {cases} {key_type} {reads}{encode_method} {validate} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{}{typealias}", enum_date_helpers(shape, &slots))
}

/// The arm of the `encode(to:)` of an externally tagged enum for the variant `case_name`, written
/// under `wire_tag`: `written`, its payload, under the name, or the name alone for a unit variant.
fn external_encode_arm(case_name: &str, wire_tag: &str, written: Option<&str>) -> String {
    let tag = swift_quoted(wire_tag);
    written.map_or_else(
        || {
            format!(
                "case .{case_name}: var single = encoder.singleValueContainer(); \
                 try single.encode({tag}); "
            )
        },
        |payload| {
            format!(
                "case .{case_name}(let payload): var container = \
                 encoder.container(keyedBy: SwiftSchemaExternalCodingKey.self); \
                 try container.encode({payload}, forKey: \
                 SwiftSchemaExternalCodingKey(stringValue: {tag})!); "
            )
        },
    )
}

/// The arm of the `encode(to:)` of an enum under a tag key for the variant `case_name`: its tag,
/// `wire_tag`, under the tag key, then `then`, the statement that writes its payload, where it
/// holds one.
fn tagged_encode_arm(case_name: &str, wire_tag: &str, then: Option<&str>) -> String {
    let tag = format!(
        "try container.encode({}, forKey: .swiftSchemaTag); ",
        swift_quoted(wire_tag)
    );
    then.map_or_else(
        || format!("case .{case_name}: {tag}"),
        |written| format!("case .{case_name}(let payload): {tag}{written}"),
    )
}

/// The statement that refuses a key beside the tag `tag_key` of a variant that is none of the
/// variant's `keys` and that the members flattened ahead of the enum did not take, as the Rust read
/// of a strict enum does for a unit variant: none for a variant that reads such a key past and for
/// an enum that names a function on itself.
fn beside_the_tag(tag_key: &str, keys: &[String], corrects: &[FunctionName]) -> String {
    if is_nonstrict() || !corrects.is_empty() {
        return String::new();
    }
    let unlisted = if keys.is_empty() {
        format!("unknown.stringValue != {}", swift_quoted(tag_key))
    } else {
        let beside: Vec<String> = iter::once(tag_key.to_owned())
            .chain(keys.iter().cloned())
            .collect();
        format!(
            "!{}.contains(unknown.stringValue)",
            preprocess::listed(&beside)
        )
    };
    format!(
        "if !(decoder.codingPath.isEmpty && \
         decoder.userInfo[CodingUserInfoKey(rawValue: \"swiftSchemaFlat\")!] != nil) {{ \
         for unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys \
         where {unlisted} && !taken.contains(unknown.stringValue) {{ \
         throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         decoder.codingPath + [unknown], debugDescription: \"unknown field \
         `\\(unknown.stringValue)`\")) }} }}; "
    )
}

/// What [`beside_the_tag`] writes in the arm of the variant `variant` of a `nonstrict` enum that
/// refuses a stray key, where the keys of its payload are known: a strict enum's own read refuses
/// such a key for every variant at once.
fn refused_in_its_arm(
    variant: &Variant,
    keys: Option<&[String]>,
    tag_key: &str,
    corrects: &[FunctionName],
) -> String {
    let Some(known) = keys.filter(|_| is_nonstrict()) else {
        return String::new();
    };
    within_variant(&variant.ident, || beside_the_tag(tag_key, known, corrects))
}

/// The Swift tokens the internally tagged shape earns (`tag = "..."`, no `content`): the tag
/// merges into the same object a `Named` payload's own fields sit in, so a variant's own payload
/// struct decodes/encodes through the *same* `decoder`/`encoder` the enum's own tag read from.
fn internal_tagged_enum_swift_source(shape: &EnumShape<'_>, aux: &mut Vec<String>) -> String {
    let &EnumShape {
        args,
        defaults: _defaults,
        export_name,
        declared,
        generic_params,
        item_enum,
        rust_ident,
        tag_attrs,
        type_parameters: _type_parameters,
        valid: _valid,
    } = shape;
    let tag_key = tag_attrs.tag.as_deref().unwrap_or("type");
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
    let mut cases = String::new();
    let mut decode_arms = String::new();
    let mut encode_arms = String::new();
    let mut ruled: Vec<(String, Vec<String>)> = Vec::new();
    let mut slots: Vec<SwiftFieldShape> = Vec::new();
    let mut twins = Vec::new();
    let mut variant_keys: Option<Vec<String>> = Some(vec![tag_key.to_owned()]);
    // A payload is read from the object the tag is in, and is not handed the tag.
    let source = read::Source::Beside(preprocess::taken_with(iter::once(tag_key)));
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = swift_member(&RenameRule::CamelCase.apply_to_variant(&rust_name));
        let wire_tag = rename_override(&variant.attrs)
            .unwrap_or_else(|| variant_rule.apply_to_variant(&rust_name));
        let payload = resolve_variant_payload(
            shape,
            variant,
            tag_attrs.rename_all_fields.as_deref(),
            None,
            aux,
        );
        ruled.push((case_name.clone(), payload.rules.clone()));
        cases.push_str(&variant_case(&case_name, payload.held.as_deref()));
        slots.extend(payload.converted.as_ref().map(|slot| slot.shape.clone()));
        variant_keys = with_keys(variant_keys, payload.keys.as_ref());
        if payload.held.is_some() {
            let from = VariantRead {
                keys: "",
                source: &source,
                stem: swift_bare(&case_name),
            };
            let read = buffered_read(
                variant,
                payload.read(
                    &from,
                    &|held| format!("self = .{case_name}({held})"),
                    &mut twins,
                ),
            );
            write!(
                decode_arms,
                "case {}: {}; ",
                aliases::case_names(&wire_tag, &aliases::written_on(&variant.attrs)),
                guarded_read(variant, &payload, (tag_key, args.preprocess), &read)
            )
            .unwrap();
            let then = format!("{}; ", payload.written_beside("payload"));
            encode_arms.push_str(&tagged_encode_arm(&case_name, &wire_tag, Some(&then)));
        } else {
            let beside = within_variant(&variant.ident, || {
                beside_the_tag(tag_key, &[], args.preprocess)
            });
            write!(
                decode_arms,
                "case {}: {beside}self = .{case_name}; ",
                aliases::case_names(&wire_tag, &aliases::written_on(&variant.attrs))
            )
            .unwrap();
            encode_arms.push_str(&tagged_encode_arm(&case_name, &wire_tag, None));
        }
    }
    let tag_coding_key = tag_coding_keys(tag_key);
    let reads = read::both(
        &format!(
            "let container = try decoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); \
             let tag = try container.decode(String.self, forKey: .swiftSchemaTag); \
             switch tag {{ {decode_arms} default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(tag)\")) }}"
        ),
        &enum_written(
            shape,
            "",
            twins,
            closed_when_strict(variant_keys, args.preprocess),
        ),
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); \
         switch self {{ {encode_arms} }} }};"
    );
    let key = read::any_key_written(&reads, &encode_method);
    let validate = enum_validate(shape, &ruled);
    let body = format!(
        "public enum {declared}{generic_params}: Codable, Sendable {{ {cases} {tag_coding_key} {reads}{key}{encode_method} {validate} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{}{typealias}", enum_date_helpers(shape, &slots))
}

/// The coding keys an internally tagged enum reads its tag through.
fn tag_coding_keys(tag_key: &str) -> String {
    format!(
        "private enum SwiftSchemaTagCodingKeys: String, CodingKey {{ case swiftSchemaTag = {} }};",
        swift_quoted(tag_key)
    )
}

/// `read`, the read of a variant under a tag alone, through the refusal that names its enum where
/// the variant is braced: serde reads such a variant from the keys its enum buffers.
fn buffered_read(variant: &Variant, read: String) -> String {
    if matches!(variant.fields, Fields::Named(_)) {
        read::flattened(&read)
    } else {
        read
    }
}

/// The statements of the arm of `variant`, under the tag `tag_key` alone, that refuse a stray key
/// and then `read` its payload, placing the refusal of a member serde reads from the content the
/// tag's object buffers at the holder.
fn guarded_read(
    variant: &Variant,
    payload: &VariantPayload,
    (tag_key, corrects): (&str, &[FunctionName]),
    read: &str,
) -> String {
    let beside = refused_in_its_arm(variant, payload.keys.as_deref(), tag_key, corrects);
    if variant_buffers_members(variant) {
        format!("{beside}{}", read::buffered(read))
    } else {
        format!("{beside}{read}")
    }
}

/// Whether the variant has named members, one of which can hold a value serde refuses as text that
/// is not the document, which serde reads from the content the tag's object buffers.
fn variant_buffers_members(variant: &Variant) -> bool {
    variant_wire_kind(variant) == VariantKind::Named
        && variant
            .fields
            .iter()
            .any(|field| get_field_def("value", &field.ty, "").reads_one_key())
}

/// The arm that reads a unit variant under a tag and a content key from an object with no content
/// key alone, as Zod reads it: its schema of the variant is a strict object of the tag alone, so a
/// content is refused, `null` among it, which serde reads.
fn adjacent_unit_arm(names: &str, case_name: &str, export_name: &str) -> String {
    format!(
        "case {names}: guard !container.contains(.swiftSchemaContent) else {{ throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: \"{export_name}: \\(tag) takes no value\")) }}; self = .{case_name}; "
    )
}

/// The Swift tokens the adjacently tagged shape earns (`tag = "...", content = "..."`): the tag
/// and the payload sit at two keys of the same object, so a variant's own payload decodes and
/// encodes through the `content` key directly, needing no shared-decoder trick.
fn adjacent_tagged_enum_swift_source(shape: &EnumShape<'_>, aux: &mut Vec<String>) -> String {
    let &EnumShape {
        args,
        defaults: _defaults,
        export_name,
        declared,
        generic_params,
        item_enum,
        rust_ident,
        tag_attrs,
        type_parameters: _type_parameters,
        valid: _valid,
    } = shape;
    let tag_key = tag_attrs.tag.as_deref().unwrap_or("type");
    let content_key = tag_attrs.content.as_deref().unwrap_or("content");
    let variant_rule = resolve_rename_rule(tag_attrs.rename_all.as_deref());
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
        let payload = resolve_variant_payload(
            shape,
            variant,
            tag_attrs.rename_all_fields.as_deref(),
            Some(content_key),
            aux,
        );
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
            write!(
                decode_arms,
                "case {}: {read}; ",
                aliases::case_names(&wire_tag, &aliases::written_on(&variant.attrs))
            )
            .unwrap();
            let then = format!(
                "try container.encode({}, forKey: .swiftSchemaContent); ",
                payload.written("payload")
            );
            encode_arms.push_str(&tagged_encode_arm(&case_name, &wire_tag, Some(&then)));
        } else {
            decode_arms.push_str(&adjacent_unit_arm(
                &aliases::case_names(&wire_tag, &aliases::written_on(&variant.attrs)),
                &case_name,
                export_name,
            ));
            encode_arms.push_str(&tagged_encode_arm(&case_name, &wire_tag, None));
        }
    }
    let (tagging_keys, closed) = adjacent_keys(tag_key, content_key, args.preprocess);
    let reads = read::both(
        &format!(
            "let container = try decoder.container(keyedBy: SwiftSchemaAdjacentCodingKeys.self); \
             let tag = try container.decode(String.self, forKey: .swiftSchemaTag); \
             switch tag {{ {decode_arms} default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(tag)\")) }}"
        ),
        &enum_written(shape, "", twins, closed),
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ \
         var container = encoder.container(keyedBy: SwiftSchemaAdjacentCodingKeys.self); \
         switch self {{ {encode_arms} }} }};"
    );
    let validate = enum_validate(shape, &ruled);
    let body = format!(
        "public enum {declared}{generic_params}: Codable, Sendable {{ {cases} {tagging_keys} {reads}{encode_method} {validate} }}"
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
        declared,
        generic_params,
        item_enum,
        rust_ident,
        tag_attrs,
        type_parameters: _type_parameters,
        valid: _valid,
    } = shape;
    let mut cases = String::new();
    let mut choosers = String::new();
    let mut decode_attempts = String::new();
    let mut encode_arms = String::new();
    let mut ruled: Vec<(String, Vec<String>)> = Vec::new();
    let mut slots: Vec<SwiftFieldShape> = Vec::new();
    let mut twins = Vec::new();
    // A payload is read from the decoder of the enum, and is handed the keys the enum is.
    let source = read::Source::Beside(preprocess::taken_with(iter::empty()));
    let mut variant_keys: Option<Vec<String>> = Some(Vec::new());
    for variant in &item_enum.variants {
        let rust_name = variant.ident.to_string();
        let case_name = swift_member(&RenameRule::CamelCase.apply_to_variant(&rust_name));
        let payload = resolve_variant_payload(
            shape,
            variant,
            tag_attrs.rename_all_fields.as_deref(),
            None,
            aux,
        );
        ruled.push((case_name.clone(), payload.rules.clone()));
        cases.push_str(&variant_case(&case_name, payload.held.as_deref()));
        slots.extend(payload.converted.as_ref().map(|slot| slot.shape.clone()));
        if payload.held.is_some() {
            variant_keys = with_keys(variant_keys, payload.keys.as_ref());
        }
        if let Some(ty) = &payload.held {
            let named = variant.ident.unraw();
            let from = VariantRead {
                keys: "",
                source: &source,
                stem: swift_bare(&case_name),
            };
            let returned = payload.read(&from, &|held| format!("return {held}"), &mut twins);
            let reader = format!("swiftSchemaRead{named}");
            let read = tried_read(&returned, declared, &reader, ty, &mut choosers);
            let tried = within_variant(&variant.ident, || {
                checked_alone(payload.keys.as_ref(), &read, declared, &mut choosers)
            });
            let check = bound_check(
                &payload.bound,
                (declared, &named.to_string(), ty),
                &mut choosers,
            );
            decode_attempts.push_str(&untagged_attempt(
                &case_name,
                &tried,
                check.as_deref(),
                payload.optional,
            ));
            write!(
                encode_arms,
                "case .{case_name}(let value): try {}.encode(to: encoder); ",
                payload.written("value")
            )
            .unwrap();
        } else {
            null_variant(&case_name, &mut decode_attempts, &mut encode_arms);
        }
    }
    let valid = shape.valid.unwrap_or(export_name);
    let reads = read::both(
        &format!(
            "{decode_attempts}throw DecodingError.typeMismatch({valid}.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"no untagged member matched\"))"
        ),
        &enum_written(
            shape,
            &choosers,
            twins,
            closed_when_strict(variant_keys, args.preprocess),
        ),
    );
    let encode_method = format!(
        "public func encode(to encoder: Encoder) throws {{ switch self {{ {encode_arms} }} }};"
    );
    let validate = enum_validate(shape, &ruled);
    let key_type = read::any_key(&choosers);
    let helpers = validate::helpers(&choosers, &validate);
    let body = format!(
        "public enum {declared}{generic_params}: Codable, Sendable {{ {cases} {key_type}{choosers}{reads}{encode_method} {validate}{helpers} }}"
    );
    let typealias = ident_typealias(rust_ident, export_name, generic_params);
    format!("{body}{}{typealias}", enum_date_helpers(shape, &slots))
}

/// The statement an untagged enum tries the variant `case_name` with: its payload read by `tried`,
/// and the variant taken where `check`, the payload's own bounds, holds. A payload Swift holds as
/// an optional is read in a `do`, where `try?` would take a read that answered `nil` for one that
/// threw.
fn untagged_attempt(case_name: &str, tried: &str, check: Option<&str>, optional: bool) -> String {
    let taken = format!("self = .{case_name}(value); return");
    match (optional, check) {
        (true, None) => format!("do {{ let value = try {tried}; {taken} }} catch {{}}; "),
        (true, Some(holds)) => {
            format!("do {{ let value = try {tried}; if {holds} {{ {taken} }} }} catch {{}}; ")
        }
        (false, None) => format!("if let value = try? {tried} {{ {taken} }}; "),
        (false, Some(holds)) => format!("if let value = try? {tried}, {holds} {{ {taken} }}; "),
    }
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
    let helpers = leaf_helpers_for(shape.declared, shape.generic_params, &shapes, None);
    if helpers.is_empty() {
        helpers
    } else {
        format!("; {helpers}")
    }
}

/// What the reads of an enum under any tagging are written from: the draft's own name, the readers
/// `beside` it, the types written for its members joined by `twins`, and the keys it refuses a key
/// beside.
const fn enum_written<'shape>(
    shape: &'shape EnumShape<'shape>,
    beside: &'shape str,
    twins: Vec<String>,
    closed: Option<Vec<String>>,
) -> preprocess::Written<'shape> {
    preprocess::Written {
        beside,
        closed,
        draft: shape.valid.is_some(),
        named: shape.declared,
        own: shape.args.preprocess,
        sealed: false,
        twins,
    }
}

/// An enum's `validate()`: the rules of the case the value holds, then the enum's own checks.
fn enum_validate(shape: &EnumShape<'_>, variants: &[(String, Vec<String>)]) -> String {
    let mut rules: Vec<String> = validate::held_case(variants).into_iter().collect();
    rules.extend(validate::own_checks(shape.args.check));
    validate::note(shape.rust_ident, None, !rules.is_empty());
    shape.valid.map_or_else(
        || validate::method(&rules),
        |valid| validate::draft_method(&rules, valid),
    )
}

/// The constants of the valid type `export_name` of `item_enum`, one for each unit case of its
/// draft `declared`.
fn unit_constants(item_enum: &ItemEnum, export_name: &str, declared: &str) -> String {
    item_enum
        .variants
        .iter()
        .filter(|variant| variant_wire_kind(variant) == VariantKind::Unit)
        .fold(String::new(), |mut acc, variant| {
            let (case_name, _) = variant_names(variant, RenameRule::CamelCase);
            write!(
                acc,
                "public static let {case_name} = {export_name}(draft: {declared}.{case_name}); "
            )
            .unwrap();
            acc
        })
}

/// The Swift tokens an enum earns: a `String`-backed enum for an all-unit, untagged-by-default
/// declaration; otherwise the tagging shape `process_enum` itself dispatches the declaration to.
fn enum_swift_tokens(declared: &ItemEnum, args: &SwiftTypeArgs<'_>) -> TokenStream {
    // Plain or not as every variant declared reads, as `process_enum` dispatches it.
    let plain = is_plain_enum(declared);
    let item_enum = &variants_on_the_wire(declared);
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
    let drafted = !args.undrafted;
    let declared_as = declared_name(&export_name, args.undrafted);
    let valid = held_at(&export_name, &generic_params);
    let shape = EnumShape {
        args,
        declared: &declared_as,
        defaults: &defaults,
        export_name: &export_name,
        generic_params: &generic_params,
        item_enum,
        rust_ident: &rust_ident,
        tag_attrs: &tag_attrs,
        type_parameters: &type_parameters,
        valid: drafted.then_some(valid.as_str()),
    };
    let mut aux = Vec::new();
    let declaration = if plain && writes_bare_variant_names {
        plain_enum_swift_source(&shape)
    } else if tag_attrs.untagged {
        untagged_enum_swift_source(&shape, &mut aux)
    } else if tag_attrs.tag.is_some() && tag_attrs.content.is_some() {
        adjacent_tagged_enum_swift_source(&shape, &mut aux)
    } else if tag_attrs.tag.is_some() {
        internal_tagged_enum_swift_source(&shape, &mut aux)
    } else {
        external_tagged_enum_swift_source(&shape, &mut aux)
    };
    preprocess::note(&rust_ident, read::takes_keys(&declaration));
    let typealias = ident_typealias(&rust_ident, &export_name, &generic_params);
    let statics = if generic_params.is_empty() && args.check.is_empty() {
        unit_constants(item_enum, &export_name, &declared_as)
    } else {
        String::new()
    };
    let body = if drafted {
        let form = ValidForm {
            conformances: if plain && writes_bare_variant_names && generic_params.is_empty() {
                ", Hashable"
            } else {
                ""
            },
            keyed: None,
            raw: plain && writes_bare_variant_names,
            statics: &statics,
        };
        let unpaired = declaration
            .strip_suffix(typealias.as_str())
            .unwrap_or(&declaration);
        format!(
            "{}{typealias}",
            with_valid_struct(unpaired, &export_name, &generic_params, &form)
        )
    } else {
        declaration
    };
    preprocess::note_entries(&rust_ident, flattened_entries(item_enum, args.preprocess));
    preprocess::note_tag_keys(&rust_ident, tag_keys(&tag_attrs, args.preprocess));
    preprocess::note_tag_and_content(
        &rust_ident,
        tag_and_content_when_flattened(item_enum, args.preprocess),
    );
    let externally_tagged = writes_bare_variant_names && !plain;
    note_unit_entries(
        &rust_ident,
        if externally_tagged {
            let mut entries = unit_entries(item_enum, &tag_attrs);
            for entry in &mut entries {
                entry.drafted = drafted;
            }
            entries
        } else {
            Vec::new()
        },
    );
    let full_source = if aux.is_empty() {
        body
    } else {
        format!("{} {body}", aux.join(" "))
    };
    swift_module_tokens(&rust_ident, item_enum.ident.span(), &full_source, &defaults)
}
