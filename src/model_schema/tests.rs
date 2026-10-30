//! Unit tests of the `#[model_schema]` expansion: its guards, and the code each shape expands to.

use super::{
    EnumCasing, FieldDefType, ModelSchemaPropMeta, apply_serde_key_omission,
    check_nullable_ts_optional_conflict, check_undescribable_std_field,
    collect_discriminated_variants, field_label, get_field_def, render_discriminated_variants,
    validate_as_number_flag, validate_nullable_flag, validate_ts_optional_flag,
};

#[cfg(feature = "serde")]
use super::{
    CheckedMember, ConstraintGate, ConstraintLeaf, FieldContext, MemberAccess,
    adjacent_collapsed_slot_guard_errors, build_field_validation, cfg_attr_guard_error,
    check_nullable_field_serialization, check_omitted_key_is_readable,
    check_optional_field_serialization, checked_member, collect_untagged_members,
    constrained_shape, enum_cfg_attr_guard_errors, generate_field_validation,
    generate_list_validation_code, generate_numeric_validation_code,
    generate_string_validation_code, has_serde_default, helper_name_stem,
    internally_tagged_guard_errors, member_access_expr, needs_injected_default,
    parse_serde_field_attributes, parse_serde_type_attributes, render_untagged_variant,
};

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::{
    AliasKind, PublishedShape, alias_map_key_guard_error, alias_undescribable_std_error,
    branded_guard_errors, check_map_key, deferred_shape_question, deferred_shape_refusals,
    ident_schema_module_name, record_shape_question, record_value_shape, register_alias_info,
};

#[cfg(all(feature = "serde", feature = "zod"))]
use super::{
    NonObject, WireLeaf, flatten_edge_guard_error, record_wire_leaves, record_zod_union_members,
    surfaced_non_object, written_non_object,
};

#[cfg(feature = "zod")]
use super::{MergedOperand, SourceAbsence};
#[cfg(feature = "jsonschema")]
use crate::field_type::is_set_wrapper;
#[cfg(feature = "serde")]
use crate::foundation::{exec_decode_with_foundation, exec_mongodb_foundation};

#[cfg(all(feature = "zod", feature = "typescript"))]
use super::maps_of_itself_through_the_self_view;

#[cfg(feature = "typescript")]
use super::tuple_struct_ts_body;

#[cfg(feature = "zod")]
use super::tuple_struct_zod_body;

#[cfg(feature = "jsonschema")]
use super::tuple_struct_json_body;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::{check_slot_wire_is_readable, tuple_struct_shape};

#[cfg(feature = "swift")]
use super::swift_width_refusals;

#[cfg(not(feature = "indexmap"))]
use super::claim_published_name;

use super::{
    VariantKind, check_variant_slot_wire_is_readable, parse_serde_key_omission, variant_wire_kind,
};

#[cfg(feature = "zod")]
use core::slice::from_ref;
use syn::spanned::Spanned as _;

#[cfg(feature = "serde")]
use crate::utils::{Declared, declared, record_plain_enum};
use crate::utils::{is_recorded_untagged_enum, record_untagged_enum};

/// The slot `$slot` of `Lettered::Held` under a tag alone, the refusals the expansion raises for
/// it, and what serde answers for a write of `$value` through the same declaration.
#[cfg(feature = "serde")]
macro_rules! refused_beside_serde {
    ($slot:ty, $value:expr) => {{
        #[derive(serde::Serialize)]
        #[serde(tag = "kind")]
        enum Lettered {
            Held($slot),
        }
        let item: syn::ItemEnum =
            syn::parse_str(concat!("enum Lettered { Held(", stringify!($slot), ") }")).unwrap();
        (
            stringify!($slot),
            internal_guard_errors(&item),
            serde_json::to_string(&Lettered::Held($value))
                .unwrap_err()
                .to_string(),
        )
    }};
}

/// The variants of [`rendered_discriminated_union`]'s enum, in the order they are declared.
const DECLARED_VARIANTS: [&str; 6] = ["Upload", "Generate", "Delete", "Rename", "Move", "Archive"];

/// The doc attributes carrying a ` ```rust example ` block that the const-parameter probes are
/// written under, held apart so every probe writes the same block.
#[cfg(feature = "zod")]
const EXAMPLE_DOC_BLOCK: &str = "/// An item carrying an example block.\n\
                                 ///\n\
                                 /// ```rust example\n\
                                 /// Probe::Held\n\
                                 /// ```\n";

/// Every pattern the `pattern` guards must decide, invalid ones first, then the valid shapes the
/// shipped tests write.
const PROBE_PATTERNS: [&str; 10] = [
    r"^ab\",
    "^ab(",
    "*ab",
    "^[a-",
    r"\p{NotAClass}",
    "^[a-z]+$",
    r"^\d{3}\.\d{3}$",
    r"^a\n[a-z]+$",
    "^/[a-z]+$",
    r"^\/[a-z]+$",
];

/// Patterns the `regex` crate parses that no JavaScript regex literal carries, one per family the
/// guard sorts them into, beside the words the refusal names each by.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
const UNPORTABLE_PROBE_PATTERNS: [(&str, &str); 6] = [
    ("(?i)abc", "inline flag directive"),
    (r"^\p{L}+$", "Unicode class"),
    // The needle is doubled because the guard error is read back off rendered `compile_error!`
    // tokens, where the string literal's own escaping is still in place.
    (r"\Aabc", r"`\\A` anchor"),
    ("^[[:alpha:]]+$", "POSIX class"),
    (r"[\w&&\d]", "`&&` class intersection"),
    (r"\x{41}", "braced code point escape"),
];

/// Every slot spelling the refusal reads, beside whether it is refused.
const SLOT_OMISSION_SPELLINGS: [(&str, bool); 6] = [
    ("skip_serializing", true),
    ("skip_serializing_if = \"Option::is_none\"", true),
    ("skip_deserializing", true),
    ("skip", false),
    ("skip_serializing, skip_deserializing", false),
    ("default", false),
];

/// The covered wrappers, under the names a dispatch reads them by.
#[cfg(any(feature = "jsonschema", feature = "serde"))]
const SEQUENCE_WRAPPERS: [&str; 5] = ["BTreeSet", "BinaryHeap", "HashSet", "Vec", "VecDeque"];

/// The module name a generated hook is written against, standing in for the one the enum's own
/// expansion registers.
#[cfg(feature = "serde")]
const UNTAGGED_MODULE: Option<&str> = Some("choice_schema");

/// The refusal a `#[model_schema_prop]` on a brand's slot earns, less the prefix `item_label`
/// supplies in front of it.
const BRANDED_SLOT_PROP_REFUSAL: &str = "`#[model_schema_prop]` is unread on the slot of a \
     `#[serde(transparent)]` newtype -- a brand publishes its inner's own schema with a `.brand()` \
     written onto it, so no key written here reaches any surface. The checks a brand does carry \
     are written on the type itself: #[model_schema(pattern = \"...\", minLength = N, maxLength = \
     N)]. Move the check there, or drop the attribute.";

/// The refusal a `#[model_schema_prop]` on any other field of a brand earns, less the prefix
/// `item_label` supplies in front of it.
const UNREAD_BRAND_FIELD_PROP_REFUSAL: &str = "`#[model_schema_prop]` is unread on a field serde \
     neither reads nor writes -- a `#[serde(transparent)]` struct is read and written as the value \
     of one field alone, and a brand publishes that field's own schema with a `.brand()` written \
     onto it, so no key written here reaches any surface. Drop the attribute.";

/// What a map keyed by a tuple is refused with, after the place it is written in.
#[cfg(feature = "jsonschema")]
const TUPLE_KEY_REFUSAL: &str = "a map key must be a value serde writes as a string, which is what \
                                 a JSON object key is \u{2014} serde writes `(_, _)` as a JSON \
                                 array";

/// The keys of a map keyed by an integer, as the description of the map names them.
#[cfg(feature = "jsonschema")]
const INTEGER_KEY_NAMES: &str =
    r#"serde_json :: json ! ({ "type" : "string" , "pattern" : "^-?[0-9]+(\\.0+)?$" }))"#;

/// A brand with named fields whose skipped marker carries a key, as a consumer writes it.
const UNREAD_BRAND_FIELD_SOURCE: &str = "#[derive(Debug, Deserialize, Serialize)] \
     #[serde(transparent)] pub struct Label { pub text: String, \
     #[model_schema_prop(maxLength = 9)] #[serde(skip)] marker: PhantomData<u8> }";

/// A transparent struct whose two derives take `count` and `text` as its value, written with named
/// fields and with slots.
const SPLIT_TRANSPARENT_SOURCES: [(&str, &str, &str); 2] = [
    (
        "#[derive(Debug, Deserialize, Serialize)] #[serde(transparent)] pub struct Split { \
         #[serde(default)] pub count: u32, #[serde(skip_serializing)] pub text: String }",
        "field `count`",
        "field `text`",
    ),
    (
        "#[derive(Debug, Deserialize, Serialize)] #[serde(transparent)] pub struct Split(\
         #[serde(default)] pub u32, #[serde(skip_serializing)] pub String);",
        "slot 0",
        "slot 1",
    ),
];

/// Every key a slot can be written with, read and unread alike.
const SLOT_PROP_KEYS: [&str; 9] = [
    "",
    "pattern = \"^[a-z]+$\"",
    "minLength = 2",
    "as = String",
    "preprocess = [\"trim\"]",
    "literal = \"fixed\"",
    "ts_optional",
    "nullable",
    "bogus_key = 3",
];

/// One of each struct shape the flag is generated on beside a struct with named fields, each with
/// the arguments it is declared under.
#[cfg(feature = "serde")]
const DECODE_WITH_STRUCT_SHAPES: [(&str, &str); 11] = [
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodePair(pub String, pub u32);",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodeWrapper(pub String);",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodeTags(pub Vec<String>);",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(transparent)] pub struct DecodeBrand(pub String);",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"], no_display",
        "#[serde(transparent)] pub struct DecodePinned(pub DecodeInner);",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodeUnit;",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"], default_types(T = String)",
        "pub struct DecodePage<T> { pub items: Vec<T>, pub total: u32 }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"], default_types(T = String)",
        "#[serde(transparent)] pub struct DecodeWrapped<T>(pub T);",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(transparent)] pub struct DecodeHolder { pub inner: String }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(transparent)] pub struct DecodeCurrent { pub inner: DecodeInner }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"], default_types(T = String)",
        "#[serde(transparent)] pub struct DecodeSleeve<T> { pub inner: T }",
    ),
];

/// What `mongodb` adds for a flagged type's typed paths: the struct of them in its module, and on
/// the type the const that holds them.
#[cfg(feature = "serde")]
const TYPED_PATHS: [&str; 2] = ["pub struct MongoFields <", "pub const MONGO_FIELDS :"];

/// What `mongodb` adds for the operations of a type that lists "mongodb": on the type the reads,
/// the count and the writes. The error and the read they answer are the crate's library's.
#[cfg(feature = "serde")]
const OPERATIONS: [&str; 12] = [
    "pub fn find_one <",
    "pub fn find_one_with < 'c ,",
    "pub fn find <",
    "pub fn find_with < 'c ,",
    "pub fn count <",
    "pub fn insert_one < 'c > (",
    "pub async fn update_one <",
    "pub async fn update_many <",
    "pub async fn delete_one <",
    "pub async fn delete_many <",
    "fn mongo_read_row (",
    "fn mongo_written_row (",
];

/// One enum per form serde writes an enum in, then one per form whose variants carry an alias or
/// are never read, each with the arguments it is declared under.
#[cfg(feature = "serde")]
const DECODE_WITH_ENUM_FORMS: [(&str, &str); 10] = [
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub enum DecodeStatus { Draft, Published }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub enum DecodeOutline { Circle { radius: f64 }, Empty, Label(String), To(i32, i32) }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(tag = \"kind\")] pub enum DecodeFill { Clear, Solid { color: String }, Versioned(DecodeInner) }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(tag = \"kind\", content = \"data\")] pub enum DecodeStroke { Dashed { gap: u32 }, Hairline, Width(u32) }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[derive(Deserialize)] #[serde(untagged)] pub enum DecodeContact { Email { address: String }, Versioned(DecodeInner) }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"], default_types(T = String)",
        "pub enum DecodeAnswer<T> { Empty, Value(T) }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub enum DecodeContour { #[serde(alias = \"Round\")] Circle { radius: f64 }, #[serde(alias = \"Blank\")] Empty, #[serde(skip)] Lost }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(tag = \"kind\")] pub enum DecodeCoating { #[serde(alias = \"Blank\")] Clear, #[serde(skip_deserializing)] Retired, Solid { color: String } }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(tag = \"kind\", content = \"data\")] pub enum DecodeDash { #[serde(alias = \"Dotted\")] Dashed { gap: u32 }, #[serde(skip_deserializing)] Faded(u32), Hairline }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[derive(Deserialize)] #[serde(untagged)] pub enum DecodeChannel { Email { address: String }, #[serde(skip_deserializing)] Pager { number: i32 } }",
    ),
];

/// One type per source serde flattens into a struct, then one enum per form serde writes an enum
/// in whose struct variant flattens a field, each with the arguments it is declared under.
#[cfg(feature = "serde")]
const DECODE_WITH_FLATTEN_FORMS: [(&str, &str); 7] = [
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodeFlatEntry { #[serde(flatten)] pub audit: DecodeAudit, \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub extra: Option<DecodeExtra>, \
         #[serde(flatten)] pub fill: DecodeFill, pub title: String }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodeFlatBag { #[serde(flatten)] pub extra: HashMap<String, i32> }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"], default_types(T = DecodeBody)",
        "pub struct DecodeFlatEnvelope<T> { #[serde(flatten)] pub body: T, pub id: String }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub enum DecodeFlatOutline { Made { #[serde(flatten)] audit: DecodeAudit, title: String }, Gone }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(tag = \"kind\")] pub enum DecodeFlatFill { Made { #[serde(flatten)] audit: DecodeAudit }, Gone }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(tag = \"kind\", content = \"data\")] pub enum DecodeFlatStroke { Made { #[serde(flatten)] audit: DecodeAudit }, Gone }",
    ),
    (
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[derive(Deserialize)] #[serde(untagged)] pub enum DecodeFlatContact { Made { #[serde(flatten)] audit: DecodeAudit }, Gone { at: u32 } }",
    ),
];

/// What an enum declaring neither container-level casing rule hands its variant walk.
const UNCASED: EnumCasing<'static> = EnumCasing {
    variant_fields: None,
    variants: None,
};

/// Every type the reach of an enum's check is asked of, beside the answer.
const TEXT_REACH: [(&str, bool); 24] = [
    ("String", true),
    ("&'a str", true),
    ("PathBuf", true),
    ("std::path::PathBuf", true),
    ("Option<String>", true),
    ("Box<str>", true),
    ("Rc<String>", true),
    ("Arc<Option<PathBuf>>", true),
    ("Cow<'a, str>", true),
    ("Option<Box<String>>", true),
    ("Vec<String>", false),
    ("Option<Vec<String>>", false),
    ("[String; 2]", false),
    ("HashSet<String>", false),
    ("HashMap<String, String>", false),
    ("(String, String)", false),
    ("RefCell<String>", false),
    ("&'a String", true),
    ("KnownKind", false),
    ("Slug<String>", false),
    ("T", false),
    ("u32", false),
    ("bool", false),
    ("char", false),
];

/// Every list spelling, with how many wrappers it is reached through and whether a brand holds
/// it as its own value.
#[cfg(feature = "serde")]
const LIST_REACH: [(&str, usize, bool); 16] = [
    ("Vec<String>", 0, true),
    ("VecDeque<Part>", 0, true),
    ("HashSet<Tag>", 0, true),
    ("BTreeSet<u8>", 0, true),
    ("BinaryHeap<u8>", 0, true),
    ("[String; 2]", 0, true),
    ("Vec<Vec<String>>", 0, true),
    ("Vec<T>", 0, true),
    ("Vec<&'a str>", 0, true),
    ("Box<[u8]>", 1, true),
    ("Arc<Vec<Part>>", 1, true),
    ("Cow<'a, [u8]>", 1, true),
    ("Option<Vec<Part>>", 1, false),
    ("Option<Box<[Part]>>", 2, false),
    ("&'a [String]", 1, false),
    ("Option<&'a Vec<u8>>", 2, false),
];

/// The flag's verdict on a declaration's sole member, read the way `process_field` reads it: the
/// omission the field declares is on the def before the guard asks its question of it.
fn ts_optional_verdict(item: &syn::ItemStruct, flag_set: bool) -> Result<(), String> {
    let field = item.fields.iter().next().unwrap();
    let mut rendered = get_field_def("", &field.ty, "");
    apply_serde_key_omission(&mut rendered, field);
    validate_ts_optional_flag(field, &rendered, flag_set)
}

#[test]
fn ts_optional_ok_on_option_field() {
    ts_optional_verdict(
        &syn::parse_quote! { struct Report { name: Option<String> } },
        true,
    )
    .unwrap();
}

#[test]
fn ts_optional_ok_on_an_option_field_whose_key_is_dropped_one_way() {
    for item in [
        syn::parse_quote! {
            struct Report { #[serde(skip_serializing_if = "Option::is_none")] name: Option<String> }
        },
        syn::parse_quote! { struct Report { #[serde(skip_serializing)] name: Option<String> } },
    ] {
        ts_optional_verdict(&item, true).unwrap();
    }
}

#[test]
fn ts_optional_ok_when_flag_unset() {
    for item in [
        syn::parse_quote! { struct Report { name: Option<String> } },
        syn::parse_quote! { struct Report { name: String } },
        syn::parse_quote! { struct Report(Option<String>); },
        syn::parse_quote! { struct Report { #[serde(skip)] name: Option<String> } },
    ] {
        ts_optional_verdict(&item, false).unwrap();
    }
}

#[test]
fn ts_optional_err_on_non_option_field() {
    let err = ts_optional_verdict(&syn::parse_quote! { struct Report { name: String } }, true)
        .unwrap_err();
    assert!(err.contains("ts_optional"));
    assert!(err.contains("Option<T>"));
}

/// A positional slot writes no key, so the flag names a spelling the tuple line has no room for.
#[test]
fn ts_optional_err_on_a_positional_slot() {
    let err = ts_optional_verdict(&syn::parse_quote! { struct Report(Option<String>); }, true)
        .unwrap_err();
    assert_eq!(
        err,
        "#[model_schema_prop(ts_optional)] requires a named field: a positional slot writes no \
         key for the flag to make optional"
    );
}

#[test]
fn ts_optional_err_on_a_member_off_the_wire() {
    let err = ts_optional_verdict(
        &syn::parse_quote! { struct Report { #[serde(skip)] name: Option<String> } },
        true,
    )
    .unwrap_err();
    assert_eq!(
        err,
        "#[model_schema_prop(ts_optional)] requires a field the wire carries: a serde attribute \
         takes this one out of both directions, so no member is written for the flag to make \
         optional"
    );
}

#[cfg(feature = "chrono")]
#[test]
fn as_number_ok_on_datetime_field() {
    validate_as_number_flag(&FieldDefType::DateTime, true).unwrap();
}

#[test]
fn as_number_ok_when_flag_unset() {
    validate_as_number_flag(&FieldDefType::String, false).unwrap();
}

#[test]
fn as_number_err_on_non_datetime_field() {
    let err = validate_as_number_flag(&FieldDefType::String, true).unwrap_err();
    assert!(err.contains("as_number"));
    assert!(err.contains("DateTime<Tz>"));
}

/// The refusal of `as_number` on a date the hook it hangs reads no value of, `written` naming it.
#[cfg(all(feature = "serde", feature = "chrono"))]
fn as_number_hook_refusal(written: &str) -> String {
    format!(
        "model_schema: field `stamp`: `#[model_schema_prop(as_number)]` writes and reads the field \
         through chrono's `ts_milliseconds` hook, which takes a `DateTime<Utc>`, or an \
         `Option<DateTime<Utc>>` through `ts_milliseconds_option`, and the field is a `{written}`. \
         Declare the field as one of those two, or write and read it through functions of your \
         own: a `with`, or a `serialize_with` beside a `deserialize_with`."
    )
}

/// `as_number` hangs chrono's millisecond hook on a `DateTime<Utc>` and an `Option` of one, and
/// is refused on any other date the field holds where its author reads and writes it through no
/// function of their own.
#[cfg(all(feature = "serde", feature = "chrono"))]
#[test]
fn as_number_is_refused_on_a_date_the_millisecond_hook_reads_no_value_of() {
    for (field, written) in [
        ("pub stamp: Vec<DateTime<Utc>>", "Vec<DateTime<Utc>>"),
        ("pub stamp: DateTime<Local>", "DateTime<Local>"),
        ("pub stamp: Box<DateTime<Utc>>", "Box<DateTime<Utc>>"),
        (
            "#[serde(serialize_with = \"shown\")] pub stamp: DateTime<FixedOffset>",
            "DateTime<FixedOffset>",
        ),
    ] {
        assert_eq!(
            refusal_texts(&expansion_over(&format!(
                "#[derive(Serialize, Deserialize)] pub struct Stamped {{ \
                 #[model_schema_prop(as_number)] {field} }}"
            ))),
            [as_number_hook_refusal(written)],
            "for `{field}`"
        );
    }
    let answered = format!("error: {}", as_number_hook_refusal("Vec<DateTime<Utc>>"));
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    for field in [
        "pub stamp: DateTime<Utc>",
        "pub stamp: chrono::DateTime<chrono::Utc>",
        "#[serde(skip_serializing_if = \"Option::is_none\")] pub stamp: Option<DateTime<Utc>>",
        "#[serde(with = \"local_millis\")] pub stamp: DateTime<Local>",
        "#[serde(serialize_with = \"shown\", deserialize_with = \"read\")] pub stamp: \
         Vec<DateTime<Utc>>",
    ] {
        let expanded = expansion_over(&format!(
            "#[derive(Serialize, Deserialize)] pub struct Stamped {{ \
             #[model_schema_prop(as_number)] {field} }}"
        ));
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for `{field}`, got: {expanded}"
        );
    }
}

/// A bound on a date under `as_number` is refused in the words of the epoch milliseconds it is
/// written as, and one on any other date in the words of the text it is written as.
#[cfg(all(feature = "serde", feature = "chrono"))]
#[test]
fn a_bound_on_a_date_under_as_number_is_refused_as_the_milliseconds_it_is_written_as() {
    let refused = refusal_texts(&expansion_over(
        "pub struct Stamped { #[model_schema_prop(as_number, minimum = 0)] pub stamp: \
         DateTime<Utc>, #[model_schema_prop(maximum = 2)] pub plain: DateTime<Utc> }",
    ));
    assert_eq!(
        refused,
        [
            "model_schema: field `stamp`: `minimum` cannot apply to `chrono::DateTime` under \
             `as_number` \u{2014} no surface reads a bound beside a date, the epoch milliseconds \
             `as_number` writes it as among them: the constraint would reach neither Zod, nor the \
             JSON schema, nor the generated validator. Drop it, or carry the milliseconds in an \
             `i64` field, which `minimum` and `maximum` bound.",
            "model_schema: field `plain`: `maximum` cannot apply to `chrono::DateTime` \u{2014} \
             this crate writes that type's schema whole, not as the plain string or number a \
             bound is spelled against, and no surface reads a bound beside it: the constraint \
             would reach neither Zod, nor the JSON schema, nor the generated validator. Drop it, \
             or carry the value in a `String` field the bound can measure.",
        ]
    );
    let answered = format!("error: {}", refused[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
}

#[test]
fn nullable_ok_on_option_field() {
    validate_nullable_flag(true, true).unwrap();
}

#[test]
fn nullable_ok_when_flag_unset() {
    validate_nullable_flag(true, false).unwrap();
    validate_nullable_flag(false, false).unwrap();
}

#[test]
fn nullable_err_on_non_option_field() {
    let err = validate_nullable_flag(false, true).unwrap_err();
    assert!(err.contains("nullable"));
    assert!(err.contains("Option<T>"));
}

#[test]
fn nullable_ts_optional_conflict_err_when_both_set() {
    let flags = ModelSchemaPropMeta {
        nullable: true,
        ts_optional: true,
        ..ModelSchemaPropMeta::default()
    };
    let err = check_nullable_ts_optional_conflict(&flags).unwrap_err();
    assert!(err.contains("nullable"));
    assert!(err.contains("ts_optional"));
}

#[test]
fn nullable_ts_optional_conflict_ok_when_only_one_set() {
    check_nullable_ts_optional_conflict(&ModelSchemaPropMeta {
        nullable: true,
        ..ModelSchemaPropMeta::default()
    })
    .unwrap();
    check_nullable_ts_optional_conflict(&ModelSchemaPropMeta {
        ts_optional: true,
        ..ModelSchemaPropMeta::default()
    })
    .unwrap();
    check_nullable_ts_optional_conflict(&ModelSchemaPropMeta::default()).unwrap();
}

#[rustversion::since(1.98)]
fn mapped_or_default<T, U>(value: Option<T>, map: impl FnOnce(T) -> U) -> U
where
    U: Default,
{
    value.map_or_default(map)
}

#[rustversion::before(1.98)]
fn mapped_or_default<T, U>(value: Option<T>, map: impl FnOnce(T) -> U) -> U
where
    U: Default,
{
    value.map(map).unwrap_or_default()
}

/// Runs the guard over the sole field of `item`, deriving `is_optional` and `is_nullable` the way
/// the generator does rather than re-sniffing the type.
#[cfg(feature = "serde")]
fn guard_result(item: &syn::ItemStruct) -> Result<(), syn::Error> {
    let field = item.fields.iter().next().unwrap();
    let field_name = mapped_or_default(field.ident.as_ref(), ToString::to_string);
    let field_def = get_field_def(&field_name, &field.ty, "");
    let is_nullable = super::parse_model_schema_prop_attributes(&field.attrs).nullable;
    check_optional_field_serialization(field, field_def.stands_for_an_option(), is_nullable)
}

/// Runs the `nullable`-key guard over the sole field of `item`, reading the flag off its
/// `model_schema_prop` attribute the way the generator does.
#[cfg(feature = "serde")]
fn nullable_guard_result(item: &syn::ItemStruct) -> Result<(), syn::Error> {
    let field = item.fields.iter().next().unwrap();
    let is_nullable = super::parse_model_schema_prop_attributes(&field.attrs).nullable;
    check_nullable_field_serialization(field, is_nullable)
}

/// Runs the omitted-key readability guard over the sole field of `item`, with the container's own
/// `default` read off the item the way the generator reads it.
#[cfg(feature = "serde")]
fn omitted_key_guard_result(item: &syn::ItemStruct) -> Result<(), syn::Error> {
    let field = item.fields.iter().next().unwrap();
    let field_name = mapped_or_default(field.ident.as_ref(), ToString::to_string);
    let field_def = get_field_def(&field_name, &field.ty, "");
    check_omitted_key_is_readable(
        field,
        field_def.stands_for_an_option(),
        has_serde_default(&item.attrs),
    )
}

/// A field of an alias of an `Option` declared above is held to both key guards as the same field
/// written `Option<T>` is: refused with no key-dropping attribute, read back under
/// `skip_serializing_if` alone, and taken under `nullable`. An alias declared below is no `Option`
/// the guards know of, and keeps the refusal of a key serde requires.
#[cfg(feature = "serde")]
#[test]
fn a_field_of_an_alias_of_an_option_declared_above_is_held_to_the_key_guards_as_an_option() {
    let kept: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip_serializing_if = "Option::is_none")]
            note: ProbeNoteMaybe,
        }
    };
    assert!(
        omitted_key_guard_result(&kept).is_err(),
        "an alias declared below is read as a key serde requires"
    );
    drop(expansion_over("pub type ProbeNoteMaybe = Option<String>;"));
    drop(super::exec_model_schema(
        syn::parse_str("default_types(T = String)").unwrap(),
        syn::parse_str("pub type ProbeHeldMaybe<T> = Option<T>;").unwrap(),
    ));
    omitted_key_guard_result(&kept).unwrap();
    let held: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip_serializing_if = "Option::is_none")]
            note: ProbeHeldMaybe<String>,
        }
    };
    omitted_key_guard_result(&held).unwrap();
    let bare: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            note: ProbeNoteMaybe,
        }
    };
    let message = guard_result(&bare).unwrap_err().to_string();
    assert!(message.contains("T | undefined"), "got: {message}");
    assert!(
        include_str!("../../README.md").contains(&format!("error: {message}")),
        "the README no longer quotes: {message}"
    );
    let nullable: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[model_schema_prop(nullable)]
            note: ProbeNoteMaybe,
        }
    };
    guard_result(&nullable).unwrap();
}

/// `nullable` and `ts_optional` are taken on a field of an alias of an `Option` declared above, as
/// on the same field written `Option<T>`.
#[test]
fn the_option_flags_are_taken_on_a_field_of_an_alias_of_an_option() {
    drop(expansion_over("pub type ProbeFlaggedMaybe = Option<u32>;"));
    let field: syn::Field = syn::parse_quote! { note: ProbeFlaggedMaybe };
    let field_def = get_field_def("note", &field.ty, "");
    validate_nullable_flag(field_def.stands_for_an_option(), true).unwrap();
    validate_ts_optional_flag(&field, &field_def, true).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn bare_option_field_is_rejected() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            note: Option<String>,
        }
    };
    let message = guard_result(&item).unwrap_err().to_string();
    assert!(message.contains("note"));
    assert!(message.contains("T | undefined"));
    assert!(message.contains("skip_serializing_if = \"Option::is_none\""));
    assert!(message.contains("nullable"));
    assert!(!message.contains("only accepts the key being absent"));
}

/// A flattened `Option`, and a flattened field of an alias of an `Option` declared above, write no
/// key of their own, so serde writes nothing for `None`: no key guard refuses one written with no
/// attribute that drops a key, in a struct or in a variant's members.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_option_with_no_key_dropping_attribute_passes_the_key_guards() {
    drop(expansion_over(
        "#[derive(Serialize, Deserialize)] pub struct ProbeFlatInner { pub a: u32 }",
    ));
    drop(expansion_over(
        "pub type ProbeFlatMaybe = Option<ProbeFlatInner>;",
    ));
    for written in ["Option<ProbeFlatInner>", "ProbeFlatMaybe"] {
        let item: syn::ItemStruct = syn::parse_str(&format!(
            "struct Report {{ #[serde(flatten)] inner: {written} }}"
        ))
        .unwrap();
        guard_result(&item).unwrap();
        omitted_key_guard_result(&item).unwrap();
        for source in [
            format!(
                "#[derive(Serialize, Deserialize)] pub struct ProbeFlatHolder {{ pub id: String, \
                 #[serde(flatten)] pub inner: {written} }}"
            ),
            format!(
                "#[derive(Serialize, Deserialize)] pub enum ProbeFlatVariants {{ Held {{ id: \
                 String, #[serde(flatten)] inner: {written} }} }}"
            ),
        ] {
            let expanded = expansion_over(&source).to_string();
            assert!(!expanded.contains("compile_error"), "got: {expanded}");
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn bare_nullable_option_field_is_accepted() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[model_schema_prop(nullable)]
            note: Option<String>,
        }
    };
    guard_result(&item).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn nullable_field_with_a_key_dropping_attribute_is_rejected() {
    for spelling in [
        "skip_serializing_if = \"Option::is_none\"",
        "skip_serializing",
        "skip",
    ] {
        let item: syn::ItemStruct = syn::parse_str(&format!(
            "struct Report {{ #[model_schema_prop(nullable)] #[serde({spelling})] note: \
             Option<String> }}"
        ))
        .unwrap();
        let message = nullable_guard_result(&item).unwrap_err().to_string();
        assert!(message.contains("note"), "{spelling}: {message}");
        assert!(message.contains("nullable"), "{spelling}: {message}");
        assert!(
            message.contains("key-dropping attribute"),
            "{spelling}: {message}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn nullable_field_with_no_key_dropping_attribute_is_accepted() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[model_schema_prop(nullable)]
            note: Option<String>,
        }
    };
    nullable_guard_result(&item).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn non_nullable_field_with_a_key_dropping_attribute_passes_the_nullable_guard() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip_serializing_if = "Option::is_none")]
            note: Option<String>,
        }
    };
    nullable_guard_result(&item).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn a_non_option_omitted_key_with_no_default_is_rejected() {
    for spelling in [
        "skip_serializing_if = \"Vec::is_empty\"",
        "skip_serializing",
    ] {
        let item: syn::ItemStruct = syn::parse_str(&format!(
            "struct Report {{ #[serde({spelling})] roles: Vec<String> }}"
        ))
        .unwrap();
        let message = omitted_key_guard_result(&item).unwrap_err().to_string();
        assert!(message.contains("roles"), "{spelling}: {message}");
        assert!(message.contains("default"), "{spelling}: {message}");
    }
}

/// Every spelling serde can already read a missing key back from.
#[cfg(feature = "serde")]
#[test]
fn an_omitted_key_serde_can_read_back_is_left_alone() {
    for item_source in [
        // `default` on the field writes the value the missing key does not carry.
        "struct Report { #[serde(default, skip_serializing_if = \"Vec::is_empty\")] roles: Vec<String> }",
        "struct Report { #[serde(default = \"mk\", skip_serializing_if = \"Vec::is_empty\")] roles: Vec<String> }",
        // A bare `skip` stops serde reading the field at all, so it supplies `Default` itself.
        "struct Report { #[serde(skip)] roles: Vec<String> }",
        // serde reads a missing `Option` field back as `None` with no `default` written.
        "struct Report { #[serde(skip_serializing_if = \"Option::is_none\")] note: Option<String> }",
        // A `default` on the container answers for every field under it.
        "#[serde(default)] struct Report { #[serde(skip_serializing_if = \"Vec::is_empty\")] roles: Vec<String> }",
        "struct Report { roles: Vec<String> }",
    ] {
        let item: syn::ItemStruct = syn::parse_str(item_source).unwrap();
        let refusal = omitted_key_guard_result(&item)
            .err()
            .map(|err| err.to_string());
        assert_eq!(refusal, None, "for: {item_source}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_positional_slot_is_not_subject_to_the_omitted_key_guard() {
    let item: syn::ItemStruct = syn::parse_str(
        "struct Report(#[serde(skip_serializing_if = \"Vec::is_empty\")] Vec<String>);",
    )
    .unwrap();
    omitted_key_guard_result(&item).unwrap();
}

/// The single-field `Report` written with `notes` at the given spelling.
#[cfg(feature = "serde")]
fn report_with(spelling: &str) -> syn::ItemStruct {
    syn::parse_str(&format!("struct Report {{ notes: {spelling} }}")).unwrap()
}

#[cfg(feature = "serde")]
#[test]
fn an_option_around_a_covered_wrapper_is_rejected_as_the_vec_spelling_is() {
    for spelling in SEQUENCE_WRAPPERS
        .iter()
        .map(|wrapper| format!("Option<{wrapper}<String>>"))
        .chain(["Option<Vec<String>>".to_owned()])
    {
        let message = guard_result(&report_with(&spelling))
            .unwrap_err()
            .to_string();
        assert!(message.contains("notes"), "{spelling}: {message}");
        assert!(
            message.contains("skip_serializing_if"),
            "{spelling}: {message}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn an_option_inside_a_covered_wrapper_leaves_the_guard_nothing_to_refuse() {
    for spelling in SEQUENCE_WRAPPERS
        .iter()
        .map(|wrapper| format!("{wrapper}<Option<String>>"))
        .chain(
            [
                "Vec<Option<String>>",
                "Vec<Vec<Option<String>>>",
                "HashSet<Vec<Option<String>>>",
                "Vec<HashSet<Option<String>>>",
                "BTreeSet<VecDeque<Option<String>>>",
                "Vec<Option<Vec<String>>>",
            ]
            .map(ToOwned::to_owned),
        )
    {
        let refusal = guard_result(&report_with(&spelling))
            .err()
            .map(|err| err.to_string());
        assert_eq!(refusal, None, "for: {spelling}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_covered_wrapper_of_a_plain_element_satisfies_the_guard() {
    for wrapper in SEQUENCE_WRAPPERS {
        guard_result(&report_with(&format!("{wrapper}<String>"))).unwrap();
    }
}

#[cfg(feature = "serde")]
#[test]
fn skip_serializing_if_satisfies_the_guard() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip_serializing_if = "Option::is_none")]
            note: Option<String>,
        }
    };
    guard_result(&item).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn any_skip_serializing_if_predicate_satisfies_the_guard() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip_serializing_if = "crate::is_boring")]
            note: Option<String>,
        }
    };
    guard_result(&item).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn skip_satisfies_the_guard() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip)]
            note: Option<String>,
        }
    };
    guard_result(&item).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn skip_serializing_satisfies_the_guard() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip_serializing)]
            note: Option<String>,
        }
    };
    guard_result(&item).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn skip_deserializing_alone_does_not_satisfy_the_guard() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip_deserializing)]
            note: Option<String>,
        }
    };
    let message = guard_result(&item).unwrap_err().to_string();
    assert!(message.contains("note"));
}

#[cfg(feature = "serde")]
#[test]
fn non_option_field_is_unaffected() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            name: String,
        }
    };
    guard_result(&item).unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn positional_option_field_is_exempt() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report(Option<String>);
    };
    guard_result(&item).unwrap();
}

/// Collects the untagged-path guard failures as rendered `compile_error!` token streams.
#[cfg(feature = "serde")]
fn untagged_guard_error_tokens(item: &mut syn::ItemEnum) -> Vec<proc_macro2::TokenStream> {
    collect_untagged_members(item, UNTAGGED_MODULE, &super::ModelSchemaArgs::default()).5
}

/// Collects the untagged-path guard failures as rendered `compile_error!` token strings.
#[cfg(feature = "serde")]
fn untagged_guard_errors(mut item: syn::ItemEnum) -> Vec<String> {
    untagged_guard_error_tokens(&mut item)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(feature = "serde")]
#[test]
fn untagged_named_variant_bare_option_is_rejected() {
    let errors = untagged_guard_errors(syn::parse_quote! {
        enum Choice {
            Report { note: Option<String> },
            Plain(i64),
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("note"), "got: {}", errors[0]);
    assert!(
        errors[0].contains("skip_serializing_if"),
        "got: {}",
        errors[0]
    );
}

#[cfg(feature = "serde")]
#[test]
fn untagged_named_variant_omitting_none_is_accepted() {
    let errors = untagged_guard_errors(syn::parse_quote! {
        enum Choice {
            Report {
                #[serde(skip_serializing_if = "Option::is_none")]
                note: Option<String>,
            },
        }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[cfg(feature = "serde")]
#[test]
fn untagged_tuple_variant_option_is_exempt() {
    let errors = untagged_guard_errors(syn::parse_quote! {
        enum Choice {
            Maybe(Option<i64>),
        }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[cfg(feature = "serde")]
#[test]
fn untagged_unsupported_variant_shapes_are_all_reported() {
    let errors = untagged_guard_errors(syn::parse_quote! {
        enum Choice {
            Bare,
            Pair(String, String),
            Note { id: String },
            Empty(),
        }
    });
    assert_eq!(errors.len(), 3, "got: {errors:?}");
    for (error, needles) in errors.iter().zip([
        ["`Bare`", "a unit variant"],
        ["`Pair`", "a tuple variant with 2 fields"],
        ["`Empty`", "a unit variant"],
    ]) {
        assert!(error.contains("compile_error"), "got: {error}");
        for needle in needles {
            assert!(error.contains(needle), "got: {error}");
        }
        assert!(
            error.contains("supports newtype (`V(T)`) and struct"),
            "got: {error}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn untagged_unsupported_variant_refusal_points_at_the_variant() {
    use syn::spanned::Spanned as _;

    let mut item: syn::ItemEnum =
        syn::parse_str("enum Choice { Note { id: String }, Pair(String, String) }").unwrap();
    let errors = untagged_guard_error_tokens(&mut item);
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert_eq!(
        errors[0].span().source_text().as_deref(),
        Some("Pair(String, String)")
    );
}

#[cfg(feature = "serde")]
#[test]
fn untagged_supported_variant_shapes_are_left_alone() {
    let errors = untagged_guard_errors(syn::parse_quote! {
        enum Choice {
            Note { id: String },
            Plain(i64),
        }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn untagged_member_reaching_a_map_key_with_no_members_is_refused() {
    register_alias_info(
        "Ledger",
        "Ledger",
        "ledger_schema",
        AliasKind::NoEnumMembers,
    );
    for member_type in [
        quote::quote! { HashMap<Ledger, u32> },
        quote::quote! { Vec<HashMap<Ledger, u32>> },
        quote::quote! { HashMap<String, HashMap<Ledger, u32>> },
    ] {
        let errors = untagged_guard_errors(syn::parse_quote! {
            enum Untagged {
                Counts { counts: #member_type },
            }
        });
        assert_eq!(errors.len(), 1, "for {member_type}, got: {errors:?}");
        assert!(
            errors[0].contains("compile_error"),
            "for {member_type}: {}",
            errors[0]
        );
        assert!(
            errors[0].contains("field `counts`"),
            "for {member_type}: {}",
            errors[0]
        );
        assert!(
            errors[0].contains("a map key must be a plain"),
            "for {member_type}: {}",
            errors[0]
        );
        assert!(
            errors[0].contains("Ledger"),
            "for {member_type}: {}",
            errors[0]
        );
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn untagged_member_carries_its_constraint_to_the_surfaces() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Choice {
            Named {
                #[model_schema_prop(minLength = 2, pattern = "^[a-z]+$")]
                name: String,
            },
        }
    };
    let (_, _, zod_parts, _, _, errors, _, _, _, _) = collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    assert!(
        zod_parts[0].contains(
            "z.string()\
             .min(2, { error: (issue) => `too short: minimum length is 2, got ${String(issue.input).length}` })\
             .check(z.regex(/^[a-z]+$/, { error: \"does not match pattern '^[a-z]+$'\" }))"
        ),
        "got: {}",
        zod_parts[0]
    );
}

#[cfg(feature = "serde")]
#[test]
fn untagged_member_constraint_generates_the_validator_and_hangs_it_on_the_member() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Choice {
            Named {
                #[model_schema_prop(minLength = 2)]
                name: String,
            },
        }
    };
    let (_, _, _, _, _, errors, validation_fns, _, _, _) = collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    assert_eq!(validation_fns.len(), 1, "got: {validation_fns:?}");
    let rendered = validation_fns[0].to_string();
    assert!(
        rendered.contains("fn validate_named_name_value"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains("fn deserialize_named_name"),
        "got: {rendered}"
    );

    let attrs = &item.variants[0].fields.iter().next().unwrap().attrs;
    let rendered_attrs = quote::quote!(#(#attrs)*).to_string();
    assert!(
        rendered_attrs.contains(r#"deserialize_with = "choice_schema::deserialize_named_name""#),
        "got: {rendered_attrs}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn untagged_member_constraint_generates_nothing_without_a_schema_module() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Choice {
            Named {
                #[model_schema_prop(minLength = 2)]
                name: String,
            },
        }
    };
    let (_, _, _, _, _, errors, validation_fns, _, _, _) =
        collect_untagged_members(&mut item, None, &super::ModelSchemaArgs::default());
    assert!(errors.is_empty(), "got: {errors:?}");
    assert!(validation_fns.is_empty(), "got: {validation_fns:?}");
    let attrs = &item.variants[0].fields.iter().next().unwrap().attrs;
    assert!(attrs.is_empty(), "got: {}", quote::quote!(#(#attrs)*));
}

#[test]
fn a_tuple_structs_slot_takes_a_check() {
    let expanded =
        expansion_over("pub struct Pair(#[model_schema_prop(minLength = 1)] pub String, pub u32);")
            .to_string();
    assert!(!expanded.contains("compile_error"), "got: {expanded}");
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_tuple_structs_slot_is_named_by_its_position_and_read_off_self() {
    let expanded = expansion_over(
        "pub struct Pair(#[model_schema_prop(minLength = 1)] pub String, \
         #[model_schema_prop(minimum = 1)] pub u32);",
    )
    .to_string();
    for written in [
        "pub fn validate_0_value",
        "pub fn deserialize_0",
        "pub fn validate_1_value",
        "pub fn deserialize_1",
        "validate_0_value (& self . 0)",
        "validate_1_value (& self . 1)",
        r#"format ! ("'{}': {}, got {}" , "0""#,
        r#"format ! ("'{}': {}, got {}" , "1""#,
    ] {
        assert!(expanded.contains(written), "{written} missing: {expanded}");
    }
    assert!(
        !expanded.contains("deserialize_with"),
        "a slot was given a serde hook: {expanded}"
    );

    let unchecked = expansion_over("pub struct Pair(pub String, pub u32);").to_string();
    assert!(!unchecked.contains("fn validate"), "got: {unchecked}");
}

#[cfg(feature = "serde")]
#[test]
fn untagged_newtype_member_constraint_is_named_by_its_position_and_hung_on_the_slot() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum MimeType {
            Known(KnownMimeType),
            Other(
                #[model_schema_prop(
                    minLength = 1,
                    maxLength = 127,
                    pattern = "^[a-z]+/[a-z0-9.+-]+$"
                )]
                String,
            ),
        }
    };
    let (_, _, _, _, _, errors, validation_fns, arms, _, _) = collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    assert_eq!(validation_fns.len(), 1, "got: {validation_fns:?}");
    let rendered = validation_fns[0].to_string();
    assert!(
        rendered.contains("fn validate_other_0_value"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains("fn deserialize_other_0"),
        "got: {rendered}"
    );

    let rendered_attrs = walked_field_attrs(item.variants[1].fields.iter());
    assert!(
        rendered_attrs.contains(r#"deserialize_with = "choice_schema::deserialize_other_0""#),
        "got: {rendered_attrs}"
    );

    let rendered_arms = quote::quote!(#(#arms),*).to_string();
    assert!(
        rendered_arms.contains("Self :: Other (member_slot_0) =>"),
        "got: {rendered_arms}"
    );
    assert!(
        rendered_arms.contains("validate_other_0_value (member_slot_0)"),
        "got: {rendered_arms}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn untagged_member_prop_attribute_is_stripped_from_the_emitted_item() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Choice {
            Named {
                #[model_schema_prop(minLength = 2)]
                #[serde(rename = "label")]
                name: String,
            },
        }
    };
    collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    let attrs = &item.variants[0].fields.iter().next().unwrap().attrs;
    assert!(
        !attrs
            .iter()
            .any(|attr| attr.path().is_ident("model_schema_prop")),
        "got: {}",
        quote::quote!(#(#attrs)*)
    );
    assert!(
        attrs.iter().any(|attr| attr.path().is_ident("serde")),
        "the serde attribute must survive the strip"
    );
}

#[cfg(feature = "serde")]
#[test]
fn untagged_member_prop_guards_apply() {
    for (member, needle) in [
        (
            quote::quote! { #[model_schema_prop(patern = "^[a-z]+$")] name: String },
            "patern",
        ),
        (
            quote::quote! { #[model_schema_prop(ts_optional)] name: String },
            "requires an Option<T> field",
        ),
        (
            quote::quote! { #[model_schema_prop(as = String)] name: i64 },
            "as = String",
        ),
    ] {
        let errors = untagged_guard_errors(syn::parse_quote! {
            enum Choice {
                Named { #member },
            }
        });
        assert_eq!(errors.len(), 1, "for {member}: {errors:?}");
        assert!(
            errors[0].contains(needle),
            "{needle} missing for {member}: {}",
            errors[0]
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn untagged_member_with_an_enumerable_map_key_is_left_alone() {
    register_alias_info("Bucket", "Bucket", "bucket_schema", AliasKind::EnumMembers);
    for member_type in [
        quote::quote! { HashMap<Bucket, u32> },
        quote::quote! { HashMap<String, u32> },
    ] {
        let errors = untagged_guard_errors(syn::parse_quote! {
            enum Untagged {
                Counts { counts: #member_type },
            }
        });
        assert!(errors.is_empty(), "for {member_type}, got: {errors:?}");
    }
}

/// The refusal every map-key spelling no surface can write earns, read off the untagged walk.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn untagged_member_reaching_an_unwritable_map_key_is_refused() {
    for (member_type, needle) in [
        (quote::quote! { HashMap<Vec<String>, u32> }, "String"),
        (quote::quote! { HashMap<[String; 2], u32> }, "String"),
        (quote::quote! { HashMap<Option<String>, u32> }, "String"),
        (
            quote::quote! { HashMap<(String, u32), u32> },
            "`(_, _)` as a JSON array",
        ),
        (
            quote::quote! { HashMap<HashMap<String, u32>, u32> },
            "`HashMap<_, _>` as a JSON object",
        ),
        (
            quote::quote! { Vec<HashMap<Option<String>, u32>> },
            "String",
        ),
    ] {
        let errors = untagged_guard_errors(syn::parse_quote! {
            enum Untagged {
                Counts { counts: #member_type },
            }
        });
        assert_eq!(errors.len(), 1, "for {member_type}, got: {errors:?}");
        assert!(
            errors[0].contains("compile_error"),
            "for {member_type}: {}",
            errors[0]
        );
        assert!(
            errors[0].contains("field `counts`"),
            "for {member_type}: {}",
            errors[0]
        );
        assert!(
            errors[0].contains(needle),
            "for {member_type}: {}",
            errors[0]
        );
    }
}

/// The JSON-schema values the untagged walk renders its members as.
#[cfg(all(feature = "serde", feature = "jsonschema"))]
fn untagged_member_values(mut item: syn::ItemEnum) -> Vec<String> {
    collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    )
    .4
    .iter()
    .map(ToString::to_string)
    .collect()
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn untagged_member_holding_a_map_renders_the_field_position_map() {
    register_alias_info("Bucket", "Bucket", "bucket_schema", AliasKind::EnumMembers);
    for map_type in [
        "HashMap<Bucket, u32>",
        "HashMap<String, u32>",
        "Vec<HashMap<String, u32>>",
        "HashMap<String, HashMap<Bucket, u32>>",
    ] {
        let member_type: syn::Type = syn::parse_str(map_type).unwrap();
        let values = untagged_member_values(syn::parse_quote! {
            enum Untagged {
                Counts { m: #member_type },
            }
        });
        assert!(
            values[0].contains(&inserted_field_value(map_type)),
            "for {map_type}, got: {}",
            values[0]
        );
    }
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn untagged_member_holding_a_tuple_renders_the_arity_bounds() {
    let values = untagged_member_values(syn::parse_quote! {
        enum Untagged {
            Pair { pair: (i64, String) },
        }
    });
    assert!(
        values[0].contains(r#""minItems" : 2usize , "maxItems" : 2usize"#),
        "got: {}",
        values[0]
    );
    assert!(
        values[0].contains(r#""items" : false"#),
        "got: {}",
        values[0]
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn untagged_member_holding_a_map_of_tuples_describes_each_value_as_its_array() {
    let values = untagged_member_values(syn::parse_quote! {
        enum Untagged {
            Rows { rows: HashMap<String, (String, u32)> },
        }
    });
    assert!(!values[0].contains("compile_error !"), "got: {}", values[0]);
    assert!(
        values[0].contains(r#""additionalProperties" : serde_json :: json ! ({ "type" : "array""#),
        "got: {}",
        values[0]
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn untagged_member_holding_an_unwritable_map_key_emits_only_the_compile_error() {
    let values = untagged_member_values(syn::parse_quote! {
        enum Untagged {
            Rows { rows: HashMap<String, HashMap<(String, u32), u8>> },
        }
    });
    assert_eq!(
        values[0].matches("compile_error !").count(),
        1,
        "got: {}",
        values[0]
    );
    assert!(
        values[0].contains(&format!("model_schema: field `rows`: {TUPLE_KEY_REFUSAL}")),
        "got: {}",
        values[0]
    );
    assert!(
        !values[0].contains("additionalProperties\" :"),
        "got: {}",
        values[0]
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn an_external_variant_holding_an_unrenderable_content_is_refused() {
    let rendered = super::external_content_rejection_value(
        "rows",
        &super::MapMemberRejection::Key(
            super::MapKeyRejection::Unwritable {
                key_name: "(_, _)".to_owned(),
                written_as: super::WRITTEN_AS_ARRAY,
            },
            proc_macro2::Span::call_site(),
        ),
    )
    .to_string();
    assert!(
        rendered.starts_with(":: core :: compile_error !"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains(&format!(
            "model_schema: variant `rows`: {TUPLE_KEY_REFUSAL}"
        )),
        "got: {rendered}"
    );
}

/// The field defs one variant of `source` declares, parsed from text so each carries the span of
/// the tokens it was written with.
#[cfg(feature = "jsonschema")]
fn variant_field_defs(source: &str, variant_name: &str) -> Vec<super::FieldDef> {
    let item: syn::ItemEnum = syn::parse_str(source).unwrap();
    item.variants
        .iter()
        .filter(|variant| variant.ident == variant_name)
        .flat_map(|variant| variant.fields.iter())
        .map(|field| {
            let name = mapped_or_default(field.ident.as_ref(), ToString::to_string);
            get_field_def(&name, &field.ty, "")
        })
        .collect()
}

/// The content sink's caret, at both contents an externally tagged variant can carry.
#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn a_refused_external_content_points_at_the_written_content() {
    const SOURCE: &str = "enum Tagged { \
        Rows(HashMap<(u32, u32), u32>), \
        Multi(String, HashMap<(u32, u32), u32>) }";

    let single = variant_field_defs(SOURCE, "Rows");
    let single_rejection = super::build_tuple_element_json_schema(&single[0]).unwrap_err();
    assert_points_only_at(
        &super::external_content_rejection_value("Rows", &single_rejection),
        "(u32, u32)",
        "a single-content variant",
    );

    let multi = variant_field_defs(SOURCE, "Multi");
    let multi_rejection = super::tuple_json_schema_value(&multi).unwrap_err();
    assert_points_only_at(
        &super::external_content_rejection_value("Multi", &multi_rejection),
        "(u32, u32)",
        "a multi-content variant",
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_adjacent_content_points_at_the_written_content() {
    const SOURCE: &str = "enum Tagged { \
        Rows(HashMap<(u32, u32), u32>), \
        Multi(String, HashMap<(u32, u32), u32>) }";

    let mut single_fields = Vec::new();
    super::push_single_tuple_json_field(
        &mut single_fields,
        "value",
        &variant_field_defs(SOURCE, "Rows")[0],
    );
    assert_points_only_at(
        &single_fields[0],
        "(u32, u32)",
        "an adjacent single content",
    );

    let mut parts = super::VariantParts {
        json_fields: Vec::new(),
        schema_code: String::new(),
        type_code: String::new(),
    };
    super::write_tuple_multiple_variant_fields(
        &variant_field_defs(SOURCE, "Multi"),
        "value",
        "Tagged",
        &mut parts,
    );
    assert_points_only_at(
        &parts.json_fields[0],
        "(u32, u32)",
        "an adjacent multi content",
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn a_refused_untagged_member_points_at_the_written_value() {
    for (member_type, context) in [
        ("HashMap<(u32, u32), u32>", "an untagged map member"),
        (
            "(String, HashMap<(u32, u32), u32>)",
            "an untagged tuple member",
        ),
    ] {
        let source = format!("enum Untagged {{ Rows {{ rows: {member_type} }} }}");
        let member = &variant_field_defs(&source, "Rows")[0];
        assert_points_only_at(
            &super::field_json_schema_value(member),
            "(u32, u32)",
            context,
        );
    }
}

/// Collects the internally tagged path's guard failures as rendered `compile_error!` token strings.
#[cfg(feature = "serde")]
fn internal_guard_errors(item: &syn::ItemEnum) -> Vec<String> {
    internally_tagged_guard_errors(item, "type")
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(feature = "serde")]
#[test]
fn internally_tagged_serializable_variants_are_accepted() {
    let errors = internal_guard_errors(&syn::parse_quote! {
        enum TagOnly {
            Bare,
            Fields { a: String },
            Wrapped(Payload),
            Boxed(Box<Payload>),
        }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

/// Each slot is refused once, as the value `value` names, quoting word for word the error serde
/// answers for a write of it.
#[cfg(feature = "serde")]
fn assert_quotes_serde(rows: &[(&str, Vec<String>, String)], value: &str) {
    let named = format!("variant `Held` of internally tagged enum `Lettered` wraps {value},");
    for (slot, errors, said) in rows {
        assert_eq!(errors.len(), 1, "got: {errors:?} for `{slot}`");
        assert!(
            errors[0].contains(&named),
            "the refusal of `{slot}` does not name it {value}: {}",
            errors[0]
        );
        assert!(
            errors[0].contains("serde refuses this one at run time")
                && errors[0].contains(&format!("`{said}`")),
            "the refusal of `{slot}` does not quote serde's `{said}`: {}",
            errors[0]
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_tagged_char_is_refused_in_the_words_serde_refuses_it() {
    assert_quotes_serde(&[refused_beside_serde!(char, 'c')], "a char");
}

#[cfg(feature = "serde")]
#[test]
fn a_tagged_string_is_refused_in_the_words_serde_refuses_it() {
    use std::path::PathBuf;
    assert_quotes_serde(
        &[
            refused_beside_serde!(String, "s".to_owned()),
            refused_beside_serde!(&'static str, "s"),
            refused_beside_serde!(Box<str>, "s".into()),
            refused_beside_serde!(PathBuf, PathBuf::from("p")),
        ],
        "a string",
    );
}

#[cfg(all(feature = "serde", feature = "chrono"))]
#[test]
fn a_tagged_date_is_refused_in_the_words_serde_refuses_it() {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
    let day = NaiveDate::from_ymd_opt(2020, 1, 2).unwrap();
    let time = NaiveTime::from_hms_opt(1, 2, 3).unwrap();
    assert_quotes_serde(
        &[
            refused_beside_serde!(NaiveDate, day),
            refused_beside_serde!(NaiveTime, time),
            refused_beside_serde!(NaiveDateTime, day.and_time(time)),
            refused_beside_serde!(DateTime<Utc>, DateTime::from_timestamp(0, 0).unwrap()),
        ],
        "a string",
    );
}

/// A date under `as_number` is read and written through the hook the key hangs on it, which writes
/// the integer of its epoch milliseconds: beside a tag and flattened, the expansion refuses it in
/// the words serde refuses that integer in.
#[cfg(all(feature = "serde", feature = "chrono"))]
#[test]
fn a_date_under_as_number_is_refused_as_the_integer_serde_writes() {
    use chrono::{DateTime, Utc};
    #[derive(serde::Serialize)]
    #[serde(tag = "kind")]
    enum Lettered {
        Held(#[serde(with = "chrono::serde::ts_milliseconds")] DateTime<Utc>),
    }
    #[derive(serde::Serialize)]
    struct Host {
        id: u8,
        #[serde(flatten, with = "chrono::serde::ts_milliseconds")]
        stamp: DateTime<Utc>,
    }
    let at = DateTime::from_timestamp_millis(1_704_164_645_678).unwrap();
    let tagged = internal_guard_errors(&syn::parse_quote! {
        enum Lettered { Held(#[model_schema_prop(as_number)] DateTime<Utc>) }
    });
    let tagged_said = serde_json::to_string(&Lettered::Held(at))
        .unwrap_err()
        .to_string();
    assert_quotes_serde(&[("DateTime<Utc>", tagged, tagged_said)], "an integer");

    let flattened = super::flatten_edge_guard_error(
        &syn::parse_quote! {
            #[serde(flatten)] #[model_schema_prop(as_number)] stamp: DateTime<Utc>
        },
        "Host",
    )
    .map_or_else(String::new, |error| error.to_string());
    let flattened_said = serde_json::to_string(&Host { id: 1, stamp: at })
        .unwrap_err()
        .to_string();
    assert_eq!(
        flattened_said,
        "can only flatten structs and maps (got an integer)"
    );
    assert!(
        flattened.contains(
            "`#[serde(flatten)]` of `DateTime<Utc>` is not written as an object \u{2014} its \
             schema describes a `integer`"
        ),
        "got: {flattened}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_tagged_boolean_is_refused_in_the_words_serde_refuses_it() {
    assert_quotes_serde(&[refused_beside_serde!(bool, true)], "a boolean");
}

#[cfg(feature = "serde")]
#[test]
fn a_tagged_integer_of_every_width_is_refused_in_the_words_serde_refuses_it() {
    assert_quotes_serde(
        &[
            refused_beside_serde!(i8, 1),
            refused_beside_serde!(i16, 1),
            refused_beside_serde!(i32, 1),
            refused_beside_serde!(i64, 1),
            refused_beside_serde!(isize, 1),
            refused_beside_serde!(u8, 1),
            refused_beside_serde!(u16, 1),
            refused_beside_serde!(u32, 1),
            refused_beside_serde!(u64, 1),
            refused_beside_serde!(usize, 1),
            refused_beside_serde!(Box<u32>, Box::new(1)),
        ],
        "an integer",
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_tagged_float_of_every_width_is_refused_in_the_words_serde_refuses_it() {
    assert_quotes_serde(
        &[
            refused_beside_serde!(f32, 1.5),
            refused_beside_serde!(f64, 1.5),
        ],
        "a float",
    );
}

/// serde's tag refuses an `Option` before it reads what the `Option` holds.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_option_is_refused_in_the_words_serde_refuses_it() {
    assert_quotes_serde(
        &[
            refused_beside_serde!(Option<u32>, Some(1)),
            refused_beside_serde!(Option<String>, None),
            refused_beside_serde!(Option<Vec<u32>>, Some(vec![1])),
            refused_beside_serde!(Option<[u32; 2]>, Some([1, 2])),
        ],
        "an optional",
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_tagged_sequence_is_refused_in_the_words_serde_refuses_it() {
    use std::collections::HashSet;
    assert_quotes_serde(
        &[
            refused_beside_serde!(Vec<u32>, vec![1]),
            refused_beside_serde!(Vec<[u32; 2]>, vec![[1, 2]]),
            refused_beside_serde!(HashSet<u32>, HashSet::from([1])),
            refused_beside_serde!(&'static [u32], &[1]),
            refused_beside_serde!(Box<[u32]>, vec![1].into_boxed_slice()),
        ],
        "a sequence",
    );
}

#[cfg(all(feature = "serde", feature = "indexmap"))]
#[test]
fn a_tagged_index_set_is_refused_in_the_words_serde_refuses_it() {
    use indexmap::IndexSet;
    assert_quotes_serde(
        &[refused_beside_serde!(IndexSet<u32>, IndexSet::from([1]))],
        "a sequence",
    );
}

/// serde writes a fixed-size array as a tuple, whatever its length is written as.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_tuple_and_a_fixed_size_array_are_refused_in_the_words_serde_refuses_them() {
    const PAIR: usize = 2;
    assert_quotes_serde(
        &[
            refused_beside_serde!((u32,), (1,)),
            refused_beside_serde!((u32, String), (1, "s".to_owned())),
        ],
        "a tuple",
    );
    assert_quotes_serde(
        &[
            refused_beside_serde!([u32; 2], [1, 2]),
            refused_beside_serde!([u32; 0], []),
            refused_beside_serde!([u32; PAIR], [1, 2]),
            refused_beside_serde!([Vec<u32>; 1], [vec![1]]),
            refused_beside_serde!(Box<[u32; 2]>, Box::new([1, 2])),
        ],
        "a fixed-size array",
    );
}

/// serde writes a range and a `Result` beside the tag, in the object the tag is written in, and
/// reads them back: the refusal names what serde writes there.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_range_and_result_are_refused_as_what_serde_writes_beside_the_tag() {
    use core::ops::Range;

    #[derive(serde::Serialize)]
    #[serde(tag = "kind")]
    enum Ranged {
        Held(Range<u32>),
    }

    #[derive(serde::Serialize)]
    #[serde(tag = "kind")]
    enum Resulted {
        Held(Result<u32, String>),
    }

    for (source, value, written, said, serde_writes) in [
        (
            "enum Ranged { Held(Range<u32>) }",
            "a range",
            "its bounds",
            serde_json::to_string(&Ranged::Held(1..3)).unwrap(),
            r#"{"kind":"Held","start":1,"end":3}"#,
        ),
        (
            "enum Resulted { Held(Result<u32, String>) }",
            "a result",
            "the key of its arm",
            serde_json::to_string(&Resulted::Held(Ok(1))).unwrap(),
            r#"{"kind":"Held","Ok":1}"#,
        ),
    ] {
        assert_eq!(
            said, serde_writes,
            "serde no longer writes {source} beside the tag"
        );
        assert_written_beside(source, value, written);
    }
}

/// serde writes an `ObjectId` beside the tag as its `$oid` key, and reads it back, as it writes a
/// flattened one: the slot builds, owned or in a `Box`, and an `Option` of it is refused as any
/// `Option` there is.
#[cfg(all(feature = "serde", feature = "mongodb"))]
#[test]
fn a_tagged_object_id_builds_as_a_flattened_one_does() {
    use bson2::oid::ObjectId;

    #[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
    #[serde(tag = "kind")]
    enum Stamped {
        Held(ObjectId),
    }

    let held = Stamped::Held(ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap());
    let written = serde_json::to_string(&held).unwrap();
    assert_eq!(
        written, r#"{"kind":"Held","$oid":"507f1f77bcf86cd799439011"}"#,
        "serde no longer writes an ObjectId beside the tag"
    );
    assert_eq!(
        serde_json::from_str::<Stamped>(&written).unwrap(),
        held,
        "serde no longer reads back the ObjectId it writes beside the tag"
    );
    for source in [
        "enum Stamped { Held(ObjectId) }",
        "enum Stamped { Held(Box<ObjectId>) }",
    ] {
        let errors = internal_guard_errors(&syn::parse_str(source).unwrap());
        assert!(errors.is_empty(), "{source} is refused: {errors:?}");
    }
    let errors = internal_guard_errors(&syn::parse_quote! {
        enum Stamped { Held(Option<ObjectId>) }
    });
    assert!(
        errors.len() == 1 && errors[0].contains("containing an optional"),
        "an Option of an ObjectId beside the tag is not refused as an optional: {errors:?}"
    );
}

/// The one refusal `source` earns names `value` and what serde writes beside the tag for it.
#[cfg(feature = "serde")]
fn assert_written_beside(source: &str, value: &str, written: &str) {
    let errors = internal_guard_errors(&syn::parse_str(source).unwrap());
    assert_eq!(errors.len(), 1, "got: {errors:?} for {source}");
    assert!(
        errors[0].contains(&format!("wraps {value}:"))
            && errors[0].contains(&format!("writes {written} beside the tag"))
            && !errors[0].contains("cannot name"),
        "the refusal of {source} does not say what serde writes: {}",
        errors[0]
    );
}

#[cfg(feature = "serde")]
#[test]
fn internally_tagged_newtype_names_the_outermost_wrapper() {
    let errors = internal_guard_errors(&syn::parse_quote! {
        enum E { V(Option<Vec<Payload>>) }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(
        errors[0].contains("containing an optional"),
        "got: {}",
        errors[0]
    );
}

#[cfg(feature = "serde")]
#[test]
fn internally_tagged_newtype_over_a_map_is_rejected_as_unnameable() {
    let errors = internal_guard_errors(&syn::parse_quote! {
        enum E { V(std::collections::HashMap<String, u32>) }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("wraps a map"), "got: {}", errors[0]);
    assert!(
        !errors[0].contains("serde refuses"),
        "serde writes a map beside the tag. Got: {}",
        errors[0]
    );
}

#[cfg(feature = "serde")]
#[test]
fn internally_tagged_tuple_variant_is_rejected() {
    let errors = internal_guard_errors(&syn::parse_quote! {
        enum E { V(String, u32) }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(
        errors[0].contains("cannot be used with tuple variants"),
        "got: {}",
        errors[0]
    );
}

/// Under a tag alone serde's derive refuses a tuple variant of no slot, skipped or not, and the
/// refusal raised ahead of every shape says so: the internally tagged guard raises none of its
/// own for it, so the declaration earns one refusal.
#[cfg(feature = "serde")]
#[test]
fn internally_tagged_empty_tuple_variant_is_refused_in_serdes_words() {
    let refusal = "model_schema: variant `Empty`: `Empty()` is a tuple variant with no field, and \
                   serde's derive refuses a tuple variant of no slot or of several under \
                   `#[serde(tag = \"kind\")]` with no `content`, under `#[serde(skip)]` or not: \
                   `#[serde(tag = \"...\")] cannot be used with tuple variants`. Write `Empty` \
                   for a unit variant.";
    for (source, skipped) in [
        (
            "#[serde(tag = \"kind\")] pub enum ProbeTaggedEmpty { Open { note: String }, Empty() }",
            "",
        ),
        (
            "#[serde(tag = \"kind\")] pub enum ProbeTaggedEmpty { Open { note: String }, \
             #[serde(skip)] Empty() }",
            "skipped ",
        ),
    ] {
        assert_refused_without_member_keys(
            "",
            source,
            &proc_macro2::Literal::string(refusal).to_string(),
        );
        let item: syn::ItemEnum = syn::parse_str(source).unwrap();
        assert!(
            internal_guard_errors(&item).is_empty(),
            "the {skipped}variant earns no second refusal"
        );
    }
    assert!(
        include_str!("../../README.md").contains(refusal),
        "the README no longer quotes this refusal:\n{refusal}"
    );
}

/// A name is not the criterion — what serde writes for it is.
#[cfg(feature = "serde")]
#[test]
fn internally_tagged_newtype_over_a_registered_plain_enum_is_rejected() {
    record_plain_enum("Hue", true);
    let errors = internal_guard_errors(&syn::parse_quote! {
        enum E { V(Hue) }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("variant `V`"), "got: {}", errors[0]);
    assert!(errors[0].contains("`Hue`"), "got: {}", errors[0]);
    assert!(
        errors[0].contains("does not write as an object"),
        "got: {}",
        errors[0]
    );
    assert!(
        errors[0].contains("Name a `content` key"),
        "got: {}",
        errors[0]
    );
}

#[cfg(feature = "serde")]
#[test]
fn internally_tagged_newtype_over_a_non_enum_or_unknown_name_is_accepted() {
    record_plain_enum("Payload", false);
    for source in ["enum E { V(Payload) }", "enum E { V(NeverRegistered) }"] {
        let errors = internal_guard_errors(&syn::parse_str(source).unwrap());
        assert!(errors.is_empty(), "got: {errors:?} for {source}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn flattening_a_registered_plain_enum_is_rejected() {
    record_plain_enum("Hue", true);
    record_plain_enum("Payload", false);

    let rejected: syn::Field = syn::parse_quote! { pub tone: Hue };
    let error = mapped_or_default(
        super::flattened_field_guard_error(&rejected, "Holder"),
        |tokens| tokens.to_string(),
    );
    assert!(error.contains("field `tone`"), "got: {error}");
    assert!(error.contains("`Holder`"), "got: {error}");
    assert!(error.contains("`Hue`"), "got: {error}");
    assert!(error.contains("#[serde(flatten)]"), "got: {error}");
    assert!(
        error.contains("does not write as an object"),
        "got: {error}"
    );

    for accepted in [
        syn::parse_quote! { pub body: Payload },
        syn::parse_quote! { pub body: NeverRegistered },
        syn::parse_quote! { pub tones: Vec<Hue> },
    ] {
        let field: syn::Field = accepted;
        assert!(
            super::flattened_field_guard_error(&field, "Holder").is_none(),
            "got a rejection for {}",
            quote::ToTokens::to_token_stream(&field)
        );
    }
}

/// Collects an enum's `cfg_attr` guard failures as rendered `compile_error!` token strings.
#[cfg(feature = "serde")]
fn enum_cfg_attr_errors(item: &syn::ItemEnum) -> Vec<String> {
    let type_meta = parse_serde_type_attributes(&item.attrs);
    enum_cfg_attr_guard_errors(item, &type_meta)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(feature = "serde")]
#[test]
fn cfg_attr_wrapped_serde_on_a_type_is_rejected() {
    let errors = enum_cfg_attr_errors(&syn::parse_quote! {
        #[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
        enum Status {
            Active,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(errors[0].contains("type `Status`"), "got: {}", errors[0]);
    assert!(errors[0].contains("cfg_attr"), "got: {}", errors[0]);
}

#[cfg(feature = "serde")]
#[test]
fn cfg_attr_wrapped_serde_on_a_variant_is_rejected() {
    let errors = enum_cfg_attr_errors(&syn::parse_quote! {
        enum Status {
            #[cfg_attr(feature = "serde", serde(rename = "active"))]
            Active,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(errors[0].contains("variant `Active`"), "got: {}", errors[0]);
}

/// Collects an item's rename-direction guard failures as rendered `compile_error!` token strings.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
fn rename_direction_errors(item: &syn::Item) -> Vec<String> {
    super::rename_direction_guard_errors(item)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[test]
fn a_container_renaming_naming_two_keys_is_rejected() {
    let errors = rename_direction_errors(&syn::parse_quote! {
        #[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
        struct Profile {
            my_field: u32,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(errors[0].contains("type `Profile`"), "got: {}", errors[0]);
    assert!(errors[0].contains("camelCase"), "got: {}", errors[0]);
    assert!(errors[0].contains("snake_case"), "got: {}", errors[0]);
}

#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[test]
fn a_field_renaming_naming_two_keys_is_rejected() {
    let errors = rename_direction_errors(&syn::parse_quote! {
        struct Profile {
            #[serde(rename(serialize = "out_name", deserialize = "in_name"))]
            value: u32,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("field `value`"), "got: {}", errors[0]);
    assert!(errors[0].contains("out_name"), "got: {}", errors[0]);
    assert!(errors[0].contains("in_name"), "got: {}", errors[0]);
}

#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[test]
fn a_variant_and_its_member_are_both_reached() {
    let errors = rename_direction_errors(&syn::parse_quote! {
        enum Event {
            #[serde(rename(serialize = "created", deserialize = "made"))]
            Created {
                #[serde(rename(serialize = "at", deserialize = "when"))]
                moment: u32,
            },
        }
    });
    assert_eq!(errors.len(), 2, "got: {errors:?}");
    assert!(
        errors.iter().any(|e| e.contains("variant `Created`")),
        "got: {errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("field `moment`")),
        "got: {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[test]
fn renamings_naming_one_key_are_left_alone() {
    let errors = rename_direction_errors(&syn::parse_quote! {
        #[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
        #[serde(bound(serialize = "T: Clone", deserialize = "T: Clone"))]
        struct Profile<T> {
            #[serde(rename(serialize = "same_name", deserialize = "same_name"))]
            my_field: T,
        }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[test]
fn each_casing_rule_of_an_enum_naming_two_keys_is_rejected() {
    for (rule, written, read) in [
        ("rename_all", "kebab-case", "snake_case"),
        ("rename_all_fields", "camelCase", "snake_case"),
    ] {
        let source = format!(
            "#[serde({rule}(serialize = \"{written}\", deserialize = \"{read}\"))] \
             pub enum Carriage {{ OverLand {{ road_name: u32 }} }}"
        );
        let errors = rename_direction_errors(&syn::parse_str(&source).unwrap());
        assert_eq!(errors.len(), 1, "for {rule}, got: {errors:?}");
        assert!(
            errors[0].contains(&format!(
                "type `Carriage`: serde `{rule}` names `{written}` when serializing and `{read}` \
                 when deserializing"
            )),
            "got: {}",
            errors[0]
        );
    }
}

/// The README quotes each refusal as it is printed.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[test]
fn a_rename_refusal_says_that_neither_a_schema_nor_a_client_can_be_written() {
    for (renaming, refusal) in [
        (
            "rename(serialize = \"written\", deserialize = \"read_too\")",
            "model_schema: field `both_ways`: serde `rename` names `written` when serializing \
             and `read_too` when deserializing, so the payload serde writes is not one serde \
             reads. A schema and an emitted client describe one key per member, so neither can \
             be written for the pair. Write both directions at one value, or use the \
             single-value spelling `rename = \"...\"`.",
        ),
        (
            "rename(serialize = \"written\")",
            "model_schema: field `both_ways`: serde `rename` names `written` when serializing \
             only, and leaves the other direction at the name it would otherwise use, so the \
             payload serde writes is not one serde reads. A schema and an emitted client \
             describe one key per member, so neither can be written for the pair. Write both \
             directions at one value, or use the single-value spelling `rename = \"...\"`.",
        ),
    ] {
        let source = format!(
            "pub struct SplitNames {{ #[serde({renaming})] pub both_ways: u32, pub plain: u32 }}"
        );
        let errors = rename_direction_errors(&syn::parse_str(&source).unwrap());
        assert_eq!(errors.len(), 1, "for {renaming}, got: {errors:?}");
        assert!(
            errors[0].contains(&proc_macro2::Literal::string(refusal).to_string()),
            "got: {}",
            errors[0]
        );
        assert!(
            include_str!("../../README.md").contains(refusal),
            "the README no longer quotes this refusal:\n{refusal}"
        );
    }
}

/// The source text each rename-direction refusal of the item `source` declares is spanned on.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
fn rename_refusal_spans(source: &str) -> Vec<String> {
    super::rename_direction_guard_errors(&syn::parse_str(source).unwrap())
        .iter()
        .flat_map(located_source_texts)
        .collect()
}

#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[test]
fn a_rename_refusal_is_spanned_on_the_key_that_names_the_two() {
    for (source, key) in [
        (
            "pub struct SplitNames { \
             #[serde(rename(serialize = \"written\", deserialize = \"read_too\"))] \
             pub both_ways: u32 }",
            "rename",
        ),
        (
            "#[serde(rename_all(serialize = \"camelCase\"))] \
             pub struct SplitNames { pub both_ways: u32 }",
            "rename_all",
        ),
        (
            "#[serde(rename_all_fields(deserialize = \"camelCase\"))] \
             pub enum Carriage { OverLand { road_name: u32 } }",
            "rename_all_fields",
        ),
    ] {
        let located = rename_refusal_spans(source);
        assert!(
            !located.is_empty() && located.iter().all(|text| text == key),
            "for {source}, got: {located:?}"
        );
    }
}

/// Each `cfg` written above the function or the constant `named` of `source`, or above its import
/// of it.
fn gates_above(source: &str, named: &str) -> Vec<String> {
    let mut gates = Vec::new();
    for item in &syn::parse_file(source).unwrap().items {
        let attrs = if let syn::Item::Fn(function) = item
            && function.sig.ident == named
        {
            &function.attrs
        } else if let syn::Item::Const(constant) = item
            && constant.ident == named
        {
            &constant.attrs
        } else if let syn::Item::Use(import) = item
            && quote::ToTokens::to_token_stream(&import.tree)
                .to_string()
                .ends_with(named)
        {
            &import.attrs
        } else {
            continue;
        };
        gates.extend(
            attrs
                .iter()
                .filter(|attr| attr.path().is_ident("cfg"))
                .map(|attr| quote::ToTokens::to_token_stream(&attr.meta).to_string()),
        );
    }
    gates
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_rename_guard_is_compiled_in_every_build_that_describes_a_type() {
    let describing = quote::quote! {
        all(
            feature = "serde",
            any(
                feature = "typescript",
                feature = "zod",
                feature = "jsonschema",
                feature = "dart",
                feature = "swift",
                feature = "kotlin"
            )
        )
    };
    let compiled = quote::quote!(cfg(#describing)).to_string();
    let left_out = quote::quote!(cfg(not(#describing))).to_string();
    let expansion = include_str!("../model_schema.rs");
    assert_eq!(
        gates_above(expansion, "rename_direction_guard_errors"),
        [compiled.as_str(), left_out.as_str()]
    );
    for (source, named) in [
        (
            include_str!("../features/serde.rs"),
            "rename_direction_rejection",
        ),
        (expansion, "rename_direction_rejection"),
        (expansion, "renamable_attribute_lists"),
        (expansion, "field_attribute_lists"),
        (expansion, "item_attrs"),
    ] {
        assert_eq!(
            gates_above(source, named),
            [compiled.as_str()],
            "for `{named}`"
        );
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_flattened_plain_enum_refusal_is_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    let registry = include_str!("../utils.rs");
    for (source, named) in [
        (expansion, "FLATTENED_PLAIN_ENUM_SCOPE"),
        (expansion, "written_as_a_plain_enum"),
        (expansion, "flattened_field_guard_error"),
        (expansion, "internally_tagged_guard_errors"),
        (expansion, "written_as_a_variant_name"),
        (registry, "is_recorded_plain_enum"),
        (registry, "record_plain_enum"),
    ] {
        assert_eq!(
            gates_above(source, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
    for named in ["collect_struct_fields", "variant_flatten_guard_errors"] {
        assert_eq!(
            gates_above(expansion, named),
            Vec::<String>::new(),
            "for `{named}`"
        );
    }
}

/// A plain enum declared above the type that flattens it, in a struct, in a variant's members,
/// beside the tag of a variant and through an alias, and one declared below it, which no
/// expansion sees.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_plain_enum_declared_above_is_refused_in_every_build_with_serde() {
    for declared in [
        "#[derive(Serialize, Deserialize)] pub enum Weather { Calm, Gale }",
        "pub type Climate = Weather;",
    ] {
        let expanded = expansion_over(declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    let logged = refusal_texts(&expansion_over(
        "#[derive(Serialize, Deserialize)] pub struct Logged { pub note: String, \
         #[serde(flatten)] pub weather: Weather }",
    ));
    assert_eq!(
        logged,
        [format!(
            "model_schema: field `weather` of `Logged` carries `#[serde(flatten)]` over \
             `Weather`, which serde does not write as an object: a plain enum writes its own \
             variant name, so it contributes no members to the object being written \u{2014} \
             serde writes that name as a key holding null, which a schema closed around the \
             remaining fields rejects. Write the field as a named member so the value gets a key \
             of its own, or flatten a type serde writes as an object. {}",
            super::FLATTENED_PLAIN_ENUM_SCOPE
        )]
    );
    let answered = format!("error: {}", logged[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    for (source, refusal) in [
        (
            "#[derive(Serialize, Deserialize)] pub struct Charted { pub note: String, \
             #[serde(flatten)] pub climate: Climate }",
            "model_schema: field `climate` of `Charted` carries `#[serde(flatten)]` over \
             `Climate`, which serde does not write as an object",
        ),
        (
            "#[derive(Serialize, Deserialize)] pub enum Logbook { Noted { note: String, \
             #[serde(flatten)] weather: Weather } }",
            "model_schema: field `weather` of `Logbook` carries `#[serde(flatten)]` over \
             `Weather`, which serde does not write as an object",
        ),
        (
            "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum Reading { \
             Held(Weather) }",
            "model_schema: variant `Held` of internally tagged enum `Reading` wraps `Weather`, \
             which serde does not write as an object",
        ),
    ] {
        let refused = refusal_texts(&expansion_over(source));
        assert!(
            refused.len() == 1
                && refused[0].starts_with(refusal)
                && refused[0].ends_with(super::FLATTENED_PLAIN_ENUM_SCOPE),
            "for {source}, got: {refused:?}"
        );
    }
    let unseen = expansion_over(
        "#[derive(Serialize, Deserialize)] pub struct Forecast { pub note: String, \
         #[serde(flatten)] pub weather: Outlook }",
    );
    assert_eq!(compile_error_count(&unseen), 0, "got: {unseen}");
}

/// Declares the plain enum and the types declared above that serde writes as its variant name:
/// a struct of one slot, a brand, a generic struct of one slot whose value is its type parameter,
/// a struct of one slot over that struct, and a struct of one slot and an alias over an `Option` of
/// the enum.
#[cfg(feature = "serde")]
fn declare_plain_enum_values() {
    for (args, declared) in [
        ("", "pub enum ProbeMood { Calm, Tense }"),
        ("no_display", "pub struct ProbeMoodSlot(ProbeMood);"),
        (
            "no_display",
            "#[serde(transparent)] pub struct ProbeMoodBrand(ProbeMood);",
        ),
        (
            "no_display, default_types(T = String)",
            "pub struct ProbeWrap<T>(T);",
        ),
        ("no_display", "pub struct ProbeMoodAgain(ProbeMoodSlot);"),
        (
            "no_display",
            "pub struct ProbeMaybeMoodSlot(Option<ProbeMood>);",
        ),
        ("", "pub type ProbeMaybeMood = Option<ProbeMood>;"),
    ] {
        let expanded = serialized_under(args, declared);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {declared}, got: {expanded}"
        );
    }
}

/// The refusal of field `field` of `holder` flattened over `inner`, which serde writes as a plain
/// enum's variant name.
#[cfg(feature = "serde")]
fn flattened_plain_enum_refusal(field: &str, holder: &str, inner: &str) -> String {
    format!(
        "model_schema: field `{field}` of `{holder}` carries `#[serde(flatten)]` over `{inner}`, \
         which serde does not write as an object: a plain enum writes its own variant name, so it \
         contributes no members to the object being written \u{2014} serde writes that name as a \
         key holding null, which a schema closed around the remaining fields rejects. Write the \
         field as a named member so the value gets a key of its own, or flatten a type serde \
         writes as an object. {}",
        super::FLATTENED_PLAIN_ENUM_SCOPE
    )
}

/// The refusal of the slot of variant `Held` of `holder`, under the tag `kind`, over `inner`,
/// which serde writes as a plain enum's variant name.
#[cfg(feature = "serde")]
fn tagged_plain_enum_refusal(holder: &str, inner: &str) -> String {
    format!(
        "model_schema: variant `Held` of internally tagged enum `{holder}` wraps `{inner}`, which \
         serde does not write as an object: a plain enum writes its own variant name, so nothing \
         of it joins the tag \u{2014} serde writes that name as a key holding null, which a schema \
         closed around the tag rejects. Name a `content` key so the value gets an object of its \
         own, or wrap it in a struct whose fields can sit beside the tag. {}",
        super::FLATTENED_PLAIN_ENUM_SCOPE
    )
}

/// A value serde writes as a plain enum's variant name by way of a type declared above, flattened
/// into a struct and into a variant's members: a struct of one slot and a generic type filled at
/// the enum, each refused with the refusal of the plain enum itself, named as written. serde writes
/// `{"id":"i","Calm":null}` for each and reads it back, so neither is told its write does not join
/// the object.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_value_serde_writes_as_a_plain_enum_is_refused_as_the_plain_enum_is() {
    declare_plain_enum_values();
    let flattened = |holder: &str, inner: &str| {
        refusal_texts(&serialized_under(
            "",
            &format!("pub struct {holder} {{ pub id: String, #[serde(flatten)] pub m: {inner} }}"),
        ))
    };
    let slot = flattened("ProbeM1", "ProbeMoodSlot");
    assert_eq!(
        slot,
        [flattened_plain_enum_refusal(
            "m",
            "ProbeM1",
            "ProbeMoodSlot"
        )]
    );
    let filled = flattened("ProbeM3", "ProbeWrap<ProbeMood>");
    assert_eq!(
        filled,
        [flattened_plain_enum_refusal(
            "m",
            "ProbeM3",
            "ProbeWrap<ProbeMood>"
        )]
    );
    for refused in [&slot, &filled] {
        let answered = format!("error: {}", refused[0]);
        assert!(
            include_str!("../../README.md").contains(&answered),
            "the README no longer quotes: {answered}"
        );
    }
    assert_eq!(
        refusal_texts(&serialized_under(
            "",
            "pub enum ProbeMoodLog { Noted { note: String, #[serde(flatten)] m: \
             ProbeWrap<ProbeMood> } }",
        )),
        [flattened_plain_enum_refusal(
            "m",
            "ProbeMoodLog",
            "ProbeWrap<ProbeMood>"
        )]
    );
}

/// The plain enum itself and under an `Option`, and a value serde writes as its variant name by
/// way of a brand, a `Box`, a chain of types declared above, and an `Option` of each or in each,
/// flattened, every one refused with the refusal of the plain enum itself, named as written.
/// serde writes a `Some` as the key naming the variant, holding null, and a `None` as no key.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_value_over_a_plain_enum_through_a_chain_or_an_option_is_refused_as_the_plain_enum_is()
 {
    declare_plain_enum_values();
    for (holder, flattened, named) in [
        ("ProbeMoodItself", "ProbeMood", "ProbeMood"),
        ("ProbeMoodMaybeItself", "Option<ProbeMood>", "ProbeMood"),
        ("ProbeMoodBranded", "ProbeMoodBrand", "ProbeMoodBrand"),
        ("ProbeMoodBoxed", "Box<ProbeMoodSlot>", "ProbeMoodSlot"),
        ("ProbeMoodChained", "ProbeMoodAgain", "ProbeMoodAgain"),
        (
            "ProbeMoodWrapped",
            "ProbeWrap<ProbeMoodSlot>",
            "ProbeWrap<ProbeMoodSlot>",
        ),
        (
            "ProbeMoodMaybeSlot",
            "Option<ProbeMoodSlot>",
            "ProbeMoodSlot",
        ),
        (
            "ProbeMoodMaybeFilled",
            "Option<ProbeWrap<ProbeMood>>",
            "ProbeWrap<ProbeMood>",
        ),
        (
            "ProbeMoodSlotOfMaybe",
            "ProbeMaybeMoodSlot",
            "ProbeMaybeMoodSlot",
        ),
        (
            "ProbeMoodFilledMaybe",
            "ProbeWrap<Option<ProbeMood>>",
            "ProbeWrap<Option<ProbeMood>>",
        ),
        ("ProbeMoodAliasOfMaybe", "ProbeMaybeMood", "ProbeMaybeMood"),
    ] {
        assert_eq!(
            flattened_beside_an_id(holder, flattened),
            [flattened_plain_enum_refusal("wrap", holder, named)],
            "for {flattened}"
        );
    }
}

/// A newtype variant under a tag alone over a value serde writes as a plain enum's variant name
/// by way of a type declared above: a struct of one slot, a brand, a generic type filled at the
/// enum and a chain of them, each refused with the refusal of the plain enum itself, named as
/// written. serde writes `{"kind":"Held","Calm":null}` for each and reads it back, so none is told
/// it cannot sit beside the tag. One that writes an `Option` of the enum is refused as an
/// `Option`, serde refusing every write of it there.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_newtype_variant_over_a_value_serde_writes_as_a_plain_enum_is_refused_as_the_plain_enum_is()
 {
    declare_plain_enum_values();
    let slot = held_under_tag("ProbeM2", "ProbeMoodSlot");
    assert_eq!(
        slot,
        [tagged_plain_enum_refusal("ProbeM2", "ProbeMoodSlot")]
    );
    let filled = held_under_tag("ProbeM4", "ProbeWrap<ProbeMood>");
    assert_eq!(
        filled,
        [tagged_plain_enum_refusal("ProbeM4", "ProbeWrap<ProbeMood>")]
    );
    for refused in [&slot, &filled] {
        let answered = format!("error: {}", refused[0]);
        assert!(
            include_str!("../../README.md").contains(&answered),
            "the README no longer quotes: {answered}"
        );
    }
    for (holder, inner, named) in [
        ("ProbeMoodItselfHeld", "ProbeMood", "ProbeMood"),
        ("ProbeMoodBrandHeld", "ProbeMoodBrand", "ProbeMoodBrand"),
        ("ProbeMoodBoxedHeld", "Box<ProbeMoodSlot>", "ProbeMoodSlot"),
        ("ProbeMoodAgainHeld", "ProbeMoodAgain", "ProbeMoodAgain"),
        (
            "ProbeMoodWrappedHeld",
            "ProbeWrap<ProbeMoodBrand>",
            "ProbeWrap<ProbeMoodBrand>",
        ),
    ] {
        assert_eq!(
            held_under_tag(holder, inner),
            [tagged_plain_enum_refusal(holder, named)],
            "for {inner}"
        );
    }
    for (holder, inner) in [
        ("ProbeMoodSlotOfMaybeHeld", "ProbeMaybeMoodSlot"),
        ("ProbeMoodFilledMaybeHeld", "ProbeWrap<Option<ProbeMood>>"),
        ("ProbeMoodAliasOfMaybeHeld", "ProbeMaybeMood"),
        ("ProbeMoodWrappedMaybeHeld", "ProbeWrap<ProbeMaybeMoodSlot>"),
    ] {
        assert_eq!(
            held_under_tag(holder, inner),
            [tagged_optional_refusal(holder, inner)],
            "for {inner}"
        );
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_flattened_non_object_refusal_is_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    let registry = include_str!("../utils.rs");
    for (source, named) in [
        (expansion, "FLATTENED_NON_OBJECT_SCOPE"),
        (expansion, "FLATTENED_IN_PLACE_SCOPE"),
        (expansion, "FLATTENED_UNIT_MEMBER_SCOPE"),
        (expansion, "flatten_edge_guard_error"),
        (expansion, "written_in_place"),
        (expansion, "written_non_object"),
        (expansion, "written_union_members"),
        (expansion, "flattened_name_non_object_wire"),
        (expansion, "flattened_name_refused_branch"),
        (expansion, "registered_non_object_wire"),
        (expansion, "written_leaves"),
        (expansion, "non_object_wire"),
        (expansion, "member_written_leaves"),
        (expansion, "published_wire_leaves"),
        (expansion, "named_wire_leaves"),
        (expansion, "written_non_object_wire"),
        (expansion, "external_variant_wire_leaves"),
        (registry, "record_non_object_wire"),
        (registry, "recorded_non_object_wire"),
    ] {
        assert_eq!(
            gates_above(source, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
    assert_eq!(
        gates_above(expansion, "variant_flatten_guard_errors"),
        Vec::<String>::new(),
        "for `variant_flatten_guard_errors`"
    );
}

/// The refusal of a flattened `inner` of `holder` that serde writes as a `named` value.
#[cfg(feature = "serde")]
fn whole_non_object_refusal(holder: &str, inner: &str, named: &str) -> String {
    format!(
        "model_schema: `{holder}`: `#[serde(flatten)]` of `{inner}` is not written as an object \
         \u{2014} its schema describes a `{named}`, which has no members to merge, and what serde \
         writes for it does not join the object being written; write the field as a named member \
         so the value gets a key of its own. {}",
        super::FLATTENED_NON_OBJECT_SCOPE
    )
}

/// The refusal of a flattened `inner` of `holder` whose union member at `branch` serde writes as
/// a plain enum's variant name.
#[cfg(feature = "serde")]
fn flattened_variant_name_refusal(holder: &str, inner: &str, branch: &str) -> String {
    format!(
        "model_schema: `{holder}`: `#[serde(flatten)]` of `{inner}` writes a union member as a \
         plain enum's variant name \u{2014} its branch {branch} describes a `string`, and serde \
         writes that name as a key holding null, which a schema closed around the remaining \
         fields rejects; write the field as a named member so the value gets a key of its own. \
         {}",
        super::FLATTENED_PLAIN_ENUM_SCOPE
    )
}

/// A brand over text, a tuple struct and an untagged enum with a member over a plain enum, each
/// flattened where it is declared above, in a struct and in a variant's members, and a brand
/// declared below, which no expansion sees.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_type_serde_writes_as_no_object_declared_above_is_refused_in_every_build_with_serde()
{
    for declared in [
        "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct RecordId(String);",
        "#[derive(Serialize, Deserialize)] pub struct Pair(String, u32);",
        "#[derive(Serialize, Deserialize)] pub enum Mood { Calm, Tense }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum Feeling { Mood(Mood), \
         Named { name: String } }",
    ] {
        let expanded = expansion_over(declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    let carded = refusal_texts(&expansion_over(
        "#[derive(Serialize, Deserialize)] pub struct Carded { #[serde(flatten)] pub id: \
         RecordId, pub name: String }",
    ));
    assert_eq!(
        carded,
        [whole_non_object_refusal("Carded", "RecordId", "string")]
    );
    let answered = format!("error: {}", carded[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] pub struct Paired { pub name: String, \
             #[serde(flatten)] pub pair: Pair }",
        )),
        [whole_non_object_refusal("Paired", "Pair", "array")]
    );
    let felt = refusal_texts(&expansion_over(
        "#[derive(Serialize, Deserialize)] pub struct Felt { #[serde(flatten)] pub feeling: \
         Feeling, pub id: String }",
    ));
    assert_eq!(
        felt,
        [flattened_variant_name_refusal("Felt", "Feeling", "1")]
    );
    let answered_felt = format!("error: {}", felt[0]);
    assert!(
        include_str!("../../README.md").contains(&answered_felt),
        "the README no longer quotes: {answered_felt}"
    );
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] pub enum Logbook { Noted { note: String, \
             #[serde(flatten)] id: RecordId } }",
        )),
        [whole_non_object_refusal("Logbook", "RecordId", "string")]
    );
    let unseen = expansion_over(
        "#[derive(Serialize, Deserialize)] pub struct Clipped { #[serde(flatten)] pub id: \
         ClipId, pub name: String }",
    );
    assert_eq!(compile_error_count(&unseen), 0, "got: {unseen}");
}

/// A struct of one slot and an untagged member, declared above, that hold a date under
/// `as_number` are refused flattened and beside a tag as the integer serde writes for it, through
/// a generic type filled at one and an alias of one too, where a date with no `as_number` is
/// refused as the text serde writes.
#[cfg(all(feature = "serde", feature = "chrono"))]
#[test]
fn a_date_under_as_number_declared_above_is_refused_as_the_integer_serde_writes() {
    for (args, declared) in [
        (
            "",
            "pub struct ProbeMillis(#[model_schema_prop(as_number)] pub DateTime<Utc>);",
        ),
        ("", "pub struct ProbeInstant(pub DateTime<Utc>);"),
        (
            "",
            "#[serde(untagged)] pub enum ProbeMillisOrFields { Fields { note: String }, \
             Millis(#[model_schema_prop(as_number)] DateTime<Utc>) }",
        ),
        (
            "default_types(T = String)",
            "pub struct ProbeWrap<T>(pub T);",
        ),
    ] {
        let expanded = serialized_under(args, declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    let aliased = expansion_over("pub type ProbeMillisAlias = ProbeMillis;");
    assert_eq!(compile_error_count(&aliased), 0, "got: {aliased}");
    let whole = "is not written as an object \u{2014} its schema describes a";
    for (inner, written) in [
        ("ProbeMillis", "integer"),
        ("ProbeWrap<ProbeMillis>", "integer"),
        ("ProbeMillisAlias", "integer"),
        ("ProbeInstant", "string"),
    ] {
        assert_eq!(
            refusal_texts(&expansion_over(&format!(
                "#[derive(Serialize, Deserialize)] pub struct ProbeFlatStamp {{ pub id: u8, \
                 #[serde(flatten)] pub stamp: {inner} }}"
            ))),
            [whole_non_object_refusal("ProbeFlatStamp", inner, written)]
        );
        assert_eq!(
            refusal_texts(&expansion_over(&format!(
                "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum \
                 ProbeTaggedStamp {{ Held({inner}) }}"
            ))),
            [tagged_non_object_refusal(
                "ProbeTaggedStamp",
                inner,
                &format!("{whole} `{written}`")
            )]
        );
    }
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] pub struct ProbeFlatChoice { pub id: u8, \
             #[serde(flatten)] pub stamp: ProbeMillisOrFields }",
        )),
        [flattened_branch_refusal(
            "ProbeFlatChoice",
            "ProbeMillisOrFields",
            "2",
            "integer"
        )]
    );
}

/// The record every build with serde writes of a date under `as_number` proves what the record a
/// Zod surface writes proves: the integer serde writes.
#[cfg(all(feature = "serde", feature = "zod", feature = "chrono"))]
#[test]
fn both_records_prove_the_integer_serde_writes_for_a_date_under_as_number() {
    for declared in [
        "pub struct Stamped(#[model_schema_prop(as_number)] pub DateTime<Utc>);",
        "#[serde(untagged)] pub enum StampedOrFields { Fields { note: String }, \
         Stamp(#[model_schema_prop(as_number)] DateTime<Utc>) }",
        "#[serde(untagged)] pub enum MaybeStamped { Fields { note: String }, \
         Stamp(#[model_schema_prop(as_number)] Option<DateTime<Utc>>) }",
    ] {
        let expanded = serialized_under("", declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    let branch = |at: &str| Some(NonObject::Branch(at.to_owned(), "integer"));
    for (flattened, proved) in [
        ("Stamped", Some(NonObject::Whole("integer"))),
        ("StampedOrFields", branch("2")),
        ("MaybeStamped", branch("2.1")),
    ] {
        let inner = get_field_def("_flattened", &syn::parse_str(flattened).unwrap(), "");
        assert_eq!(written_non_object(&inner), proved, "for `{flattened}`");
        assert_eq!(surfaced_non_object(&inner), proved, "for `{flattened}`");
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_tagged_non_object_refusal_is_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    for named in [
        "tagged_non_object_message",
        "internally_tagged_guard_errors",
        "tagged_content",
        "process_internally_tagged_enum",
        "written_non_object",
        "FLATTENED_NON_OBJECT_SCOPE",
        "TAGGED_UNIT_WRITTEN",
    ] {
        assert_eq!(
            gates_above(expansion, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
}

/// The refusal of the slot of variant `Held` of `holder`, under the tag `kind`, over `inner`,
/// which `written` says serde writes as no object.
#[cfg(feature = "serde")]
fn tagged_non_object_refusal(holder: &str, inner: &str, written: &str) -> String {
    format!(
        "model_schema: variant `Held` of internally tagged enum `{holder}` wraps `{inner}`, \
         which {written}, which cannot sit beside the tag: `#[serde(tag = \"kind\")]` with no \
         `content` writes the variant's data as members of the object the tag is written in, and \
         only a value serde writes as an object has members to put there. Name a `content` key \
         so the value gets an object of its own, or wrap it in a struct whose fields can sit \
         beside the tag. {}",
        super::FLATTENED_NON_OBJECT_SCOPE
    )
}

/// A newtype variant under a tag alone over a brand over text, a tuple struct and an untagged
/// enum with a member that is no object, each declared above, and the slots that stay accepted:
/// a brand over a struct, an untagged enum of objects, a type declared below, and an enum under a
/// tag and a content key.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_newtype_variant_over_a_type_serde_writes_as_no_object_is_refused_in_every_build_with_serde()
 {
    for declared in [
        "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct ProbeId(String);",
        "#[derive(Serialize, Deserialize)] pub struct ProbePair(String, u32);",
        "#[derive(Serialize, Deserialize)] pub struct ProbeFields { pub note: String }",
        "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct ProbeBody(ProbeFields);",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum ProbeEither { \
         Fields(ProbeFields), Text(String) }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum ProbeObjects { \
         Fields(ProbeFields), Body(ProbeBody) }",
    ] {
        let expanded = expansion_over(declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    let whole = "is not written as an object \u{2014} its schema describes a";
    let reading = refusal_texts(&expansion_over(
        "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum ProbeReading { \
         Held(ProbeId) }",
    ));
    assert_eq!(
        reading,
        [tagged_non_object_refusal(
            "ProbeReading",
            "ProbeId",
            &format!("{whole} `string`")
        )]
    );
    let answered = format!("error: {}", reading[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum ProbePaired { \
             Held(ProbePair) }",
        )),
        [tagged_non_object_refusal(
            "ProbePaired",
            "ProbePair",
            &format!("{whole} `array`")
        )]
    );
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum ProbeChoice { \
             Held(ProbeEither) }",
        )),
        [tagged_non_object_refusal(
            "ProbeChoice",
            "ProbeEither",
            "writes a union member that is not an object \u{2014} its branch 2 describes a \
             `string`"
        )]
    );
    for accepted in [
        "#[serde(tag = \"kind\")] pub enum ProbeHolding { Held(ProbeBody) }",
        "#[serde(tag = \"kind\")] pub enum ProbeChosen { Held(ProbeObjects) }",
        "#[serde(tag = \"kind\")] pub enum ProbeLater { Held(ProbeUnseen) }",
        "#[serde(tag = \"kind\", content = \"value\")] pub enum ProbeAdjacent { Held(ProbeId), \
         Paired(ProbePair) }",
    ] {
        let expanded = expansion_over(&format!("#[derive(Serialize, Deserialize)] {accepted}"));
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {accepted}, got: {expanded}"
        );
    }
}

/// A newtype variant under a tag alone over an untagged enum declared above with a member serde
/// writes as a variant's name beside the tag, a plain enum's and an externally tagged enum's unit
/// variant's, and the same enums flattened.
#[cfg(feature = "serde")]
#[test]
fn a_union_member_written_as_a_variant_name_is_refused_with_the_key_serde_writes_for_it() {
    for declared in [
        "#[derive(Serialize, Deserialize)] pub enum ProbeMood { Calm, Tense }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum ProbeFeeling { \
         Mood(ProbeMood), Named { name: String } }",
        "#[derive(Serialize, Deserialize)] pub enum ProbeTack { Bare, Wrapped { b: bool } }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum ProbeTackEither { \
         Named { name: String }, Tack(ProbeTack) }",
    ] {
        let expanded = expansion_over(declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    let held = |holder: &str, inner: &str, member: &str, branch: &str, scope: &str| {
        format!(
            "model_schema: variant `Held` of internally tagged enum `{holder}` wraps `{inner}`, \
             which writes a union member as {member} \u{2014} its branch {branch} describes a \
             `string`, and serde writes that name beside the tag as a key holding null, which a \
             schema closed around the tag rejects. Name a `content` key so the value gets an \
             object of its own, or wrap it in a struct whose fields can sit beside the tag. \
             {scope}"
        )
    };
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum ProbeM9 { \
             Held(ProbeFeeling) }",
        )),
        [held(
            "ProbeM9",
            "ProbeFeeling",
            "a plain enum's variant name",
            "1",
            super::FLATTENED_PLAIN_ENUM_SCOPE
        )]
    );
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum ProbeTacked { \
             Held(ProbeTackEither) }",
        )),
        [held(
            "ProbeTacked",
            "ProbeTackEither",
            "the name of an externally tagged enum's unit variant",
            "2.1",
            super::FLATTENED_NON_OBJECT_SCOPE
        )]
    );
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] pub struct ProbeM8 { pub id: String, \
             #[serde(flatten)] pub feeling: ProbeFeeling }",
        )),
        [flattened_variant_name_refusal(
            "ProbeM8",
            "ProbeFeeling",
            "1"
        )]
    );
    assert_eq!(
        refusal_texts(&expansion_over(
            "#[derive(Serialize, Deserialize)] pub struct ProbeTackFlat { pub id: String, \
             #[serde(flatten)] pub tack: ProbeTackEither }",
        )),
        [format!(
            "model_schema: `ProbeTackFlat`: `#[serde(flatten)]` of `ProbeTackEither` writes a \
             union member as the name of an externally tagged enum's unit variant \u{2014} its \
             branch 2.1 describes a `string`, and serde writes that name as a key holding null, \
             which a schema closed around the remaining fields rejects; write the field as a \
             named member so the value gets a key of its own. {}",
            super::FLATTENED_NON_OBJECT_SCOPE
        )]
    );
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_tagged_optional_refusal_is_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    let registry = include_str!("../utils.rs");
    for (source, named) in [
        (expansion, "tagged_optional_message"),
        (expansion, "writes_an_optional"),
        (expansion, "internally_tagged_guard_errors"),
        (expansion, "non_object_wire"),
        (expansion, "TAGGED_OPTIONAL_SCOPE"),
        (registry, "record_non_object_wire"),
        (registry, "recorded_non_object_wire"),
    ] {
        assert_eq!(
            gates_above(source, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
}

/// The refusal of the slot of variant `Held` of `holder`, under the tag `kind`, over `inner`,
/// which serde writes as an `Option`.
#[cfg(feature = "serde")]
fn tagged_optional_refusal(holder: &str, inner: &str) -> String {
    format!(
        "model_schema: variant `Held` of internally tagged enum `{holder}` wraps `{inner}`, \
         which serde writes as an optional, which cannot sit beside the tag: `#[serde(tag = \
         \"kind\")]` with no `content` writes the variant's data as members of the object the tag \
         is written in, and only a value serde writes as an object has members to put there. \
         serde refuses this one at run time \u{2014} `cannot serialize tagged newtype variant \
         {holder}::Held containing an optional`. Name a `content` key so the value gets an object \
         of its own, or wrap it in a struct whose fields can sit beside the tag. {}",
        super::TAGGED_OPTIONAL_SCOPE
    )
}

/// A newtype variant under a tag alone over a struct of one slot, a brand and an alias over an
/// `Option`, an alias of that struct and a struct of one slot over it, each declared above, and
/// the shapes that stay accepted: that struct flattened into a struct and into a variant's
/// members, a slot declared below, and an enum under a tag and a content key.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_newtype_variant_over_a_type_serde_writes_as_an_optional_is_refused_in_every_build_with_serde()
 {
    for declared in [
        "#[derive(Serialize, Deserialize)] pub struct ProbeFields { pub note: String }",
        "#[derive(Serialize, Deserialize)] pub struct ProbeMaybe(Option<ProbeFields>);",
        "pub type ProbeMaybeAlias = Option<ProbeFields>;",
        "pub type ProbeMaybeAgain = ProbeMaybe;",
        "#[derive(Serialize, Deserialize)] pub struct ProbeMaybeHeld(ProbeMaybe);",
    ] {
        let expanded = expansion_over(declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    // A build that describes a type refuses a brand over an `Option` itself; the record is written
    // either way.
    let brand = expansion_over(
        "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct ProbeMaybeBrand(Box<\
         Option<ProbeFields>>);",
    );
    assert_eq!(
        compile_error_count(&brand),
        usize::from(cfg!(any(
            feature = "typescript",
            feature = "zod",
            feature = "jsonschema",
            feature = "dart",
            feature = "swift",
            feature = "kotlin"
        ))),
        "got: {brand}"
    );
    let optional = refusal_texts(&expansion_over(
        "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum ProbeOptional { \
         Held(ProbeMaybe) }",
    ));
    assert_eq!(
        optional,
        [tagged_optional_refusal("ProbeOptional", "ProbeMaybe")]
    );
    let answered = format!("error: {}", optional[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    for (holder, inner) in [
        ("ProbeBranded", "ProbeMaybeBrand"),
        ("ProbeAliased", "ProbeMaybeAlias"),
        ("ProbeAgain", "ProbeMaybeAgain"),
        ("ProbeHeldTwice", "ProbeMaybeHeld"),
    ] {
        assert_eq!(
            refusal_texts(&expansion_over(&format!(
                "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum {holder} {{ \
                 Held(Box<{inner}>) }}"
            ))),
            [tagged_optional_refusal(holder, inner)],
            "for {inner}"
        );
    }
    for accepted in [
        "pub struct ProbeFlat { pub id: String, #[serde(flatten)] pub maybe: ProbeMaybe }",
        "pub enum ProbeFlatVariant { Noted { id: String, #[serde(flatten)] maybe: ProbeMaybe } }",
        "#[serde(tag = \"kind\")] pub enum ProbeFlatTagged { Noted { id: String, \
         #[serde(flatten)] maybe: ProbeMaybe } }",
        "#[serde(tag = \"kind\")] pub enum ProbeLater { Held(ProbeMaybeUnseen) }",
        "#[serde(tag = \"kind\", content = \"value\")] pub enum ProbeAdjacent { \
         Held(ProbeMaybe) }",
    ] {
        let expanded = expansion_over(&format!("#[derive(Serialize, Deserialize)] {accepted}"));
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {accepted}, got: {expanded}"
        );
    }
}

/// The refusal of a flattened `inner` of `holder` whose union member at `branch` serde writes as
/// a `named` value.
#[cfg(feature = "serde")]
fn flattened_branch_refusal(holder: &str, inner: &str, branch: &str, named: &str) -> String {
    format!(
        "model_schema: `{holder}`: `#[serde(flatten)]` of `{inner}` writes a union member that is \
         not an object \u{2014} its branch {branch} describes a `{named}`, which has no members to \
         merge, and what serde writes for that member does not join the object being written; \
         write the field as a named member so the value gets a key of its own. {}",
        super::FLATTENED_NON_OBJECT_SCOPE
    )
}

/// A newtype variant under a tag alone over a type declared above that serde writes as an
/// `Option` of a value with a leaf written as no object is refused as an `Option`, which serde's
/// tag refuses before it reads the value, and the same type flattened is refused with its branch.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_optional_of_a_value_written_as_no_object_is_refused_as_an_optional() {
    for declared in [
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum ProbeTextMember { Obj { \
         a: String }, Text(String) }",
        "#[derive(Serialize, Deserialize)] pub struct ProbeMaybeCountSlot(Option<u32>);",
        "pub type ProbeMaybeTextAlias = Option<ProbeTextMember>;",
        "#[derive(Serialize, Deserialize)] pub struct ProbeSlotMaybeText(Option<ProbeTextMember>);",
    ] {
        let expanded = expansion_over(declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    for (holder, inner, branch, named) in [
        ("ProbeHeldMaybeCount", "ProbeMaybeCountSlot", "1", "integer"),
        ("ProbeHeldMaybeText", "ProbeMaybeTextAlias", "2", "string"),
        (
            "ProbeHeldSlotMaybeText",
            "ProbeSlotMaybeText",
            "2",
            "string",
        ),
    ] {
        assert_eq!(
            refusal_texts(&expansion_over(&format!(
                "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum {holder} {{ \
                 Held({inner}) }}"
            ))),
            [tagged_optional_refusal(holder, inner)],
            "for {inner}"
        );
        let flattened = format!("{holder}Flat");
        let written: syn::Type = syn::parse_str(inner).unwrap();
        let skipped = if get_field_def("held", &written, "").stands_for_an_option() {
            "default, skip_serializing_if = \"Option::is_none\", "
        } else {
            ""
        };
        assert_eq!(
            refusal_texts(&expansion_over(&format!(
                "#[derive(Serialize, Deserialize)] pub struct {flattened} {{ pub id: String, \
                 #[serde({skipped}flatten)] pub held: {inner} }}"
            ))),
            [flattened_branch_refusal(&flattened, inner, branch, named)],
            "for {inner} flattened"
        );
    }
}

/// `()` beside a tag and flattened, in each way it is reached: written in place, through a struct
/// of one slot over it, through a generic type filled at it, and as a member of an untagged enum.
/// serde writes it as the tag alone, or as no key, and reads neither back as a union member.
#[cfg(feature = "serde")]
#[test]
fn a_unit_beside_a_tag_or_flattened_is_refused_in_the_words_of_what_serde_writes() {
    for declared in [
        "#[derive(Serialize, Deserialize)] pub struct ProbeFields { pub note: String }",
        "#[derive(Serialize, Deserialize)] pub struct ProbeUnitHeld(());",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum ProbeUnitMember { \
         Obj(ProbeFields), Nothing(()) }",
    ] {
        let expanded = expansion_over(declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    let wrap = serialized_under(
        "no_display, default_types(T = String)",
        "pub struct ProbeWrap<T>(T);",
    );
    assert_eq!(compile_error_count(&wrap), 0, "got: {wrap}");
    let tagged = |source: &str| {
        refusal_texts(&expansion_over(&format!(
            "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum ProbeUnitTagged \
             {{ Held({source}) }}"
        )))
    };
    let written_beside_the_tag = "beside the tag serde writes the variant as the tag alone, and \
         reads it back, while the TypeScript type and the Zod schema of the variant intersect the \
         tag's object with `null`, which admits no value.";
    let in_place = tagged("()");
    assert_eq!(
        in_place,
        [format!(
            "model_schema: variant `Held` of internally tagged enum `ProbeUnitTagged` wraps `()`, \
             which serde writes as `null`: {written_beside_the_tag} Name a `content` key so the \
             value gets an object of its own, or write `Held` as a unit variant, which serde \
             writes as the tag alone as well."
        )]
    );
    let answered = format!("error: {}", in_place[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    for inner in ["ProbeUnitHeld", "ProbeWrap<()>"] {
        assert_eq!(
            tagged(inner),
            [format!(
                "model_schema: variant `Held` of internally tagged enum `ProbeUnitTagged` wraps \
                 `{inner}`, which serde writes as `()` \u{2014} its schema describes a `null`: \
                 {written_beside_the_tag} Name a `content` key so the value gets an object of its \
                 own, or write `Held` as a unit variant, which serde writes as the tag alone as \
                 well. {}",
                super::FLATTENED_NON_OBJECT_SCOPE
            )],
            "for {inner}"
        );
    }
    assert_eq!(
        tagged("ProbeUnitMember"),
        [format!(
            "model_schema: variant `Held` of internally tagged enum `ProbeUnitTagged` wraps \
             `ProbeUnitMember`, which writes a union member as `()` \u{2014} its branch 2 \
             describes a `null`, for which serde writes the tag alone, and which it does not read \
             back as that member. Name a `content` key so the value gets an object of its own, or \
             wrap it in a struct whose fields can sit beside the tag. {}",
            super::FLATTENED_NON_OBJECT_SCOPE
        )]
    );
    let flattened = |source: &str| {
        refusal_texts(&expansion_over(&format!(
            "#[derive(Serialize, Deserialize)] pub struct ProbeUnitFlat {{ pub id: String, \
             #[serde(flatten)] pub held: {source} }}"
        )))
    };
    for inner in ["ProbeUnitHeld", "ProbeWrap<()>"] {
        assert_eq!(
            flattened(inner),
            [format!(
                "model_schema: `ProbeUnitFlat`: `#[serde(flatten)]` of `{inner}` is written as \
                 `()` \u{2014} its schema describes a `null`: serde writes no key for it, and \
                 reads the object back, while the TypeScript type and the Zod schema intersect the \
                 object with `null`, which admits no value; write the field as a named member so \
                 the value gets a key of its own. {}",
                super::FLATTENED_NON_OBJECT_SCOPE
            )],
            "for {inner}"
        );
    }
    assert_eq!(
        flattened("ProbeUnitMember"),
        [format!(
            "model_schema: `ProbeUnitFlat`: `#[serde(flatten)]` of `ProbeUnitMember` writes a \
             union member as `()` \u{2014} its branch 2 describes a `null`, for which serde writes \
             no key, and which it does not read back as that member; write the field as a named \
             member so the value gets a key of its own. {}",
            super::FLATTENED_UNIT_MEMBER_SCOPE
        )]
    );
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_tagged_filled_refusal_is_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    let registry = include_str!("../utils.rs");
    for (source, named) in [
        (expansion, "value_parameter_position"),
        (expansion, "written_as_its_slot"),
        (expansion, "value_filling"),
        (expansion, "filled_argument"),
        (expansion, "filled_spelling"),
        (expansion, "filled_non_object"),
        (expansion, "tagged_non_object_message"),
        (expansion, "tagged_optional_message"),
        (registry, "record_value_parameter"),
        (registry, "value_parameter"),
    ] {
        assert_eq!(
            gates_above(source, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
}

/// The expansion of `source`, deriving both serde traits, under the `#[model_schema(...)]`
/// arguments `args`.
#[cfg(feature = "serde")]
fn serialized_under(args: &str, source: &str) -> proc_macro2::TokenStream {
    super::exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(&format!("#[derive(Serialize, Deserialize)] {source}")).unwrap(),
    )
}

/// Declares the types the generic slots of the refusal of a filled generic type are held to: a
/// struct, a struct of one slot over text and one over an `Option`, and a struct of one slot, a
/// brand over a `Box` and an alias whose value is their type parameter, one of them through
/// another.
#[cfg(feature = "serde")]
fn declare_filled_probes() {
    for (args, declared) in [
        ("", "pub struct ProbeFields { pub note: String }"),
        ("", "pub struct ProbeText(String);"),
        ("", "pub struct ProbeMaybe(Option<ProbeFields>);"),
        (
            "no_display, default_types(T = String)",
            "pub struct ProbeWrap<T>(T);",
        ),
        (
            "default_types(T = String)",
            "#[serde(transparent)] pub struct ProbeBoxed<T>(Box<T>);",
        ),
        (
            "no_display, default_types(U = String)",
            "pub struct ProbeOuter<U>(ProbeWrap<U>);",
        ),
        (
            "default_types(V = String)",
            "pub type ProbeWrapAlias<V> = ProbeWrap<V>;",
        ),
    ] {
        let expanded = serialized_under(args, declared);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {declared}, got: {expanded}"
        );
    }
}

/// The refusals of variant `Held` of `holder`, under the tag `kind` alone, over `slot`.
#[cfg(feature = "serde")]
fn held_under_tag(holder: &str, slot: &str) -> Vec<String> {
    refusal_texts(&serialized_under(
        "",
        &format!("#[serde(tag = \"kind\")] pub enum {holder} {{ Held({slot}) }}"),
    ))
}

/// A newtype variant under a tag alone over a generic type declared above whose value is its type
/// parameter, filled at a value serde writes as no object, through a struct of one slot, a brand,
/// an alias and a chain of them, refused as the slot of the same shape no parameter fills is.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_newtype_variant_over_a_generic_type_filled_at_no_object_is_refused_in_every_build_with_serde()
 {
    declare_filled_probes();
    let whole = "is not written as an object \u{2014} its schema describes a";
    let text = held_under_tag("ProbeFilledText", "ProbeWrap<String>");
    assert_eq!(
        text,
        [tagged_non_object_refusal(
            "ProbeFilledText",
            "ProbeWrap<String>",
            &format!("{whole} `string`")
        )]
    );
    assert_eq!(
        held_under_tag("ProbeTextHeld", "ProbeText"),
        [tagged_non_object_refusal(
            "ProbeTextHeld",
            "ProbeText",
            &format!("{whole} `string`")
        )],
        "the slot of the same shape no parameter fills"
    );
    let answered = format!("error: {}", text[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    assert_eq!(
        held_under_tag("ProbeFilledCount", "Box<ProbeWrap<u32>>"),
        [tagged_non_object_refusal(
            "ProbeFilledCount",
            "ProbeWrap<u32>",
            &format!("{whole} `integer`")
        )]
    );
    for (holder, slot, named) in [
        ("ProbeFilledList", "ProbeWrap<Vec<String>>", "array"),
        ("ProbeFilledFlag", "ProbeWrap<bool>", "boolean"),
        ("ProbeFilledBrand", "ProbeBoxed<String>", "string"),
        ("ProbeFilledChain", "ProbeOuter<f64>", "number"),
        ("ProbeFilledAlias", "ProbeWrapAlias<String>", "string"),
        ("ProbeFilledTwice", "ProbeWrap<ProbeWrap<char>>", "string"),
        ("ProbeFilledNamed", "ProbeWrap<ProbeText>", "string"),
    ] {
        assert_eq!(
            held_under_tag(holder, slot),
            [tagged_non_object_refusal(
                holder,
                slot,
                &format!("{whole} `{named}`")
            )],
            "for {slot}"
        );
    }
}

/// A newtype variant under a tag alone over a generic type declared above whose value is its type
/// parameter, filled at an `Option` or at a type serde writes as one, directly and through a
/// chain, refused as the slot of the same shape no parameter fills is.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_newtype_variant_over_a_generic_type_filled_at_an_optional_is_refused_in_every_build_with_serde()
 {
    declare_filled_probes();
    let optional = held_under_tag("ProbeFilledOptional", "ProbeWrap<Option<ProbeFields>>");
    assert_eq!(
        optional,
        [tagged_optional_refusal(
            "ProbeFilledOptional",
            "ProbeWrap<Option<ProbeFields>>"
        )]
    );
    assert_eq!(
        held_under_tag("ProbeMaybeHeld", "ProbeMaybe"),
        [tagged_optional_refusal("ProbeMaybeHeld", "ProbeMaybe")],
        "the slot of the same shape no parameter fills"
    );
    let answered = format!("error: {}", optional[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    for (holder, slot) in [
        ("ProbeFilledOptionalText", "ProbeWrap<Option<String>>"),
        ("ProbeFilledMaybe", "ProbeWrap<ProbeMaybe>"),
        (
            "ProbeFilledOptionalChain",
            "ProbeOuter<Option<ProbeFields>>",
        ),
    ] {
        assert_eq!(
            held_under_tag(holder, slot),
            [tagged_optional_refusal(holder, slot)],
            "for {slot}"
        );
    }
}

/// The generic slots under a tag alone that stay accepted: a filling serde writes as an object,
/// directly and through a chain, a type declared below, the enum's own parameter, and an enum under
/// a tag and a content key.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_newtype_variant_over_a_generic_type_filled_at_an_object_is_accepted() {
    declare_filled_probes();
    for (args, accepted) in [
        (
            "",
            "#[serde(tag = \"kind\")] pub enum ProbeFilledFields { Held(ProbeWrap<ProbeFields>) }",
        ),
        (
            "",
            "#[serde(tag = \"kind\")] pub enum ProbeFilledOuter { Held(ProbeOuter<ProbeFields>) }",
        ),
        (
            "",
            "#[serde(tag = \"kind\")] pub enum ProbeFilledLater { Held(ProbeWrapUnseen<String>) }",
        ),
        (
            "",
            "#[serde(tag = \"kind\", content = \"value\")] pub enum ProbeFilledAdjacent { \
             Held(ProbeWrap<String>) }",
        ),
        (
            "default_types(X = ProbeFields)",
            "#[serde(tag = \"kind\")] pub enum ProbeFilledByItsOwn<X> { Held(ProbeWrap<X>) }",
        ),
    ] {
        let expanded = serialized_under(args, accepted);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {accepted}, got: {expanded}"
        );
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_filled_holder_and_flatten_refusals_are_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    for named in [
        "non_object_wire",
        "without_its_option",
        "under_an_option",
        "filled_wire_leaves",
        "filled_union_members",
        "writes_an_optional",
        "flatten_edge_guard_error",
        "flattened_name_non_object",
        "refused_branch",
        "flattened_filling_non_object",
        "filled_non_object",
        "filled_spelling",
        "value_filling",
    ] {
        assert_eq!(
            gates_above(expansion, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
}

/// Declares, beside the types [`declare_filled_probes`] declares, the types whose value is a
/// generic type declared above filled at an argument: a struct of one slot, a brand and an alias
/// filled at a value serde writes as no object, at an `Option` and at a struct, a struct of one
/// slot over an `Option` of one, and a struct of one slot over `Option<String>`.
#[cfg(feature = "serde")]
fn declare_filled_holders() {
    declare_filled_probes();
    for (args, declared) in [
        (
            "no_display",
            "pub struct ProbeTextFilled(ProbeWrap<String>);",
        ),
        (
            "no_display",
            "#[serde(transparent)] pub struct ProbeCountFilled(Box<ProbeWrap<u32>>);",
        ),
        ("", "pub type ProbeFlagFilled = ProbeWrap<bool>;"),
        (
            "no_display",
            "pub struct ProbeListFilled(ProbeOuter<Vec<String>>);",
        ),
        (
            "no_display",
            "pub struct ProbeNamedFilled(ProbeWrap<ProbeText>);",
        ),
        (
            "no_display",
            "pub struct ProbeMaybeFilled(ProbeWrap<Option<ProbeFields>>);",
        ),
        (
            "no_display",
            "pub struct ProbeMaybeTextFilled(Option<ProbeWrap<String>>);",
        ),
        (
            "no_display",
            "pub struct ProbeFieldsFilled(ProbeWrap<ProbeFields>);",
        ),
        (
            "no_display",
            "pub struct ProbeLaterFilled(ProbeWrapUnseen<String>);",
        ),
        ("", "pub struct ProbeMaybeText(Option<String>);"),
    ] {
        let expanded = serialized_under(args, declared);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {declared}, got: {expanded}"
        );
    }
}

/// A newtype variant under a tag alone over a type declared above whose value is a generic type
/// declared above filled at a value serde writes as no object, a struct of one slot, a brand, an
/// alias and a chain of them, refused as the slot over the filling itself is, and over one filled
/// at an `Option`, refused as the slot over that `Option` is.
#[cfg(feature = "serde")]
#[test]
fn a_tagged_newtype_variant_over_a_holder_of_a_generic_type_filled_at_no_object_is_refused_in_every_build_with_serde()
 {
    declare_filled_holders();
    let whole = "is not written as an object \u{2014} its schema describes a";
    let text = held_under_tag("ProbeHeldTextFilled", "ProbeTextFilled");
    assert_eq!(
        text,
        [tagged_non_object_refusal(
            "ProbeHeldTextFilled",
            "ProbeTextFilled",
            &format!("{whole} `string`")
        )]
    );
    assert_eq!(
        held_under_tag("ProbeHeldText", "ProbeText"),
        [tagged_non_object_refusal(
            "ProbeHeldText",
            "ProbeText",
            &format!("{whole} `string`")
        )],
        "the slot over the filling itself"
    );
    let answered = format!("error: {}", text[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    for (holder, slot, inner, named) in [
        (
            "ProbeHeldCountFilled",
            "ProbeCountFilled",
            "ProbeCountFilled",
            "integer",
        ),
        (
            "ProbeHeldFlagFilled",
            "ProbeFlagFilled",
            "ProbeFlagFilled",
            "boolean",
        ),
        (
            "ProbeHeldListFilled",
            "Box<ProbeListFilled>",
            "ProbeListFilled",
            "array",
        ),
        (
            "ProbeHeldNamedFilled",
            "ProbeNamedFilled",
            "ProbeNamedFilled",
            "string",
        ),
        (
            "ProbeHeldFilledHolder",
            "ProbeWrap<ProbeTextFilled>",
            "ProbeWrap<ProbeTextFilled>",
            "string",
        ),
    ] {
        assert_eq!(
            held_under_tag(holder, slot),
            [tagged_non_object_refusal(
                holder,
                inner,
                &format!("{whole} `{named}`")
            )],
            "for {slot}"
        );
    }
    assert_eq!(
        held_under_tag("ProbeHeldMaybeFilled", "ProbeMaybeFilled"),
        [tagged_optional_refusal(
            "ProbeHeldMaybeFilled",
            "ProbeMaybeFilled"
        )]
    );
    assert_eq!(
        held_under_tag("ProbeHeldMaybe", "ProbeMaybe"),
        [tagged_optional_refusal("ProbeHeldMaybe", "ProbeMaybe")],
        "the slot over that `Option` itself"
    );
}

/// The refusal of a flattened `inner` of `holder` whose union member at `branch` serde writes as a
/// `named` value.
#[cfg(feature = "serde")]
fn branch_non_object_refusal(holder: &str, inner: &str, branch: &str, named: &str) -> String {
    format!(
        "model_schema: `{holder}`: `#[serde(flatten)]` of `{inner}` writes a union member that is \
         not an object \u{2014} its branch {branch} describes a `{named}`, which has no members to \
         merge, and what serde writes for that member does not join the object being written; \
         write the field as a named member so the value gets a key of its own. {}",
        super::FLATTENED_NON_OBJECT_SCOPE
    )
}

/// The refusals of `holder`, a struct that flattens `flattened` beside a member of its own.
#[cfg(feature = "serde")]
fn flattened_beside_an_id(holder: &str, flattened: &str) -> Vec<String> {
    let written: syn::Type = syn::parse_str(flattened).unwrap();
    let attribute = if get_field_def("wrap", &written, "").stands_for_an_option() {
        "#[serde(default, flatten, skip_serializing_if = \"Option::is_none\")]"
    } else {
        "#[serde(flatten)]"
    };
    refusal_texts(&serialized_under(
        "",
        &format!("pub struct {holder} {{ pub id: String, {attribute} pub wrap: {flattened} }}"),
    ))
}

/// A generic type declared above whose value is its type parameter, flattened into a struct and
/// into a variant's members at a filling serde writes as no object, directly, under `Box`, under
/// an `Option`, through a brand, an alias and a chain, each refused as the type of the same shape
/// no parameter fills is; and a type declared above whose value is one, flattened, refused as the
/// type over the filling itself is.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_generic_type_filled_at_no_object_is_refused_in_every_build_with_serde() {
    declare_filled_holders();
    let text = flattened_beside_an_id("ProbeFlatText", "ProbeWrap<String>");
    assert_eq!(
        text,
        [whole_non_object_refusal(
            "ProbeFlatText",
            "ProbeWrap<String>",
            "string"
        )]
    );
    assert_eq!(
        flattened_beside_an_id("ProbeFlatTextItself", "ProbeText"),
        [whole_non_object_refusal(
            "ProbeFlatTextItself",
            "ProbeText",
            "string"
        )],
        "the type of the same shape no parameter fills"
    );
    let answered = format!("error: {}", text[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
    for (holder, flattened, inner, named) in [
        (
            "ProbeFlatCount",
            "Box<ProbeWrap<u32>>",
            "ProbeWrap<u32>",
            "integer",
        ),
        (
            "ProbeFlatList",
            "ProbeWrap<Vec<String>>",
            "ProbeWrap<Vec<String>>",
            "array",
        ),
        (
            "ProbeFlatBrand",
            "ProbeBoxed<String>",
            "ProbeBoxed<String>",
            "string",
        ),
        (
            "ProbeFlatChain",
            "ProbeOuter<f64>",
            "ProbeOuter<f64>",
            "number",
        ),
        (
            "ProbeFlatAlias",
            "ProbeWrapAlias<bool>",
            "ProbeWrapAlias<bool>",
            "boolean",
        ),
        (
            "ProbeFlatNamed",
            "ProbeWrap<ProbeText>",
            "ProbeWrap<ProbeText>",
            "string",
        ),
        (
            "ProbeFlatOptional",
            "Option<ProbeWrap<String>>",
            "ProbeWrap<String>",
            "string",
        ),
        (
            "ProbeFlatOptionalItself",
            "Option<ProbeText>",
            "ProbeText",
            "string",
        ),
        (
            "ProbeFlatHolder",
            "ProbeTextFilled",
            "ProbeTextFilled",
            "string",
        ),
    ] {
        assert_eq!(
            flattened_beside_an_id(holder, flattened),
            [whole_non_object_refusal(holder, inner, named)],
            "for {flattened}"
        );
    }
    assert_eq!(
        refusal_texts(&serialized_under(
            "",
            "pub enum ProbeFlatLog { Noted { note: String, #[serde(flatten)] wrap: \
             ProbeWrap<String> } }",
        )),
        [whole_non_object_refusal(
            "ProbeFlatLog",
            "ProbeWrap<String>",
            "string"
        )]
    );
}

/// A generic type declared above whose value is its type parameter, flattened at an `Option` of
/// text, and a type declared above whose value is an `Option` of one filled at text, each refused
/// with the branch of the `Option`, as a struct of one slot over `Option<String>` is.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_generic_type_filled_at_an_option_of_no_object_is_refused_in_every_build_with_serde()
{
    declare_filled_holders();
    for (holder, flattened) in [
        ("ProbeFlatMaybeText", "ProbeWrap<Option<String>>"),
        ("ProbeFlatMaybeTextItself", "ProbeMaybeText"),
        ("ProbeFlatMaybeTextHolder", "ProbeMaybeTextFilled"),
    ] {
        assert_eq!(
            flattened_beside_an_id(holder, flattened),
            [branch_non_object_refusal(holder, flattened, "1", "string")],
            "for {flattened}"
        );
    }
}

/// The shapes left alone: a generic type filled at a struct and at an `Option` of one, flattened,
/// and a type whose value is one, flattened and in a tagged slot; a generic type declared below,
/// flattened and filling a type declared above; and a generic type filled at the holder's own
/// type parameter.
#[cfg(feature = "serde")]
#[test]
fn a_generic_type_filled_at_an_object_is_accepted_flattened_and_held() {
    declare_filled_holders();
    for (args, accepted) in [
        (
            "",
            "pub struct ProbeFlatFields { pub id: String, #[serde(flatten)] pub wrap: \
             ProbeWrap<ProbeFields> }",
        ),
        (
            "",
            "pub struct ProbeFlatMaybeFields { pub id: String, #[serde(flatten)] pub wrap: \
             ProbeWrap<Option<ProbeFields>> }",
        ),
        (
            "",
            "pub struct ProbeFlatFieldsHolder { pub id: String, #[serde(flatten)] pub wrap: \
             ProbeFieldsFilled }",
        ),
        (
            "",
            "#[serde(tag = \"kind\")] pub enum ProbeHeldFieldsFilled { Held(ProbeFieldsFilled) }",
        ),
        (
            "",
            "pub struct ProbeFlatLater { pub id: String, #[serde(flatten)] pub wrap: \
             ProbeWrapUnseen<String> }",
        ),
        (
            "",
            "pub struct ProbeFlatLaterHolder { pub id: String, #[serde(flatten)] pub wrap: \
             ProbeLaterFilled }",
        ),
        (
            "",
            "#[serde(tag = \"kind\")] pub enum ProbeHeldLaterFilled { Held(ProbeLaterFilled) }",
        ),
        (
            "default_types(X = ProbeFields)",
            "pub struct ProbeFlatOwn<X> { pub id: String, #[serde(flatten)] pub wrap: \
             ProbeWrap<X> }",
        ),
    ] {
        let expanded = serialized_under(args, accepted);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {accepted}, got: {expanded}"
        );
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_filled_untagged_member_refusals_are_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    for named in [
        "non_object_wire",
        "member_written_leaves",
        "without_its_option",
        "under_an_option",
        "value_filling",
        "written_union_members",
        "written_non_object",
        "flatten_edge_guard_error",
        "tagged_non_object_message",
    ] {
        assert_eq!(
            gates_above(expansion, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
}

/// Declares, beside the types [`declare_filled_probes`] declares, the untagged enums with an object
/// member and a newtype member: over a generic type declared above filled at text, a number, an
/// `Option` and a struct, over one declared below, and over each filling itself.
#[cfg(feature = "serde")]
fn declare_filled_members() {
    declare_filled_probes();
    for declared in [
        "pub enum ProbeTextMember { Obj { a: String }, Text(String) }",
        "pub enum ProbeWrapMember { Obj { a: String }, Text(ProbeWrap<String>) }",
        "pub enum ProbeCountMember { Obj { a: String }, Count(f64) }",
        "pub enum ProbeChainMember { Obj { a: String }, Count(Box<ProbeOuter<f64>>) }",
        "pub enum ProbeMaybeCountMember { Obj { a: String }, Count(Option<u32>) }",
        "pub enum ProbeMaybeWrapMember { Obj { a: String }, Count(ProbeWrap<Option<u32>>) }",
        "pub enum ProbeWrapMaybeMember { Obj { a: String }, Count(Option<ProbeWrap<u32>>) }",
        "pub enum ProbeFieldsMember { Obj { a: String }, Fields(ProbeWrap<ProbeFields>) }",
        "pub enum ProbeLaterMember { Obj { a: String }, Later(ProbeWrapUnseen<String>) }",
    ] {
        let untagged = format!("#[serde(untagged)] {declared}");
        let expanded = serialized_under("", &untagged);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {declared}, got: {expanded}"
        );
    }
}

/// An untagged enum with a member over a generic type declared above filled at text, flattened
/// beside a member and held by a newtype variant under a tag alone, each refused with the branch
/// the enum with the member over text itself is refused with.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_member_over_a_generic_type_filled_at_no_object_is_refused_in_every_build_with_serde()
{
    declare_filled_members();
    let flattened = refusal_texts(&serialized_under(
        "",
        "pub struct ProbeM5 { pub id: String, #[serde(flatten)] pub m: ProbeWrapMember }",
    ));
    assert_eq!(
        flattened,
        [branch_non_object_refusal(
            "ProbeM5",
            "ProbeWrapMember",
            "2",
            "string"
        )]
    );
    assert_eq!(
        flattened_beside_an_id("ProbeFlatTextMember", "ProbeTextMember"),
        [branch_non_object_refusal(
            "ProbeFlatTextMember",
            "ProbeTextMember",
            "2",
            "string"
        )],
        "the enum with the member over text itself"
    );
    let branch = "writes a union member that is not an object \u{2014} its branch 2 describes a \
                  `string`";
    assert_eq!(
        held_under_tag("ProbeM6", "ProbeWrapMember"),
        [tagged_non_object_refusal(
            "ProbeM6",
            "ProbeWrapMember",
            branch
        )]
    );
    assert_eq!(
        held_under_tag("ProbeHeldTextMember", "ProbeTextMember"),
        [tagged_non_object_refusal(
            "ProbeHeldTextMember",
            "ProbeTextMember",
            branch
        )],
        "the enum with the member over text itself"
    );
    for quoted in [
        branch_non_object_refusal("ProbeFlatWrapMember", "ProbeWrapMember", "2", "string"),
        tagged_non_object_refusal("ProbeHeldWrapMember", "ProbeWrapMember", branch),
    ] {
        let answered = format!("error: {quoted}");
        assert!(
            include_str!("../../README.md").contains(&answered),
            "the README no longer quotes: {answered}"
        );
    }
    assert_eq!(
        flattened_beside_an_id("ProbeFlatWrapMember", "ProbeWrapMember"),
        [branch_non_object_refusal(
            "ProbeFlatWrapMember",
            "ProbeWrapMember",
            "2",
            "string"
        )],
        "the README's flattened holder"
    );
    assert_eq!(
        held_under_tag("ProbeHeldWrapMember", "ProbeWrapMember"),
        [tagged_non_object_refusal(
            "ProbeHeldWrapMember",
            "ProbeWrapMember",
            branch
        )],
        "the README's tagged holder"
    );
}

/// An untagged member over a generic type declared above filled at a number through a chain and
/// under `Box`, and at an `Option` of one, flattened and held under a tag alone, each refused with
/// the branch the member over that filling itself is refused with.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_member_over_a_generic_type_filled_through_a_chain_or_at_an_option_is_refused_as_its_filling()
 {
    declare_filled_members();
    for (filled, itself, branch, named) in [
        ("ProbeChainMember", "ProbeCountMember", "2", "number"),
        (
            "ProbeMaybeWrapMember",
            "ProbeMaybeCountMember",
            "2.1",
            "integer",
        ),
        (
            "ProbeWrapMaybeMember",
            "ProbeMaybeCountMember",
            "2.1",
            "integer",
        ),
    ] {
        for held in [filled, itself] {
            let flat = format!("ProbeFlat{held}");
            assert_eq!(
                flattened_beside_an_id(&flat, held),
                [branch_non_object_refusal(&flat, held, branch, named)],
                "for {held} flattened"
            );
            let tagged = format!("ProbeHeld{held}");
            assert_eq!(
                held_under_tag(&tagged, held),
                [tagged_non_object_refusal(
                    &tagged,
                    held,
                    &format!(
                        "writes a union member that is not an object \u{2014} its branch \
                         {branch} describes a `{named}`"
                    )
                )],
                "for {held} under a tag"
            );
        }
    }
}

/// The untagged members left alone: one over a generic type filled at a struct, and one over a
/// generic type declared below, flattened and held under a tag alone.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_member_over_a_generic_type_filled_at_an_object_or_declared_below_is_accepted() {
    declare_filled_members();
    for held in ["ProbeFieldsMember", "ProbeLaterMember"] {
        for accepted in [
            format!(
                "pub struct ProbeFlat{held} {{ pub id: String, #[serde(flatten)] pub m: {held} }}"
            ),
            format!("#[serde(tag = \"kind\")] pub enum ProbeHeld{held} {{ Held({held}) }}"),
        ] {
            let expanded = serialized_under("", &accepted);
            assert_eq!(
                compile_error_count(&expanded),
                0,
                "for {accepted}, got: {expanded}"
            );
        }
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_untagged_parameter_refusals_are_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    let registry = include_str!("../utils.rs");
    for (source, named) in [
        (expansion, "union_parameters_of"),
        (expansion, "member_parameter_leaves"),
        (expansion, "written_union_members"),
        (expansion, "member_written_leaves"),
        (expansion, "union_spelling"),
        (expansion, "written_non_object"),
        (expansion, "flatten_edge_guard_error"),
        (expansion, "tagged_non_object_message"),
        (registry, "union_parameters"),
        (registry, "record_union_parameters"),
    ] {
        assert_eq!(
            gates_above(source, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
}

/// Declares, beside the types [`declare_filled_members`] declares, the generic untagged enums with
/// an object member and a member that is their own type parameter: itself, under `Box`, under an
/// `Option`, in a generic type declared above, forwarded to another such enum, beside a member
/// written as text, and at the second of two parameters; an alias of one filled at text; and the
/// enums and the alias with each such member written over a filling itself.
#[cfg(feature = "serde")]
fn declare_own_members() {
    declare_filled_members();
    for (args, declared) in [
        (
            "default_types(X = String)",
            "pub enum ProbeOwnMember<X> { Obj { a: String }, Held(X) }",
        ),
        (
            "default_types(X = String)",
            "pub enum ProbeOwnBoxed<X> { Obj { a: String }, Held(Box<X>) }",
        ),
        (
            "default_types(X = String)",
            "pub enum ProbeOwnMaybe<X> { Obj { a: String }, Held(Option<X>) }",
        ),
        (
            "default_types(X = String)",
            "pub enum ProbeOwnWrapped<X> { Obj { a: String }, Held(ProbeWrap<X>) }",
        ),
        (
            "default_types(Y = String)",
            "pub enum ProbeOwnForwarded<Y> { Obj { a: String }, Inner(ProbeOwnMember<Y>) }",
        ),
        (
            "default_types(X = String)",
            "pub enum ProbeOwnBeside<X> { Obj { a: String }, Held(X), Text(String) }",
        ),
        (
            "default_types(A = String, B = String)",
            "pub enum ProbeOwnPair<A, B> { Obj { a: String }, First(A), Second(B) }",
        ),
        (
            "",
            "pub enum ProbeNestedTextMember { Obj { a: String }, Inner(ProbeTextMember) }",
        ),
        (
            "",
            "pub enum ProbeBesideCountMember { Obj { a: String }, Held(u32), Text(String) }",
        ),
        (
            "",
            "pub enum ProbeBesideFieldsMember { Obj { a: String }, Held(ProbeFields), Text(String) }",
        ),
        (
            "",
            "pub enum ProbeSecondTextMember { Obj { a: String }, First(ProbeFields), Second(String) }",
        ),
    ] {
        let untagged = format!("#[serde(untagged)] {declared}");
        let expanded = serialized_under(args, &untagged);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {declared}, got: {expanded}"
        );
    }
    for declared in [
        "pub type ProbeOwnAlias = ProbeOwnMember<String>;",
        "pub type ProbeTextAlias = ProbeTextMember;",
    ] {
        let expanded = serialized_under("", declared);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {declared}, got: {expanded}"
        );
    }
}

/// The refusals of `flattened` flattened beside a member, and of `flattened` held by a newtype
/// variant under a tag alone, each holder named after `holder`.
#[cfg(feature = "serde")]
fn flattened_and_held(holder: &str, flattened: &str) -> [Vec<String>; 2] {
    [
        flattened_beside_an_id(&format!("ProbeFlat{holder}"), flattened),
        held_under_tag(&format!("ProbeHeld{holder}"), flattened),
    ]
}

/// The refusals [`flattened_and_held`] answers where the union named `inner` writes its member at
/// `branch` as a `named` value.
#[cfg(feature = "serde")]
fn branch_refusals(holder: &str, inner: &str, branch: &str, named: &str) -> [Vec<String>; 2] {
    [
        vec![branch_non_object_refusal(
            &format!("ProbeFlat{holder}"),
            inner,
            branch,
            named,
        )],
        vec![tagged_non_object_refusal(
            &format!("ProbeHeld{holder}"),
            inner,
            &format!(
                "writes a union member that is not an object \u{2014} its branch {branch} \
                 describes a `{named}`"
            ),
        )],
    ]
}

/// An untagged enum whose member is its own type parameter, filled at text, flattened beside a
/// member and held by a newtype variant under a tag alone, each refused with the branch the enum
/// with the member over text itself is refused with, and named with its argument.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_enum_filled_at_no_object_at_its_own_parameter_is_refused_in_every_build_with_serde()
{
    declare_own_members();
    assert_eq!(
        flattened_and_held("OwnMember", "ProbeOwnMember<String>"),
        branch_refusals("OwnMember", "ProbeOwnMember<String>", "2", "string")
    );
    assert_eq!(
        flattened_and_held("TextMember", "ProbeTextMember"),
        branch_refusals("TextMember", "ProbeTextMember", "2", "string"),
        "the enum with the member over text itself"
    );
    for quoted in branch_refusals("OwnMember", "ProbeOwnMember<String>", "2", "string") {
        let answered = format!("error: {}", quoted[0]);
        assert!(
            include_str!("../../README.md").contains(&answered),
            "the README no longer quotes: {answered}"
        );
    }
}

/// An untagged enum whose member is its own type parameter under `Box`, under an `Option`, in a
/// generic type declared above, forwarded to another such enum, beside a member written as text
/// and at the second of two parameters, and an alias of one, each flattened and held under a tag
/// alone at a filling serde writes as no object, refused with the branch the enum with the member
/// over that filling itself is refused with.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_enum_filled_at_its_own_parameter_through_a_wrapper_a_holder_or_another_enum_is_refused_as_its_filling()
 {
    declare_own_members();
    for (index, (filled, itself, branch, named)) in [
        ("ProbeOwnBoxed<String>", "ProbeTextMember", "2", "string"),
        (
            "ProbeOwnMaybe<u32>",
            "ProbeMaybeCountMember",
            "2.1",
            "integer",
        ),
        ("ProbeOwnWrapped<f64>", "ProbeCountMember", "2", "number"),
        (
            "ProbeOwnForwarded<String>",
            "ProbeNestedTextMember",
            "2.2",
            "string",
        ),
        (
            "ProbeOwnBeside<u32>",
            "ProbeBesideCountMember",
            "2",
            "integer",
        ),
        (
            "ProbeOwnBeside<ProbeFields>",
            "ProbeBesideFieldsMember",
            "3",
            "string",
        ),
        (
            "ProbeOwnPair<ProbeFields, String>",
            "ProbeSecondTextMember",
            "3",
            "string",
        ),
        ("ProbeOwnAlias", "ProbeTextAlias", "2", "string"),
    ]
    .into_iter()
    .enumerate()
    {
        let holder = format!("Own{index}");
        assert_eq!(
            flattened_and_held(&holder, filled),
            branch_refusals(&holder, filled, branch, named),
            "for {filled}"
        );
        let direct = format!("Direct{index}");
        assert_eq!(
            flattened_and_held(&direct, itself),
            branch_refusals(&direct, itself, branch, named),
            "for {itself}, the shape over the filling itself"
        );
    }
}

/// The generic untagged enums left alone: each filled at a struct, filled at a type parameter of
/// the holder, and one declared below, flattened and held under a tag alone.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_enum_filled_at_an_object_at_its_own_parameter_or_declared_below_is_accepted() {
    declare_own_members();
    for (index, filled) in [
        "ProbeOwnMember<ProbeFields>",
        "ProbeOwnBoxed<ProbeFields>",
        "ProbeOwnWrapped<ProbeFields>",
        "ProbeOwnForwarded<ProbeFields>",
        "ProbeOwnPair<ProbeFields, ProbeFields>",
        "ProbeOwnLater<String>",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            flattened_and_held(&format!("Accepted{index}"), filled),
            [Vec::<String>::new(), Vec::new()],
            "for {filled}"
        );
    }
    for accepted in [
        "pub struct ProbeFlatOwnParameter<T> { pub id: String, #[serde(flatten)] pub wrap: \
         ProbeOwnMember<T> }",
        "#[serde(tag = \"kind\")] pub enum ProbeHeldOwnParameter<T> { Held(ProbeOwnMember<T>) }",
    ] {
        let expanded = serialized_under("default_types(T = ProbeFields)", accepted);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {accepted}, got: {expanded}"
        );
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_refusals_of_a_slot_over_an_untagged_enum_are_compiled_in_every_build_with_serde() {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    let registry = include_str!("../utils.rs");
    for (source, named) in [
        (expansion, "non_object_wire"),
        (expansion, "written_as_its_slot"),
        (expansion, "filled_union_members"),
        (expansion, "written_union_members"),
        (expansion, "written_non_object"),
        (expansion, "flatten_edge_guard_error"),
        (expansion, "tagged_non_object_message"),
        (registry, "record_non_object_wire"),
        (registry, "recorded_non_object_wire"),
    ] {
        assert_eq!(
            gates_above(source, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
}

/// Declares, beside the types [`declare_own_members`] declares, the structs of one slot and the
/// brands over an untagged enum: with a member written as text, through a generic type declared
/// above, at the enum's own parameter, with every member an object, and over an enum declared
/// below; and an untagged enum with a member over one of them.
#[cfg(feature = "serde")]
fn declare_slot_members() {
    declare_own_members();
    for (args, declared) in [
        ("no_display", "pub struct ProbeSlotText(ProbeTextMember);"),
        (
            "no_display",
            "#[serde(transparent)] pub struct ProbeBrandText(ProbeTextMember);",
        ),
        (
            "no_display",
            "pub struct ProbeSlotFilled(ProbeWrap<ProbeTextMember>);",
        ),
        (
            "no_display",
            "pub struct ProbeSlotOwn(ProbeOwnMember<String>);",
        ),
        (
            "no_display",
            "pub struct ProbeSlotObjects(ProbeFieldsMember);",
        ),
        (
            "no_display",
            "#[serde(transparent)] pub struct ProbeBrandObjects(ProbeFieldsMember);",
        ),
        ("no_display", "pub struct ProbeSlotBelow(ProbeUnionBelow);"),
        (
            "",
            "#[serde(untagged)] pub enum ProbeNestedSlotMember { Obj { a: String }, \
             Inner(ProbeSlotText) }",
        ),
    ] {
        let expanded = serialized_under(args, declared);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {declared}, got: {expanded}"
        );
    }
}

/// A struct of one slot and a brand over an untagged enum with a member written as text, flattened
/// beside a member and held by a newtype variant under a tag alone, each refused with the branch
/// an alias of the same enum is refused with, and named as written.
#[cfg(feature = "serde")]
#[test]
fn a_slot_over_an_untagged_enum_with_a_member_that_is_no_object_is_refused_in_every_build_with_serde()
 {
    declare_slot_members();
    assert_eq!(
        flattened_and_held("TextAlias", "ProbeTextAlias"),
        branch_refusals("TextAlias", "ProbeTextAlias", "2", "string"),
        "the alias of the same enum"
    );
    for holder in ["SlotText", "BrandText"] {
        let inner = format!("Probe{holder}");
        assert_eq!(
            flattened_and_held(holder, &inner),
            branch_refusals(holder, &inner, "2", "string"),
            "for {inner}"
        );
    }
    for quoted in branch_refusals("SlotText", "ProbeSlotText", "2", "string") {
        let answered = format!("error: {}", quoted[0]);
        assert!(
            include_str!("../../README.md").contains(&answered),
            "the README no longer quotes: {answered}"
        );
    }
}

/// A struct of one slot over an untagged enum through a generic type declared above and at the
/// enum's own parameter, a generic type declared above filled at a struct of one slot over one,
/// and an untagged enum with a member over one, each flattened and held under a tag alone,
/// refused with the branch the enum the slot holds is refused with.
#[cfg(feature = "serde")]
#[test]
fn a_slot_over_an_untagged_enum_through_a_generic_type_or_held_by_one_is_refused_as_the_enum() {
    declare_slot_members();
    for (index, (held, branch)) in [
        ("ProbeSlotFilled", "2"),
        ("ProbeSlotOwn", "2"),
        ("ProbeWrap<ProbeSlotText>", "2"),
        ("ProbeNestedSlotMember", "2.2"),
    ]
    .into_iter()
    .enumerate()
    {
        let holder = format!("Slot{index}");
        assert_eq!(
            flattened_and_held(&holder, held),
            branch_refusals(&holder, held, branch, "string"),
            "for {held}"
        );
    }
}

/// The holders left alone: a struct of one slot and a brand over an untagged enum whose every
/// member is an object, a struct of one slot over an enum declared below, and a holder declared
/// below, flattened and held under a tag alone.
#[cfg(feature = "serde")]
#[test]
fn a_slot_over_an_untagged_enum_of_objects_or_declared_below_is_accepted() {
    declare_slot_members();
    for (index, held) in [
        "ProbeSlotObjects",
        "ProbeBrandObjects",
        "ProbeSlotBelow",
        "ProbeSlotUnseen",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            flattened_and_held(&format!("SlotAccepted{index}"), held),
            [Vec::<String>::new(), Vec::new()],
            "for {held}"
        );
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_refusals_of_a_holder_of_an_untagged_enum_at_its_own_parameter_are_compiled_in_every_build_with_serde()
 {
    let with_serde = quote::quote!(cfg(feature = "serde")).to_string();
    let expansion = include_str!("../model_schema.rs");
    let registry = include_str!("../utils.rs");
    for (source, named) in [
        (expansion, "union_parameters_of"),
        (expansion, "filled_union_parameters"),
        (expansion, "named_union_parameters"),
        (expansion, "member_parameter_leaves"),
        (expansion, "written_union_members"),
        (expansion, "union_spelling"),
        (registry, "union_parameters"),
        (registry, "record_union_parameters"),
    ] {
        assert_eq!(
            gates_above(source, named),
            [with_serde.as_str()],
            "for `{named}`"
        );
    }
}

/// Declares, beside the types [`declare_slot_members`] declares, the generic alias, struct of one
/// slot and brand whose value is an untagged enum filled at their own type parameter: the enum
/// whose member is its own type parameter, through an alias of one, in a generic type declared
/// above, and an enum declared below; and an untagged enum with a member over the alias.
#[cfg(feature = "serde")]
fn declare_own_holders() {
    declare_slot_members();
    for (args, declared) in [
        (
            "default_types(T = String)",
            "pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;",
        ),
        (
            "no_display, default_types(T = String)",
            "pub struct ProbeSlotOwnOf<T>(ProbeOwnMember<T>);",
        ),
        (
            "no_display, default_types(T = String)",
            "#[serde(transparent)] pub struct ProbeBrandOwnOf<T>(ProbeOwnMember<T>);",
        ),
        (
            "default_types(U = String)",
            "pub type ProbeOwnAliasChain<U> = ProbeOwnAliasOf<U>;",
        ),
        (
            "no_display, default_types(T = String)",
            "pub struct ProbeSlotWrapOwnOf<T>(ProbeWrap<ProbeOwnMember<T>>);",
        ),
        (
            "default_types(T = String)",
            "pub type ProbeOwnAliasBelow<T> = ProbeOwnLater<T>;",
        ),
        (
            "default_types(Y = String)",
            "#[serde(untagged)] pub enum ProbeOwnHolderMember<Y> { Obj { a: String }, \
             Inner(ProbeOwnAliasOf<Y>) }",
        ),
    ] {
        let expanded = serialized_under(args, declared);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {declared}, got: {expanded}"
        );
    }
}

/// A generic alias, struct of one slot and brand whose value is an untagged enum filled at their
/// own type parameter, each filled at text, flattened beside a member and held by a newtype
/// variant under a tag alone, refused with the branch the enum filled at text is refused with,
/// and named with their arguments.
#[cfg(feature = "serde")]
#[test]
fn a_holder_of_an_untagged_enum_at_its_own_parameter_filled_at_no_object_is_refused_in_every_build_with_serde()
 {
    declare_own_holders();
    assert_eq!(
        flattened_and_held("OwnMember", "ProbeOwnMember<String>"),
        branch_refusals("OwnMember", "ProbeOwnMember<String>", "2", "string"),
        "the enum filled at text itself"
    );
    for holder in ["OwnAliasOf", "SlotOwnOf", "BrandOwnOf"] {
        let filled = format!("Probe{holder}<String>");
        assert_eq!(
            flattened_and_held(holder, &filled),
            branch_refusals(holder, &filled, "2", "string"),
            "for {filled}"
        );
    }
    for quoted in branch_refusals("OwnAliasOf", "ProbeOwnAliasOf<String>", "2", "string") {
        let answered = format!("error: {}", quoted[0]);
        assert!(
            include_str!("../../README.md").contains(&answered),
            "the README no longer quotes: {answered}"
        );
    }
}

/// An alias of such an alias, a struct of one slot over a generic type declared above filled at
/// the enum, a generic type declared above filled at such a struct, and an untagged enum with a
/// member over such an alias, each filled at text, flattened and held under a tag alone, refused
/// with the branch the shape over the enum filled at text is refused with.
#[cfg(feature = "serde")]
#[test]
fn a_holder_of_an_untagged_enum_at_its_own_parameter_through_another_type_is_refused_as_its_filling()
 {
    declare_own_holders();
    for (index, (filled, itself, branch)) in [
        ("ProbeOwnAliasChain<String>", "ProbeOwnMember<String>", "2"),
        ("ProbeSlotWrapOwnOf<String>", "ProbeOwnMember<String>", "2"),
        (
            "ProbeWrap<ProbeSlotOwnOf<String>>",
            "ProbeWrap<ProbeSlotOwn>",
            "2",
        ),
        (
            "ProbeOwnHolderMember<String>",
            "ProbeNestedTextMember",
            "2.2",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let holder = format!("OwnHolder{index}");
        assert_eq!(
            flattened_and_held(&holder, filled),
            branch_refusals(&holder, filled, branch, "string"),
            "for {filled}"
        );
        let direct = format!("OwnDirect{index}");
        assert_eq!(
            flattened_and_held(&direct, itself),
            branch_refusals(&direct, itself, branch, "string"),
            "for {itself}, the shape over the filling itself"
        );
    }
}

/// The holders left alone: each filled at a struct, a holder over an enum declared below, a
/// holder declared below, and each filled at a type parameter of the type that holds it,
/// flattened and held under a tag alone.
#[cfg(feature = "serde")]
#[test]
fn a_holder_of_an_untagged_enum_at_its_own_parameter_filled_at_an_object_or_declared_below_is_accepted()
 {
    declare_own_holders();
    for (index, filled) in [
        "ProbeOwnAliasOf<ProbeFields>",
        "ProbeSlotOwnOf<ProbeFields>",
        "ProbeBrandOwnOf<ProbeFields>",
        "ProbeOwnAliasChain<ProbeFields>",
        "ProbeSlotWrapOwnOf<ProbeFields>",
        "ProbeOwnHolderMember<ProbeFields>",
        "ProbeOwnAliasBelow<String>",
        "ProbeOwnAliasUnseen<String>",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            flattened_and_held(&format!("OwnHolderAccepted{index}"), filled),
            [Vec::<String>::new(), Vec::new()],
            "for {filled}"
        );
    }
    for accepted in [
        "pub struct ProbeFlatOwnAliasParameter<T> { pub id: String, #[serde(flatten)] pub wrap: \
         ProbeOwnAliasOf<T> }",
        "#[serde(tag = \"kind\")] pub enum ProbeHeldSlotOwnParameter<T> { \
         Held(ProbeSlotOwnOf<T>) }",
    ] {
        let expanded = serialized_under("default_types(T = ProbeFields)", accepted);
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {accepted}, got: {expanded}"
        );
    }
}

/// What the record every build with serde writes proves of each flattened type is what the record
/// a Zod surface writes proves, so the refusal reads the same with a schema surface and without.
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn the_record_every_build_with_serde_writes_proves_what_the_zod_surface_proves() {
    for declared in [
        "#[derive(Serialize, Deserialize)] pub struct Plain { pub a: String }",
        "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct Slug(String);",
        "#[derive(Serialize, Deserialize)] pub struct Flag(bool);",
        "#[derive(Serialize, Deserialize)] pub struct Pair(String, u32);",
        "#[derive(Serialize, Deserialize)] pub struct Bag(Vec<String>);",
        "#[derive(Serialize, Deserialize)] pub struct MaybeCount(Option<i64>);",
        "#[derive(Serialize, Deserialize)] pub struct MaybePlain(Option<Plain>);",
        "#[derive(Serialize, Deserialize)] pub enum Hue { Blue, Red }",
        "#[derive(Serialize, Deserialize)] pub enum Bare { Empty, Held(Plain) }",
        "#[derive(Serialize, Deserialize)] #[serde(tag = \"kind\")] pub enum Tagged { One(Plain) }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum Scalar { Obj(Plain), \
         Text(String) }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum Nullable { Obj(Plain), \
         Maybe(Option<Plain>) }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum Named { Obj(Plain), \
         Slug(Slug), Hue(Hue), Bare(Bare), Bag(Bag), Maybe(MaybePlain) }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum Nested { Inner(Scalar), \
         Obj(Plain) }",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum Objects { First(Plain), \
         Second(Plain) }",
        "pub type SlugAlias = Slug;",
        "pub type ScalarAlias = Scalar;",
        "pub type Ids = Vec<String>;",
    ] {
        let expanded = expansion_over(declared);
        assert_eq!(compile_error_count(&expanded), 0, "got: {expanded}");
    }
    let branch = |at: &str, named| Some(NonObject::Branch(at.to_owned(), named));
    for (flattened, proved) in [
        ("Plain", None),
        ("Slug", Some(NonObject::Whole("string"))),
        ("Flag", Some(NonObject::Whole("boolean"))),
        ("Pair", Some(NonObject::Whole("array"))),
        ("Bag", Some(NonObject::Whole("array"))),
        ("MaybeCount", branch("1", "integer")),
        ("MaybePlain", None),
        ("Hue", None),
        ("Bare", None),
        ("Tagged", None),
        ("Scalar", branch("2", "string")),
        ("Nullable", branch("2.2", "null")),
        ("Named", branch("2", "string")),
        ("Nested", branch("1.2", "string")),
        ("Objects", None),
        ("SlugAlias", Some(NonObject::Whole("string"))),
        ("ScalarAlias", branch("2", "string")),
        ("Ids", Some(NonObject::Whole("array"))),
        ("Option<Slug>", Some(NonObject::Whole("string"))),
        ("Vec<Slug>", None),
        ("Unseen", None),
    ] {
        let inner = get_field_def("_flattened", &syn::parse_str(flattened).unwrap(), "");
        assert_eq!(written_non_object(&inner), proved, "for `{flattened}`");
        assert_eq!(surfaced_non_object(&inner), proved, "for `{flattened}`");
    }
}

/// The struct a build of the clients alone took, each client naming `both_ways`, a key serde
/// neither writes nor reads.
#[cfg(all(
    feature = "serde",
    feature = "dart",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
#[test]
fn a_rename_naming_two_keys_is_refused_where_only_a_client_is_emitted() {
    let source = "pub struct SplitNames { \
                  #[serde(rename(serialize = \"written\", deserialize = \"read_too\"))] \
                  pub both_ways: u32, pub plain: u32 }";
    let expanded = expansion_over(source).to_string();
    assert_eq!(
        expanded.matches("compile_error").count(),
        1,
        "got: {expanded}"
    );
    assert!(
        expanded.contains(
            "model_schema: field `both_ways`: serde `rename` names `written` when serializing \
             and `read_too` when deserializing, so the payload serde writes is not one serde \
             reads. A schema and an emitted client describe one key per member, so neither can \
             be written for the pair."
        ),
        "got: {expanded}"
    );
    assert!(!expanded.contains("dart_definition"), "got: {expanded}");
    let located = rename_refusal_spans(source);
    assert!(
        !located.is_empty() && located.iter().all(|text| text == "rename"),
        "got: {located:?}"
    );
}

/// A map key serde writes as an array is refused where only a client is emitted, in the words the
/// schema surfaces use.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn a_tuple_map_key_is_refused_in_every_build_that_describes_a_type() {
    let expanded =
        expansion_over("pub struct Ledger { pub by_pair: HashMap<(u8, u8), String> }").to_string();
    assert_eq!(
        expanded.matches("compile_error").count(),
        1,
        "got: {expanded}"
    );
    assert!(
        expanded.contains(
            "model_schema: field `by_pair`: a map key must be a value serde writes as a string, \
             which is what a JSON object key is \u{2014} serde writes `(_, _)` as a JSON array, and \
             refuses to serialize a map keyed by one at all"
        ),
        "got: {expanded}"
    );
}

#[cfg(all(
    feature = "serde",
    feature = "mongodb",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn an_object_id_map_key_is_refused_in_every_build_that_describes_a_type() {
    let expanded =
        expansion_over("pub struct Ledger { pub by_id: HashMap<ObjectId, String> }").to_string();
    assert_eq!(
        expanded.matches("compile_error").count(),
        1,
        "got: {expanded}"
    );
    assert!(
        expanded.contains("serde writes `ObjectId` as a JSON object"),
        "got: {expanded}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn nested_list_and_optional_map_keys_are_refused_in_every_build_that_describes_a_type() {
    for (field_type, said) in [
        (
            "HashMap<HashMap<String, u8>, String>",
            "`HashMap<_, _>` as a JSON object",
        ),
        (
            "HashMap<Vec<String>, String>",
            "this key is a sequence of `String`",
        ),
        (
            "HashMap<Option<String>, String>",
            "this key is an `Option<String>`",
        ),
    ] {
        let expanded = expansion_over(&format!("pub struct Ledger {{ pub by_key: {field_type} }}"))
            .to_string();
        assert_eq!(
            expanded.matches("compile_error").count(),
            1,
            "for `{field_type}`, got: {expanded}"
        );
        assert!(
            expanded.contains(said),
            "for `{field_type}`, got: {expanded}"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn a_string_map_key_is_left_alone_in_every_build_that_describes_a_type() {
    let expanded =
        expansion_over("pub struct Ledger { pub by_name: HashMap<String, u8> }").to_string();
    assert_eq!(
        expanded.matches("compile_error").count(),
        0,
        "got: {expanded}"
    );
}

/// The refusals below are the ones the schema surfaces already give, held in a build of the clients
/// alone.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
fn refusal_in_every_build(source: &str, said: &str) {
    let expanded = expansion_over(source).to_string();
    assert_eq!(
        expanded.matches("compile_error").count(),
        1,
        "for `{source}`, got: {expanded}"
    );
    assert!(expanded.contains(said), "for `{source}`, got: {expanded}");
}

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn a_brand_over_an_option_is_refused_in_every_build_that_describes_a_type() {
    refusal_in_every_build(
        "#[serde(transparent)] pub struct Handle(Option<String>);",
        "branded newtype `Handle` wraps an `Option`",
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn a_tuple_slot_dropped_from_one_direction_is_refused_in_every_build_that_describes_a_type() {
    refusal_in_every_build(
        "pub struct Pair(#[serde(skip_serializing)] String, u32);",
        "slot 0 of tuple struct `Pair` carries a serde attribute that drops it from only one of \
         serde's two directions",
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn an_alias_of_a_tuple_keyed_map_is_refused_in_every_build_that_describes_a_type() {
    refusal_in_every_build(
        "pub type Index = HashMap<(u8, u8), String>;",
        "type alias `Index`: a map key must be a value serde writes as a string",
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn a_const_handed_to_a_written_type_is_refused_in_every_build_that_describes_a_type() {
    refusal_in_every_build(
        "pub struct Grid<const N: usize> { pub cells: Cells<N> }",
        "cannot render the const parameter `N` as a type argument",
    );
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_map_key_guard_is_compiled_in_every_build_that_describes_a_type() {
    let describing = quote::quote! {
        any(
            feature = "typescript",
            feature = "zod",
            feature = "jsonschema",
            all(
                feature = "serde",
                any(feature = "dart", feature = "swift", feature = "kotlin")
            )
        )
    };
    let compiled = quote::quote!(cfg(#describing)).to_string();
    let expansion = include_str!("../model_schema.rs");
    for named in [
        "WRITTEN_AS_ARRAY",
        "WRITTEN_AS_NULL",
        "WRITTEN_AS_OBJECT",
        "sequence_wrapper_element",
        "map_key_path",
        "unwritable_key",
        "key_rejection",
        "map_key_element_name",
        "map_key_rejection",
        "non_enum_map_key_message",
        "sequenced_map_key_message",
        "optional_map_key_message",
        "unwritable_map_key_message",
        "map_key_rejection_message",
        "check_map_key",
    ] {
        assert_eq!(
            gates_above(expansion, named),
            [compiled.as_str()],
            "for `{named}`"
        );
    }
}

/// No local build is of the clients alone, so the gates are read off the sources.
#[test]
fn the_brand_slot_alias_and_const_guards_are_compiled_in_every_build_that_describes_a_type() {
    let describing = quote::quote! {
        any(
            feature = "typescript",
            feature = "zod",
            feature = "jsonschema",
            all(
                feature = "serde",
                any(feature = "dart", feature = "swift", feature = "kotlin")
            )
        )
    };
    let compiled = quote::quote!(cfg(#describing)).to_string();
    let expansion = include_str!("../model_schema.rs");
    for named in [
        "branded_guard_errors",
        "branded_option_inner_error",
        "branded_constraint_inner_error",
        "branded_pattern_error",
        "branded_inner_value_surface",
        "branded_undescribable_std_error",
        "branded_map_key_error",
        "held_as_a_list",
        "declared_default_field",
        "declared_default_type_name",
        "declared_default_constraint_message",
        "named_inner_constraint_message",
        "non_string_inner_shape",
        "surface_field_def",
        "check_slot_wire_is_readable",
        "slot_attributes_reach_the_wire",
        "alias_map_key_guard_error",
        "collect_const_arguments",
        "bare_ident",
        "const_parameter_argument_message",
    ] {
        assert_eq!(
            gates_above(expansion, named),
            [compiled.as_str()],
            "for `{named}`"
        );
    }
    let gates = gates_above(expansion, "const_parameter_argument_errors");
    assert_eq!(
        gates,
        [compiled, quote::quote!(cfg(not(#describing))).to_string()],
        "for `const_parameter_argument_errors`"
    );
}

#[cfg(feature = "serde")]
#[test]
fn cfg_attr_without_serde_leaves_an_enum_alone() {
    let errors = enum_cfg_attr_errors(&syn::parse_quote! {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[serde(rename_all = "lowercase")]
        enum Status {
            #[cfg_attr(feature = "serde", doc = "only documented in serde builds")]
            Active,
        }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

/// Runs the field walk the way [`process_field`] does and renders the `cfg_attr` guard failure.
#[cfg(feature = "serde")]
fn field_cfg_attr_error(item: &syn::ItemStruct) -> Option<String> {
    let field = item.fields.iter().next()?;
    let name = mapped_or_default(field.ident.as_ref(), ToString::to_string);
    parse_serde_field_attributes(&field.attrs)
        .cfg_attr_rejection
        .as_ref()
        .map(|rejection| cfg_attr_guard_error(rejection, &field_label(&name)).to_string())
}

#[cfg(feature = "serde")]
#[test]
fn cfg_attr_wrapped_serde_on_a_field_is_rejected() {
    let error = field_cfg_attr_error(&syn::parse_quote! {
        struct Report {
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            note: Option<String>,
        }
    })
    .unwrap();
    assert!(error.contains("compile_error"), "got: {error}");
    assert!(error.contains("field `note`"), "got: {error}");
    assert!(error.contains("#[serde(...)]"), "got: {error}");
}

/// Runs the field walk the way [`process_field`] does and returns the undescribable-std guard's
/// refusal.
fn field_undescribable_std_refusal(item: &syn::ItemStruct) -> Option<syn::Error> {
    let field = item.fields.iter().next()?;
    let name = mapped_or_default(field.ident.as_ref(), ToString::to_string);
    let field_def = get_field_def(&name, &field.ty, "");
    check_undescribable_std_field(field, &field_def, &field_label(&name)).err()
}

/// [`field_undescribable_std_refusal`] rendered as the `compile_error!` tokens it becomes.
fn field_undescribable_std_error(item: &syn::ItemStruct) -> Option<String> {
    field_undescribable_std_refusal(item).map(|err| err.to_compile_error().to_string())
}

/// [`field_undescribable_std_refusal`]'s message, unrendered, so the wording can be pinned exactly
/// rather than through token escaping.
fn field_undescribable_std_message(item: &syn::ItemStruct) -> Option<String> {
    field_undescribable_std_refusal(item).map(|err| err.to_string())
}

#[test]
fn an_os_string_field_is_rejected_by_name() {
    let error = field_undescribable_std_error(&syn::parse_quote! {
        struct Report {
            location: OsString,
        }
    })
    .unwrap();
    assert!(error.contains("compile_error"), "got: {error}");
    assert!(error.contains("field `location`"), "got: {error}");
    assert!(error.contains("`OsString`"), "got: {error}");
    assert!(error.contains("externally tagged enum"), "got: {error}");
}

#[test]
fn a_wrapped_os_str_field_is_rejected_by_its_own_name() {
    let error = field_undescribable_std_error(&syn::parse_quote! {
        struct Report {
            location: Box<OsStr>,
        }
    })
    .unwrap();
    assert!(error.contains("`OsStr`"), "got: {error}");
}

/// The path types the borrowed-form rule takes in are the ones this guard must not catch.
#[test]
fn a_path_field_is_left_alone() {
    for ty in [
        quote::quote! { PathBuf },
        quote::quote! { Box<Path> },
        quote::quote! { Cow<'static, Path> },
        quote::quote! { String },
    ] {
        let error = field_undescribable_std_error(&syn::parse_quote! {
            struct Report {
                location: #ty,
            }
        });
        assert!(error.is_none(), "for {ty}, got: {error:?}");
    }
}

#[test]
fn a_once_lock_field_is_rejected_by_name() {
    let error = field_undescribable_std_error(&syn::parse_quote! {
        struct Probe {
            guarded: OnceLock<u32>,
        }
    })
    .unwrap();
    assert!(error.contains("compile_error"), "got: {error}");
    assert!(error.contains("field `guarded`"), "got: {error}");
    assert!(error.contains("`OnceLock`"), "got: {error}");
    assert!(error.contains("Serialize"), "got: {error}");
}

#[test]
fn a_wrapped_once_lock_field_is_rejected_by_its_own_name() {
    let error = field_undescribable_std_error(&syn::parse_quote! {
        struct Probe {
            guarded: Vec<Option<OnceLock<u32>>>,
        }
    })
    .unwrap();
    assert!(error.contains("`OnceLock`"), "got: {error}");
    assert!(!error.contains("`Vec`"), "got: {error}");
}

#[test]
fn a_lifetime_parameterized_guard_field_is_rejected_by_name() {
    for (spelling, expected) in [
        (quote::quote! { MutexGuard<'a, u32> }, "`MutexGuard`"),
        (quote::quote! { Ref<'a, u32> }, "`Ref`"),
        (
            quote::quote! { RwLockReadGuard<'a, u32> },
            "`RwLockReadGuard`",
        ),
    ] {
        let error = field_undescribable_std_error(&syn::parse_quote! {
            struct Probe {
                guarded: #spelling,
            }
        })
        .unwrap();
        assert!(error.contains(expected), "for {spelling}, got: {error}");
    }
}

#[test]
fn a_schematizable_field_is_left_alone_by_the_std_wrapper_guard() {
    for ty in [
        quote::quote! { Inner },
        quote::quote! { Box<u32> },
        quote::quote! { RefCell<u32> },
        quote::quote! { Arc<Inner> },
        quote::quote! { String },
    ] {
        let error = field_undescribable_std_error(&syn::parse_quote! {
            struct Probe {
                guarded: #ty,
            }
        });
        assert!(error.is_none(), "for {ty}, got: {error:?}");
    }
}

#[test]
fn the_field_refusal_for_a_platform_string_reads_exactly_this() {
    let message = field_undescribable_std_message(&syn::parse_quote! {
        struct Report {
            location: OsString,
        }
    })
    .unwrap();
    assert_eq!(
        message,
        "model_schema: field `location` reaches `OsString`, which serde writes as an externally \
         tagged enum naming the target platform (`{\"Unix\":[u8, ...]}` or \
         `{\"Windows\":[u16, ...]}`), not a string, so no schema can describe it portably. Use \
         `String`, or `PathBuf` for a filesystem path."
    );
}

#[test]
fn the_field_refusal_for_an_unsupported_wrapper_reads_exactly_this() {
    let message = field_undescribable_std_message(&syn::parse_quote! {
        struct Probe {
            guarded: OnceLock<u32>,
        }
    })
    .unwrap();
    assert_eq!(
        message,
        "model_schema: field `guarded` reaches `OnceLock`, which serde implements neither \
         `Serialize` nor `Deserialize` for, so there is no wire form for a schema to describe. \
         Store the value `OnceLock` holds directly, or leave this field out of the serialized \
         shape."
    );
}

#[test]
fn the_field_refusal_for_a_linked_list_reads_exactly_this() {
    let message = field_undescribable_std_message(&syn::parse_quote! {
        struct Queue {
            pending: LinkedList<String>,
        }
    })
    .unwrap();
    assert_eq!(
        message,
        "model_schema: field `pending` reaches `LinkedList`, which serde writes as the same JSON \
         array `Vec<T>` writes, so nothing on the wire tells the two apart and this crate \
         describes only the one spelling. Use `Vec<T>`, or `VecDeque<T>` where values are pushed \
         at both ends."
    );
}

#[test]
fn the_covered_sequence_spellings_are_left_alone_by_the_std_wrapper_guard() {
    for spelling in [
        quote::quote! { Vec<String> },
        quote::quote! { VecDeque<String> },
        quote::quote! { HashSet<String> },
        quote::quote! { BTreeSet<String> },
        quote::quote! { BinaryHeap<String> },
    ] {
        let error = field_undescribable_std_error(&syn::parse_quote! {
            struct Queue {
                pending: #spelling,
            }
        });
        assert!(error.is_none(), "for {spelling}, got: {error:?}");
    }
}

#[test]
fn a_wrapped_linked_list_field_is_rejected_by_its_own_name() {
    for spelling in [
        quote::quote! { Vec<LinkedList<String>> },
        quote::quote! { Option<LinkedList<String>> },
        quote::quote! { HashMap<String, LinkedList<String>> },
        quote::quote! { (u8, LinkedList<String>) },
    ] {
        let error = field_undescribable_std_error(&syn::parse_quote! {
            struct Queue {
                pending: #spelling,
            }
        })
        .unwrap();
        assert!(
            error.contains("`LinkedList`"),
            "for {spelling}, got: {error}"
        );
    }
}

#[test]
fn the_slot_refusal_still_carries_the_label_a_slot_is_named_by() {
    let message = field_undescribable_std_message(&syn::parse_quote! {
        struct Probe(pub OnceLock<u32>);
    })
    .unwrap();
    assert!(
        message.starts_with("model_schema: tuple field reaches `OnceLock`,"),
        "got: {message}"
    );
}

#[test]
fn a_field_reaching_both_is_named_by_its_platform_string() {
    let message = field_undescribable_std_message(&syn::parse_quote! {
        struct Probe {
            held: (OnceLock<u32>, OsString),
        }
    })
    .unwrap();
    assert!(message.contains("`OsString`"), "got: {message}");
    assert!(message.contains("externally tagged"), "got: {message}");
}

/// The refusals [`super::any_json_rejections`] answers for `source` under
/// `#[model_schema(args)]`, each as its message beside the text it is spanned on.
fn any_json_refusals(args: &str, source: &str) -> Vec<(String, Option<String>)> {
    let item: syn::Item = syn::parse_str(source).unwrap();
    let parsed = super::parse_model_schema_args(syn::parse_str(args).unwrap());
    assert!(parsed.arg_rejection.is_none(), "for {args}");
    super::any_json_rejections(&item, &parsed)
        .iter()
        .map(|refusal| (refusal.to_string(), refusal.span().source_text()))
        .collect()
}

/// The sentence a refusal of a JSON value states after its subject.
fn any_json_sentence(subject: &str, name: &str) -> String {
    format!(
        "model_schema: {subject} reaches `serde_json::{name}`, which holds any JSON value, so no \
         target can describe it. Declare the shape that value takes as a type of its own, and \
         write that type in its place."
    )
}

#[test]
fn a_field_typed_a_json_value_is_refused_naming_the_field() {
    assert_eq!(
        any_json_refusals(
            "",
            "pub struct Holding { pub count: u8, pub v: serde_json::Value }"
        ),
        [(
            any_json_sentence("type `Holding`: field `v`", "Value"),
            Some("serde_json::Value".to_owned())
        )]
    );
}

/// The full path is refused with or without a leading `::`, `Map` at any arguments.
#[test]
fn a_json_value_written_by_its_full_path_is_refused_in_each_spelling() {
    for (written, name) in [
        ("serde_json::Value", "Value"),
        ("::serde_json::Value", "Value"),
        ("serde_json::Map<String, serde_json::Value>", "Map"),
        ("::serde_json::Map<String, u8>", "Map"),
    ] {
        assert_eq!(
            any_json_refusals("", &format!("pub struct Holding {{ pub v: {written} }}")),
            [(
                any_json_sentence("type `Holding`: field `v`", name),
                Some(written.to_owned())
            )],
            "for {written}"
        );
    }
}

#[test]
fn a_json_value_is_refused_at_any_depth_of_a_field() {
    for written in [
        "Option<Vec<HashMap<String, serde_json::Value>>>",
        "(u32, serde_json::Value)",
        "Box<::serde_json::Value>",
        "[serde_json::Value; 2]",
        "&'static [serde_json::Value]",
        "Vec<Option<serde_json::Value>>",
        "BTreeMap<String, serde_json::Value>",
        "Wrapper<serde_json::Value>",
        "Result<u8, serde_json::Value>",
    ] {
        let refusals = any_json_refusals("", &format!("pub struct Holding {{ pub v: {written} }}"));
        assert_eq!(
            refusals,
            [(
                any_json_sentence("type `Holding`: field `v`", "Value"),
                Some(written.to_owned())
            )],
            "for {written}"
        );
    }
}

#[test]
fn every_member_of_an_enum_that_holds_a_json_value_is_refused_by_its_place() {
    let refusals = any_json_refusals(
        "",
        "pub enum Holding { Named { kept: u8, v: serde_json::Value }, Lone(serde_json::Value), \
         Pair(u8, serde_json::Map<String, u8>), Empty }",
    );
    assert_eq!(
        refusals,
        [
            (
                any_json_sentence("type `Holding`: field `v` of variant `Named`", "Value"),
                Some("serde_json::Value".to_owned())
            ),
            (
                any_json_sentence("type `Holding`: slot 0 of variant `Lone`", "Value"),
                Some("serde_json::Value".to_owned())
            ),
            (
                any_json_sentence("type `Holding`: slot 1 of variant `Pair`", "Map"),
                Some("serde_json::Map<String, u8>".to_owned())
            ),
        ]
    );
}

#[test]
fn a_slot_a_brand_and_a_flattened_member_holding_a_json_value_are_refused() {
    for (source, subject, spanned) in [
        (
            "pub struct Holding(pub u8, pub serde_json::Value);",
            "type `Holding`: slot 1",
            "serde_json::Value",
        ),
        (
            "#[serde(transparent)] pub struct Holding(pub serde_json::Value);",
            "type `Holding`: slot 0",
            "serde_json::Value",
        ),
        (
            "#[serde(transparent)] pub struct Holding { pub raw: serde_json::Value }",
            "type `Holding`: field `raw`",
            "serde_json::Value",
        ),
        (
            "pub struct Holding { #[serde(flatten)] pub rest: HashMap<String, serde_json::Value> }",
            "type `Holding`: field `rest`",
            "HashMap<String, serde_json::Value>",
        ),
    ] {
        assert_eq!(
            any_json_refusals("", source),
            [(
                any_json_sentence(subject, "Value"),
                Some(spanned.to_owned())
            )],
            "for {source}"
        );
    }
    assert_eq!(
        any_json_refusals(
            "",
            "pub struct Holding { #[serde(flatten)] pub rest: serde_json::Map<String, Value> }"
        ),
        [(
            any_json_sentence("type `Holding`: field `rest`", "Map"),
            Some("serde_json::Map<String, Value>".to_owned())
        )]
    );
}

#[test]
fn an_alias_and_a_default_filling_of_a_json_value_are_refused() {
    assert_eq!(
        any_json_refusals("", "pub type Raw = serde_json::Value;"),
        [(
            any_json_sentence("type `Raw`: the type it names", "Value"),
            Some("serde_json::Value".to_owned())
        )]
    );
    assert_eq!(
        any_json_refusals("", "pub type Raw = Vec<serde_json::Map<String, u8>>;"),
        [(
            any_json_sentence("type `Raw`: the type it names", "Map"),
            Some("Vec<serde_json::Map<String, u8>>".to_owned())
        )]
    );
    assert_eq!(
        any_json_refusals(
            "default_types(HeldType = serde_json::Value)",
            "pub struct Probe<HeldType> { pub held: HeldType }"
        ),
        [(
            any_json_sentence("type `Probe`: `default_types` entry `HeldType`", "Value"),
            Some("serde_json::Value".to_owned())
        )]
    );
}

/// A bare name is no refusal at expansion: the compiler is asked whether it is `serde_json`'s.
#[test]
fn a_bare_value_or_map_is_refused_nowhere_at_expansion() {
    for source in [
        "pub struct Holding { pub v: Value }",
        "pub struct Holding { pub v: Option<Vec<HashMap<String, Value>>> }",
        "pub struct Holding { pub v: Map<String, Value> }",
        "pub enum Holding { Lone(Value), Pair(u8, Map<String, u8>) }",
        "pub type Raw = Vec<Map<String, u8>>;",
    ] {
        assert_eq!(any_json_refusals("", source), [], "for {source}");
    }
}

/// A member serde neither writes nor reads, and every member of a variant it neither writes nor
/// reads, is on no surface: neither the full path nor a bare name of a JSON value is refused or
/// checked there. Written one way, the member is refused as before.
#[test]
fn a_json_value_serde_neither_writes_nor_reads_is_refused_nowhere() {
    for source in [
        "pub struct Holding { pub note: String, #[serde(skip)] pub v: serde_json::Value }",
        "pub struct Holding { #[serde(skip_serializing, skip_deserializing)] pub v: Value }",
        "pub struct Holding(String, #[serde(skip)] serde_json::Map<String, u8>);",
        "pub enum Holding { Open { note: String }, #[serde(skip)] Closed { v: serde_json::Value }, \
         #[serde(skip)] Lone(Value) }",
    ] {
        assert_eq!(any_json_refusals("", source), [], "for {source}");
        assert_eq!(
            any_json_checked("", source),
            Vec::<String>::new(),
            "for {source}"
        );
    }
    assert_eq!(
        any_json_refusals(
            "",
            "pub struct Holding { #[serde(skip_deserializing)] pub v: serde_json::Value }"
        ),
        [(
            any_json_sentence("type `Holding`: field `v`", "Value"),
            Some("serde_json::Value".to_owned())
        )]
    );
}

/// The checks [`super::any_json_checks`] writes beside `source` under `#[model_schema(args)]`.
fn any_json_checked(args: &str, source: &str) -> Vec<String> {
    let item: syn::Item = syn::parse_str(source).unwrap();
    let parsed = super::parse_model_schema_args(syn::parse_str(args).unwrap());
    assert!(parsed.arg_rejection.is_none(), "for {args}");
    super::any_json_checks(&item, &parsed)
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The check of `written`, the bare `name` it writes, that fails with the refusal of `subject`.
fn any_json_check_of(name: &str, written: &str, subject: &str) -> String {
    super::ranges::any_json_check(
        name,
        &syn::parse_str(written).unwrap(),
        &any_json_sentence(subject, name),
    )
    .to_string()
}

/// The check the compiler answers for a bare `Value`: an inherent const of the probe answers only
/// for `serde_json`'s own type, and every path it writes starts at `::`.
#[test]
fn a_bare_value_is_checked_by_a_probe_the_compiler_evaluates() {
    let message = any_json_sentence("type `Mapping`: the type it names", "Value");
    assert_eq!(
        any_json_checked("", "pub type Mapping = IndexMap<String, Value>;"),
        [quote::quote! {
            const _: () = {
                struct Probe<T: ?::core::marker::Sized>(::core::marker::PhantomData<T>);
                trait Fallback {
                    const ANY_JSON: bool = false;
                }
                impl<T: ?::core::marker::Sized> Fallback for Probe<T> {}
                impl Probe<::serde_json::Value> {
                    const ANY_JSON: bool = true;
                }
                ::core::assert!(
                    Probe::<::serde_json::Value>::ANY_JSON,
                    "the probe answers for serde_json's own type"
                );
                ::core::assert!(!Probe::<Value>::ANY_JSON, #message);
            };
        }
        .to_string()]
    );
}

/// A bare `Map` is held to `serde_json`'s map of JSON values, and the probe is written as it is
/// written, with its arguments.
#[test]
fn a_bare_map_is_checked_with_its_arguments() {
    let checked = any_json_checked("", "pub struct Holding { pub v: Map<String, u8> }");
    assert_eq!(
        checked,
        [any_json_check_of(
            "Map",
            "Map<String, u8>",
            "type `Holding`: field `v`"
        )]
    );
    assert!(
        checked[0].contains(
            ":: serde_json :: Map < :: std :: string :: String , :: serde_json :: Value >"
        ),
        "got: {checked:?}"
    );
}

/// Each bare name in each position is checked once per spelling, under the position's subject:
/// a member, a variant's member, an alias's target and a `default_types` filling.
#[test]
fn every_position_that_writes_a_bare_name_is_checked() {
    assert_eq!(
        any_json_checked(
            "",
            "pub struct Holding { pub v: (Value, Vec<Value>), pub w: Option<Map<String, Value>> }"
        ),
        [
            any_json_check_of("Value", "Value", "type `Holding`: field `v`"),
            any_json_check_of("Map", "Map<String, Value>", "type `Holding`: field `w`"),
            any_json_check_of("Value", "Value", "type `Holding`: field `w`"),
        ]
    );
    assert_eq!(
        any_json_checked(
            "",
            "pub enum Holding { Named { v: Value }, Lone(Value), Empty }"
        ),
        [
            any_json_check_of(
                "Value",
                "Value",
                "type `Holding`: field `v` of variant `Named`"
            ),
            any_json_check_of("Value", "Value", "type `Holding`: slot 0 of variant `Lone`"),
        ]
    );
    assert_eq!(
        any_json_checked(
            "default_types(HeldType = Value)",
            "pub struct Probe<HeldType> { pub held: HeldType }"
        ),
        [any_json_check_of(
            "Value",
            "Value",
            "type `Probe`: `default_types` entry `HeldType`"
        )]
    );
}

/// The probe's own names are numbered where the written type names them, so it never hides a
/// type of the author's.
#[test]
fn the_probe_takes_names_the_written_type_does_not() {
    let checked = any_json_checked("", "pub struct Holding { pub v: Map<Probe, Fallback> }");
    assert_eq!(checked.len(), 1, "got: {checked:?}");
    assert!(
        checked[0].contains("struct Probe2 <") && checked[0].contains("trait Fallback2 {"),
        "got: {checked:?}"
    );
}

/// A parameter, the item itself and a type the crate declared are what their names name, and a
/// name with arguments `serde_json`'s does not take is another type: none is refused, and none is
/// checked.
#[test]
fn a_name_the_item_or_the_crate_declares_is_no_json_value() {
    for source in [
        "pub struct Pair<Key, Value> { pub key: Key, pub value: Value }",
        "pub struct Value { pub children: Vec<Value> }",
        "pub enum Map { Leaf(u8), Nested(Box<Map>) }",
        "pub struct Holding { pub count: u8, pub ratio: f64 }",
        "pub struct Holding<T> { pub v: Value<T> }",
        "pub struct Holding { pub v: Map }",
        "pub struct Holding { pub v: <Raw as Read>::Value }",
        "pub struct Holding { pub v: crate::Value }",
    ] {
        assert_eq!(any_json_refusals("", source), [], "for {source}");
        assert_eq!(
            any_json_checked("", source),
            Vec::<String>::new(),
            "for {source}"
        );
    }
    assert!(super::claim_published_name("Value", "Value").is_none());
    for source in [
        "pub struct Holding { pub v: Option<Value> }",
        "pub type Mapping = IndexMap<String, Value>;",
    ] {
        assert_eq!(any_json_refusals("", source), [], "for {source}");
        assert_eq!(
            any_json_checked("", source),
            Vec::<String>::new(),
            "for {source}"
        );
    }
}

/// A `Map` whose arguments name what a check beside the item cannot, one of the item's own
/// parameters, `Self` or a lifetime, is not checked; a bare `Value` among them still is.
#[test]
fn a_map_written_with_the_items_own_parameters_is_not_checked() {
    for source in [
        "pub struct Holding<T> { pub v: Map<String, T> }",
        "pub struct Holding<'a> { pub v: Map<&'a str, u8> }",
        "pub struct Holding<const N: usize> { pub v: Map<String, [u8; N]> }",
        "pub struct Holding { pub v: Map<String, Box<Self>> }",
    ] {
        assert_eq!(
            any_json_checked("", source),
            Vec::<String>::new(),
            "for {source}"
        );
    }
    assert_eq!(
        any_json_checked("", "pub struct Holding<T> { pub v: Map<T, Value> }"),
        [any_json_check_of(
            "Value",
            "Value",
            "type `Holding`: field `v`"
        )]
    );
    assert_eq!(
        any_json_checked("", "pub struct Holding { pub v: Map<&'static str, u8> }"),
        [any_json_check_of(
            "Map",
            "Map<&'static str, u8>",
            "type `Holding`: field `v`"
        )]
    );
}

/// The expansion of an item that writes a bare name holds the check beside the item, and no
/// refusal.
#[test]
fn an_item_with_a_bare_value_expands_with_its_check_and_no_refusal() {
    let expanded = super::exec_model_schema(
        proc_macro2::TokenStream::new(),
        syn::parse_str("pub struct Holding { pub v: Value, pub w: Map<String, Value> }").unwrap(),
    );
    let written = expanded.to_string();
    assert_eq!(
        identifiers_named(expanded, "compile_error"),
        0,
        "got: {written}"
    );
    for check in [
        any_json_check_of("Value", "Value", "type `Holding`: field `v`"),
        any_json_check_of("Map", "Map<String, Value>", "type `Holding`: field `w`"),
        any_json_check_of("Value", "Value", "type `Holding`: field `w`"),
    ] {
        assert!(written.contains(&check), "no {check} in: {written}");
    }
}

/// The README quotes the refusal as the build answers it: in its own section and in the list of
/// what an upgrade asks of a crate.
#[test]
fn the_readme_states_the_refusal_of_a_json_value_as_the_build_answers_it() {
    let refusals = any_json_refusals("", "pub struct Holding { pub v: serde_json::Value }");
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    let answered = format!("error: {}", refusals[0].0);
    assert_eq!(
        include_str!("../../README.md").matches(&answered).count(),
        2,
        "the README no longer quotes: {answered}"
    );
}

/// The item is refused ahead of every shape, with one refusal for each position and nothing of
/// the item's own expansion beside them.
#[test]
fn an_item_holding_a_json_value_expands_to_its_refusals_alone() {
    let expanded = super::exec_model_schema(
        proc_macro2::TokenStream::new(),
        syn::parse_str(
            "pub struct Holding { pub v: serde_json::Value, pub w: Vec<::serde_json::Value> }",
        )
        .unwrap(),
    );
    let written = expanded.to_string();
    assert_eq!(
        identifiers_named(expanded, "compile_error"),
        2,
        "got: {written}"
    );
    assert!(!written.contains("impl Holding"), "got: {written}");
}

/// The refusals [`super::undescribable_primitive_rejections`] answers for `source`, each as its
/// message beside the text it is spanned on.
fn undescribable_primitive_refusals(source: &str) -> Vec<(String, Option<String>)> {
    let item: syn::Item = syn::parse_str(source).unwrap();
    super::undescribable_primitive_rejections(&item)
        .iter()
        .map(|refusal| (refusal.to_string(), refusal.span().source_text()))
        .collect()
}

/// The refusal of `primitive` reached at `subject` of the type `Wide`.
fn undescribable_primitive_sentence(subject: &str, primitive: &str) -> String {
    let instead = if primitive.starts_with('f') {
        "Hold the value as an `f32` or an `f64`."
    } else {
        "Hold the value as an `i64` or a `u64`, or as a `#[model_schema]` brand over `String` that \
         carries its digits."
    };
    format!(
        "model_schema: type `Wide`: {subject} reaches `{primitive}`, which no surface describes: \
         no TypeScript, Zod, JSON Schema, Dart, Swift or Kotlin type is written for it. {instead}"
    )
}

#[test]
fn each_undescribable_primitive_is_refused_at_any_depth_of_a_member() {
    for primitive in ["i128", "u128", "f16", "f128"] {
        for written in [
            primitive.to_owned(),
            format!("Option<{primitive}>"),
            format!("Vec<{primitive}>"),
            format!("HashMap<String, {primitive}>"),
            format!("BTreeMap<{primitive}, u8>"),
            format!("(u8, {primitive})"),
            format!("[{primitive}; 2]"),
            format!("Box<{primitive}>"),
            format!("Wrapper<{primitive}>"),
            format!("::core::primitive::{primitive}"),
            format!("std::primitive::{primitive}"),
        ] {
            assert_eq!(
                undescribable_primitive_refusals(&format!(
                    "pub struct Wide {{ pub count: u8, pub x: {written} }}"
                )),
                [(
                    undescribable_primitive_sentence("field `x`", primitive),
                    Some(written.clone())
                )],
                "for {written}"
            );
        }
    }
}

#[test]
fn an_undescribable_primitive_is_refused_in_every_member_of_an_enum_by_its_place() {
    let refusals = undescribable_primitive_refusals(
        "#[serde(tag = \"kind\")] pub enum Wide { Held(i128), Named { kept: u8, x: Option<u128> \
         }, Empty }",
    );
    assert_eq!(
        refusals,
        [
            (
                undescribable_primitive_sentence("slot 0 of variant `Held`", "i128"),
                Some("i128".to_owned())
            ),
            (
                undescribable_primitive_sentence("field `x` of variant `Named`", "u128"),
                Some("Option<u128>".to_owned())
            ),
        ]
    );
}

#[test]
fn an_alias_of_an_undescribable_primitive_is_refused() {
    assert_eq!(
        undescribable_primitive_refusals("pub type Wide = Vec<f128>;"),
        [(
            undescribable_primitive_sentence("the type it names", "f128"),
            Some("Vec<f128>".to_owned())
        )]
    );
}

/// A member serde neither writes nor reads, and a name the item declares as a type parameter, are
/// not refused.
#[test]
fn an_undescribable_primitive_off_the_wire_or_named_by_a_parameter_is_not_refused() {
    assert_eq!(
        undescribable_primitive_refusals("pub struct Wide { #[serde(skip)] pub x: i128 }"),
        []
    );
    assert_eq!(
        undescribable_primitive_refusals("pub struct Wide<u128> { pub x: u128 }"),
        []
    );
}

/// The item expands to the refusal alone: no reference to a module named after the primitive,
/// which nothing publishes.
#[test]
fn an_item_holding_an_undescribable_primitive_expands_to_its_refusal_alone() {
    let expanded = super::exec_model_schema(
        proc_macro2::TokenStream::new(),
        syn::parse_str("pub struct FieldWide { pub x: i128 }").unwrap(),
    );
    let written = expanded.to_string();
    assert_eq!(
        identifiers_named(expanded, "compile_error"),
        1,
        "got: {written}"
    );
    assert!(!written.contains("i128_schema"), "got: {written}");
    assert!(!written.contains("impl FieldWide"), "got: {written}");
}

/// The README quotes the refusal as the build answers it.
#[test]
fn the_readme_states_the_refusal_of_an_undescribable_primitive_as_the_build_answers_it() {
    let item: syn::Item = syn::parse_str("pub struct FieldWide { pub x: i128 }").unwrap();
    let refusals = super::undescribable_primitive_rejections(&item);
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    let answered = format!("error: {}", refusals[0]);
    assert!(
        include_str!("../../README.md").contains(&answered),
        "the README no longer quotes: {answered}"
    );
}

/// Runs the field walk the way [`super::process_field`] does and renders the map-key guard failure
/// the field's written type earns, or the empty string when it earns none.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn field_map_key_error(field_type: &proc_macro2::TokenStream) -> String {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            counts: #field_type,
        }
    };
    let field = item.fields.iter().next().unwrap();
    let field_def = get_field_def("counts", &field.ty, "");
    check_map_key(field, &field_def, &field_label("counts"))
        .err()
        .map_or_else(String::new, |err| err.to_compile_error().to_string())
}

/// A `u64` or `usize` field is refused for Swift; `i64` earns nothing.
#[cfg(feature = "swift")]
#[test]
fn a_u64_or_usize_field_is_refused_for_swift() {
    let item: syn::Item = syn::parse_quote! {
        struct Report {
            count: u64,
        }
    };
    let tokens = swift_width_refusals(&item)
        .into_iter()
        .map(|error| error.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(tokens.contains("compile_error"), "got: {tokens}");
    assert!(
        tokens.contains(
            "model_schema: field `count`: `u64` has no Swift mapping; the Swift target refuses \
             unsigned 64-bit and pointer-sized integers"
        ),
        "got: {tokens}"
    );

    let usize_item: syn::Item = syn::parse_quote! {
        struct Report {
            total: usize,
        }
    };
    let usize_tokens = swift_width_refusals(&usize_item)
        .into_iter()
        .map(|error| error.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        usize_tokens.contains(
            "field `total`: `usize` has no Swift mapping; the Swift target refuses unsigned \
             64-bit and pointer-sized integers"
        ),
        "got: {usize_tokens}"
    );

    let i64_item: syn::Item = syn::parse_quote! {
        struct Report {
            count: i64,
        }
    };
    assert!(swift_width_refusals(&i64_item).is_empty());
}

/// No Swift type declares a member serde neither writes nor reads, nor a member of a variant it
/// neither writes nor reads, so the width guard reads past both; a member under one one-way skip
/// is written or read, and is refused.
#[cfg(feature = "swift")]
#[test]
fn the_swift_width_guard_reads_past_what_serde_neither_writes_nor_reads() {
    let unwired: [syn::Item; 4] = [
        syn::parse_quote! { struct Report { note: String, #[serde(skip)] count: u64 } },
        syn::parse_quote! {
            struct Report { #[serde(skip_serializing, skip_deserializing)] total: usize }
        },
        syn::parse_quote! { struct Report(String, #[serde(skip)] u64); },
        syn::parse_quote! {
            enum Report { Open { note: String }, #[serde(skip)] Closed { count: u64 } }
        },
    ];
    for item in &unwired {
        let refusals = swift_width_refusals(item);
        assert!(refusals.is_empty(), "got: {refusals:?}");
    }
    let one_way: [syn::Item; 3] = [
        syn::parse_quote! { struct Report { #[serde(skip_serializing)] count: u64 } },
        syn::parse_quote! { struct Report { #[serde(skip_deserializing)] count: u64 } },
        syn::parse_quote! {
            enum Report { Open { note: String }, #[serde(skip_serializing)] Closed { count: u64 } }
        },
    ];
    for item in &one_way {
        let refusals = swift_width_refusals(item);
        assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_map_key_proved_to_lack_enum_members_is_refused_wherever_it_is_written() {
    register_alias_info("Doc", "Doc", "doc_schema", AliasKind::NoEnumMembers);
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for field_type in [
        quote::quote! { HashMap<Doc, u32> },
        quote::quote! { HashMap<String, HashMap<Doc, u32>> },
        quote::quote! { HashMap<Slot, HashMap<Doc, u32>> },
        quote::quote! { Vec<HashMap<Doc, u32>> },
        quote::quote! { Option<HashMap<Doc, u32>> },
        quote::quote! { (String, HashMap<Doc, u32>) },
        quote::quote! { Wrapper<HashMap<Doc, u32>> },
        quote::quote! { HashMap<Doc, HashMap<Doc, u32>> },
    ] {
        let error = field_map_key_error(&field_type);
        assert!(error.contains("compile_error"), "for {field_type}: {error}");
        assert!(
            error.contains("model_schema: field `counts`: a map key must be a plain"),
            "for {field_type}: {error}"
        );
        assert!(error.contains("Doc"), "for {field_type}: {error}");
    }
}

#[cfg(all(
    feature = "indexmap",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn an_index_map_keyed_by_a_struct_is_refused_in_the_sentence_a_hash_map_gets() {
    register_alias_info("Doc", "Doc", "doc_schema", AliasKind::NoEnumMembers);
    let hashed = field_map_key_error(&quote::quote! { HashMap<Doc, u32> });
    assert!(
        hashed.contains("model_schema: field `counts`: a map key must be a plain"),
        "got: {hashed}"
    );
    for field_type in [
        quote::quote! { IndexMap<Doc, u32> },
        quote::quote! { IndexMap<Doc, u32, RandomState> },
        quote::quote! { indexmap::IndexMap<Doc, u32> },
    ] {
        assert_eq!(field_map_key_error(&field_type), hashed, "for {field_type}");
    }
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    assert_eq!(
        field_map_key_error(&quote::quote! { IndexMap<Slot, u32> }),
        ""
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_sequence_wrapped_map_key_is_refused_wherever_it_is_written() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for (field_type, element) in [
        (quote::quote! { HashMap<Vec<Slot>, u32> }, "Slot"),
        (quote::quote! { HashMap<[Slot; 2], u32> }, "Slot"),
        (quote::quote! { HashMap<HashSet<Slot>, u32> }, "Slot"),
        (quote::quote! { HashMap<Vec<Vec<Slot>>, u32> }, "Slot"),
        (quote::quote! { HashMap<Vec<String>, u32> }, "String"),
        (quote::quote! { HashMap<Vec<u32>, u64> }, "u32"),
        (
            quote::quote! { HashMap<String, HashMap<Vec<Slot>, u32>> },
            "Slot",
        ),
        (
            quote::quote! { HashMap<Slot, HashMap<Vec<Slot>, u32>> },
            "Slot",
        ),
        (quote::quote! { Vec<HashMap<Vec<Slot>, u32>> }, "Slot"),
        (quote::quote! { Option<HashMap<Vec<Slot>, u32>> }, "Slot"),
        (quote::quote! { (String, HashMap<Vec<Slot>, u32>) }, "Slot"),
        (quote::quote! { Wrapper<HashMap<Vec<Slot>, u32>> }, "Slot"),
    ] {
        let error = field_map_key_error(&field_type);
        assert!(error.contains("compile_error"), "for {field_type}: {error}");
        assert!(
            error.contains(
                "model_schema: field `counts`: a map key must be a value serde writes as a string"
            ),
            "for {field_type}: {error}"
        );
        assert!(error.contains(element), "for {field_type}: {error}");
    }
}

/// A key whose own type writes a JSON array or a JSON object earns the same refusal a
/// sequence-wrapped one does: serde raises `key must be a string` and refuses the whole map, so
/// there is no object for any surface to describe. The key is named by the shape it was written as.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn assert_unwritable_map_key(field_type: &proc_macro2::TokenStream, key_name: &str) {
    let error = field_map_key_error(field_type);
    assert!(error.contains("compile_error"), "for {field_type}: {error}");
    assert!(
        error.contains(
            "model_schema: field `counts`: a map key must be a value serde writes as a string"
        ),
        "for {field_type}: {error}"
    );
    assert!(error.contains(key_name), "for {field_type}: {error}");
    assert!(
        error.contains("refuses to serialize a map keyed by one"),
        "for {field_type}: {error}"
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_map_key_serde_refuses_to_write_is_refused_wherever_it_is_written() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for (field_type, key_name) in [
        (quote::quote! { HashMap<(Slot, Slot), u32> }, "(_, _)"),
        (quote::quote! { HashMap<(String, u32), u32> }, "(_, _)"),
        (
            quote::quote! { HashMap<HashMap<String, u32>, u32> },
            "HashMap<_, _>",
        ),
        (
            quote::quote! { HashMap<BTreeMap<String, u32>, u32> },
            "HashMap<_, _>",
        ),
        (
            quote::quote! { HashMap<String, HashMap<(Slot, Slot), u32>> },
            "(_, _)",
        ),
        (
            quote::quote! { HashMap<Slot, HashMap<(Slot, Slot), u32>> },
            "(_, _)",
        ),
        (quote::quote! { Vec<HashMap<(Slot, Slot), u32>> }, "(_, _)"),
        (
            quote::quote! { Option<HashMap<(Slot, Slot), u32>> },
            "(_, _)",
        ),
        (
            quote::quote! { (String, HashMap<(Slot, Slot), u32>) },
            "(_, _)",
        ),
        (
            quote::quote! { Wrapper<HashMap<(Slot, Slot), u32>> },
            "(_, _)",
        ),
    ] {
        assert_unwritable_map_key(&field_type, key_name);
    }
}

#[cfg(all(
    feature = "mongodb",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn an_object_id_map_key_is_refused_wherever_it_is_written() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for field_type in [
        quote::quote! { HashMap<ObjectId, u32> },
        quote::quote! { HashMap<String, HashMap<ObjectId, u32>> },
        quote::quote! { HashMap<Slot, HashMap<ObjectId, u32>> },
        quote::quote! { Vec<HashMap<ObjectId, u32>> },
        quote::quote! { (String, HashMap<ObjectId, u32>) },
        quote::quote! { Wrapper<HashMap<ObjectId, u32>> },
    ] {
        assert_unwritable_map_key(&field_type, "ObjectId");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_optional_map_key_is_refused_wherever_it_is_written() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for (field_type, inner) in [
        (quote::quote! { HashMap<Option<Slot>, u32> }, "Slot"),
        (quote::quote! { HashMap<Option<String>, u32> }, "String"),
        (quote::quote! { HashMap<Option<u32>, u64> }, "u32"),
        (
            quote::quote! { HashMap<String, HashMap<Option<Slot>, u32>> },
            "Slot",
        ),
        (
            quote::quote! { HashMap<Slot, HashMap<Option<Slot>, u32>> },
            "Slot",
        ),
        (quote::quote! { Vec<HashMap<Option<Slot>, u32>> }, "Slot"),
        (quote::quote! { Option<HashMap<Option<Slot>, u32>> }, "Slot"),
        (
            quote::quote! { (String, HashMap<Option<Slot>, u32>) },
            "Slot",
        ),
        (
            quote::quote! { Wrapper<HashMap<Option<Slot>, u32>> },
            "Slot",
        ),
    ] {
        let error = field_map_key_error(&field_type);
        assert!(error.contains("compile_error"), "for {field_type}: {error}");
        assert!(
            error.contains(
                "model_schema: field `counts`: a map key must be a value serde writes as a string"
            ),
            "for {field_type}: {error}"
        );
        assert!(
            error.contains(&format!("Option<{inner}>")),
            "for {field_type}: {error}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_key_wrapped_twice_is_named_by_its_outermost_wrapper() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    let sequenced = field_map_key_error(&quote::quote! { HashMap<Option<Vec<Slot>>, u32> });
    assert!(sequenced.contains("is a sequence of"), "got: {sequenced}");
    let optional = field_map_key_error(&quote::quote! { HashMap<Option<(Slot, Slot)>, u32> });
    assert!(optional.contains("Option<(_, _)>"), "got: {optional}");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_string_wire_brand_keys_a_map_the_way_a_string_does() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    register_alias_info(
        "CorrelationId",
        "CorrelationId",
        "correlation_id_schema",
        AliasKind::StringWire,
    );
    register_alias_info("Tick", "Tick", "tick_schema", AliasKind::NoEnumMembers);
    for field_type in [
        quote::quote! { HashMap<CorrelationId, u32> },
        quote::quote! { HashMap<String, HashMap<CorrelationId, u32>> },
        quote::quote! { HashMap<Slot, HashMap<CorrelationId, u32>> },
        quote::quote! { HashMap<CorrelationId, HashMap<CorrelationId, u32>> },
        quote::quote! { Vec<HashMap<CorrelationId, u32>> },
        quote::quote! { (String, HashMap<CorrelationId, u32>) },
        quote::quote! { Wrapper<HashMap<CorrelationId, u32>> },
    ] {
        let error = field_map_key_error(&field_type);
        assert!(error.is_empty(), "for {field_type}, got: {error}");
    }

    let refused = field_map_key_error(&quote::quote! { HashMap<Tick, u32> });
    assert!(
        refused.contains("a map key must be a plain"),
        "got: {refused}"
    );
    assert!(refused.contains("Tick"), "got: {refused}");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_map_key_that_may_have_enum_members_is_left_alone() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for field_type in [
        quote::quote! { HashMap<Slot, u32> },
        quote::quote! { HashMap<String, u32> },
        quote::quote! { HashMap<Ghost, u32> },
        quote::quote! { HashMap<u32, u64> },
        quote::quote! { HashMap<bool, u64> },
        quote::quote! { HashMap<String, HashMap<Slot, u32>> },
        quote::quote! { HashMap<Slot, Vec<u32>> },
        quote::quote! { HashMap<String, Vec<Slot>> },
        quote::quote! { HashMap<String, Option<Slot>> },
        quote::quote! { HashMap<String, (Slot, Slot)> },
        quote::quote! { HashMap<String, HashMap<String, u32>> },
        quote::quote! { Vec<HashMap<Slot, u32>> },
    ] {
        let error = field_map_key_error(&field_type);
        assert!(error.is_empty(), "for {field_type}, got: {error}");
    }
}

#[cfg(all(
    feature = "chrono",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_chrono_map_key_is_left_alone() {
    for field_type in [
        quote::quote! { HashMap<NaiveDate, u32> },
        quote::quote! { HashMap<NaiveTime, u32> },
        quote::quote! { HashMap<NaiveDateTime, u32> },
        quote::quote! { HashMap<DateTime<Utc>, u32> },
    ] {
        let error = field_map_key_error(&field_type);
        assert!(error.is_empty(), "for {field_type}, got: {error}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_targeting_a_map_key_with_no_members_is_refused() {
    register_alias_info("Doc", "Doc", "doc_schema", AliasKind::NoEnumMembers);
    let alias: syn::ItemType = syn::parse_quote! {
        pub type CountsByDoc = HashMap<Doc, u32>;
    };
    let field_def = get_field_def("CountsByDocType", &alias.ty, "");
    let error = alias_map_key_guard_error(&alias, &field_def)
        .unwrap_or_default()
        .to_string();
    assert!(error.contains("compile_error"), "got: {error}");
    assert!(
        error.contains("model_schema: type alias `CountsByDoc`: a map key must be a plain"),
        "got: {error}"
    );
    assert!(!error.contains("CountsByDocType"), "got: {error}");
    assert!(error.contains("Doc"), "got: {error}");
}

/// The rendered `compile_error!` an alias target reaching a std type serde has no wire form for
/// earns, or the empty string when it earns none.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn alias_undescribable_std_error_text(source: &str) -> String {
    let alias: syn::ItemType = syn::parse_str(source).unwrap();
    let field_def = get_field_def("Probe", &alias.ty, "");
    alias_undescribable_std_error(&alias, &field_def)
        .unwrap_or_default()
        .to_string()
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_targeting_an_undescribable_std_type_is_refused() {
    for (target, subject, named, wire) in [
        (
            "pub type Slot = OnceLock<u32>;",
            "model_schema: type alias `Slot`",
            "`OnceLock`",
            "Serialize",
        ),
        (
            "pub type Location = OsString;",
            "model_schema: type alias `Location`",
            "`OsString`",
            "externally",
        ),
        (
            "pub type Nested = Vec<OnceLock<u32>>;",
            "model_schema: type alias `Nested`",
            "`OnceLock`",
            "Serialize",
        ),
        (
            "pub type Keyed = HashMap<String, OsString>;",
            "model_schema: type alias `Keyed`",
            "`OsString`",
            "externally",
        ),
        (
            "pub type Pending = LinkedList<String>;",
            "model_schema: type alias `Pending`",
            "`LinkedList`",
            "Vec<T>",
        ),
    ] {
        let error = alias_undescribable_std_error_text(target);
        for needle in ["compile_error", subject, named, wire] {
            assert!(
                error.contains(needle),
                "{needle} missing for {target}: {error}"
            );
        }
        assert!(
            !error.contains("Probe"),
            "the field-def name leaked for {target}: {error}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_targeting_a_describable_type_is_left_alone() {
    for target in [
        "pub type Slug = String;",
        "pub type Slugs = Vec<String>;",
        "pub type Held = RefCell<u32>;",
        "pub type Located = PathBuf;",
    ] {
        let error = alias_undescribable_std_error_text(target);
        assert!(error.is_empty(), "for {target}, got: {error}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_violating_both_type_level_guards_earns_both_refusals() {
    register_alias_info(
        "Ledger",
        "Ledger",
        "ledger_schema",
        AliasKind::NoEnumMembers,
    );
    let alias: syn::ItemType = syn::parse_quote! {
        pub type Audited = HashMap<Ledger, OsString>;
    };
    let field_def = get_field_def("Audited", &alias.ty, "");
    assert!(
        alias_undescribable_std_error(&alias, &field_def).is_some(),
        "the std-type guard found nothing"
    );
    assert!(
        alias_map_key_guard_error(&alias, &field_def).is_some(),
        "the map-key guard found nothing"
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_data_suffixed_alias_is_named_by_its_written_ident() {
    register_alias_info("Tally", "Tally", "tally_schema", AliasKind::NoEnumMembers);
    let held: syn::ItemType = syn::parse_quote! {
        pub type SlotData = OnceLock<u32>;
    };
    let held_def = get_field_def("SlotType", &held.ty, "");
    let held_error = alias_undescribable_std_error(&held, &held_def)
        .unwrap_or_default()
        .to_string();
    assert!(
        held_error.contains("type alias `SlotData`"),
        "got: {held_error}"
    );
    assert!(!held_error.contains("SlotType"), "got: {held_error}");

    let keyed: syn::ItemType = syn::parse_quote! {
        pub type CountsData = HashMap<Tally, u32>;
    };
    let keyed_def = get_field_def("CountsType", &keyed.ty, "");
    let keyed_error = alias_map_key_guard_error(&keyed, &keyed_def)
        .unwrap_or_default()
        .to_string();
    assert!(
        keyed_error.contains("type alias `CountsData`"),
        "got: {keyed_error}"
    );
    assert!(!keyed_error.contains("CountsType"), "got: {keyed_error}");
}

/// A refused item still publishes the schema module every reference to it addresses.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_refused_item_publishes_the_module_a_reference_to_it_resolves_to() {
    let ident = syn::Ident::new("CountsByDoc", proc_macro2::Span::call_site());
    let module = super::refused_item_schema_module(&ident).to_string();
    assert!(
        module.contains(&format!(
            "pub mod {}",
            ident_schema_module_name("CountsByDoc")
        )),
        "got: {module}"
    );
    // The refusal is the one diagnostic the author reads, so the module adds none of its own.
    assert!(!module.contains("compile_error"), "got: {module}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_items_module_answers_the_call_a_reference_emits() {
    let span = proc_macro2::Span::call_site();
    let ident = syn::Ident::new("CountsByRefusedDoc", span);
    let module = super::refused_item_schema_module(&ident).to_string();
    let addressed = super::sibling_schema_module_ident("CountsByRefusedDoc", span).to_string();
    assert!(
        module.contains(&format!("pub mod {addressed}")),
        "got: {module}"
    );
    assert!(module.contains("json_schema_within"), "got: {module}");
}

/// Every attribute the walked fields are left carrying, rendered as the emitted item carries them.
#[cfg(feature = "serde")]
fn walked_field_attrs<'field>(fields: impl Iterator<Item = &'field syn::Field>) -> String {
    let attrs: Vec<&syn::Attribute> = fields.flat_map(|field| field.attrs.iter()).collect();
    quote::quote!(#(#attrs)*).to_string()
}

/// Every field of an enum, in the order the walks visit them.
#[cfg(feature = "serde")]
fn enum_fields(item: &syn::ItemEnum) -> impl Iterator<Item = &syn::Field> {
    item.variants
        .iter()
        .flat_map(|variant| variant.fields.iter())
}

#[cfg(feature = "serde")]
#[test]
fn a_refused_pattern_leaves_no_hook_naming_the_dropped_module() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct ProbeCaret {
            #[model_schema_prop(pattern = "^.$")]
            anything: String,
        }
    };
    let errors = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("probe_caret_schema"),
        "ProbeCaret",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    )
    .4;
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(
        errors[0].to_string().contains("any-character class"),
        "got: {}",
        errors[0]
    );
    let rendered = walked_field_attrs(item.fields.iter());
    assert_eq!(rendered, "", "got: {rendered}");
}

#[cfg(feature = "serde")]
#[test]
fn a_constraint_refused_its_placement_leaves_no_hook_naming_the_dropped_module() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Guarded {
            #[model_schema_prop(minLength = 2)]
            note: RefCell<String>,
        }
    };
    let errors = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("guarded_schema"),
        "Guarded",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    )
    .4;
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(
        errors[0]
            .to_string()
            .contains("unsupported on a field reached through `RefCell`"),
        "got: {}",
        errors[0]
    );
    let rendered = walked_field_attrs(item.fields.iter());
    assert_eq!(rendered, "", "got: {rendered}");
}

/// A literal on a field reached through an interior-mutability wrapper is refused: the validator
/// has no way to reach the value it compares.
#[cfg(feature = "serde")]
#[test]
fn a_literal_reached_through_an_interior_mutability_wrapper_is_refused() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Guarded {
            #[model_schema_prop(literal = "x")]
            note: RefCell<String>,
        }
    };
    let errors = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("guarded_schema"),
        "Guarded",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    )
    .4;
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(
        errors[0].to_string().contains(
            "`literal` is unsupported on a field reached through `RefCell` \u{2014} unlike `Box`, \
             `Rc`, `Arc` and `Cow`, it does not implement `Deref`"
        ),
        "got: {}",
        errors[0]
    );
}

/// A refusal at build time and a helper name the field as its author wrote it. A report made as
/// the program runs names the key serde writes.
#[cfg(feature = "serde")]
#[test]
fn a_renamed_field_is_refused_by_its_rust_name_and_reported_under_its_key() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Guarded {
            #[model_schema_prop(minLength = 2)]
            held_note: RefCell<String>,
            #[model_schema_prop(minLength = 2)]
            item_label: String,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        Some("camelCase"),
        Some("guarded_schema"),
        "Guarded",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert_eq!(collected.4.len(), 1, "got: {:?}", collected.4);
    let refusal = collected.4[0].to_string();
    assert!(
        refusal.contains("field `held_note`") && !refusal.contains("heldNote"),
        "got: {refusal}"
    );

    let helpers = collected
        .2
        .iter()
        .map(ToString::to_string)
        .collect::<String>();
    assert!(
        helpers.contains("fn validate_item_label_value") && !helpers.contains("itemLabel_value"),
        "got: {helpers}"
    );
    assert!(
        helpers.contains(r#""itemLabel""#) && !helpers.contains(r#""item_label""#),
        "got: {helpers}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_refused_map_key_leaves_no_hook_naming_the_dropped_module() {
    register_alias_info("Doc", "Doc", "doc_schema", AliasKind::NoEnumMembers);
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Choice {
            Named {
                #[model_schema_prop(minLength = 2)]
                name: String,
                counts: HashMap<Doc, u32>,
            },
        }
    };
    let errors = collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    )
    .5;
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(
        errors[0].to_string().contains("a map key must be a plain"),
        "got: {}",
        errors[0]
    );
    let rendered = walked_field_attrs(enum_fields(&item));
    assert_eq!(rendered, "", "got: {rendered}");
}

#[cfg(feature = "serde")]
#[test]
fn a_struct_field_that_clears_the_guards_is_hung_with_no_hook() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(rename = "label")]
            #[model_schema_prop(minLength = 2)]
            name: String,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("report_schema"),
        "Report",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);

    let generated = collected
        .2
        .iter()
        .map(ToString::to_string)
        .collect::<String>();
    assert!(
        generated.contains("fn validate_name_value"),
        "the constraint is still read and its validator still written: {generated}"
    );

    let rendered = walked_field_attrs(item.fields.iter());
    assert_eq!(
        rendered,
        quote::quote! {
            #[serde(rename = "label")]
        }
        .to_string(),
        "got: {rendered}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_field_whose_type_could_publish_a_validator_contributes_a_body_that_runs_it() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Enrolment {
            slug: Slug,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("enrolment_schema"),
        "Enrolment",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);
    assert!(
        collected.2.is_empty(),
        "the field declares no constraint of its own, so there is no per-field validator to \
         publish: {:?}",
        collected
            .2
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );

    let body = collected
        .3
        .iter()
        .map(ToString::to_string)
        .collect::<String>();
    assert!(
        body.contains("trait UnpublishedValidate"),
        "a type that published no validator has to answer something: {body}"
    );
    assert!(
        body.contains("impl < T : ? :: core :: marker :: Sized > UnpublishedValidate for & T"),
        "implemented for `&T` so it sits one autoref step below any other blanket `validate()` \
         the call site can see, rather than tying with it: {body}"
    );
    assert!(
        body.contains("value_0 . validate ()"),
        "the field's own validator is what runs: {body}"
    );
    assert!(
        body.contains(r#"nested_under ("slug" , violation)"#),
        "the field the value was reached through is written into the report the value's own type \
         made, so one quoted run carries the whole path: {body}"
    );
    assert!(
        body.contains(r#"format ! ("'{field}.{named}'{tail}")"#)
            && body.contains(r#"format ! ("'{field}': {violation}")"#),
        "a report naming a member of its own is written into; a brand's, naming none, has the \
         field put in front of it instead: {body}"
    );

    let rendered = walked_field_attrs(item.fields.iter());
    assert_eq!(
        rendered, "",
        "reaching a bound from the validator hangs nothing on the field. Got: {rendered}"
    );
}

/// The place a nested report is written under: the key, then the index each list level of the
/// walk binds, innermost last.
#[cfg(feature = "serde")]
#[test]
fn a_nested_report_is_placed_under_its_key_and_the_index_of_each_list_level() {
    let place = |spelling: &str, under: Option<&str>| {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let held: Vec<super::PlaceStep> = under
            .map(|key| super::PlaceStep::Written(key.to_owned()))
            .into_iter()
            .collect();
        super::nested_place(&super::held_at(
            &held,
            &super::nested_shape(&ty).unwrap(),
            1,
        ))
        .map(|written| written.to_string())
    };
    assert_eq!(place("Slug", None), None);
    assert_eq!(place("Option<Slug>", Some("slug")).unwrap(), r#""slug""#);
    assert_eq!(
        place("Vec<Option<Vec<Slug>>>", Some("slugs")).unwrap(),
        r#"& format ! ("slugs.{index_1}.{index_3}")"#
    );
    assert_eq!(
        place("Vec<Slug>", None).unwrap(),
        r#"& format ! ("{index_1}")"#
    );
    assert_eq!(
        place("Vec<Slug>", Some("{slugs}")).unwrap(),
        r#"& format ! ("{{slugs}}.{index_1}")"#,
        "a brace of the key is text, where a brace of the template opens an argument"
    );

    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Enrolment {
            slug_list: Vec<Option<Vec<Slug>>>,
        }
    };
    let body = super::collect_struct_fields(
        &mut item.fields,
        Some("camelCase"),
        Some("enrolment_schema"),
        "Enrolment",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    )
    .3
    .iter()
    .map(ToString::to_string)
    .collect::<String>();
    for written in [
        "for (index_1 , value_1) in value_0 . into_iter () . enumerate ()",
        "for (index_3 , value_3) in value_2 . into_iter () . enumerate ()",
        r#"nested_under (& format ! ("slugList.{index_1}.{index_3}") , violation)"#,
    ] {
        assert!(body.contains(written), "{written} is not in {body}");
    }
}

/// What the fields of `item` add to `validate()`, under camel case, as one text.
#[cfg(feature = "serde")]
fn walked_bodies(item: &mut syn::ItemStruct) -> String {
    let generics = item.generics.clone();
    super::collect_struct_fields(
        &mut item.fields,
        Some("camelCase"),
        Some("terminal_schema"),
        "Terminal",
        &generics,
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    )
    .3
    .iter()
    .map(ToString::to_string)
    .collect()
}

#[cfg(feature = "serde")]
#[test]
fn a_map_of_declared_types_is_walked_entry_by_entry_under_the_key_serde_writes() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Terminal {
            parcels_by_dock: BTreeMap<String, Parcel>,
            docks_in_order: Vec<Option<HashMap<Dock, Vec<Parcel>>>>,
        }
    };
    let body = walked_bodies(&mut item);
    for written in [
        "impl < K : ? :: core :: marker :: Sized + serde :: Serialize > :: core :: fmt :: Display \
         for MapKey < '_ , K >",
        "serde :: Serialize :: serialize (self . 0 , formatter)",
        "for (entry_1 , value_1) in value_0 { if let",
        "let key_1 = MapKey (entry_1) . written_key () ; errors . extend",
        r#"nested_under ("parcelsByDock" , & keyed_under (key_1 . as_deref () , violation))"#,
        "for (entry_3 , value_3) in value_2 { for",
        "let key_3 = MapKey (entry_3) . written_key () ; errors . extend",
        r#"nested_under (& format ! ("docksInOrder.{index_1}") , & keyed_under (key_3 . as_deref () , & nested_under (& format ! ("{index_4}") , violation)))"#,
    ] {
        assert!(body.contains(written), "{written} is not in {body}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_tuple_written_as_a_fields_type_is_walked_position_by_position() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Terminal {
            deck_pairs: Vec<(String, Parcel, Option<Parcel>)>,
        }
    };
    let body = walked_bodies(&mut item);
    for written in [
        "for (index_1 , value_1) in value_0 . into_iter () . enumerate ()",
        "let value_2 = & value_1 . 1 ;",
        r#"nested_under (& format ! ("deckPairs.{index_1}.1") , violation)"#,
        "let value_2 = & value_1 . 2 ;",
        "if let :: core :: option :: Option :: Some (value_3) = value_2",
        r#"nested_under (& format ! ("deckPairs.{index_1}.2") , violation)"#,
    ] {
        assert!(body.contains(written), "{written} is not in {body}");
    }
    assert!(
        !body.contains("value_1 . 0"),
        "a position that holds no declared type is not read: {body}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_slot_of_a_tuple_struct_is_walked_and_reported_under_its_position() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Consignment(String, Parcel, Vec<Parcel>);
    };
    let body: String = super::collect_tuple_slots(
        &mut item.fields,
        None,
        "consignment_schema",
        "Consignment",
        &syn::Generics::default(),
        false,
        false,
    )
    .validate_bodies
    .iter()
    .map(ToString::to_string)
    .collect();
    for written in [
        "let value_0 = & self . 1 ;",
        r#"nested_under ("1" , violation)"#,
        "let value_0 = & self . 2 ;",
        r#"nested_under (& format ! ("2.{index_1}") , violation)"#,
    ] {
        assert!(body.contains(written), "{written} is not in {body}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_slot_of_a_tagged_variant_is_walked_and_reported_under_the_variants_name() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Freight {
            Boxed(Parcel),
            Split(String, Parcel),
        }
    };
    let arms: String = collect_discriminated_variants(
        &mut item,
        UNCASED,
        Some("freight_schema"),
        &super::ModelSchemaArgs::default(),
    )
    .3
    .iter()
    .map(ToString::to_string)
    .collect();
    for written in [
        "Self :: Boxed (member_slot_0)",
        "let value_0 = member_slot_0 ;",
        r#"nested_under ("Boxed" , violation)"#,
        "Self :: Split (_ , member_slot_1)",
        "let value_0 = member_slot_1 ;",
        r#"nested_under ("Split.1" , violation)"#,
    ] {
        assert!(arms.contains(written), "{written} is not in {arms}");
    }
}

/// A map, a tuple and a slot that hold no declared type are not walked, a type parameter and a
/// set of strings among what they hold.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_map_a_tuple_and_a_slot_that_hold_no_declared_type_add_nothing_to_validate() {
    let mut fields: syn::ItemStruct = syn::parse_quote! {
        struct Terminal<Held> {
            counts: BTreeMap<String, u32>,
            held_by_dock: BTreeMap<String, Held>,
            held_pair: (u32, Held),
            labels: HashMap<String, Vec<String>>,
            nested: Option<BTreeMap<String, (bool, char)>>,
            pair: (String, u32),
            sets_by_dock: BTreeMap<String, HashSet<String>>,
        }
    };
    assert_eq!(walked_bodies(&mut fields), "");

    let mut slots: syn::ItemStruct = syn::parse_quote! {
        struct Docket<Held>(String, u32, BTreeMap<String, u32>, (bool, Held), HashSet<String>, Held);
    };
    let generics = slots.generics.clone();
    let walked = super::collect_tuple_slots(
        &mut slots.fields,
        None,
        "docket_schema",
        "Docket",
        &generics,
        false,
        false,
    );
    assert_eq!(
        walked
            .validate_bodies
            .iter()
            .map(ToString::to_string)
            .collect::<String>(),
        ""
    );

    let mut variants: syn::ItemEnum = syn::parse_quote! {
        enum Seal {
            Mapped(BTreeMap<String, u32>),
            Paired(String, (u32, bool)),
            Listed(HashSet<String>),
        }
    };
    let arms = collect_discriminated_variants(
        &mut variants,
        UNCASED,
        Some("seal_schema"),
        &super::ModelSchemaArgs::default(),
    )
    .3;
    assert!(
        arms.is_empty(),
        "got: {:?}",
        arms.iter().map(ToString::to_string).collect::<Vec<_>>()
    );
}

/// A set is described as a declared type is, so a field written as one was walked to whatever it
/// holds before a map and a tuple were read, and still is.
#[cfg(feature = "serde")]
#[test]
fn a_field_written_as_a_set_is_walked_as_it_was() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Terminal {
            dock_names: HashSet<String>,
        }
    };
    let body = walked_bodies(&mut item);
    for written in [
        "for (index_1 , value_1) in value_0 . into_iter () . enumerate ()",
        "value_1 . validate ()",
        r#"nested_under (& format ! ("dockNames.{index_1}") , violation)"#,
    ] {
        assert!(body.contains(written), "{written} is not in {body}");
    }
}

/// The reader hung on a field that holds a declared type is hung on no map and no tuple.
#[cfg(feature = "serde")]
#[test]
fn a_map_and_a_tuple_of_declared_types_are_hung_with_no_reader() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        #[derive(serde::Deserialize)]
        struct Terminal {
            deck_pair: (String, Parcel),
            parcels_by_dock: BTreeMap<String, Parcel>,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        Some("camelCase"),
        Some("terminal_schema"),
        "Terminal",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: true,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);
    assert_eq!(walked_field_attrs(item.fields.iter()), "");
    assert!(
        collected.2.is_empty(),
        "got: {:?}",
        collected
            .2
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_field_holding_a_declared_type_is_read_through_one_that_names_the_field() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Enrolment {
            organization_id: Slug,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        Some("camelCase"),
        Some("enrolment_schema"),
        "Enrolment",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: true,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);
    // The reader is one of the item's own read, hung on the copy serde's derive reads.
    assert_eq!(walked_field_attrs(item.fields.iter()), "");
    assert!(
        collected.2.is_empty(),
        "got: {:?}",
        collected
            .2
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );

    let read = super::judged_read::of_struct(&mut item, true).to_string();
    for written in [
        "# [serde (deserialize_with = \"read_unjudged_organization_id\")] organization_id : Slug",
        "fn read_unjudged_organization_id < 'de , D > (deserializer : D) -> :: core :: result :: \
         Result < Slug , D :: Error >",
        "< Slug > :: model_schema_read_unjudged (deserializer) . map (Unjudged)",
        // The key as the wire spells it: the name serde was reading for, and the one the schema
        // reports.
        "nested_under (\"organizationId\" , violation)",
    ] {
        assert!(read.contains(written), "missing `{written}`, got: {read}");
    }
    assert!(!read.contains("\"organization_id\""), "got: {read}");
}

#[cfg(feature = "serde")]
#[test]
fn the_names_a_read_declares_are_never_names_the_item_writes() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        #[derive(serde::Deserialize)]
        struct Enrolment {
            first: Unjudged,
            second: Vec<ReadUnjudged>,
            third: ModelSchemaRemote,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("enrolment_schema"),
        "Enrolment",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: true,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);
    let read = super::judged_read::of_struct(&mut item, true).to_string();
    for written in [
        "struct Unjudged2 (Unjudged) ;",
        "< Unjudged > :: model_schema_read_unjudged (deserializer) . map (Unjudged2)",
        "struct Unjudged (ReadUnjudged) ;",
        "trait ReadUnjudged2 < 'de > : serde :: Deserialize < 'de >",
        "impl < 'de , T : serde :: Deserialize < 'de >> ReadUnjudged2 < 'de > for T { }",
        "struct ModelSchemaRemote2 {",
        "< ModelSchemaRemote2 > :: deserialize (deserializer)",
    ] {
        assert!(read.contains(written), "missing `{written}`, got: {read}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_field_the_unjudged_read_does_not_reach_keeps_a_reader_in_the_schema_module() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Enrolment {
            organization_id: Arc<[Slug]>,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        Some("camelCase"),
        Some("enrolment_schema"),
        "Enrolment",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: true,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);

    let rendered = walked_field_attrs(item.fields.iter());
    assert_eq!(
        rendered,
        quote::quote! {
            #[serde(deserialize_with = "enrolment_schema::deserialize_named_organization_id")]
        }
        .to_string(),
        "got: {rendered}"
    );

    let published = collected
        .2
        .iter()
        .map(ToString::to_string)
        .collect::<String>();
    assert!(
        published.contains("\"organizationId\""),
        "the name written is the key as the wire spells it, that being the name serde was reading \
         for and the one the schema reports. Got: {published}"
    );
    assert!(
        !published.contains("\"organization_id\""),
        "got: {published}"
    );
    let read = super::judged_read::of_struct(&mut item, true).to_string();
    assert!(!read.contains("read_unjudged_"), "got: {read}");
}

#[cfg(feature = "serde")]
#[test]
fn a_field_the_reader_would_displace_is_left_to_read_itself() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        #[derive(serde::Deserialize)]
        struct Enrolment {
            #[serde(flatten)]
            merged: Merged,
            #[serde(deserialize_with = "read_it_myself")]
            mine: Slug,
            #[serde(skip)]
            absent: Slug,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("enrolment_schema"),
        "Enrolment",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: true,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    let rendered = walked_field_attrs(item.fields.iter());
    assert!(
        !rendered.contains("deserialize_named_"),
        "serde admits one reader per field, and each of these already has one or has no read at \
         all. Got: {rendered}"
    );
    let published = collected
        .2
        .iter()
        .map(ToString::to_string)
        .collect::<String>();
    assert!(
        !published.contains("deserialize_named_"),
        "a reader nothing is hung with is a function nothing calls. Got: {published}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_container_that_is_not_read_back_publishes_no_reader() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        #[derive(serde::Serialize)]
        struct Enrolment {
            slug: Slug,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("enrolment_schema"),
        "Enrolment",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: super::derives_deserialize(&item.attrs),
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(
        collected.2.is_empty(),
        "got: {:?}",
        collected
            .2
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(walked_field_attrs(item.fields.iter()), "");
}

#[cfg(feature = "serde")]
#[test]
fn a_field_the_crate_renders_itself_contributes_no_body_to_reach_into() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Plain {
            count: u32,
            name: String,
            tags: Vec<String>,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("plain_schema"),
        "Plain",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);
    assert!(
        collected.3.is_empty(),
        "got: {:?}",
        collected
            .3
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
}

#[cfg(feature = "serde")]
#[test]
fn an_optional_struct_field_is_hung_with_neither_the_hook_nor_the_default_it_answers_for() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            #[serde(skip_serializing_if = "Option::is_none")]
            #[model_schema_prop(minLength = 2)]
            name: Option<String>,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("report_schema"),
        "Report",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);
    let rendered = walked_field_attrs(item.fields.iter());
    assert_eq!(
        rendered,
        quote::quote! {
            #[serde(skip_serializing_if = "Option::is_none")]
        }
        .to_string(),
        "the declaration's own attribute and nothing beside it. Got: {rendered}"
    );
}

/// The untagged member is the other half: it earns the hook, and therefore earns the `default` too.
#[cfg(feature = "serde")]
#[test]
fn an_optional_untagged_member_is_hung_with_the_hook_and_the_default_it_answers_for() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Choice {
            Named {
                #[serde(skip_serializing_if = "Option::is_none")]
                #[model_schema_prop(minLength = 2)]
                name: Option<String>,
            },
        }
    };
    let (_, _, _, _, _, errors, _, _, _, _) = collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    let attrs = &item.variants[0].fields.iter().next().unwrap().attrs;
    let rendered = quote::quote!(#(#attrs)*).to_string();
    assert_eq!(
        rendered,
        quote::quote! {
            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(deserialize_with = "choice_schema::deserialize_named_name")]
            #[serde(default)]
        }
        .to_string(),
        "the declaration's own attribute, then the hook, then the `default` the hook needs. Got: \
         {rendered}"
    );
}

/// The type the parser reads a field's written spelling as, rendered the way every surface receives
/// it: one `FieldDef`, so a spelling that parses alike describes alike wherever it is dispatched.
fn parsed_field_type(field_type: &proc_macro2::TokenStream) -> String {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct Report {
            counts: #field_type,
        }
    };
    let field = item.fields.iter().next().unwrap();
    format!("{:?}", get_field_def("counts", &field.ty, "").field_type)
}

#[test]
fn a_container_written_with_a_hasher_parses_as_the_container_without_one() {
    for (written, implied) in [
        (
            quote::quote! { HashMap<String, u32, FxBuildHasher> },
            quote::quote! { HashMap<String, u32> },
        ),
        (
            quote::quote! { HashSet<String, FxBuildHasher> },
            quote::quote! { HashSet<String> },
        ),
        (
            quote::quote! { HashMap<String, HashSet<u32, FxBuildHasher>> },
            quote::quote! { HashMap<String, HashSet<u32>> },
        ),
        (
            quote::quote! { Option<HashSet<u32, FxBuildHasher>> },
            quote::quote! { Option<HashSet<u32>> },
        ),
    ] {
        assert_eq!(
            parsed_field_type(&written),
            parsed_field_type(&implied),
            "for {written}"
        );
    }
}

#[test]
fn a_container_short_of_its_wire_arity_still_falls_through_as_a_sibling() {
    assert_eq!(
        parsed_field_type(&quote::quote! { HashMap<String> }),
        parsed_field_type(&quote::quote! { Wrapper<String> }).replace("Wrapper", "HashMap"),
    );
}

/// Runs the field walk the way [`super::process_field`] does and renders the guard failures its
/// `model_schema_prop` attributes earn, so a refused key and an unparseable `pattern` are read off
/// the same channel that carries them to the emitted item.
fn field_prop_guard_errors_in_scope(item: &syn::ItemStruct, parameters: &[String]) -> Vec<String> {
    let field = item.fields.iter().next().unwrap();
    let name = mapped_or_default(field.ident.as_ref(), ToString::to_string);
    let written = get_field_def(&name, &field.ty, "");
    let mut rendered = written.clone();
    rendered.erase_type_parameters(parameters);
    apply_serde_key_omission(&mut rendered, field);
    let meta = super::parse_model_schema_prop_attributes(&field.attrs);
    super::collect_field_guard_errors(field, &rendered, &written, &name, &meta, true, Vec::new())
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn field_prop_guard_errors(item: &syn::ItemStruct) -> Vec<String> {
    field_prop_guard_errors_in_scope(item, &[])
}

/// The messages [`field_prop_guard_errors`] carries, read back unescaped.
fn field_prop_guard_messages(item: &syn::ItemStruct) -> Vec<String> {
    field_prop_guard_errors(item)
        .iter()
        .flat_map(|error| literal_texts(&error.parse().unwrap()))
        .collect()
}

#[test]
fn the_field_pattern_guard_follows_the_regex_crate() {
    for pattern in PROBE_PATTERNS {
        let rejected = regex::Regex::new(pattern).is_err();
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(pattern = #pattern)]
                name: String,
            }
        });
        assert_eq!(
            errors.len(),
            usize::from(rejected),
            "for {pattern}, got: {errors:?}"
        );
    }
}

#[test]
fn a_field_pattern_the_regex_crate_rejects_names_the_field_and_quotes_the_parse_error() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(pattern = r"^ab\")]
            name: String,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    for needle in [
        "compile_error",
        "field `name`",
        "pattern",
        "regex parse error",
        "incomplete escape sequence",
    ] {
        assert!(
            errors[0].contains(needle),
            "{needle} missing: {}",
            errors[0]
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_field_pattern_javascript_cannot_carry_names_the_field_and_the_construct() {
    for (pattern, construct) in UNPORTABLE_PROBE_PATTERNS {
        assert!(
            regex::Regex::new(pattern).is_ok(),
            "{pattern} is not a pattern the `regex` crate accepts, so it probes nothing"
        );
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(pattern = #pattern)]
                name: String,
            }
        });
        assert_eq!(errors.len(), 1, "for {pattern}, got: {errors:?}");
        for needle in ["compile_error", "field `name`", construct] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {pattern}: {}",
                errors[0]
            );
        }
    }
}

#[test]
fn a_field_pattern_naming_a_group_the_rust_way_clears_the_guard() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(pattern = r"^(?P<word>[a-z]+)$")]
            name: String,
        }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[test]
fn a_field_pattern_admitting_every_value_names_the_field_and_says_so() {
    for pattern in ["", "^", "$", "|", "a*", "^a*"] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(pattern = #pattern)]
                name: String,
            }
        });
        assert_eq!(errors.len(), 1, "for {pattern:?}, got: {errors:?}");
        for needle in [
            "compile_error",
            "field `name`",
            "admits every value",
            "constrains nothing",
        ] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {pattern:?}: {}",
                errors[0]
            );
        }
    }
}

#[test]
fn a_field_pattern_that_is_one_assertion_and_nothing_else_names_the_field_and_says_so() {
    for pattern in [r"\b", r"\B"] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(pattern = #pattern)]
                name: String,
            }
        });
        assert_eq!(errors.len(), 1, "for {pattern:?}, got: {errors:?}");
        for needle in [
            "compile_error",
            "field `name`",
            "one look-around assertion and nothing else",
            "clippy::trivial_regex",
        ] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {pattern:?}: {}",
                errors[0]
            );
        }
    }
}

#[test]
fn a_field_pattern_written_out_of_the_same_pieces_that_still_constrains_clears_the_guard() {
    for pattern in ["^$", "^a*$", r"\b\w+"] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(pattern = #pattern)]
                name: String,
            }
        });
        assert!(errors.is_empty(), "for {pattern:?}, got: {errors:?}");
    }
}

/// A field carrying no `pattern` at all must not acquire one of these errors.
#[test]
fn an_unpatterned_field_is_left_alone() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(minLength = 3)]
            name: String,
        }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

/// The reported repro: two misspelled keys, which emitted `z.string()` with nothing on it.
#[test]
fn a_misspelled_field_prop_key_names_the_field_and_the_key_as_written() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(patern = "^[a-z]+$", minLenght = 3)]
            name: String,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    for needle in ["compile_error", "field `name`", "patern", "pattern"] {
        assert!(
            errors[0].contains(needle),
            "{needle} missing: {}",
            errors[0]
        );
    }
}

#[test]
fn a_field_prop_value_the_parser_cannot_read_names_the_field() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(minLength = "3")]
            name: String,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("field `name`"), "got: {}", errors[0]);
}

#[test]
fn a_refused_key_and_an_unparseable_pattern_are_both_reported() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(pattern = r"^ab\", patern = "^[a-z]+$")]
            name: String,
        }
    });
    assert_eq!(errors.len(), 2, "got: {errors:?}");
    assert!(errors[0].contains("patern"), "got: {}", errors[0]);
    assert!(
        errors[1].contains("regex parse error"),
        "got: {}",
        errors[1]
    );
}

#[cfg(feature = "chrono")]
#[test]
fn a_bound_on_a_chrono_field_is_refused() {
    for (constraint, key) in [
        (quote::quote! { minLength = 30 }, "minLength"),
        (quote::quote! { maxLength = 30 }, "maxLength"),
        (quote::quote! { pattern = "^[0-9-]+$" }, "pattern"),
        (quote::quote! { minimum = 5 }, "minimum"),
        (quote::quote! { maximum = 5 }, "maximum"),
    ] {
        for field_type in [
            quote::quote! { chrono::NaiveDate },
            quote::quote! { Option<chrono::NaiveTime> },
            quote::quote! { Box<chrono::NaiveDateTime> },
            quote::quote! { chrono::DateTime<chrono::Utc> },
        ] {
            let errors = field_prop_guard_errors(&syn::parse_quote! {
                struct Report {
                    #[model_schema_prop(#constraint)]
                    when: #field_type,
                }
            });
            assert_eq!(errors.len(), 1, "for {key} on {field_type}: {errors:?}");
            for needle in ["compile_error", "field `when`", key, "chrono::"] {
                assert!(
                    errors[0].contains(needle),
                    "{needle} missing for {key} on {field_type}: {}",
                    errors[0]
                );
            }
        }
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_bound_on_an_object_id_field_is_refused() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(minLength = 30, pattern = "^[a-f]+$")]
            id: ObjectId,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    for needle in [
        "compile_error",
        "field `id`",
        "minLength",
        "pattern",
        "ObjectId",
    ] {
        assert!(
            errors[0].contains(needle),
            "{needle} missing: {}",
            errors[0]
        );
    }
}

#[cfg(all(feature = "chrono", feature = "mongodb"))]
#[test]
fn a_fixed_shape_field_without_a_bound_is_left_alone() {
    for field in [
        quote::quote! { when: chrono::NaiveDate },
        quote::quote! { #[model_schema_prop(as_number)] when: chrono::DateTime<chrono::Utc> },
        quote::quote! { #[model_schema_prop(preprocess = ["trim"])] when: chrono::NaiveDate },
        quote::quote! { id: ObjectId },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

#[test]
fn a_bound_on_a_map_or_tuple_field_is_refused() {
    for (constraint, key) in [
        (quote::quote! { minLength = 30 }, "minLength"),
        (quote::quote! { maxLength = 30 }, "maxLength"),
        (quote::quote! { pattern = "^[a-z]+$" }, "pattern"),
        (quote::quote! { minimum = 5 }, "minimum"),
        (quote::quote! { maximum = 5 }, "maximum"),
    ] {
        for (field_type, shape) in [
            (quote::quote! { HashMap<String, String> }, "a map"),
            (quote::quote! { Option<HashMap<String, u32>> }, "a map"),
            (quote::quote! { (String, String) }, "a tuple"),
            (quote::quote! { Option<(String, u32)> }, "a tuple"),
        ] {
            let errors = field_prop_guard_errors(&syn::parse_quote! {
                struct Report {
                    #[model_schema_prop(#constraint)]
                    labels: #field_type,
                }
            });
            assert_eq!(errors.len(), 1, "for {key} on {field_type}: {errors:?}");
            for needle in ["compile_error", "field `labels`", key, shape, "brand"] {
                assert!(
                    errors[0].contains(needle),
                    "{needle} missing for {key} on {field_type}: {}",
                    errors[0]
                );
            }
        }
    }
}

#[test]
fn a_length_or_a_pattern_on_a_numeric_field_is_refused() {
    for (constraint, key) in [
        (quote::quote! { minLength = 3 }, "minLength"),
        (quote::quote! { maxLength = 3 }, "maxLength"),
        (quote::quote! { pattern = "^[0-9]+$" }, "pattern"),
    ] {
        for (field_type, declared) in [
            (quote::quote! { u32 }, "`u32`"),
            (quote::quote! { f64 }, "`f64`"),
            (quote::quote! { isize }, "`isize`"),
            (quote::quote! { Option<i64> }, "`Option<i64>`"),
            (quote::quote! { Box<f32> }, "`Box<f32>`"),
            (quote::quote! { &'static u16 }, "`&'static u16`"),
        ] {
            let errors = field_prop_guard_errors(&syn::parse_quote! {
                struct Report {
                    #[model_schema_prop(#constraint)]
                    count: #field_type,
                }
            });
            assert_eq!(errors.len(), 1, "for {key} on {field_type}: {errors:?}");
            for needle in [
                "compile_error",
                "field `count`",
                key,
                declared,
                "writes a number",
                "`minimum` or `maximum`",
            ] {
                assert!(
                    errors[0].contains(needle),
                    "{needle} missing for {key} on {field_type}: {}",
                    errors[0]
                );
            }
        }
    }
}

#[test]
fn a_range_on_a_string_field_is_refused() {
    for (constraint, key) in [
        (quote::quote! { minimum = 5 }, "minimum"),
        (quote::quote! { maximum = 5 }, "maximum"),
    ] {
        for (field_type, declared) in [
            (quote::quote! { String }, "`String`"),
            (quote::quote! { Option<String> }, "`Option<String>`"),
            (quote::quote! { &'static str }, "`&'static str`"),
            (quote::quote! { Cow<'static, str> }, "`Cow<'static, str>`"),
            (quote::quote! { Box<str> }, "`Box<str>`"),
            (quote::quote! { PathBuf }, "`PathBuf`"),
        ] {
            let errors = field_prop_guard_errors(&syn::parse_quote! {
                struct Report {
                    #[model_schema_prop(#constraint)]
                    name: #field_type,
                }
            });
            assert_eq!(errors.len(), 1, "for {key} on {field_type}: {errors:?}");
            for needle in [
                "compile_error",
                "field `name`",
                key,
                declared,
                "writes a string",
                "`minLength`, `maxLength` or `pattern`",
            ] {
                assert!(
                    errors[0].contains(needle),
                    "{needle} missing for {key} on {field_type}: {}",
                    errors[0]
                );
            }
        }
    }
}

#[test]
fn a_bound_of_the_wrong_kind_is_named_without_the_keys_beside_it() {
    let on_a_number = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(minimum = 1, minLength = 3, pattern = "^[0-9]+$")]
            count: u32,
        }
    });
    assert_eq!(on_a_number.len(), 1, "got: {on_a_number:?}");
    assert!(
        on_a_number[0].contains("`minLength`, `pattern` cannot apply"),
        "got: {}",
        on_a_number[0]
    );
    assert!(
        !on_a_number[0].contains("`minimum`,"),
        "got: {}",
        on_a_number[0]
    );

    let on_a_string = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(minLength = 3, minimum = 1, maximum = 9)]
            name: String,
        }
    });
    assert_eq!(on_a_string.len(), 1, "got: {on_a_string:?}");
    assert!(
        on_a_string[0].contains("`minimum`, `maximum` cannot apply"),
        "got: {}",
        on_a_string[0]
    );
}

#[test]
fn a_range_on_a_bool_or_a_char_field_is_refused() {
    for (constraint, field_type, refused, written) in [
        (
            quote::quote! { minimum = 5 },
            quote::quote! { bool },
            "`minimum` cannot apply to a `bool` field",
            "writes a boolean",
        ),
        (
            quote::quote! { minimum = 5 },
            quote::quote! { Option<bool> },
            "`minimum` cannot apply to a `Option<bool>` field",
            "writes a boolean",
        ),
        (
            quote::quote! { maximum = 5 },
            quote::quote! { char },
            "`maximum` cannot apply to a `char` field",
            "writes one character",
        ),
        (
            quote::quote! { maximum = 5 },
            quote::quote! { Box<char> },
            "`maximum` cannot apply to a `Box<char>` field",
            "writes one character",
        ),
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(#constraint)]
                held: #field_type,
            }
        });
        assert_eq!(
            errors.len(),
            1,
            "for {constraint} on {field_type}: {errors:?}"
        );
        for needle in [
            "compile_error",
            "field `held`",
            refused,
            "a range is measured on a number",
            written,
            "nor the generated validator. Drop it.",
        ] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {constraint} on {field_type}: {}",
                errors[0]
            );
        }
    }
}

/// The README prints each refusal, held here to what the guard says.
#[test]
fn the_readme_shows_the_refusal_of_a_bound_of_the_wrong_kind() {
    let readme = include_str!("../../README.md");
    let on_a_list: syn::ItemStruct = syn::parse_quote! {
        struct Article {
            #[model_schema_prop(minLength = 1, pattern = "^[a-z]+$")]
            tags: Vec<String>,
        }
    };
    let on_a_number: syn::ItemStruct = syn::parse_quote! {
        struct Product {
            #[model_schema_prop(minLength = 3)]
            age_restriction: u32,
        }
    };
    let on_a_string: syn::ItemStruct = syn::parse_quote! {
        struct UserProfile {
            #[model_schema_prop(minimum = 3)]
            username: String,
        }
    };
    for (item, shown) in [
        (
            on_a_number,
            "model_schema: field `age_restriction`: `minLength` cannot apply to a `u32` field \u{2014} \
             a length or a pattern is measured on a string, and this field writes a number: the \
             constraint would reach neither Zod, nor the JSON schema, nor the generated \
             validator. Bound a number with `minimum` or `maximum`, or drop it.",
        ),
        (
            on_a_string,
            "model_schema: field `username`: `minimum` cannot apply to a `String` field \u{2014} a \
             range is measured on a number, and this field writes a string: the constraint would \
             reach neither Zod, nor the JSON schema, nor the generated validator. Bound a string \
             with `minLength`, `maxLength` or `pattern`, or drop it.",
        ),
        (
            on_a_list,
            "model_schema: field `tags`: `pattern` cannot apply to a `Vec<String>` field \u{2014} a \
             key measures the field's own value, and this field writes a list, which has no text \
             for a pattern and no number for a range. `minLength` and `maxLength` count the \
             list's items, and no key written here reaches them. A rule for each item goes on the \
             item's own type: declare the item as a `#[serde(transparent)]` brand that carries \
             the `pattern`.",
        ),
    ] {
        let errors = field_prop_guard_errors(&item);
        assert_eq!(errors.len(), 1, "got: {errors:?}");
        assert!(errors[0].contains(shown), "got: {}", errors[0]);
        assert!(
            readme.contains(&format!("error: {shown}")),
            "the README no longer shows: {shown}"
        );
    }
}

#[test]
fn a_bound_of_the_fields_own_kind_is_left_alone() {
    for field in [
        quote::quote! { #[model_schema_prop(minimum = 0, maximum = 9)] count: u32 },
        quote::quote! { #[model_schema_prop(minimum = 0.5)] ratio: Option<f64> },
        quote::quote! { #[model_schema_prop(maxLength = 9)] counts: Vec<i16> },
        quote::quote! { #[model_schema_prop(minLength = 3, pattern = "^[a-z]+$")] name: String },
        quote::quote! { #[model_schema_prop(maxLength = 9)] name: &'static str },
        quote::quote! { #[model_schema_prop(minLength = 3)] names: Vec<&'static str> },
        quote::quote! { #[model_schema_prop(minLength = 3)] path: PathBuf },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

/// A range is spelled against a number this crate writes itself.
#[test]
fn a_range_on_a_field_typed_with_a_named_type_is_refused() {
    for (constraint, key) in [
        (quote::quote! { minimum = 5 }, "minimum"),
        (quote::quote! { maximum = 5 }, "maximum"),
    ] {
        for field_type in [quote::quote! { Count }, quote::quote! { Option<Count> }] {
            let errors = field_prop_guard_errors(&syn::parse_quote! {
                struct Report {
                    #[model_schema_prop(#constraint)]
                    total: #field_type,
                }
            });
            assert_eq!(errors.len(), 1, "for {key} on {field_type}: {errors:?}");
            for needle in ["compile_error", "field `total`", key, "`Count`"] {
                assert!(
                    errors[0].contains(needle),
                    "{needle} missing for {key} on {field_type}: {}",
                    errors[0]
                );
            }
        }
    }
}

#[test]
fn a_length_on_a_field_typed_with_a_generic_named_type_is_refused() {
    for (constraint, key) in [
        (quote::quote! { minLength = 3 }, "minLength"),
        (quote::quote! { maxLength = 3 }, "maxLength"),
        (quote::quote! { pattern = "^[a-z]+$" }, "pattern"),
    ] {
        for field_type in [
            quote::quote! { UserId<String> },
            quote::quote! { Option<UserId<String>> },
        ] {
            let errors = field_prop_guard_errors(&syn::parse_quote! {
                struct Report {
                    #[model_schema_prop(#constraint)]
                    owner: #field_type,
                }
            });
            assert_eq!(errors.len(), 1, "for {key} on {field_type}: {errors:?}");
            for needle in ["compile_error", "field `owner`", key, "`UserId<"] {
                assert!(
                    errors[0].contains(needle),
                    "{needle} missing for {key} on {field_type}: {}",
                    errors[0]
                );
            }
        }
    }
}

#[test]
fn a_length_on_a_field_typed_with_a_named_type_is_left_to_the_compiler() {
    for field_type in [
        quote::quote! { Slug },
        quote::quote! { Option<Slug> },
        quote::quote! { Box<Slug> },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(minLength = 3, pattern = "^[a-z]+$")]
                slug: #field_type,
            }
        });
        assert!(errors.is_empty(), "for {field_type}: {errors:?}");
    }
}

/// Every spelling of a list a field is written as, holding each kind of item a bound on the item
/// itself is refused on.
fn list_spellings() -> Vec<proc_macro2::TokenStream> {
    vec![
        quote::quote! { Vec<String> },
        quote::quote! { Vec<u8> },
        quote::quote! { [f32; 2] },
        quote::quote! { Option<Vec<Part>> },
        quote::quote! { Vec<Vec<String>> },
        quote::quote! { Vec<UserId<String>> },
        quote::quote! { Vec<HashMap<String, u32>> },
        quote::quote! { Vec<(String, u32)> },
        quote::quote! { Vec<IdType> },
        quote::quote! { Option<Vec<IdType>> },
        quote::quote! { VecDeque<Slug> },
        quote::quote! { HashSet<Count> },
        quote::quote! { BTreeSet<String> },
        quote::quote! { BinaryHeap<u8> },
        quote::quote! { Box<[String]> },
        quote::quote! { &'static [u8] },
        #[cfg(feature = "chrono")]
        quote::quote! { Vec<chrono::NaiveDateTime> },
        #[cfg(feature = "mongodb")]
        quote::quote! { Vec<ObjectId> },
    ]
}

/// A length written on a list counts the list, which every item type leaves it free to do.
#[test]
fn a_length_on_a_list_is_left_alone_whatever_the_list_holds() {
    for constraint in [
        quote::quote! { minLength = 1 },
        quote::quote! { maxLength = 3 },
        quote::quote! { minLength = 1, maxLength = 3 },
    ] {
        for field_type in list_spellings() {
            let errors = field_prop_guard_errors_in_scope(
                &syn::parse_quote! {
                    struct Report {
                        #[model_schema_prop(#constraint)]
                        held: #field_type,
                    }
                },
                &["IdType".to_owned()],
            );
            assert!(
                errors.is_empty(),
                "for {constraint} on {field_type}: {errors:?}"
            );
        }
    }
}

/// A list has no text for a pattern and no number for a range, whatever it holds.
#[test]
fn a_pattern_or_a_range_on_a_list_is_refused() {
    let on_a_brand = "brand that carries the `pattern`";
    let no_place = "a range for each number of a list has no place to be written";
    for (constraint, key, advice) in [
        (
            quote::quote! { pattern = "^[a-z]+$" },
            "pattern",
            on_a_brand,
        ),
        (quote::quote! { minimum = 1 }, "minimum", no_place),
        (quote::quote! { maximum = 9 }, "maximum", no_place),
    ] {
        for field_type in list_spellings() {
            let errors = field_prop_guard_errors_in_scope(
                &syn::parse_quote! {
                    struct Report {
                        #[model_schema_prop(#constraint)]
                        held: #field_type,
                    }
                },
                &["IdType".to_owned()],
            );
            assert_eq!(errors.len(), 1, "for {key} on {field_type}: {errors:?}");
            for needle in [
                "compile_error",
                "field `held`",
                &format!("`{key}` cannot apply"),
                "this field writes a list",
                "`minLength` and `maxLength` count the list's items",
                advice,
            ] {
                assert!(
                    errors[0].contains(needle),
                    "{needle} missing for {key} on {field_type}: {}",
                    errors[0]
                );
            }
        }
    }
}

/// The refusal on a list names the keys a list cannot carry, and no length written beside them.
#[test]
fn the_refusal_on_a_list_names_the_keys_that_reach_nothing() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(minLength = 1, pattern = "^[a-z]+$", minimum = 1, maximum = 9)]
            tags: Vec<String>,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    for needle in [
        "field `tags`: `pattern`, `minimum`, `maximum` cannot apply to a `Vec<String>` field",
        "brand that carries the `pattern`",
        "a range for each number of a list has no place to be written",
    ] {
        assert!(errors[0].contains(needle), "{needle}: {}", errors[0]);
    }
}

#[test]
fn a_map_or_tuple_field_without_a_bound_is_left_alone() {
    for field in [
        quote::quote! { labels: HashMap<String, String> },
        quote::quote! { pair: (String, u32) },
        quote::quote! { #[model_schema_prop(preprocess = ["trim"])] pair: (String, u32) },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

#[test]
fn a_literal_whose_kind_the_field_cannot_carry_is_refused() {
    for (constraint, field_type, carrier) in [
        (
            quote::quote! { literal = true },
            quote::quote! { String },
            "bool",
        ),
        (
            quote::quote! { literal = false },
            quote::quote! { String },
            "bool",
        ),
        (
            quote::quote! { literal = "x" },
            quote::quote! { bool },
            "String",
        ),
        (
            quote::quote! { literal = 214 },
            quote::quote! { String },
            "numeric",
        ),
        (
            quote::quote! { literal = true },
            quote::quote! { i32 },
            "bool",
        ),
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(#constraint)]
                flag: #field_type,
            }
        });
        assert_eq!(
            errors.len(),
            1,
            "for {constraint} on {field_type}: {errors:?}"
        );
        for needle in ["compile_error", "field `flag`", "literal", carrier] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {constraint} on {field_type}: {}",
                errors[0]
            );
        }
    }
}

#[test]
fn a_literal_whose_kind_the_field_can_carry_is_left_alone() {
    for field in [
        quote::quote! { #[model_schema_prop(literal = true)] flag: bool },
        quote::quote! { #[model_schema_prop(literal = false)] flag: bool },
        quote::quote! { #[model_schema_prop(literal = "x")] flag: String },
        quote::quote! { #[model_schema_prop(literal = 214)] flag: i32 },
        quote::quote! { #[model_schema_prop(literal = 3.5)] flag: f64 },
        quote::quote! { #[model_schema_prop(literal = true)] flag: Option<bool> },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

/// A numeric literal no value of the field's number is would refuse every value it holds.
#[test]
fn a_literal_the_fields_number_holds_no_value_of_is_refused() {
    for (literal, field_type, number) in [
        (quote::quote! { 300 }, quote::quote! { u8 }, "u8"),
        (quote::quote! { -1 }, quote::quote! { Option<u32> }, "u32"),
        (quote::quote! { 1.5 }, quote::quote! { i64 }, "i64"),
        (quote::quote! { 128 }, quote::quote! { Vec<i8> }, "i8"),
        (quote::quote! { 16777217 }, quote::quote! { f32 }, "f32"),
    ] {
        let messages = field_prop_guard_messages(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(literal = #literal)]
                flag: #field_type,
            }
        });
        let written = literal.to_string().replace(' ', "");
        assert_eq!(
            messages,
            [format!(
                "model_schema: field `flag`: `literal = {written}` cannot apply to a `{number}` \
                 field — no `{number}` is {written}, so the literal would refuse every value the \
                 field holds. Write a literal a `{number}` holds, or declare the field as a \
                 number that holds {written}."
            )],
            "for {literal} on {field_type}"
        );
    }
}

/// A numeric literal is left alone where a value of the field's number is it.
#[test]
fn a_literal_the_fields_number_holds_is_left_alone() {
    for field in [
        quote::quote! { #[model_schema_prop(literal = 255)] flag: u8 },
        quote::quote! { #[model_schema_prop(literal = -128)] flag: i8 },
        quote::quote! { #[model_schema_prop(literal = 0.1)] flag: f32 },
        quote::quote! { #[model_schema_prop(literal = 1.25)] flag: Vec<f32> },
        quote::quote! { #[model_schema_prop(literal = 0.1)] flag: f64 },
        quote::quote! { #[model_schema_prop(literal = 4294967295)] flag: u32 },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

#[test]
fn a_literal_beside_a_rule_is_refused() {
    for (keys, field_type, written, rules) in [
        (
            quote::quote! { literal = "x", minLength = 3 },
            quote::quote! { String },
            "\"x\"",
            "minLength",
        ),
        (
            quote::quote! { literal = "x", maxLength = 9 },
            quote::quote! { String },
            "\"x\"",
            "maxLength",
        ),
        (
            quote::quote! { literal = "x", pattern = "^x$" },
            quote::quote! { String },
            "\"x\"",
            "pattern",
        ),
        (
            quote::quote! { literal = "x", check = ["exactly_x"] },
            quote::quote! { String },
            "\"x\"",
            "check",
        ),
        (
            quote::quote! { literal = 214, minimum = 5 },
            quote::quote! { u32 },
            "214",
            "minimum",
        ),
        (
            quote::quote! { literal = 214, maximum = 900 },
            quote::quote! { u32 },
            "214",
            "maximum",
        ),
        (
            quote::quote! { literal = true, check = ["fn_name"] },
            quote::quote! { bool },
            "true",
            "check",
        ),
        (
            quote::quote! { minLength = 1, literal = "x", pattern = "^x$", check = ["exactly_x"] },
            quote::quote! { Option<String> },
            "\"x\"",
            "minLength`, `pattern`, `check",
        ),
    ] {
        let messages = field_prop_guard_messages(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(#keys)]
                flag: #field_type,
            }
        });
        assert_eq!(
            messages.len(),
            1,
            "for {keys} on {field_type}: {messages:?}"
        );
        assert_eq!(
            messages[0],
            format!(
                "model_schema: field `flag`: `literal = {written}` takes no other rule, and \
                 `{rules}` is written beside it — a literal admits one value, so the rule is \
                 always met or never met. Drop `{rules}`, or drop the literal."
            ),
            "for {keys} on {field_type}"
        );
    }
    let shown = field_prop_guard_messages(&syn::parse_quote! {
        pub struct Envelope {
            #[model_schema_prop(literal = "v1", minLength = 3)]
            pub version: String,
        }
    });
    assert_eq!(shown.len(), 1, "got: {shown:?}");
    assert!(
        include_str!("../../README.md").contains(&format!("error: {}\n", shown[0])),
        "the README no longer shows the refusal: {}",
        shown[0]
    );
}

#[test]
fn a_literal_beside_a_key_that_holds_the_value_to_no_rule_is_left_alone() {
    for field in [
        quote::quote! { #[model_schema_prop(as = String, literal = "x")] flag: String },
        quote::quote! { #[model_schema_prop(literal = "x", preprocess = ["trimmed"])] flag: String },
        quote::quote! { #[model_schema_prop(literal = "x", ts_optional)] flag: Option<String> },
        quote::quote! { #[model_schema_prop(literal = "x", nullable)] flag: Option<String> },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

#[test]
fn a_bound_on_a_parameter_typed_field_is_refused() {
    for (constraint, key) in [
        (quote::quote! { minLength = 30 }, "minLength"),
        (quote::quote! { maxLength = 30 }, "maxLength"),
        (quote::quote! { pattern = "^[a-z]+$" }, "pattern"),
        (quote::quote! { minimum = 5 }, "minimum"),
        (quote::quote! { maximum = 5 }, "maximum"),
    ] {
        for field_type in [quote::quote! { IdType }, quote::quote! { Option<IdType> }] {
            let errors = field_prop_guard_errors_in_scope(
                &syn::parse_quote! {
                    struct Report {
                        #[model_schema_prop(#constraint)]
                        labels: #field_type,
                    }
                },
                &["IdType".to_owned()],
            );
            assert_eq!(errors.len(), 1, "for {key} on {field_type}: {errors:?}");
            for needle in [
                "compile_error",
                "field `labels`",
                key,
                "type parameter",
                "IdType",
                "brand",
            ] {
                assert!(
                    errors[0].contains(needle),
                    "{needle} missing for {key} on {field_type}: {}",
                    errors[0]
                );
            }
        }
    }
}

#[test]
fn a_parameter_in_scope_only_refuses_the_field_that_carries_a_bound() {
    for (field, parameters) in [
        (quote::quote! { labels: IdType }, &["IdType".to_owned()][..]),
        (
            quote::quote! { #[model_schema_prop(preprocess = ["trim"])] labels: IdType },
            &["IdType".to_owned()][..],
        ),
        (
            quote::quote! { #[model_schema_prop(minLength = 30)] labels: String },
            &["IdType".to_owned()][..],
        ),
        (
            quote::quote! { #[model_schema_prop(minLength = 30)] labels: IdType },
            &[][..],
        ),
    ] {
        let errors = field_prop_guard_errors_in_scope(
            &syn::parse_quote! {
                struct Report {
                    #field
                }
            },
            parameters,
        );
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

/// The docs a field's meta earns, read off the walk that writes them, inside an item declaring
/// `parameters`.
fn field_docs_after_meta_in_scope(item: &syn::ItemStruct, parameters: &[String]) -> String {
    let field = item.fields.iter().next().unwrap();
    let name = mapped_or_default(field.ident.as_ref(), ToString::to_string);
    let mut field_def = get_field_def(&name, &field.ty, "");
    field_def.erase_type_parameters(parameters);
    let meta = super::parse_model_schema_prop_attributes(&field.attrs);
    super::apply_model_schema_prop_meta(&mut field_def, meta, &name);
    field_def.docs
}

fn field_docs_after_meta(item: &syn::ItemStruct) -> String {
    field_docs_after_meta_in_scope(item, &[])
}

#[test]
fn the_constraint_docs_are_written_only_where_the_bound_is_kept() {
    assert!(
        field_docs_after_meta(&syn::parse_quote! {
            struct Report {
                #[model_schema_prop(minLength = 30)]
                name: String,
            }
        })
        .contains("Minimum length: 30"),
        "a bound the surfaces render says so in the docs"
    );
    for field in [
        quote::quote! { #[model_schema_prop(minLength = 30)] labels: HashMap<String, String> },
        quote::quote! { #[model_schema_prop(maximum = 5)] pair: (u32, u32) },
    ] {
        let docs = field_docs_after_meta(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(docs.is_empty(), "for {field}, got: {docs}");
    }
}

#[test]
fn the_constraint_docs_are_silent_for_a_bound_of_the_wrong_kind() {
    for field in [
        quote::quote! { #[model_schema_prop(minLength = 3, maxLength = 9)] count: u32 },
        quote::quote! { #[model_schema_prop(minLength = 3)] count: Option<Box<f64>> },
        quote::quote! { #[model_schema_prop(minimum = 5, maximum = 9)] name: String },
        quote::quote! { #[model_schema_prop(minimum = 5)] name: &'static str },
        quote::quote! { #[model_schema_prop(minimum = 5, maximum = 9)] counts: Vec<f64> },
        quote::quote! { #[model_schema_prop(minimum = 5)] flag: bool },
        quote::quote! { #[model_schema_prop(maximum = 5)] letter: char },
    ] {
        let docs = field_docs_after_meta(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(docs.is_empty(), "for {field}, got: {docs}");
    }

    let on_a_number = field_docs_after_meta(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(minLength = 3, minimum = 5)]
            count: u32,
        }
    });
    assert!(on_a_number.contains("Minimum value: 5"), "{on_a_number}");
    assert!(!on_a_number.contains("Minimum length"), "{on_a_number}");

    let on_a_borrowed_string = field_docs_after_meta(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(minLength = 3, minimum = 5)]
            name: &'static str,
        }
    });
    assert!(
        on_a_borrowed_string.contains("Minimum length: 3"),
        "{on_a_borrowed_string}"
    );
    assert!(
        !on_a_borrowed_string.contains("Minimum value"),
        "{on_a_borrowed_string}"
    );
}

/// A list's count is stated in the words a string's length is, whatever the list holds.
#[test]
fn the_constraint_docs_state_the_count_a_list_is_held_to() {
    for field_type in list_spellings() {
        let docs = field_docs_after_meta_in_scope(
            &syn::parse_quote! {
                struct Report {
                    #[model_schema_prop(minLength = 1, maxLength = 3, minimum = 5)]
                    held: #field_type,
                }
            },
            &["IdType".to_owned()],
        );
        assert_eq!(
            docs, " * held\n * \n * Minimum length: 1\n * Maximum length: 3",
            "for {field_type}"
        );
    }
}

#[test]
fn the_constraint_docs_are_silent_for_a_parameter_typed_field() {
    let parameters = ["IdType".to_owned()];
    for field in [
        quote::quote! { #[model_schema_prop(minLength = 30)] id: IdType },
        quote::quote! { #[model_schema_prop(maximum = 5)] id: Option<IdType> },
        quote::quote! { #[model_schema_prop(minimum = 5)] id: Vec<IdType> },
    ] {
        let docs = field_docs_after_meta_in_scope(
            &syn::parse_quote! {
                struct Report {
                    #field
                }
            },
            &parameters,
        );
        assert!(docs.is_empty(), "for {field}, got: {docs}");
    }

    assert!(
        field_docs_after_meta_in_scope(
            &syn::parse_quote! {
                struct Report {
                    #[model_schema_prop(minLength = 30)]
                    id: String,
                }
            },
            &parameters,
        )
        .contains("Minimum length: 30"),
        "a bound the surfaces render says so in the docs, parameters in scope or not"
    );
}

/// `u64` and `usize` have no Kotlin mapping, refused the same way Swift's own width refusal is.
#[cfg(feature = "kotlin")]
#[test]
fn a_kotlin_field_reaching_u64_or_usize_is_refused() {
    for (ty, width) in [
        (quote::quote! { u64 }, "u64"),
        (quote::quote! { usize }, "usize"),
        (quote::quote! { Vec<u64> }, "u64"),
        (quote::quote! { HashMap<String, usize> }, "usize"),
        (quote::quote! { (String, u64) }, "u64"),
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                count: #ty,
            }
        });
        assert_eq!(errors.len(), 1, "for {ty}: {errors:?}");
        let message = format!(
            "field `count`: `{width}` has no Kotlin mapping; the Kotlin target refuses unsigned \
             64-bit and pointer-sized integers"
        );
        assert!(
            errors[0].contains(&message),
            "for {ty}, expected {message:?}, got: {}",
            errors[0]
        );
    }
}

/// A member serde neither writes nor reads is declared by no Kotlin type, so its width is refused
/// nothing; one under a single one-way skip is still refused.
#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_width_guard_reads_past_a_member_serde_neither_writes_nor_reads() {
    for item in [
        syn::parse_quote! { struct Report { #[serde(skip)] count: u64 } },
        syn::parse_quote! {
            struct Report { #[serde(skip_serializing, skip_deserializing)] count: usize }
        },
    ] {
        let errors = field_prop_guard_errors(&item);
        assert!(errors.is_empty(), "got: {errors:?}");
    }
    for item in [
        syn::parse_quote! { struct Report { #[serde(skip_serializing)] count: u64 } },
        syn::parse_quote! { struct Report { #[serde(skip_deserializing)] count: u64 } },
    ] {
        let errors = field_prop_guard_errors(&item);
        assert_eq!(errors.len(), 1, "got: {errors:?}");
        assert!(
            errors[0].contains("`u64` has no Kotlin mapping"),
            "got: {}",
            errors[0]
        );
    }
}

/// Through the whole expansion, no guard of a member's type refuses a member serde neither writes
/// nor reads, nor a member of a variant it neither writes nor reads: a width neither client maps,
/// a std type no schema describes, a map keyed by a value serde writes as no string, and a JSON
/// value. Each is refused where serde writes it.
#[cfg(all(feature = "serde", feature = "kotlin", feature = "swift"))]
#[test]
fn no_guard_of_a_members_type_refuses_what_serde_neither_writes_nor_reads() {
    let expanded = |source: &str| {
        super::exec_model_schema(
            proc_macro2::TokenStream::new(),
            syn::parse_str(&format!("#[derive(Serialize, Deserialize)] {source}")).unwrap(),
        )
    };
    for source in [
        "pub struct ProbeUnwiredHold { pub note: String, #[serde(skip)] pub count: u64, \
         #[serde(skip)] pub listed: std::collections::LinkedList<String>, #[serde(skip)] pub \
         platform: std::ffi::OsString, #[serde(skip)] pub keyed: \
         std::collections::BTreeMap<(u8, u8), u32>, #[serde(skip)] pub held: serde_json::Value }",
        "pub enum ProbeUnwiredTally { Open { note: String }, #[serde(skip)] Closed { count: u64, \
         listed: std::collections::LinkedList<String>, keyed: std::collections::BTreeMap<(u8, \
         u8), u32>, held: serde_json::Value } }",
    ] {
        let tokens = expanded(source);
        assert_eq!(
            compile_error_count(&tokens),
            0,
            "for {source}, got: {tokens}"
        );
    }
    for (member, refusal) in [
        ("pub count: u64", "`u64` has no Swift mapping"),
        ("pub count: u64", "`u64` has no Kotlin mapping"),
        (
            "pub listed: std::collections::LinkedList<String>",
            "reaches `LinkedList`",
        ),
        (
            "pub keyed: std::collections::BTreeMap<(u8, u8), u32>",
            "a map key must be a value serde writes as a string",
        ),
        ("pub held: serde_json::Value", "reaches `serde_json::Value`"),
    ] {
        let source = format!(
            "pub struct ProbeHalfWiredHold {{ pub note: String, #[serde(skip_deserializing)] \
             {member} }}"
        );
        let tokens = expanded(&source).to_string();
        assert!(tokens.contains(refusal), "for {source}, got: {tokens}");
    }
}

/// A field written at one of the enclosing item's own type parameters is never refused.
#[cfg(feature = "kotlin")]
#[test]
fn a_kotlin_type_parameter_is_never_refused_even_when_it_could_be_filled_with_u64() {
    let errors = field_prop_guard_errors_in_scope(
        &syn::parse_quote! {
            struct Wrapper<T> {
                value: T,
            }
        },
        &["T".to_owned()],
    );
    assert_eq!(errors.len(), 0, "got: {errors:?}");
}

#[test]
fn an_as_naming_another_type_is_refused() {
    let errors = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(as = String)]
            id: i64,
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    for needle in ["compile_error", "field `id`", "as = String", "i64"] {
        assert!(
            errors[0].contains(needle),
            "{needle} missing: {}",
            errors[0]
        );
    }
}

#[test]
fn an_as_naming_the_rendered_type_is_accepted() {
    for field in [
        quote::quote! { #[model_schema_prop(as = String)] name: String },
        quote::quote! { #[model_schema_prop(as = String)] name: Option<String> },
        quote::quote! { #[model_schema_prop(as = String)] name: Vec<String> },
        quote::quote! { #[model_schema_prop(as = Vec<String>)] name: Vec<String> },
        quote::quote! { #[model_schema_prop(as = Inner)] name: Option<Inner> },
        quote::quote! { #[model_schema_prop(as = String)] name: PathBuf },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

#[test]
fn the_field_prop_misuses_leave_by_the_guard_channel() {
    for (field, needle) in [
        (
            quote::quote! { #[model_schema_prop(ts_optional)] name: String },
            "requires an Option<T> field",
        ),
        (
            quote::quote! { #[model_schema_prop(as_number)] name: u32 },
            "requires a chrono DateTime<Tz> field",
        ),
        (
            quote::quote! { #[model_schema_prop(as = String, preprocess = ["trim"])] name: String },
            "cannot be written on the same field",
        ),
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert_eq!(errors.len(), 1, "for {field}: {errors:?}");
        for expected in ["compile_error", "field `name`", needle] {
            assert!(
                errors[0].contains(expected),
                "{expected} missing for {field}: {}",
                errors[0]
            );
        }
    }
}

#[test]
fn the_flag_leaves_by_the_guard_channel_where_it_has_no_key_to_make_optional() {
    let slot = field_prop_guard_errors(&syn::parse_quote! {
        struct Report(#[model_schema_prop(ts_optional)] Option<String>);
    });
    assert_eq!(slot.len(), 1, "{slot:?}");
    for expected in ["compile_error", "tuple field", "requires a named field"] {
        assert!(
            slot[0].contains(expected),
            "{expected} missing: {}",
            slot[0]
        );
    }

    let off_the_wire = field_prop_guard_errors(&syn::parse_quote! {
        struct Report {
            #[model_schema_prop(ts_optional)]
            #[serde(skip)]
            name: Option<String>,
        }
    });
    assert_eq!(off_the_wire.len(), 1, "{off_the_wire:?}");
    for expected in [
        "compile_error",
        "field `name`",
        "requires a field the wire carries",
    ] {
        assert!(
            off_the_wire[0].contains(expected),
            "{expected} missing: {}",
            off_the_wire[0]
        );
    }
}

/// The valid spellings of the same three keys stay valid.
#[test]
fn the_field_prop_flags_on_the_shapes_they_fit_are_accepted() {
    for field in [
        quote::quote! { #[model_schema_prop(ts_optional)] name: Option<String> },
        quote::quote! {
            #[model_schema_prop(ts_optional)]
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<String>
        },
        quote::quote! { #[model_schema_prop(preprocess = ["trim"])] name: String },
        quote::quote! { #[model_schema_prop(as = String, minLength = 1)] name: String },
    ] {
        let errors = field_prop_guard_errors(&syn::parse_quote! {
            struct Report {
                #field
            }
        });
        assert!(errors.is_empty(), "for {field}: {errors:?}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn cfg_attr_without_serde_leaves_a_field_alone() {
    let error = field_cfg_attr_error(&syn::parse_quote! {
        struct Report {
            #[cfg_attr(feature = "serde", doc = "only documented in serde builds")]
            #[serde(skip_serializing_if = "Option::is_none")]
            note: Option<String>,
        }
    });
    assert!(error.is_none(), "got: {error:?}");
}

/// Reports whether `tokens` contains a `#[cfg(...)]` / `#![cfg(...)]` attribute at any nesting
/// depth.
fn contains_cfg_attribute(tokens: proc_macro2::TokenStream) -> bool {
    let mut in_attr_prefix = false;
    for tree in tokens {
        match tree {
            proc_macro2::TokenTree::Punct(punct) => {
                let ch = punct.as_char();
                in_attr_prefix = ch == '#' || (in_attr_prefix && ch == '!');
            }
            proc_macro2::TokenTree::Group(group) => {
                let is_cfg_attr = in_attr_prefix
                    && group.delimiter() == proc_macro2::Delimiter::Bracket
                    && matches!(
                        group.stream().into_iter().next(),
                        Some(proc_macro2::TokenTree::Ident(ident)) if ident == "cfg"
                    );
                if is_cfg_attr || contains_cfg_attribute(group.stream()) {
                    return true;
                }
                in_attr_prefix = false;
            }
            proc_macro2::TokenTree::Ident(_) | proc_macro2::TokenTree::Literal(_) => {
                in_attr_prefix = false;
            }
        }
    }
    false
}

/// A `cfg` attribute in the macro's output is resolved against the *consumer's* feature table,
/// not tixschema's, so an item emitted per tixschema's features can call a method the consumer
/// cfg'd away. Every feature decision must be made while building the tokens, never emitted.
fn assert_no_cfg_attribute(tokens: &proc_macro2::TokenStream, what: &str) {
    assert!(
        !contains_cfg_attribute(tokens.clone()),
        "{what} emitted a cfg attribute into generated code: {tokens}"
    );
}

#[test]
fn the_cfg_probe_sees_an_emitted_cfg_attribute() {
    assert!(contains_cfg_attribute(quote::quote! {
        #[cfg(feature = "zod")]
        pub fn zod_schema() -> String { String::new() }
    }));
    assert!(contains_cfg_attribute(quote::quote! {
        pub fn wrapper() {
            #[cfg(feature = "zod")]
            let _ = 1;
        }
    }));
    assert_no_cfg_attribute(
        &quote::quote! {
            pub fn zod_schema() -> String { String::new() }
        },
        "a cfg-free token stream",
    );
}

#[cfg(feature = "zod")]
#[test]
fn struct_schema_example_carries_no_cfg_attribute() {
    let name: syn::Ident = syn::parse_quote!(Report);
    let generics = syn::Generics::default();
    let example: proc_macro2::TokenStream = "Report { id: 1 }".parse().unwrap();
    let tokens = super::item_schema_example_method(
        from_ref(&example),
        &name,
        &generics,
        &super::ModelSchemaArgs::default(),
    )
    .unwrap();
    assert_no_cfg_attribute(&tokens, "item_schema_example_method");
}

#[cfg(feature = "zod")]
#[test]
fn struct_schema_example_instantiates_every_type_parameter() {
    let name: syn::Ident = syn::parse_quote!(Report);
    let example: proc_macro2::TokenStream = "Report { id: 1 }".parse().unwrap();
    for (generics, expected) in [
        (syn::Generics::default(), "let value : Report ="),
        (
            syn::parse_quote!(<A>),
            "let value : Report < :: std :: string :: String > =",
        ),
        (
            syn::parse_quote!(<A, B>),
            "let value : Report < :: std :: string :: String , :: std :: string :: String > =",
        ),
        (syn::parse_quote!(<'a>), "let value : Report ="),
    ] {
        let rendered = super::item_schema_example_method(
            from_ref(&example),
            &name,
            &generics,
            &super::ModelSchemaArgs::default(),
        )
        .unwrap()
        .to_string();
        assert!(rendered.contains(expected), "Got: {rendered}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn struct_schema_example_instantiates_each_parameter_at_its_declared_filling() {
    let name: syn::Ident = syn::parse_quote!(Report);
    let example: proc_macro2::TokenStream = "Report { id: 1 }".parse().unwrap();
    let generics: syn::Generics = syn::parse_quote!(<A, B>);
    let count: (syn::Ident, syn::Type) = (syn::parse_quote!(A), syn::parse_quote!(u32));
    let held: (syn::Ident, syn::Type) = (syn::parse_quote!(B), syn::parse_quote!(Vec<u8>));
    for (default_types, expected) in [
        (
            vec![count.clone()],
            "let value : Report < u32 , :: std :: string :: String > =",
        ),
        (
            vec![held.clone()],
            "let value : Report < :: std :: string :: String , Vec < u8 > > =",
        ),
        (
            vec![held, count],
            "let value : Report < u32 , Vec < u8 > > =",
        ),
    ] {
        let args = super::ModelSchemaArgs {
            default_types,
            ..Default::default()
        };
        let rendered =
            super::item_schema_example_method(from_ref(&example), &name, &generics, &args)
                .unwrap()
                .to_string();
        assert!(rendered.contains(expected), "Got: {rendered}");
    }
}

/// A filling written as `String` is annotated as its author wrote it.
#[cfg(feature = "zod")]
#[test]
fn an_unfilled_parameter_falls_back_to_the_standard_string_by_its_full_path() {
    let name: syn::Ident = syn::parse_quote!(Report);
    let example: proc_macro2::TokenStream = "Report { id: 1 }".parse().unwrap();
    let generics: syn::Generics = syn::parse_quote!(<A>);
    let filled = super::ModelSchemaArgs {
        default_types: vec![(syn::parse_quote!(A), syn::parse_quote!(String))],
        ..Default::default()
    };
    let render = |args: &super::ModelSchemaArgs| {
        super::item_schema_example_method(from_ref(&example), &name, &generics, args)
            .unwrap()
            .to_string()
    };
    let written = render(&filled);
    assert!(
        written.contains("let value : Report < String > ="),
        "got: {written}"
    );
    let fallen_back = render(&super::ModelSchemaArgs::default());
    assert!(
        fallen_back.contains("let value : Report < :: std :: string :: String > ="),
        "got: {fallen_back}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn every_block_is_built_in_a_scope_of_its_own_and_the_first_is_the_example() {
    use crate::features::js_literal;

    let name: syn::Ident = syn::parse_quote!(Report);
    let blocks: [proc_macro2::TokenStream; 2] = [
        "Report { id: 1 }".parse().unwrap(),
        "Report { id: 2 }".parse().unwrap(),
    ];
    let rendered = super::item_schema_example_method(
        &blocks,
        &name,
        &syn::Generics::default(),
        &super::ModelSchemaArgs::default(),
    )
    .unwrap()
    .to_string();
    let writer = js_literal::writer();
    let written = quote::quote! {
        pub fn schema_examples() -> ::std::vec::Vec<serde_json::Value> {
            ::std::vec![
                {
                    let value: Report = { Report { id: 1 } };
                    serde_json::to_value(&value).unwrap()
                },
                {
                    let value: Report = { Report { id: 2 } };
                    serde_json::to_value(&value).unwrap()
                }
            ]
        }

        fn schema_examples_zod_text() -> ::std::string::String {
            #writer
            let texts: ::std::vec::Vec<::std::string::String> = ::std::vec![
                {
                    let value: Report = { Report { id: 1 } };
                    tixschema_bigint_writer::text(&value)
                },
                {
                    let value: Report = { Report { id: 2 } };
                    tixschema_bigint_writer::text(&value)
                }
            ];
            ::std::format!("[{}]", texts.join(","))
        }

        pub fn schema_example() -> serde_json::Value {
            Self::schema_examples().swap_remove(0)
        }
    }
    .to_string();
    assert_eq!(rendered, written);
}

#[cfg(feature = "zod")]
#[test]
fn a_type_that_writes_no_block_gets_no_example_method() {
    let name: syn::Ident = syn::parse_quote!(Report);
    assert!(
        super::item_schema_example_method(
            &[],
            &name,
            &syn::Generics::default(),
            &super::ModelSchemaArgs::default(),
        )
        .is_none()
    );
    assert!(
        super::build_branded_schema_example(&[], &name, &[], &super::ModelSchemaArgs::default())
            .is_empty()
    );
}

/// A crate that lists no `serde_json` keeps building where its types write no example.
#[cfg(feature = "zod")]
#[test]
fn a_type_that_writes_no_block_names_no_serde_json_for_its_empty_list() {
    let module: syn::Ident = syn::parse_quote!(report_schema);
    let zod_delegates: Vec<String> = [
        super::build_struct_delegate_items(&module, "Report", "Report", &[], None),
        super::build_struct_delegate_items(
            &module,
            "Report",
            "Report",
            &["IdType".to_owned()],
            None,
        ),
        super::build_branded_delegate_items(&module, false),
    ]
    .into_iter()
    .flatten()
    .map(|item| item.to_string())
    .filter(|item| item.contains("fn zod_schema"))
    .collect();

    assert_eq!(zod_delegates.len(), 3, "{zod_delegates:?}");
    for delegate in zod_delegates {
        assert!(
            delegate.contains("let examples_json = \"[]\" ;"),
            "Got: {delegate}"
        );
        assert!(!delegate.contains("serde_json"), "Got: {delegate}");
        assert!(!delegate.contains("schema_example"), "Got: {delegate}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_type_that_writes_a_block_reads_its_list_off_schema_examples() {
    let module: syn::Ident = syn::parse_quote!(report_schema);
    let methods = quote::quote! { pub fn schema_examples() {} };
    let zod_delegates: Vec<String> = [
        super::build_struct_delegate_items(&module, "Report", "Report", &[], Some(&methods)),
        super::build_branded_delegate_items(&module, true),
    ]
    .into_iter()
    .flatten()
    .map(|item| item.to_string())
    .filter(|item| item.contains("fn zod_schema"))
    .collect();

    assert_eq!(zod_delegates.len(), 2, "{zod_delegates:?}");
    for delegate in zod_delegates {
        assert!(
            delegate.contains("let examples_json = Self :: schema_examples_zod_text () ;"),
            "Got: {delegate}"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn declared_default_renders_each_shape_the_table_describes() {
    register_alias_info(
        "DocumentId",
        "DocumentId",
        "document_id_schema",
        AliasKind::NoEnumMembers,
    );
    super::record_zod_factory("DocumentId", true);
    super::record_zod_default_arguments("DocumentId", vec!["z.string()".to_owned()]);

    for (parameter, filled_at, expected) in [
        ("IdType", quote::quote! { String }, "z.string()"),
        ("DateType", quote::quote! { f64 }, "z.number()"),
        (
            "IdType",
            quote::quote! { DocumentId<String> },
            "z.lazy(() => DocumentId$SchemaDefault)",
        ),
    ] {
        let ty: syn::Type = syn::parse2(filled_at.clone()).unwrap();
        let default_types = vec![(
            syn::Ident::new(parameter, proc_macro2::Span::call_site()),
            ty,
        )];
        let field = super::declared_default_field(parameter, &default_types);
        assert_eq!(
            super::default_zod_rendering(&field).into_argument(),
            expected,
            "for `{parameter} = {filled_at}`"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn declared_default_renders_each_wrapped_shape_the_table_describes() {
    register_alias_info(
        "DocumentId",
        "DocumentId",
        "document_id_schema",
        AliasKind::NoEnumMembers,
    );
    super::record_zod_factory("DocumentId", true);
    super::record_zod_default_arguments("DocumentId", vec!["z.string()".to_owned()]);

    for (parameter, filled_at, expected) in [
        (
            "IdType",
            quote::quote! { Vec<DocumentId<String>> },
            "z.lazy(() => z.array(DocumentId$SchemaFactory(z.string())))",
        ),
        (
            "IdType",
            quote::quote! { Option<DocumentId<String>> },
            "z.lazy(() => z.nullable(DocumentId$SchemaFactory(z.string())))",
        ),
        (
            "IdType",
            quote::quote! { Vec<String> },
            "z.array(z.string())",
        ),
        (
            "IdType",
            quote::quote! { HashMap<String, DocumentId<String>> },
            "z.lazy(() => z.record(z.string(), DocumentId$SchemaFactory(z.string())))",
        ),
        (
            "IdType",
            quote::quote! { (String, DocumentId<String>) },
            "z.lazy(() => z.tuple([z.string(), DocumentId$SchemaFactory(z.string())]))",
        ),
    ] {
        let ty: syn::Type = syn::parse2(filled_at.clone()).unwrap();
        let default_types = vec![(
            syn::Ident::new(parameter, proc_macro2::Span::call_site()),
            ty,
        )];
        let field = super::declared_default_field(parameter, &default_types);
        assert_eq!(
            super::default_zod_rendering(&field).into_argument(),
            expected,
            "for `{parameter} = {filled_at}`"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_default_naming_a_sibling_at_other_than_its_own_default_calls_the_factory() {
    register_alias_info(
        "DocumentId",
        "DocumentId",
        "document_id_schema",
        AliasKind::NoEnumMembers,
    );
    super::record_zod_factory("DocumentId", true);
    super::record_zod_default_arguments("DocumentId", vec!["z.string()".to_owned()]);

    let ty: syn::Type = syn::parse_quote!(DocumentId<u32>);
    let default_types = vec![(syn::parse_quote!(IdType), ty)];
    let field = super::declared_default_field("IdType", &default_types);
    assert_eq!(
        super::default_zod_rendering(&field).into_argument(),
        "z.lazy(() => DocumentId$SchemaFactory(z.number().int().min(0).max(4294967295)))"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_parameter_with_no_declared_default_falls_back_to_string() {
    let field = super::declared_default_field("IdType", &[]);
    assert_eq!(
        super::default_zod_rendering(&field).into_argument(),
        "z.string()"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn branded_json_schema_method_carries_no_cfg_attribute() {
    let args = super::ModelSchemaArgs::default();
    for inner in branded_json_inners() {
        let tokens = super::build_branded_json_schema_method(&args, &inner, "DocumentId", &[]);
        assert_no_cfg_attribute(&tokens, "build_branded_json_schema_method");
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_brand_over_an_unrenderable_slot_is_refused() {
    let inner = get_field_def(
        "_inner",
        &syn::parse_str("HashMap<String, HashMap<(u32, u32), u32>>").unwrap(),
        "",
    );
    let rendered =
        super::branded_slot_json_schema(&super::ModelSchemaArgs::default(), &inner, "DocumentId")
            .to_string();
    assert!(
        rendered.starts_with(":: core :: compile_error !"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains(&format!("model_schema: `DocumentId`: {TUPLE_KEY_REFUSAL}")),
        "got: {rendered}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_brand_inner_points_at_the_written_inner() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct DocumentId(pub HashMap<String, HashMap<(u32, u32), u32>>);")
            .unwrap();
    let field = item.fields.iter().next().unwrap();
    let inner = get_field_def("_inner", &field.ty, "");
    assert_points_only_at(
        &super::branded_slot_json_schema(&super::ModelSchemaArgs::default(), &inner, "DocumentId"),
        "(u32, u32)",
        "a brand's inner",
    );
}

/// Every shape [`super::branded_json_inner`] resolves to, so the dispatch is covered whole. The
/// `Slot` and `Chrono` shapes build their bodies through [`super::branded_slot_json_schema`] and
/// [`super::branded_chrono_schema`], which is where a stray `cfg` attribute would land unseen.
#[cfg(feature = "jsonschema")]
fn branded_json_inners() -> Vec<super::BrandedJsonInner> {
    let composite: syn::Type = syn::parse_quote!(Vec<String>);
    vec![
        #[cfg(feature = "chrono")]
        super::BrandedJsonInner::Chrono(super::ChronoText::Format("date-time")),
        #[cfg(feature = "chrono")]
        super::BrandedJsonInner::Chrono(super::ChronoText::Pattern("^\\d$".to_owned())),
        #[cfg(feature = "mongodb")]
        super::BrandedJsonInner::ObjectId,
        super::BrandedJsonInner::Scalar("string".to_owned()),
        super::BrandedJsonInner::Slot(Box::new(super::get_field_def("_inner", &composite, ""))),
    ]
}

#[cfg(feature = "zod")]
#[test]
fn branded_schema_example_carries_no_cfg_attribute() {
    let name: syn::Ident = syn::parse_quote!(DocumentId);
    let example: proc_macro2::TokenStream = "DocumentId(\"abc\".to_string())".parse().unwrap();
    for generic_params in [
        Vec::new(),
        vec!["A".to_owned()],
        vec!["A".to_owned(), "B".to_owned()],
    ] {
        let tokens = super::build_branded_schema_example(
            from_ref(&example),
            &name,
            &generic_params,
            &super::ModelSchemaArgs::default(),
        );
        assert_no_cfg_attribute(&tokens, "build_branded_schema_example");
    }
}

#[cfg(feature = "zod")]
#[test]
fn branded_schema_example_instantiates_every_parameter() {
    let name: syn::Ident = syn::parse_quote!(DocumentId);
    let example: proc_macro2::TokenStream = "DocumentId(\"abc\".to_string())".parse().unwrap();
    for (generic_params, expected) in [
        (Vec::new(), "let value : DocumentId ="),
        (
            vec!["A".to_owned()],
            "let value : DocumentId < :: std :: string :: String > =",
        ),
        (
            vec!["A".to_owned(), "B".to_owned()],
            "let value : DocumentId < :: std :: string :: String , :: std :: string :: String > =",
        ),
    ] {
        let rendered = super::build_branded_schema_example(
            from_ref(&example),
            &name,
            &generic_params,
            &super::ModelSchemaArgs::default(),
        )
        .to_string();
        assert!(rendered.contains(expected), "Got: {rendered}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn branded_schema_example_instantiates_each_parameter_at_its_declared_filling() {
    let name: syn::Ident = syn::parse_quote!(DocumentId);
    let example: proc_macro2::TokenStream = "DocumentId(\"abc\".to_string())".parse().unwrap();
    let args = super::ModelSchemaArgs {
        default_types: vec![(syn::parse_quote!(A), syn::parse_quote!(u32))],
        ..Default::default()
    };
    let rendered = super::build_branded_schema_example(
        from_ref(&example),
        &name,
        &["A".to_owned(), "B".to_owned()],
        &args,
    )
    .to_string();
    assert!(
        rendered.contains("let value : DocumentId < u32 , :: std :: string :: String > ="),
        "Got: {rendered}"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn plain_enum_ts_definition_carries_no_cfg_attribute() {
    let tokens = super::generate_plain_enum_ts_definition_method(
        " * Status",
        "Status",
        "Status",
        "",
        "  'a'",
        "export function Status$Variant(value: unknown): string { return \"\"; }",
    );
    assert_no_cfg_attribute(&tokens, "generate_plain_enum_ts_definition_method");
}

#[cfg(feature = "typescript")]
#[test]
fn discriminated_enum_ts_definition_carries_no_cfg_attribute() {
    let tokens = super::generate_discriminated_enum_ts_definition_method(
        " * Shape",
        "Shape",
        "Shape",
        "",
        "  'a'",
        "export function Shape$Variant(value: unknown): string { return \"\"; }",
    );
    assert_no_cfg_attribute(&tokens, "generate_discriminated_enum_ts_definition_method");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn alias_json_schema_method_carries_no_cfg_attribute() {
    let alias: syn::ItemType = syn::parse_quote!(
        pub type AliasIdent = String;
    );
    let field_def = super::get_field_def("AliasType", &alias.ty, "");
    let tokens = super::generate_alias_json_schema_method(
        &alias,
        "AliasType",
        &field_def,
        &super::ModelSchemaArgs::default(),
    );
    assert_no_cfg_attribute(&tokens, "generate_alias_json_schema_method");
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_alias_of_an_unrenderable_target_emits_only_the_compile_error() {
    for alias_source in [
        "pub type Rows = HashMap<String, HashMap<(u32, u32), u32>>;",
        "pub type Rows = Vec<HashMap<String, HashMap<(u32, u32), u32>>>;",
        "pub type Rows = (String, HashMap<(u32, u32), u32>);",
    ] {
        let alias: syn::ItemType = syn::parse_str(alias_source).unwrap();
        let field_def = super::get_field_def("RowsType", &alias.ty, "");
        let tokens = super::generate_alias_json_schema_method(
            &alias,
            "RowsType",
            &field_def,
            &super::ModelSchemaArgs::default(),
        )
        .to_string();
        assert!(
            tokens.contains("compile_error !"),
            "for {alias_source}, got: {tokens}"
        );
        assert!(
            tokens.contains(&format!(
                "model_schema: type alias `Rows`: {TUPLE_KEY_REFUSAL}"
            )),
            "for {alias_source}, got: {tokens}"
        );
        assert!(
            !tokens.contains("type alias `RowsType`"),
            "for {alias_source}, got: {tokens}"
        );
        assert!(
            tokens.contains("= :: core :: compile_error !"),
            "the description is the diagnostic, not a schema: for {alias_source}, got: {tokens}"
        );
        assert!(
            !tokens.contains("\"type\""),
            "no schema for the rejected target is left beside the diagnostic: \
             for {alias_source}, got: {tokens}"
        );
    }
}

/// The caret has to land on the tokens the author edits.
#[cfg(feature = "jsonschema")]
#[test]
fn an_alias_rejection_points_at_the_written_target() {
    let alias: syn::ItemType = syn::parse_str("pub type Rows = HashMap<(u32, u32), u32>;").unwrap();
    let field_def = super::get_field_def("RowsType", &alias.ty, "");
    let tokens = super::generate_alias_json_schema_method(
        &alias,
        "RowsType",
        &field_def,
        &super::ModelSchemaArgs::default(),
    );
    let located = located_source_texts(&tokens);
    assert!(
        located.iter().any(|text| text == "HashMap"),
        "got: {located:?}"
    );
}

#[test]
fn a_reworded_refusal_keeps_both_ends_of_what_it_is_spanned_on() {
    let written: syn::Type = syn::parse_str("&'static str").unwrap();
    let rejection = syn::Error::new_spanned(&written, "refused");
    let tokens = super::attr_guard_error(&rejection, "type `Labelled`");
    let located = located_source_texts(&tokens);
    assert_eq!(located.first().map(String::as_str), Some("&"));
    assert_eq!(located.last().map(String::as_str), Some("str"));
    assert_eq!(
        tokens.to_string(),
        ":: core :: compile_error ! { \"model_schema: type `Labelled`: refused\" }"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_data_suffixed_alias_rejection_names_its_written_ident() {
    let alias: syn::ItemType = syn::parse_quote! {
        pub type RowsData = HashMap<(u32, u32), u32>;
    };
    let field_def = super::get_field_def("RowsType", &alias.ty, "");
    let tokens = super::generate_alias_json_schema_method(
        &alias,
        "RowsType",
        &field_def,
        &super::ModelSchemaArgs::default(),
    )
    .to_string();
    assert!(tokens.contains("type alias `RowsData`"), "got: {tokens}");
    assert!(!tokens.contains("type alias `RowsType`"), "got: {tokens}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_alias_type_parameter_is_erased_at_every_depth() {
    for alias_source in [
        "pub type Holder<V> = V;",
        "pub type Holder<V> = Vec<V>;",
        "pub type Holder<V> = Option<V>;",
        "pub type Holder<V> = (String, V);",
        "pub type Holder<V> = HashMap<String, V>;",
        "pub type Holder<V> = HashMap<String, Vec<V>>;",
    ] {
        let alias: syn::ItemType = syn::parse_str(alias_source).unwrap();
        let field_def = super::get_field_def("HolderType", &alias.ty, "");
        let tokens = super::generate_alias_json_schema_method(
            &alias,
            "HolderType",
            &field_def,
            &super::ModelSchemaArgs::default(),
        )
        .to_string();
        assert!(
            !tokens.contains("_schema :: Schema ::"),
            "for {alias_source}, got: {tokens}"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn an_alias_type_parameter_is_erased_at_every_depth_on_the_value_surface() {
    for alias_source in [
        "pub type Holder<V> = V;",
        "pub type Holder<V> = Vec<V>;",
        "pub type Holder<V> = Option<V>;",
        "pub type Holder<V> = (String, V);",
        "pub type Holder<V> = HashMap<String, V>;",
        "pub type Holder<V> = HashMap<String, Vec<V>>;",
    ] {
        let alias: syn::ItemType = syn::parse_str(alias_source).unwrap();
        let field_def = super::get_field_def("HolderType", &alias.ty, "");
        let tokens =
            super::generate_alias_zod_method(&alias, "HolderType", "Holder", &field_def, &[])
                .to_string();
        assert!(
            !tokens.contains("V$Schema"),
            "for {alias_source}, got: {tokens}"
        );
        assert!(
            tokens.contains("HolderType$SchemaFactory"),
            "for {alias_source}, got: {tokens}"
        );
    }
}

#[test]
fn no_json_schema_emission_carries_a_warning_key() {
    for (file, source) in [
        ("model_schema.rs", include_str!("../model_schema.rs")),
        (
            "features/jsonschema.rs",
            include_str!("../features/jsonschema.rs"),
        ),
    ] {
        assert!(!source.contains("\"warning\""), "in: {file}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn alias_zod_method_carries_no_cfg_attribute() {
    let alias: syn::ItemType = syn::parse_quote!(
        pub type Alias = String;
    );
    let ty: syn::Type = syn::parse_quote!(String);
    let field_def = super::get_field_def("AliasType", &ty, "");
    let tokens = super::generate_alias_zod_method(&alias, "AliasType", "Alias", &field_def, &[]);
    assert_no_cfg_attribute(&tokens, "generate_alias_zod_method");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn alias_ts_definition_method_carries_no_cfg_attribute() {
    let alias: syn::ItemType = syn::parse_quote!(
        /// An aliased identifier.
        pub type AliasIdent = String;
    );
    let ty: syn::Type = syn::parse_quote!(String);
    let field_def = super::get_field_def("AliasType", &ty, "");
    let tokens = super::generate_alias_ts_definition_method(&alias, "AliasType", &field_def);
    assert_no_cfg_attribute(&tokens, "generate_alias_ts_definition_method");
}

/// Collects a branded newtype's guard failures as rendered `compile_error!` token strings.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_errors(item: &syn::ItemStruct) -> Vec<String> {
    branded_errors_with(item, &super::ModelSchemaArgs::default())
}

/// [`branded_errors`] for a brand carrying `model_schema` arguments.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn branded_errors_with(item: &syn::ItemStruct, args: &super::ModelSchemaArgs) -> Vec<String> {
    branded_guard_errors(item, args)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_over_an_undescribable_std_inner_is_refused() {
    for (inner, named, wire) in [
        (quote::quote! { OnceLock<u32> }, "`OnceLock`", "Serialize"),
        (quote::quote! { OsString }, "`OsString`", "externally"),
        (
            quote::quote! { Vec<OnceLock<u32>> },
            "`OnceLock`",
            "Serialize",
        ),
        (
            quote::quote! { LinkedList<String> },
            "`LinkedList`",
            "Vec<T>",
        ),
    ] {
        let errors = branded_errors(&syn::parse_quote! {
            #[serde(transparent)]
            struct SlotId(pub #inner);
        });
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        for needle in ["compile_error", "model_schema: type `SlotId`", named, wire] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {inner}: {}",
                errors[0]
            );
        }
    }
}

/// A brand over an inner every surface describes earns nothing.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_over_a_describable_inner_is_left_alone() {
    for inner in [
        quote::quote! { String },
        quote::quote! { RefCell<u32> },
        quote::quote! { PathBuf },
    ] {
        let errors = branded_errors(&syn::parse_quote! {
            #[serde(transparent)]
            struct SlotId(pub #inner);
        });
        assert!(errors.is_empty(), "for {inner}, got: {errors:?}");
    }
}

/// The `model_schema` arguments of a brand carrying a lone `pattern`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn pattern_args() -> super::ModelSchemaArgs {
    super::parse_model_schema_args(quote::quote! { pattern = "^[a-z]+$" })
}

/// A brand's guard failures for the given `pattern`, over a `String` inner that clears every other
/// guard.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn brand_pattern_errors(pattern: &str) -> Vec<String> {
    branded_errors_with(
        &syn::parse_quote! {
            #[serde(transparent)]
            struct UserId(pub String);
        },
        &super::parse_model_schema_args(quote::quote! { pattern = #pattern }),
    )
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn the_brand_pattern_guard_follows_the_regex_crate() {
    for pattern in PROBE_PATTERNS {
        let rejected = regex::Regex::new(pattern).is_err();
        let errors = brand_pattern_errors(pattern);
        assert_eq!(
            errors.len(),
            usize::from(rejected),
            "for {pattern}, got: {errors:?}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_pattern_the_regex_crate_rejects_names_the_type_and_quotes_the_parse_error() {
    let errors = brand_pattern_errors(r"^ab\");
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    for needle in [
        "compile_error",
        "type `UserId`",
        "pattern",
        "regex parse error",
        "incomplete escape sequence",
    ] {
        assert!(
            errors[0].contains(needle),
            "{needle} missing: {}",
            errors[0]
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_pattern_javascript_cannot_carry_names_the_type_and_the_construct() {
    for (pattern, construct) in UNPORTABLE_PROBE_PATTERNS {
        let errors = brand_pattern_errors(pattern);
        assert_eq!(errors.len(), 1, "for {pattern}, got: {errors:?}");
        for needle in ["compile_error", "type `UserId`", construct] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {pattern}: {}",
                errors[0]
            );
        }
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_pattern_naming_a_group_the_rust_way_clears_the_guard() {
    let errors = brand_pattern_errors("^(?P<word>[a-z]+)$");
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_pattern_admitting_every_value_names_the_type_and_says_so() {
    for pattern in ["", "^", "$", "|", "a*", "^a*"] {
        let errors = brand_pattern_errors(pattern);
        assert_eq!(errors.len(), 1, "for {pattern:?}, got: {errors:?}");
        for needle in [
            "compile_error",
            "type `UserId`",
            "admits every value",
            "constrains nothing",
        ] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {pattern:?}: {}",
                errors[0]
            );
        }
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_pattern_that_is_one_assertion_and_nothing_else_names_the_type_and_says_so() {
    for pattern in [r"\b", r"\B"] {
        let errors = brand_pattern_errors(pattern);
        assert_eq!(errors.len(), 1, "for {pattern:?}, got: {errors:?}");
        for needle in [
            "compile_error",
            "type `UserId`",
            "one look-around assertion and nothing else",
            "clippy::trivial_regex",
        ] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {pattern:?}: {}",
                errors[0]
            );
        }
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_pattern_that_still_constrains_clears_the_guard() {
    for pattern in ["^$", "^a*$", r"\b\w+"] {
        let errors = brand_pattern_errors(pattern);
        assert!(errors.is_empty(), "for {pattern:?}, got: {errors:?}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_over_a_map_with_an_unwritable_key_is_refused() {
    register_alias_info("Doc", "Doc", "doc_schema", AliasKind::NoEnumMembers);
    for (inner, needle) in [
        (
            quote::quote! { HashMap<Doc, u32> },
            "a map key must be a plain",
        ),
        (quote::quote! { HashMap<Vec<Doc>, u32> }, "is a sequence of"),
        (
            quote::quote! { HashMap<(String, u32), u32> },
            "serde writes `(_, _)`",
        ),
        (quote::quote! { HashMap<Option<Doc>, u32> }, "Option<Doc>"),
        (
            quote::quote! { Vec<HashMap<Doc, u32>> },
            "a map key must be a plain",
        ),
    ] {
        let errors = branded_errors(&syn::parse_quote! {
            #[serde(transparent)]
            struct Wrap(pub #inner);
        });
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        assert!(
            errors[0].contains("compile_error"),
            "for {inner}: {}",
            errors[0]
        );
        assert!(
            errors[0].contains("model_schema: type `Wrap`"),
            "for {inner}: {}",
            errors[0]
        );
        assert!(errors[0].contains(needle), "for {inner}: {}", errors[0]);
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_over_a_writable_map_key_clears_the_guard() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for inner in [
        quote::quote! { String },
        quote::quote! { HashMap<String, u32> },
        quote::quote! { HashMap<Slot, u32> },
        quote::quote! { Vec<HashMap<String, u32>> },
    ] {
        let errors = branded_errors(&syn::parse_quote! {
            #[serde(transparent)]
            struct Wrap(pub #inner);
        });
        assert!(errors.is_empty(), "for {inner}, got: {errors:?}");
    }
}

/// A tuple as a map's value is described as its fixed array, as in every other position.
#[cfg(feature = "jsonschema")]
#[test]
fn a_brand_over_a_map_with_a_tuple_value_builds() {
    for inner in [
        quote::quote! { HashMap<String, (u32, u32)> },
        quote::quote! { Vec<HashMap<String, (u32, u32)>> },
        quote::quote! { (String, HashMap<String, (u32, u32)>) },
    ] {
        let errors = branded_errors(&syn::parse_quote! {
            #[serde(transparent)]
            struct Wrap(pub #inner);
        });
        assert!(errors.is_empty(), "for {inner}, got: {errors:?}");
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_brand_over_a_renderable_slot_clears_the_guard() {
    for inner in [
        quote::quote! { (String, u32) },
        quote::quote! { HashMap<String, u32> },
        quote::quote! { Vec<Vec<i32>> },
        quote::quote! { Vec<Option<String>> },
        quote::quote! { [u8; 4] },
    ] {
        let errors = branded_errors(&syn::parse_quote! {
            #[serde(transparent)]
            struct Wrap(pub #inner);
        });
        assert!(errors.is_empty(), "for {inner}, got: {errors:?}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn each_string_constraint_alone_rejects_a_numeric_inner() {
    for args in [
        quote::quote! { pattern = "^[a-z]+$" },
        quote::quote! { minLength = 3 },
        quote::quote! { maxLength = 8 },
    ] {
        let rendered = format!("{args}");
        let errors = branded_errors_with(
            &syn::parse_quote! {
                #[serde(transparent)]
                struct BadNum(pub u64);
            },
            &super::parse_model_schema_args(args),
        );
        assert_eq!(errors.len(), 1, "for {rendered}, got: {errors:?}");
        assert!(errors[0].contains("numeric"), "got: {}", errors[0]);
    }
}

/// The shapes whose surfaces read the string constraints as something other than a string check.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_a_non_string_inner_are_rejected() {
    for (inner, shape) in [
        ("u64", "numeric"),
        ("i32", "numeric"),
        ("f64", "numeric"),
        ("bool", "boolean"),
        ("Vec<String>", "`pattern` on a list"),
        ("[u8; 4]", "`pattern` on a list"),
        ("BTreeSet<String>", "`pattern` on a list"),
        ("BinaryHeap<String>", "`pattern` on a list"),
        ("HashSet<String>", "`pattern` on a list"),
        ("VecDeque<String>", "`pattern` on a list"),
        ("Box<[String]>", "`pattern` on a list"),
        ("RefCell<Vec<String>>", "container"),
        ("HashMap<String, String>", "container"),
        ("(String, String)", "container"),
    ] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let errors = branded_errors_with(
            &syn::parse_quote! {
                #[serde(transparent)]
                struct Branded(pub #ty);
            },
            &pattern_args(),
        );
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
        assert!(errors[0].contains("`Branded`"), "got: {}", errors[0]);
        assert!(errors[0].contains(shape), "for {inner}, got: {}", errors[0]);
    }
}

/// A brand over a list takes the two lengths, which count its items, whatever the list holds.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_length_over_a_list_inner_passes_and_a_pattern_beside_it_is_refused() {
    for inner in [
        "Vec<String>",
        "Vec<Part>",
        "[u8; 4]",
        "BTreeSet<String>",
        "BinaryHeap<u8>",
        "HashSet<String>",
        "VecDeque<Part>",
        "Vec<Vec<String>>",
        "Box<[String]>",
        "Arc<Vec<Part>>",
        "Vec<T>",
        "HashSet<T>",
    ] {
        let generics = if inner.contains("T>") { "<T>" } else { "" };
        let item: syn::ItemStruct = syn::parse_str(&format!(
            "#[serde(transparent)] struct Listed{generics}(pub {inner});"
        ))
        .unwrap();
        for lengths in [
            quote::quote! { minLength = 1 },
            quote::quote! { maxLength = 3 },
            quote::quote! { minLength = 1, maxLength = 3 },
        ] {
            let errors = branded_errors_with(&item, &super::parse_model_schema_args(lengths));
            assert!(errors.is_empty(), "for {inner}, got: {errors:?}");
        }
        let errors = branded_errors_with(
            &item,
            &super::parse_model_schema_args(quote::quote! { minLength = 1, pattern = "^[a-z]+$" }),
        );
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        for needle in [
            "branded newtype `Listed` writes `pattern` on a list",
            "`minLength` and `maxLength` count the list's items",
            "brand that carries the `pattern`",
        ] {
            assert!(errors[0].contains(needle), "{needle}: {}", errors[0]);
        }
    }
}

/// A list no transparent wrapper holds is refused the lengths, as it was.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_length_over_a_list_behind_a_cell_or_a_borrow_is_refused() {
    for inner in [
        "RefCell<Vec<String>>",
        "Mutex<Vec<u8>>",
        "&'static [String]",
    ] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let errors = branded_errors_with(
            &syn::parse_quote! {
                #[serde(transparent)]
                struct Listed(pub #ty);
            },
            &super::parse_model_schema_args(quote::quote! { minLength = 1 }),
        );
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        assert!(errors[0].contains("container"), "got: {}", errors[0]);
    }
}

/// The inners that carry the constraints faithfully.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_a_string_shaped_inner_pass() {
    for inner in [
        "String",
        "PathBuf",
        "ObjectId",
        "SomeOtherBrand",
        "U",
        "SomeWrapper<String>",
    ] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let errors = branded_errors_with(
            &syn::parse_quote! {
                #[serde(transparent)]
                struct Branded<T>(pub #ty);
            },
            &pattern_args(),
        );
        assert!(errors.is_empty(), "for {inner}, got: {errors:?}");
    }
}

/// Registers a name carrying the value surface a `#[model_schema()]` item's own expansion would
/// have recorded for it, standing in for that expansion.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn seed_value_shape(rust_ident: &str, shape: Option<&'static str>) {
    seed_published_shape(rust_ident, PublishedShape::Flat(shape));
}

/// The same, for a name whose own target is one of its parameters and which therefore records a
/// position rather than a word.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn seed_published_shape(rust_ident: &str, shape: PublishedShape) {
    register_alias_info(
        rust_ident,
        rust_ident,
        &ident_schema_module_name(rust_ident),
        AliasKind::NoEnumMembers,
    );
    record_value_shape(rust_ident, shape);
}

/// A brand's guard failures for a `pattern` over the named inner, spelled as a bare name.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn brand_over_named_inner_errors(inner: &str) -> Vec<String> {
    let ty: syn::Type = syn::parse_str(inner).unwrap();
    branded_errors_with(
        &syn::parse_quote! {
            #[serde(transparent)]
            struct Branded(pub #ty);
        },
        &pattern_args(),
    )
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_a_named_inner_the_registry_answers_for_are_rejected() {
    for (inner, shape) in [
        ("OpaqueSibling", "opaque"),
        ("NumericSibling", "numeric"),
        ("BooleanSibling", "boolean"),
        ("ContainerSibling", "container"),
        ("ObjectSibling", "object"),
        ("EnumeratedSibling", "enumerated"),
        ("UnionSibling", "union"),
        ("NullableSibling", "nullable"),
    ] {
        seed_value_shape(inner, Some(shape));
        let errors = brand_over_named_inner_errors(inner);
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
        assert!(errors[0].contains("`Branded`"), "got: {}", errors[0]);
        assert!(errors[0].contains(inner), "got: {}", errors[0]);
        assert!(errors[0].contains(shape), "for {inner}, got: {}", errors[0]);
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_a_named_inner_the_registry_calls_a_string_pass() {
    seed_value_shape("StringSibling", None);
    let errors = brand_over_named_inner_errors("StringSibling");
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_a_name_the_registry_cannot_answer_for_pass() {
    let bare = brand_over_named_inner_errors("SiblingRegisteredNowhere");
    assert!(bare.is_empty(), "got: {bare:?}");

    for inner in ["UnregisteredGeneric<String>", "UnregisteredGeneric<u32>"] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let carrying_a_fixed_argument = branded_errors_with(
            &syn::parse_quote! {
                #[serde(transparent)]
                struct Branded<T>(pub #ty);
            },
            &pattern_args(),
        );
        assert!(
            carrying_a_fixed_argument.is_empty(),
            "for {inner}, got: {carrying_a_fixed_argument:?}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_a_name_written_over_the_brands_own_parameter_are_rejected() {
    for inner in [
        "UnregisteredGeneric<T>",
        "UnregisteredGeneric<Vec<T>>",
        "UnregisteredGeneric<String, U>",
        "UnregisteredGeneric<HashMap<String, T>>",
        "UnregisteredGeneric<(u32, U)>",
    ] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let errors = branded_errors_with(
            &syn::parse_quote! {
                #[serde(transparent)]
                struct Branded<T, U>(pub #ty);
            },
            &pattern_args(),
        );
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
        assert!(errors[0].contains("`Branded`"), "got: {}", errors[0]);
        assert!(
            errors[0].contains("UnregisteredGeneric"),
            "for {inner}, got: {}",
            errors[0]
        );
        assert!(
            errors[0].contains("type parameter"),
            "for {inner}, got: {}",
            errors[0]
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_a_registered_name_carrying_the_brands_parameter_are_rejected() {
    seed_value_shape("RegisteredStringGeneric", None);
    let errors = branded_errors_with(
        &syn::parse_quote! {
            #[serde(transparent)]
            struct Branded<T>(pub RegisteredStringGeneric<T>);
        },
        &pattern_args(),
    );
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(
        errors[0].contains("RegisteredStringGeneric"),
        "got: {}",
        errors[0]
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_records_the_value_surface_its_inner_publishes() {
    seed_value_shape("RecordedOpaqueSibling", Some("opaque"));
    seed_value_shape("RecordedStringSibling", None);
    for (inner, expected) in [
        ("String", None),
        ("PathBuf", None),
        ("u32", Some("numeric")),
        ("bool", Some("boolean")),
        ("Vec<String>", Some("container")),
        ("(String, String)", Some("container")),
        ("RecordedOpaqueSibling", Some("opaque")),
        ("RecordedStringSibling", None),
        ("SiblingRegisteredNowhere", None),
    ] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let item: syn::ItemStruct = syn::parse_quote! {
            #[serde(transparent)]
            struct Branded(pub #ty);
        };
        assert_eq!(
            brand_surface(&item).shape,
            PublishedShape::Flat(expected),
            "for {inner}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_over_its_own_parameter_records_the_position_it_publishes() {
    for (inner, expected) in [
        ("TagType", PublishedShape::Parameter(0)),
        ("ValueType", PublishedShape::Parameter(1)),
        ("Vec<TagType>", PublishedShape::Flat(Some("container"))),
        (
            "HashMap<String, ValueType>",
            PublishedShape::Flat(Some("container")),
        ),
        (
            "(TagType, ValueType)",
            PublishedShape::Flat(Some("container")),
        ),
        ("Option<TagType>", PublishedShape::Flat(Some("nullable"))),
        ("String", PublishedShape::Flat(None)),
    ] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let item: syn::ItemStruct = syn::parse_quote! {
            #[serde(transparent)]
            struct Branded<TagType, ValueType>(pub #ty);
        };
        assert_eq!(brand_surface(&item).shape, expected, "for {inner}");
    }
}

/// The surface a brand's own registration records, built the way `register_branded_newtype` builds
/// it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn brand_surface(item: &syn::ItemStruct) -> super::Surface {
    super::Surface::written(
        &super::branded_inner_value_surface(&item.generics, item.fields.iter().next().unwrap()),
        &super::type_parameters_in_scope(&item.generics),
    )
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_a_parameter_publisher_read_the_argument_written_for_it() {
    seed_published_shape("PublishesItsParameter", PublishedShape::Parameter(0));
    let admitted = brand_over_named_inner_errors("PublishesItsParameter<String>");
    assert!(admitted.is_empty(), "got: {admitted:?}");

    for (inner, shape) in [
        ("PublishesItsParameter<u32>", "numeric"),
        ("PublishesItsParameter<bool>", "boolean"),
        ("PublishesItsParameter<Vec<String>>", "container"),
    ] {
        let errors = brand_over_named_inner_errors(inner);
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        assert!(errors[0].contains("`Branded`"), "got: {}", errors[0]);
        assert!(
            errors[0].contains("PublishesItsParameter"),
            "for {inner}, got: {}",
            errors[0]
        );
        assert!(errors[0].contains(shape), "for {inner}, got: {}", errors[0]);
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_recorded_position_is_read_off_the_arguments_the_reference_writes() {
    seed_published_shape("PublishesItsSecond", PublishedShape::Parameter(1));
    let errors = brand_over_named_inner_errors("PublishesItsSecond<String, u32>");
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("numeric"), "got: {}", errors[0]);

    let unwritten = brand_over_named_inner_errors("PublishesItsSecond<String>");
    assert!(unwritten.is_empty(), "got: {unwritten:?}");
}

/// The question a constrained brand leaves where the registry has no record, spelled as the brand's
/// own expansion would leave it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn ask_about(brand: &syn::ItemStruct) -> bool {
    deferred_shape_question(brand, &pattern_args()).is_some_and(|question| {
        record_shape_question(question);
        true
    })
}

/// A brand written over the given inner, under a name that is the same in both declaration orders
/// so the two refusals can be read against each other.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn brand_over(inner: &str) -> syn::ItemStruct {
    let ty: syn::Type = syn::parse_str(inner).unwrap();
    syn::parse_quote! {
        #[serde(transparent)]
        struct Branded(pub #ty);
    }
}

/// What the expansion registering a name emits for the questions asked about it, as rendered
/// `compile_error!` token strings.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn deferred_refusals_for(rust_ident: &str) -> Vec<String> {
    let name = syn::Ident::new(rust_ident, proc_macro2::Span::call_site());
    deferred_shape_refusals(Some(&name))
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_question_left_where_the_registry_was_silent_is_answered_by_the_later_registration() {
    let brand = brand_over("AnsweredLater<u32>");
    assert!(
        branded_errors_with(&brand, &pattern_args()).is_empty(),
        "the brand's own expansion has nothing to refuse it on"
    );
    assert!(ask_about(&brand));
    assert!(
        deferred_refusals_for("AnsweredLater").is_empty(),
        "nothing has registered under the name yet"
    );

    seed_published_shape("AnsweredLater", PublishedShape::Parameter(0));
    let errors = deferred_refusals_for("AnsweredLater");
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(errors[0].contains("`Branded`"), "got: {}", errors[0]);
    assert!(errors[0].contains("`AnsweredLater`"), "got: {}", errors[0]);
    assert!(errors[0].contains("numeric"), "got: {}", errors[0]);
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn both_orders_of_the_same_pair_refuse_in_one_wording() {
    assert!(ask_about(&brand_over("WordingBelow<u32>")));
    seed_published_shape("WordingBelow", PublishedShape::Parameter(0));
    let below = deferred_refusals_for("WordingBelow");
    assert_eq!(below.len(), 1, "got: {below:?}");

    seed_published_shape("WordingAbove", PublishedShape::Parameter(0));
    let above = brand_over_named_inner_errors("WordingAbove<u32>");
    assert_eq!(above.len(), 1, "got: {above:?}");

    assert_eq!(
        below[0].replace("WordingBelow", "Inner"),
        above[0].replace("WordingAbove", "Inner")
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_question_a_string_argument_answers_settles_silently() {
    assert!(ask_about(&brand_over("SettlesSilently<String>")));
    seed_published_shape("SettlesSilently", PublishedShape::Parameter(0));
    let errors = deferred_refusals_for("SettlesSilently");
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_question_no_registration_reaches_stays_unanswered() {
    assert!(ask_about(&brand_over("NeverRegisters<u32>")));
    let errors = deferred_refusals_for("NeverRegisters");
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_the_registry_answers_leaves_no_question() {
    seed_published_shape("AlreadyAnswering", PublishedShape::Parameter(0));
    for inner in ["AlreadyAnswering<u32>", "AlreadyAnswering<String>"] {
        assert!(
            deferred_shape_question(&brand_over(inner), &pattern_args()).is_none(),
            "for {inner}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_with_nothing_to_append_asks_nothing() {
    assert!(
        deferred_shape_question(
            &brand_over("Unasked<u32>"),
            &super::ModelSchemaArgs::default()
        )
        .is_none()
    );
    for inner in ["Vec<Unasked>", "BTreeSet<Unasked>", "String", "u32"] {
        assert!(
            deferred_shape_question(&brand_over(inner), &pattern_args()).is_none(),
            "for {inner}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_deferred_answer_reads_the_record_the_consult_would_have_read() {
    assert!(ask_about(&brand_over("FlatlyNumeric")));
    seed_value_shape("FlatlyNumeric", Some("numeric"));
    let flat = deferred_refusals_for("FlatlyNumeric");
    assert_eq!(flat.len(), 1, "got: {flat:?}");
    assert!(flat[0].contains("numeric"), "got: {}", flat[0]);

    assert!(ask_about(&brand_over("FlatlyString")));
    seed_value_shape("FlatlyString", None);
    assert_eq!(deferred_refusals_for("FlatlyString"), Vec::<String>::new());

    assert!(ask_about(&brand_over("PublishesUnwritten<String>")));
    seed_published_shape("PublishesUnwritten", PublishedShape::Parameter(1));
    assert_eq!(
        deferred_refusals_for("PublishesUnwritten"),
        Vec::<String>::new()
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_tuple_struct_records_the_value_surface_its_slots_publish() {
    for (decl, expected) in [
        ("struct Slots(pub String);", PublishedShape::Flat(None)),
        (
            "struct Slots(pub Option<String>);",
            PublishedShape::Flat(Some("nullable")),
        ),
        (
            "struct Slots(pub u32);",
            PublishedShape::Flat(Some("numeric")),
        ),
        (
            "struct Slots(pub Vec<String>);",
            PublishedShape::Flat(Some("container")),
        ),
        (
            "struct Slots(pub String, pub u32);",
            PublishedShape::Flat(Some("container")),
        ),
        ("struct Slots();", PublishedShape::Flat(Some("container"))),
        ("struct Slots<T>(pub T);", PublishedShape::Parameter(0)),
        (
            "struct Slots<T>(pub Vec<T>);",
            PublishedShape::Flat(Some("container")),
        ),
    ] {
        let item: syn::ItemStruct = syn::parse_str(decl).unwrap();
        assert_eq!(
            super::tuple_struct_surface(
                &item.fields,
                &super::type_parameters_in_scope(&item.generics)
            )
            .shape,
            expected,
            "for {decl}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_the_brands_own_type_parameter_consult_the_declared_default() {
    for inner in ["T", "U"] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let errors = branded_errors_with(
            &syn::parse_quote! {
                #[serde(transparent)]
                struct Branded<T, U>(pub #ty);
            },
            &pattern_args(),
        );
        assert!(errors.is_empty(), "for {inner}, got: {errors:?}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_the_brands_own_type_parameter_with_a_string_default_pass() {
    let errors = branded_errors_with(
        &syn::parse_quote! {
            #[serde(transparent)]
            struct Branded<T>(pub T);
        },
        &super::parse_model_schema_args(quote::quote! {
            pattern = "^[a-z]+$", default_types(T = String)
        }),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
}

/// A bare, a wrapped, and an unrelated parameter each substitute as expected.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn declared_defaults_are_substituted_through_the_inners_shape() {
    let parameters = ["IdType".to_owned()];
    let default_types = vec![(syn::parse_quote!(IdType), syn::parse_quote!(String))];

    let bare: syn::Type = syn::parse_quote!(IdType);
    assert_eq!(
        super::substitute_declared_defaults(&bare, &parameters, &default_types),
        syn::parse_quote!(String)
    );

    let wrapped: syn::Type = syn::parse_quote!(Box<IdType>);
    assert_eq!(
        super::substitute_declared_defaults(&wrapped, &parameters, &default_types),
        syn::parse_quote!(Box<String>)
    );

    let unrelated: syn::Type = syn::parse_quote!(u32);
    assert_eq!(
        super::substitute_declared_defaults(&unrelated, &parameters, &default_types),
        syn::parse_quote!(u32)
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn string_constraints_over_the_brands_own_type_parameter_with_a_non_string_default_are_rejected() {
    let errors = branded_errors_with(
        &syn::parse_quote! {
            #[serde(transparent)]
            struct Branded<T>(pub T);
        },
        &super::parse_model_schema_args(quote::quote! {
            pattern = "^[a-z]+$", default_types(T = u32)
        }),
    );
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    for needle in [
        "compile_error",
        "`Branded`",
        "`T`",
        "`u32`",
        "declared default",
    ] {
        assert!(
            errors[0].contains(needle),
            "{needle} missing: {}",
            errors[0]
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_unconstrained_brand_over_a_non_string_inner_passes() {
    for inner in [
        "u64",
        "bool",
        "Vec<String>",
        "BTreeSet<String>",
        "VecDeque<String>",
    ] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let errors = branded_errors(&syn::parse_quote! {
            #[serde(transparent)]
            struct Branded(pub #ty);
        });
        assert!(errors.is_empty(), "for {inner}, got: {errors:?}");
    }
}

/// The message has to name the surfaces that disagree, or it reads as an arbitrary restriction.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn the_constraint_guard_message_names_the_constraints_and_the_surfaces() {
    let errors = branded_errors_with(
        &syn::parse_quote! {
            #[serde(transparent)]
            struct BadNum(pub u64);
        },
        &pattern_args(),
    );
    for needle in ["pattern", "minLength", "maxLength", "Zod", "JSON Schema"] {
        assert!(
            errors[0].contains(needle),
            "{needle} missing: {}",
            errors[0]
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn cfg_attr_wrapped_serde_on_a_branded_type_is_rejected() {
    let errors = branded_errors(&syn::parse_quote! {
        #[serde(transparent)]
        #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
        struct UserId(pub String);
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(errors[0].contains("type `UserId`"), "got: {}", errors[0]);
    assert!(errors[0].contains("cfg_attr"), "got: {}", errors[0]);
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn cfg_attr_wrapped_serde_on_a_branded_inner_slot_is_rejected() {
    let errors = branded_errors(&syn::parse_quote! {
        #[serde(transparent)]
        struct UserId(#[cfg_attr(feature = "serde", serde(rename = "inner"))] pub String);
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(errors[0].contains("tuple field"), "got: {}", errors[0]);
    assert!(errors[0].contains("#[serde(...)]"), "got: {}", errors[0]);
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn branded_newtype_over_option_is_rejected() {
    let errors = branded_errors(&syn::parse_quote! {
        #[serde(transparent)]
        struct MaybeUserId(pub Option<String>);
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(errors[0].contains("Option"), "got: {}", errors[0]);
    assert!(errors[0].contains("null"), "got: {}", errors[0]);
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn generic_branded_newtype_over_option_is_rejected() {
    let errors = branded_errors(&syn::parse_quote! {
        #[serde(transparent)]
        struct MaybeId<T>(pub Option<T>);
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("Option"), "got: {}", errors[0]);
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn compliant_branded_newtype_passes_every_guard() {
    let errors = branded_errors(&syn::parse_quote! {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[serde(transparent)]
        struct UserId(#[cfg_attr(feature = "serde", doc = "documented in serde builds")] pub String);
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

#[test]
fn no_display_is_accepted_as_a_bare_flag_and_as_a_named_bool() {
    assert!(super::parse_model_schema_args(quote::quote! { no_display }).no_display);
    assert!(super::parse_model_schema_args(quote::quote! { no_display = true }).no_display);
    assert!(!super::parse_model_schema_args(quote::quote! { no_display = false }).no_display);
    assert!(!super::parse_model_schema_args(proc_macro2::TokenStream::new()).no_display);
}

#[test]
fn no_display_coexists_with_the_named_args() {
    let args = super::parse_model_schema_args(quote::quote! {
        name = "Slug", pattern = "^[a-z]+$", minLength = 1, maxLength = 8, no_display
    });
    assert_eq!(args.name_override.as_deref(), Some("Slug"));
    assert_eq!(args.pattern.as_deref(), Some("^[a-z]+$"));
    assert_eq!(args.min_length, Some(1));
    assert_eq!(args.max_length, Some(8));
    assert!(args.no_display);
}

/// The parser's refusal of `args`, rendered, or `None` when it read them whole.
fn args_rejection(args: proc_macro2::TokenStream) -> Option<String> {
    super::parse_model_schema_args(args)
        .arg_rejection
        .as_ref()
        .map(ToString::to_string)
}

/// The reported repro: a misspelled `name` compiled clean and emitted the unrenamed schema.
#[test]
fn a_misspelled_name_argument_is_refused_by_the_name_as_written() {
    let rejection = args_rejection(quote::quote! { nme = "Renamed" }).unwrap();
    assert!(rejection.contains("nme"), "got: {rejection}");
    assert!(rejection.contains("name"), "got: {rejection}");
}

#[test]
fn a_name_no_identifier_can_be_spelled_from_is_refused() {
    for value in [
        "",
        " ",
        "Not Valid",
        "9Leading",
        "Foo$Bar",
        "Foo::Bar",
        "\u{dc}n\u{ef}code",
    ] {
        let rejection = args_rejection(quote::quote! { name = #value });
        assert!(rejection.is_some(), "accepted `name = {value:?}`");
    }
}

#[test]
fn a_name_an_identifier_can_be_spelled_from_is_read() {
    for value in ["Slug", "_Slug", "Slug2", "slug_case", "S"] {
        let args = super::parse_model_schema_args(quote::quote! { name = #value });
        assert_eq!(
            args.arg_rejection.map(|e| e.to_string()),
            None,
            "for {value}"
        );
        assert_eq!(args.name_override.as_deref(), Some(value));
    }
}

#[test]
fn an_exported_name_can_carry_no_double_quote() {
    for value in ["Weird\"Name", "\"", "\"Quoted\""] {
        assert!(
            args_rejection(quote::quote! { name = #value }).is_some(),
            "accepted `name = {value:?}`"
        );
        assert!(
            syn::parse_str::<syn::Ident>(value).is_err(),
            "tokenized {value:?} as an ident"
        );
    }
    assert_eq!(
        syn::Ident::new_raw("type", proc_macro2::Span::call_site()).to_string(),
        "r#type"
    );
    // The unrenamed path takes the ident whole; the renamed path takes the
    // refused-unless-identifier override verbatim.
    assert_eq!(
        super::compute_item_export_name("PayloadData", None),
        "PayloadData"
    );
    assert_eq!(
        super::compute_item_export_name("Payload", Some("Renamed")),
        "Renamed"
    );
}

/// What each of `declarations` — a source and the `model_schema` arguments written on it — earns
/// for the name it publishes, rendered, and claimed in the order written, which is the order
/// `exec_model_schema` claims them in.
fn published_name_refusals(declarations: &[(&str, &str)]) -> Vec<Vec<String>> {
    declarations
        .iter()
        .map(|&(source, args)| {
            let item: syn::Item = syn::parse_str(source).unwrap();
            let parsed = super::parse_model_schema_args(syn::parse_str(args).unwrap());
            assert_eq!(
                parsed.arg_rejection.as_ref().map(ToString::to_string),
                None,
                "for {args}"
            );
            super::published_name_collision_errors(&item, &parsed)
                .iter()
                .map(ToString::to_string)
                .collect()
        })
        .collect()
}

#[test]
fn a_second_declaration_publishing_a_taken_name_is_refused() {
    let refusals = published_name_refusals(&[
        (
            "pub struct FirstUnderRustName { pub label: String }",
            "name = \"OneSharedName\"",
        ),
        (
            "pub enum SecondUnderRustName { One }",
            "name = \"OneSharedName\"",
        ),
    ]);
    assert!(refusals[0].is_empty(), "got: {:?}", refusals[0]);
    assert_eq!(refusals[1].len(), 1, "got: {:?}", refusals[1]);
    for needle in [
        "compile_error",
        "OneSharedName",
        "FirstUnderRustName",
        "SecondUnderRustName",
    ] {
        assert!(
            refusals[1][0].contains(needle),
            "{needle} missing: {}",
            refusals[1][0]
        );
    }
}

#[test]
fn an_override_reaching_an_undeclared_items_own_name_is_refused() {
    let refusals = published_name_refusals(&[
        ("pub struct PlainlyNamed { pub label: String }", ""),
        (
            "pub struct AliasedOntoPlain { pub label: String }",
            "name = \"PlainlyNamed\"",
        ),
        ("pub struct TakenByOverride { pub label: String }", ""),
    ]);
    assert!(refusals[0].is_empty(), "got: {:?}", refusals[0]);
    assert_eq!(refusals[1].len(), 1, "got: {:?}", refusals[1]);
    assert!(refusals[2].is_empty(), "got: {:?}", refusals[2]);
}

#[test]
fn a_declaration_reclaiming_the_name_it_holds_is_not_refused() {
    let source = "pub struct ReadTwice { pub label: String }";
    for refusal in published_name_refusals(&[(source, ""), (source, "")]) {
        assert!(refusal.is_empty(), "got: {refusal:?}");
    }
}

#[test]
fn an_alias_claims_the_suffixed_name_it_publishes() {
    let refusals = published_name_refusals(&[
        ("pub type Measure = u32;", ""),
        ("pub struct Measure { pub label: String }", ""),
        (
            "pub struct Aliased { pub label: String }",
            "name = \"MeasureType\"",
        ),
    ]);
    assert!(refusals[0].is_empty(), "got: {:?}", refusals[0]);
    assert!(refusals[1].is_empty(), "got: {:?}", refusals[1]);
    assert_eq!(refusals[2].len(), 1, "got: {:?}", refusals[2]);
    assert!(
        refusals[2][0].contains("MeasureType"),
        "got: {}",
        refusals[2][0]
    );
}

#[test]
fn no_argument_the_parser_reads_is_rejected() {
    let probes: [proc_macro2::TokenStream; 9] = [
        quote::quote! { name = "Renamed" },
        quote::quote! { pattern = "^[a-z]+$" },
        quote::quote! { minLength = 1 },
        quote::quote! { maxLength = 50 },
        quote::quote! { no_display },
        quote::quote! { default_types(IdType = String) },
        quote::quote! { enable = ["decode_with"] },
        quote::quote! { check = ["withinParcel"] },
        quote::quote! { preprocess = ["withoutFragile"] },
    ];
    assert_eq!(probes.len(), super::KNOWN_ARGS.len());

    let offered = args_rejection(quote::quote! { bogus_flag }).unwrap();
    for name in super::KNOWN_ARGS {
        assert!(offered.contains(name), "{name} not offered: {offered}");
    }
    for probe in probes {
        let rendered = probe.to_string();
        assert_eq!(args_rejection(probe), None, "for {rendered}");
    }
}

/// `enable` turns on each entry it lists, in any order, and nothing where it is not written.
#[test]
fn enable_turns_on_the_entries_it_lists() {
    for (written, arky, decode_with, mongodb) in [
        ("", false, false, false),
        ("enable = [\"decode_with\"]", false, true, false),
        ("enable = [\"decode_with\", \"mongodb\"]", false, true, true),
        ("enable = [\"mongodb\", \"decode_with\"]", false, true, true),
        ("enable = [\"decode_with\",]", false, true, false),
        (
            "name = \"Renamed\", enable = [\"decode_with\"]",
            false,
            true,
            false,
        ),
        ("enable = [\"arky\"]", true, false, false),
        (
            "enable = [\"decode_with\", \"arky\", \"mongodb\"]",
            true,
            true,
            true,
        ),
    ] {
        let args = super::parse_model_schema_args(syn::parse_str(written).unwrap());
        assert!(args.arg_rejection.is_none(), "for {written}");
        assert_eq!(args.enable.arky, arky, "for {written}");
        assert_eq!(args.enable.decode_with, decode_with, "for {written}");
        assert_eq!(args.enable.mongodb, mongodb, "for {written}");
    }
}

/// `"arky"` writes the two conversions under the `arky` feature, and nothing without it.
#[test]
fn the_arky_entry_writes_both_conversions_under_its_feature() {
    let expanded = expansion_under(
        "enable = [\"arky\"]",
        "pub struct Note { pub title: String }",
    );
    assert!(!expanded.contains("compile_error"), "got: {expanded}");
    for call in [
        "arky :: value :: to_value (value)",
        "arky :: value :: from_value (value)",
    ] {
        assert_eq!(
            expanded.contains(call),
            cfg!(feature = "arky"),
            "for {call}, got: {expanded}"
        );
    }
}

/// A type alias names a type the conversions would have to be written for, in every build.
#[test]
fn the_arky_entry_is_refused_on_a_type_alias() {
    let expanded = expansion_under("enable = [\"arky\"]", "pub type Short = String;");
    assert!(expanded.contains("compile_error"), "got: {expanded}");
    assert!(
        expanded
            .contains("cannot be written on a type alias: `Short` is another name for `String`"),
        "got: {expanded}"
    );
}

/// The refusal `written` earns, with the text it is spanned on.
fn enable_refusal(written: &str) -> (String, String) {
    let rejection = super::parse_model_schema_args(syn::parse_str(written).unwrap())
        .arg_rejection
        .unwrap();
    (
        rejection.to_string(),
        rejection.span().source_text().unwrap_or_default(),
    )
}

/// The bare flag is refused, however it is written, with the line that replaces it.
#[test]
fn the_bare_decode_with_flag_is_refused_with_the_enable_list_it_moved_to() {
    for written in ["decode_with", "decode_with = true", "decode_with = false"] {
        assert_eq!(
            enable_refusal(written),
            (
                "tixschema: `decode_with` is now an entry of the `enable` list: write \
                 `#[model_schema(enable = [\"decode_with\"])]`"
                    .to_owned(),
                written.to_owned(),
            ),
            "for {written}"
        );
    }
}

/// `"mongodb"` alone is refused, spanned on the entry, with the list that is meant.
#[test]
fn enable_refuses_mongodb_without_decode_with() {
    assert_eq!(
        enable_refusal("enable = [\"mongodb\"]"),
        (
            "tixschema: `\"mongodb\"` requires `\"decode_with\"`: write `enable = \
             [\"decode_with\", \"mongodb\"]`"
                .to_owned(),
            "\"mongodb\"".to_owned(),
        )
    );
}

/// An entry the list does not read is refused, spanned on it, with the two it reads.
#[test]
fn enable_refuses_an_unknown_entry() {
    for (written, entry) in [
        ("enable = [\"decode_wth\"]", "\"decode_wth\""),
        ("enable = [\"decode_with\", \"bson\"]", "\"bson\""),
        ("enable = [\"decode_with\", \"Mongodb\"]", "\"Mongodb\""),
    ] {
        assert_eq!(
            enable_refusal(written),
            (
                format!(
                    "tixschema: unknown entry `{entry}`; the entries are `\"arky\"`, \
                     `\"decode_with\"` and `\"mongodb\"`"
                ),
                entry.to_owned(),
            ),
            "for {written}"
        );
    }
}

/// An entry written twice is refused, spanned on the second.
#[test]
fn enable_refuses_an_entry_written_twice() {
    for (written, entry) in [
        ("enable = [\"decode_with\", \"decode_with\"]", "decode_with"),
        (
            "enable = [\"decode_with\", \"mongodb\", \"mongodb\"]",
            "mongodb",
        ),
    ] {
        let rejection = super::parse_model_schema_args(syn::parse_str(written).unwrap())
            .arg_rejection
            .unwrap();
        let quoted = format!("\"{entry}\"");
        assert_eq!(
            rejection.to_string(),
            format!("tixschema: `{quoted}` is written twice"),
            "for {written}"
        );
        assert_eq!(
            rejection.span().start().column,
            written.rfind(&quoted).unwrap(),
            "for {written}"
        );
    }
}

/// An empty list is refused, spanned on it.
#[test]
fn enable_refuses_an_empty_list() {
    assert_eq!(
        enable_refusal("enable = []"),
        (
            "tixschema: the list is empty; remove `enable`".to_owned(),
            "[]".to_owned()
        )
    );
}

/// `enable` written in any form but a list of string literals is refused with the form it takes.
#[test]
fn enable_refuses_what_is_no_list_of_string_literals() {
    for (written, spanned) in [
        ("enable", "enable"),
        ("enable(decode_with)", "enable(decode_with)"),
        ("enable = \"decode_with\"", "\"decode_with\""),
        ("enable = [decode_with]", "decode_with"),
        ("enable = [\"decode_with\", 3]", "3"),
    ] {
        assert_eq!(
            enable_refusal(written),
            (
                "`model_schema` argument `enable` takes a list of string literals, written \
                 `enable = [\"decode_with\"]` or `enable = [\"decode_with\", \"mongodb\"]`"
                    .to_owned(),
                spanned.to_owned(),
            ),
            "for {written}"
        );
    }
}

/// A refusal of `enable` refuses the item before it is expanded: no method is written for it.
#[test]
fn a_refused_enable_list_writes_nothing_of_the_recovering_decode() {
    for args in [
        "decode_with",
        "enable = [\"mongodb\"]",
        "enable = [\"decode_wth\"]",
        "enable = [\"decode_with\", \"decode_with\"]",
        "enable = []",
    ] {
        let expanded = expansion_under(args, "pub struct InvoiceLookup { pub number: String }");
        assert!(expanded.contains("compile_error"), "for {args}: {expanded}");
        assert!(
            !expanded.contains("from_value_with"),
            "for {args}: {expanded}"
        );
    }
}

/// What the two initializers write in this build, as text.
#[cfg(feature = "serde")]
fn both_libraries() -> String {
    let mut written = exec_decode_with_foundation(&quote::quote! {});
    written.extend(exec_mongodb_foundation(&quote::quote! {}));
    written.to_string()
}

/// The whole expansion of `source` under `#[model_schema(...)]` written with `args`.
fn expansion_under(args: &str, source: &str) -> String {
    super::exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    )
    .to_string()
}

/// A transparent struct that serde writes without a key it still requires on read is refused only
/// when it derives both directions.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn a_one_direction_transparent_struct_is_not_refused_for_a_skipped_key() {
    let body = "#[serde(transparent)] pub struct Received { #[serde(default)] pub count: u32, \
                #[serde(skip_serializing)] pub text: String }";
    for derives in ["Debug, Deserialize", "Debug, Serialize"] {
        let expanded = expansion_under("", &format!("#[derive({derives})] {body}"));
        assert!(!expanded.contains("compile_error"), "{derives}: {expanded}");
    }
    let expanded = expansion_under(
        "",
        &format!("#[derive(Debug, Serialize, Deserialize)] {body}"),
    );
    assert!(expanded.contains("compile_error"), "got: {expanded}");
}

/// How many times `tokens` write `name` as an identifier, the text of a literal aside.
fn identifiers_named(tokens: proc_macro2::TokenStream, name: &str) -> usize {
    tokens
        .into_iter()
        .map(|tree| match tree {
            proc_macro2::TokenTree::Ident(ident) => usize::from(ident == name),
            proc_macro2::TokenTree::Group(group) => identifiers_named(group.stream(), name),
            proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => 0,
        })
        .sum()
}

/// Holds the expansion of `source` under `#[model_schema(args)]` to one `compile_error!`, which
/// says `refusal`, and to no `#[model_schema_prop]` left for rustc to report beside it.
fn assert_refused_without_member_keys(args: &str, source: &str, refusal: &str) {
    let expanded = super::exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    );
    let written = expanded.to_string();
    assert!(written.contains(refusal), "for {source}, got: {written}");
    assert_eq!(
        identifiers_named(expanded.clone(), "compile_error"),
        1,
        "for {source}, got: {written}"
    );
    assert_eq!(
        identifiers_named(expanded, "model_schema_prop"),
        0,
        "for {source}, got: {written}"
    );
}

#[test]
fn an_item_refused_before_its_members_are_walked_is_re_emitted_without_their_keys() {
    for (args, source, refusal) in [
        (
            "unread = 1",
            "pub struct Label { #[model_schema_prop(maxLength = 9)] pub text: String }",
            "type `Label`: unknown `model_schema` argument `unread`",
        ),
        (
            "default_types(IdType = String)",
            "pub struct Label { #[model_schema_prop(maxLength = 9)] pub text: String }",
            "`default_types` entry `IdType` names a type parameter, but this item declares none",
        ),
        (
            "",
            "pub enum Size { Empty(), Named { #[model_schema_prop(maxLength = 9)] label: String } }",
            "variant `Empty`: `Empty()` is a tuple variant with no field",
        ),
        (
            "default_types(IdType = String)",
            "pub enum Size { Named { #[model_schema_prop(maxLength = 9)] label: String }, Bare }",
            "`default_types` entry `IdType` names a type parameter, but this item declares none",
        ),
        (
            "enable = [\"decode_with\"]",
            "pub struct Label<'text> { #[model_schema_prop(maxLength = 9)] pub text: String, pub \
             held: &'text str }",
            "cannot be written on a type that borrows",
        ),
        (
            "minLength = 1",
            "pub struct Label { #[model_schema_prop(maxLength = 9)] pub text: String }",
            "type `Label`: `minLength` is supported only on a branded newtype",
        ),
        (
            "minLength = 2",
            "pub enum Size { S, Named { #[model_schema_prop(maxLength = 9)] label: String } }",
            "type `Size`: variant `S` is written `S`, which breaks `minLength = 2` written on the \
             enum",
        ),
        (
            "minLength = 1",
            "pub enum Shape { Circle { #[model_schema_prop(minimum = 1)] r: u32 } }",
            "type `Shape`: `minLength` reaches nothing here",
        ),
        (
            "minLength = 2",
            "pub enum Shape { Fixed { #[model_schema_prop(literal = \"x\")] word: String } }",
            "type `Shape`: `minLength` reaches nothing here",
        ),
    ] {
        assert_refused_without_member_keys(args, source, refusal);
    }
}

#[test]
fn an_item_refused_for_a_name_another_publishes_is_re_emitted_without_its_members_keys() {
    let first = expansion_under(
        "name = \"Parcel\"",
        "pub struct FirstParcel { pub text: String }",
    );
    assert!(!first.contains("compile_error"), "got: {first}");
    assert_refused_without_member_keys(
        "name = \"Parcel\"",
        "pub struct SecondParcel { #[model_schema_prop(maxLength = 9)] pub text: String }",
        "type `SecondParcel` publishes as `Parcel`, which type `FirstParcel` already publishes as",
    );
}

#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[test]
fn an_item_refused_for_a_serde_attribute_is_re_emitted_without_its_members_keys() {
    assert_refused_without_member_keys(
        "",
        "pub struct Label { #[model_schema_prop(maxLength = 9)] #[serde(rename(serialize = \
         \"label\", deserialize = \"text\"))] pub text: String }",
        "field `text`: serde `rename` names `label` when serializing and `text` when deserializing",
    );
}

#[cfg(feature = "zod")]
#[test]
fn an_item_refused_for_an_example_beside_a_const_is_re_emitted_without_its_members_keys() {
    assert_refused_without_member_keys(
        "",
        &format!(
            "{EXAMPLE_DOC_BLOCK}pub struct Label<const SLOTS: usize> {{ \
             #[model_schema_prop(maxLength = 9)] pub text: String }}"
        ),
        "type `Label`: `#[model_schema]` cannot build a `schema_example()` for an item that \
         declares a const parameter",
    );
}

#[cfg(any(feature = "typescript", feature = "jsonschema"))]
#[test]
fn an_item_refused_for_a_const_argument_is_re_emitted_without_its_members_keys() {
    assert_refused_without_member_keys(
        "",
        "pub struct Label<const SLOTS: usize> { #[model_schema_prop(maxLength = 9)] pub text: \
         String, pub slots: Slots<SLOTS> }",
        "type `Label`: `#[model_schema]` cannot render the const parameter `SLOTS` as a type \
         argument",
    );
}

#[cfg(not(feature = "indexmap"))]
#[test]
fn an_item_refused_for_a_name_the_indexmap_feature_reads_is_re_emitted_without_its_members_keys() {
    assert_refused_without_member_keys(
        "",
        "pub struct Label { #[model_schema_prop(maxLength = 9)] pub text: String, pub counts: \
         IndexMap<String, u32> }",
        "`IndexMap` is read as a map under tixschema's `indexmap` feature",
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_shape_refused_before_its_members_are_walked_is_re_emitted_without_their_keys() {
    for (source, refusal) in [
        (
            "#[cfg_attr(feature = \"wire\", serde(rename_all = \"camelCase\"))] pub struct Label \
             { #[model_schema_prop(maxLength = 9)] pub text: String }",
            "type `Label`: cfg_attr-wrapped serde attribute is invisible to model_schema",
        ),
        (
            "#[cfg_attr(feature = \"wire\", serde(rename_all = \"camelCase\"))] pub enum Size { \
             Named { #[model_schema_prop(maxLength = 9)] label: String } }",
            "type `Size`: cfg_attr-wrapped serde attribute is invisible to model_schema",
        ),
        (
            "#[serde(tag = \"kind\", content = \"data\")] pub enum Size { Gone(#[serde(skip)] \
             u32), Named { #[model_schema_prop(maxLength = 9)] label: String } }",
            "variant `Gone` of enum `Size` declares one slot and takes it off the wire in both \
             directions",
        ),
        (
            "#[serde(tag = \"kind\")] pub enum Size { Pair(u32, u32), Named { \
             #[model_schema_prop(maxLength = 9)] label: String } }",
            "variant `Pair` of internally tagged enum `Size` is a tuple variant",
        ),
    ] {
        assert_refused_without_member_keys("", source, refusal);
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_refused_by_its_guards_is_re_emitted_without_its_members_keys() {
    assert_refused_without_member_keys(
        "",
        "#[serde(transparent)] pub struct Label { pub text: Option<String>, \
         #[model_schema_prop(maxLength = 9)] #[serde(skip)] marker: \
         ::core::marker::PhantomData<u8> }",
        "type `Label`: `#[model_schema_prop]` is unread on a field serde neither reads nor writes",
    );
}

#[test]
fn a_string_check_on_a_struct_that_is_no_brand_is_refused() {
    for (args, check) in [
        ("minLength = 3", "minLength"),
        ("maxLength = 9", "maxLength"),
        ("pattern = \"^[a-z]+$\"", "pattern"),
    ] {
        let expanded = expansion_under(args, "pub struct Named { pub name: String }");
        assert!(expanded.contains("compile_error"), "got: {expanded}");
        assert!(
            expanded.contains(&format!(
                "type `Named`: `{check}` is supported only on a branded newtype: a \
                 `#[serde(transparent)]` struct serde writes as the value of one field. On a \
                 field, write it in `#[model_schema_prop(...)]`."
            )),
            "got: {expanded}"
        );
    }
}

#[test]
fn a_string_check_on_an_enum_builds_under_every_tagging() {
    for args in [
        "minLength = 1",
        "maxLength = 9",
        "pattern = \"^[A-Za-z]+$\"",
        "minLength = 1, maxLength = 9, pattern = \"^[A-Za-z]+$\"",
        "name = \"Level\", minLength = 1",
    ] {
        for source in [
            "pub enum Tier { Gold, Silver }",
            "pub enum Tier { Named { label: String }, Bare }",
            "#[serde(tag = \"kind\")] pub enum Tier { Named { label: String }, Bare }",
            "#[serde(tag = \"kind\", content = \"data\")] pub enum Tier { Named { label: String }, Bare }",
            "#[serde(untagged)] pub enum Tier { Text(String), Count(u32) }",
        ] {
            let expanded = expansion_under(args, source);
            assert!(
                !expanded.contains("compile_error"),
                "for `{args}` on `{source}`: {expanded}"
            );
        }
    }
}

#[test]
fn an_enum_with_no_string_check_is_not_refused() {
    let expanded = expansion_under("", "pub enum Tier { Gold, Silver }");
    assert!(!expanded.contains("compile_error"), "got: {expanded}");
}

#[test]
fn a_unit_variants_name_that_breaks_the_enums_check_is_a_build_error() {
    let readme = include_str!("../../README.md");
    let sized = expansion_under(
        "minLength = 2, maxLength = 3",
        "pub enum Size { S, Xl, Xxxxl }",
    );
    assert_eq!(sized.matches("compile_error").count(), 2, "got: {sized}");
    for refusal in [
        "type `Size`: variant `S` is written `S`, which breaks `minLength = 2` written on the enum",
        "type `Size`: variant `Xxxxl` is written `Xxxxl`, which breaks `maxLength = 3` written on \
         the enum",
    ] {
        assert!(sized.contains(refusal), "got: {sized}");
        assert!(
            readme.contains(&format!("error: model_schema: {refusal}\n")),
            "the README no longer shows: {refusal}"
        );
    }

    let matched = expansion_under(
        "minLength = 3, pattern = \"^[a-z]+$\"",
        "pub enum Tier { Ab, gold, Named { label: String } }",
    );
    assert_eq!(
        matched.matches("compile_error").count(),
        1,
        "got: {matched}"
    );
    assert!(
        matched.contains(
            "type `Tier`: variant `Ab` is written `Ab`, which breaks `minLength = 3` and `pattern \
             = "
        ),
        "got: {matched}"
    );
}

#[test]
fn the_name_of_a_variant_that_carries_data_is_not_measured() {
    let expanded = expansion_under(
        "minLength = 9",
        "pub enum Tier { Named { label: String }, Slot(String) }",
    );
    assert!(!expanded.contains("compile_error"), "got: {expanded}");
}

#[cfg(feature = "serde")]
#[test]
fn each_name_serde_writes_or_reads_for_a_unit_variant_is_measured() {
    for (args, source, refusal) in [
        (
            "maxLength = 4",
            "pub enum Kind { #[serde(rename = \"application/pdf\")] Pdf, Txt }",
            "type `Kind`: variant `Pdf` is written `application/pdf`, which breaks `maxLength = 4` \
             written on the enum",
        ),
        (
            "maxLength = 5",
            "#[serde(rename_all = \"SCREAMING_SNAKE_CASE\")] pub enum Kind { TooLong, Ok }",
            "type `Kind`: variant `TooLong` is written `TOO_LONG`, which breaks `maxLength = 5` \
             written on the enum",
        ),
        (
            "maxLength = 3",
            "pub enum Kind { #[serde(alias = \"portable\", alias = \"Pdf\")] Pdf, Txt }",
            "type `Kind`: variant `Pdf` is read `portable`, which breaks `maxLength = 3` written \
             on the enum",
        ),
        (
            "maxLength = 3",
            "#[serde(tag = \"kind\")] pub enum Kind { #[serde(rename = \"portable\")] Pdf, Txt }",
            "type `Kind`: variant `Pdf` is written `portable`, which breaks `maxLength = 3` \
             written on the enum",
        ),
    ] {
        let expanded = expansion_under(args, source);
        assert_eq!(
            expanded.matches("compile_error").count(),
            1,
            "got: {expanded}"
        );
        assert!(expanded.contains(refusal), "got: {expanded}");
    }
}

/// A build that describes the type refuses a rename whose two directions differ, ahead of the
/// check.
#[cfg(all(
    feature = "serde",
    not(any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))
))]
#[test]
fn a_unit_variants_read_name_is_measured_where_it_is_not_the_written_one() {
    let expanded = expansion_under(
        "maxLength = 3",
        "pub enum Kind { #[serde(rename(serialize = \"pdf\", deserialize = \"portable\"))] Pdf }",
    );
    assert_eq!(
        expanded.matches("compile_error").count(),
        1,
        "got: {expanded}"
    );
    assert!(
        expanded.contains(
            "type `Kind`: variant `Pdf` is read `portable`, which breaks `maxLength = 3` written \
             on the enum"
        ),
        "got: {expanded}"
    );
}

/// An untagged enum writes a unit variant as `null`, and refuses it for that, wherever the list
/// writes `untagged`.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_unit_variant_has_no_name_to_measure() {
    for attribute in [
        "#[serde(untagged)]",
        "#[serde(untagged, rename_all = \"camelCase\")]",
        "#[serde(rename_all = \"camelCase\", untagged)]",
    ] {
        let expanded = expansion_under(
            "minLength = 9",
            &format!("{attribute} pub enum Loose {{ Nothing, Text(String) }}"),
        );
        assert!(
            !expanded.contains("which breaks"),
            "under {attribute}, got: {expanded}"
        );
        assert!(
            expanded.contains("`Nothing` is a unit variant"),
            "under {attribute}, got: {expanded}"
        );
    }
}

#[test]
fn an_enum_whose_names_all_fit_its_check_publishes_what_it_did() {
    let source = "pub enum Tier { Gold, Silver }";
    let checked = expansion_under(
        "minLength = 4, maxLength = 6, pattern = \"^[A-Z][a-z]+$\"",
        source,
    );
    assert!(!checked.contains("compile_error"), "got: {checked}");
    assert_eq!(checked, expansion_under("", source));
}

#[test]
fn an_enums_string_check_that_reaches_nothing_is_refused() {
    for (args, check) in [
        ("minLength = 1", "minLength"),
        ("maxLength = 9", "maxLength"),
        ("pattern = \"^[a-z]+$\"", "pattern"),
        ("name = \"Outline\", minLength = 1", "minLength"),
    ] {
        for source in [
            "pub enum Shape { Circle { r: u32 } }",
            "pub enum Shape { Listed(Vec<String>), Named { label: Label }, Sized(u32) }",
            "#[serde(tag = \"kind\")] pub enum Shape { Circle { r: u32 } }",
            "#[serde(untagged)] pub enum Shape { Count(u32), Named(Label) }",
            "pub enum Shape { Fixed { #[model_schema_prop(literal = \"x\")] word: String } }",
            "#[serde(tag = \"kind\")] pub enum Shape { Fixed { #[model_schema_prop(literal = \
             \"x\")] word: String } }",
            "pub enum Shape { Fixed(#[model_schema_prop(literal = \"x\")] String) }",
        ] {
            let expanded = expansion_under(args, source);
            assert_eq!(
                expanded.matches("compile_error").count(),
                1,
                "got: {expanded}"
            );
            assert!(
                expanded.contains(&format!(
                    "type `Shape`: `{check}` reaches nothing here: an enum's check holds each \
                     variant written with no data and each field written as a string or a path \
                     that carries no `literal`, and `Shape` has neither"
                )),
                "got: {expanded}"
            );
        }
    }
    assert!(
        include_str!("../../README.md").contains(
            "error: model_schema: type `Shape`: `minLength` reaches nothing here: an enum's check \
             holds each variant written with no data and each field written as a string or a \
             path that carries no `literal`, and `Shape` has neither\n"
        ),
        "the README no longer shows the refusal"
    );
}

#[test]
fn an_enums_check_beside_a_literal_field_holds_the_unit_variants_name() {
    let expanded = expansion_under(
        "maxLength = 9",
        "pub enum Shape { Empty, Fixed { #[model_schema_prop(literal = \"x\")] word: String } }",
    );
    assert!(!expanded.contains("compile_error"), "got: {expanded}");
}

#[test]
fn a_pattern_no_surface_reads_is_refused_on_an_enum() {
    let expanded = expansion_under(
        "pattern = \"(\"",
        "pub enum Tier { Named { label: String }, Bare }",
    );
    assert_eq!(
        expanded.matches("compile_error").count(),
        1,
        "got: {expanded}"
    );
    assert!(
        expanded.contains("type `Tier`: `pattern` is not a regex"),
        "got: {expanded}"
    );
}

#[test]
fn an_enums_check_reaches_a_field_written_as_a_string_or_a_path() {
    for (spelling, reached) in TEXT_REACH {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        assert_eq!(super::written_as_text(&ty), reached, "for {spelling}");
    }
}

/// The reach is the constraint walk's own: a text leaf, which a list is not.
#[cfg(feature = "serde")]
#[test]
fn the_reach_of_an_enums_check_is_the_constraint_walks() {
    for (spelling, reached) in TEXT_REACH {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let walked = constrained_shape(&ty)
            .is_some_and(|shape| matches!(shape.leaf, ConstraintLeaf::Path | ConstraintLeaf::Str));
        assert_eq!(walked, reached, "for {spelling}");
    }
}

/// What `member_prop_meta` holds a member written as `source` to under the enum's `enum_check`.
fn held_member(enum_check: &str, source: &str) -> ModelSchemaPropMeta {
    let args = super::parse_model_schema_args(syn::parse_str(enum_check).unwrap());
    let item: syn::ItemStruct = syn::parse_str(&format!("struct Probe {{ {source} }}")).unwrap();
    super::member_prop_meta(item.fields.iter().next().unwrap(), Some(&args))
}

#[test]
fn a_member_is_held_to_its_own_lengths_and_its_enums() {
    let enum_check = "minLength = 2, maxLength = 5";
    for (source, min_length, max_length) in [
        ("label: String", Some(2), Some(5)),
        (
            "#[model_schema_prop(minLength = 3)] label: String",
            Some(3),
            Some(5),
        ),
        (
            "#[model_schema_prop(minLength = 1, maxLength = 9)] label: String",
            Some(2),
            Some(5),
        ),
        (
            "#[model_schema_prop(maxLength = 4)] label: Option<String>",
            Some(2),
            Some(4),
        ),
        (
            "#[model_schema_prop(minLength = 1)] labels: Vec<String>",
            Some(1),
            None,
        ),
        ("label: Label", None, None),
        ("count: u32", None, None),
    ] {
        let held = held_member(enum_check, source);
        assert_eq!(held.min_length, min_length, "for {source}");
        assert_eq!(held.max_length, max_length, "for {source}");
    }
}

/// A literal takes no other rule, so an enum's check leaves a member that carries one alone.
#[test]
fn a_member_that_carries_a_literal_is_held_to_nothing_of_its_enums_check() {
    for source in [
        "#[model_schema_prop(literal = \"x\")] label: String",
        "#[model_schema_prop(literal = \"x\")] label: Option<String>",
    ] {
        let held = held_member(
            "minLength = 2, maxLength = 5, pattern = \"^[a-z]+$\"",
            source,
        );
        assert_eq!(held.min_length, None, "for {source}");
        assert_eq!(held.max_length, None, "for {source}");
        assert_eq!(held.pattern, None, "for {source}");
        #[cfg(any(
            feature = "serde",
            feature = "zod",
            feature = "dart",
            feature = "swift"
        ))]
        assert_eq!(held.patterns().count(), 0, "for {source}");
    }
}

#[test]
fn an_enums_check_beside_a_literal_field_builds_under_every_tagging() {
    for args in ["minLength = 2", "maxLength = 9", "pattern = \"^[a-z]+$\""] {
        for tagging in [
            "",
            "#[serde(tag = \"kind\")]",
            "#[serde(tag = \"kind\", content = \"held\")]",
            "#[serde(untagged)]",
        ] {
            let expanded = expansion_under(
                args,
                &format!(
                    "{tagging} pub enum Label {{ Fixed {{ #[model_schema_prop(literal = \"x\")] \
                     word: String, note: String }} }}"
                ),
            );
            assert!(
                !expanded.contains("compile_error"),
                "for {args} under {tagging}: {expanded}"
            );
        }
    }
}

#[cfg(any(feature = "serde", feature = "zod"))]
#[test]
fn a_member_is_held_to_its_own_pattern_and_its_enums() {
    let enum_check = "pattern = \"^[a-z]+$\"";
    for (source, patterns) in [
        ("label: String", vec!["^[a-z]+$"]),
        (
            "#[model_schema_prop(pattern = \"^a\")] label: String",
            vec!["^a", "^[a-z]+$"],
        ),
        (
            "#[model_schema_prop(pattern = \"^[a-z]+$\")] label: String",
            vec!["^[a-z]+$"],
        ),
        (
            "#[model_schema_prop(pattern = \"^a\")] label: Label",
            vec!["^a"],
        ),
        ("labels: Vec<String>", vec![]),
    ] {
        let held = held_member(enum_check, source);
        assert_eq!(
            held.patterns().collect::<Vec<_>>(),
            patterns,
            "for {source}"
        );
    }
    let unchecked = held_member("", "#[model_schema_prop(pattern = \"^a\")] label: String");
    assert_eq!(unchecked.patterns().collect::<Vec<_>>(), vec!["^a"]);
}

#[test]
fn a_string_check_on_a_type_alias_is_refused() {
    for (args, check) in [
        ("minLength = 1", "minLength"),
        ("maxLength = 9", "maxLength"),
        ("pattern = \"^[A-Z]{3}$\"", "pattern"),
        ("name = \"UnlistedMimeType\", minLength = 1", "minLength"),
    ] {
        let expanded = expansion_under(args, "pub type UnlistedMimeType = String;");
        assert!(expanded.contains("compile_error"), "got: {expanded}");
        assert!(
            expanded.contains(&format!(
                "type `UnlistedMimeType`: `{check}` cannot be written on a type alias"
            )),
            "got: {expanded}"
        );
        assert!(
            expanded.contains("is another name for `String`"),
            "got: {expanded}"
        );
    }
}

#[test]
fn a_string_check_on_an_alias_of_a_brand_is_refused_naming_the_brand() {
    let brand = expansion_under("", "#[serde(transparent)] pub struct Sku(pub String);");
    assert!(!brand.contains("compile_error"), "got: {brand}");

    let expanded = expansion_under("minLength = 3", "pub type Short = Sku;");
    assert!(expanded.contains("compile_error"), "got: {expanded}");
    assert!(
        expanded.contains(
            "type `Short`: `minLength` cannot be written on a type alias: `Short` is another \
             name for `Sku`"
        ),
        "got: {expanded}"
    );
}

/// With no default declared for `T`, `jsonschema` refuses the alias over that ahead of its check.
#[test]
fn a_string_check_on_a_generic_type_alias_is_refused() {
    let undeclared = expansion_under("maxLength = 9", "pub type Named<T> = Vec<T>;");
    assert!(undeclared.contains("compile_error"), "got: {undeclared}");

    let expanded = expansion_under(
        "maxLength = 9, default_types(T = String)",
        "pub type Named<T> = Vec<T>;",
    );
    assert!(expanded.contains("compile_error"), "got: {expanded}");
    assert!(
        expanded.contains(
            "type `Named`: `maxLength` cannot be written on a type alias: `Named` is another \
             name for `Vec<T>`"
        ),
        "got: {expanded}"
    );
}

#[test]
fn an_alias_with_no_string_check_is_not_refused() {
    let expanded = expansion_under("name = \"DocumentId\"", "pub type DocumentId = String;");
    assert!(!expanded.contains("compile_error"), "got: {expanded}");
}

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_string_check_on_a_brand_is_refused_where_no_schema_surface_is_on() {
    let expanded = expansion_under(
        "minLength = 3",
        "#[serde(transparent)] pub struct Code(pub String);",
    );
    assert!(expanded.contains("compile_error"), "got: {expanded}");
    assert!(
        expanded.contains(
            "type `Code`: `minLength` on a brand is enforced by the reader generated with its \
             schema surface, and this build generates none"
        ),
        "got: {expanded}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn decode_with_walks_a_transparent_struct_with_a_named_field_as_the_value_of_its_field() {
    let expanded = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(transparent)] pub struct DecodeHolder { pub inner: String }",
    );
    assert!(!expanded.contains("compile_error"), "got: {expanded}");
    let (_, added) = expanded.split_once("pub fn from_value_with").unwrap();
    // A build with `dart`, `swift` or `kotlin` on writes that surface's module after the `impl`.
    let walker = added
        .split_once("pub mod ")
        .map_or(added, |(methods, _)| methods);
    assert!(
        walker.contains(
            "{ out . extend (decode_holder_schema :: value_leaf (found , Self :: \
             decode_with_read_unjudged , | read | serde_json :: to_value (read) . ok () , \
             path . to_vec () , & [(\"String\" , & [] , 0)] , issue)) ; }"
        ),
        "got: {walker}"
    );
    assert!(
        walker.contains("-> :: std :: vec :: Vec < & 'a str > { :: std :: vec :: Vec :: new () }"),
        "got: {walker}"
    );
    for unwritten in ["inner", "\"Missing\""] {
        assert!(
            !walker.contains(unwritten),
            "found `{unwritten}`, got: {walker}"
        );
    }
    assert_eq!(
        expanded.matches("pub mod decode_holder_schema {").count(),
        1
    );
}

#[cfg(feature = "serde")]
#[test]
fn decode_with_is_generated_on_every_enum_form_serde_writes() {
    for (args, source) in DECODE_WITH_ENUM_FORMS {
        let expanded = expansion_under(args, source);
        assert!(
            !expanded.contains("compile_error"),
            "for {source}, got: {expanded}"
        );
        for emitted in [
            "pub fn from_value_with < F > (",
            "pub fn from_value_piped (",
            "fn decode_with_value_report (",
            "pub fn decode_with_value_issues < I > (",
            "pub fn decode_with_value_named (",
            "pub fn decode_with_value_fields < 'a , I > (",
            "pub use crate :: __tixschema_decode_with :: {",
        ] {
            assert!(
                expanded.contains(emitted),
                "for {source}, missing `{emitted}`, got: {expanded}"
            );
        }
        assert_eq!(
            expanded.contains("pub fn from_bson_with < F > ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("pub fn from_bson_piped ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("pub use crate :: __tixschema_mongodb :: {"),
            cfg!(feature = "mongodb"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("pub fn decode_with_bson_fields < 'a , I > ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("pub fn decode_with_bson_named ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        for under_mongodb in TYPED_PATHS.iter().chain(&OPERATIONS) {
            assert_eq!(
                expanded.contains(under_mongodb),
                cfg!(feature = "mongodb"),
                "for {source} and `{under_mongodb}`, got: {expanded}"
            );
        }
        assert_eq!(
            expanded.matches("_schema {").count(),
            1,
            "for {source}, got: {expanded}"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn decode_with_reads_an_untagged_enums_constrained_member_through_its_hook() {
    let expanded = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[derive(Deserialize)] #[serde(untagged)] pub enum DecodeReach { \
         Email { #[model_schema_prop(minLength = 3)] address: String }, Phone { digits: String } }",
    );
    for emitted in [
        "# [serde (deserialize_with = \"decode_reach_schema::deserialize_email_address\")] address : String",
        "pub fn deserialize_email_address < 'de , D > (deserializer : D)",
        "decode_reach_schema :: value_leaf (held , decode_reach_schema :: deserialize_email_address , \
         | read : & String | serde_json :: to_value (read) . ok () ,",
        "decode_reach_schema :: value_leaf (held , < String as serde :: Deserialize > :: deserialize ,",
    ] {
        assert!(
            expanded.contains(emitted),
            "missing `{emitted}`, got: {expanded}"
        );
    }
    assert_eq!(expanded.matches("pub mod decode_reach_schema {").count(), 1);
}

#[cfg(feature = "serde")]
#[test]
fn decode_with_is_generated_on_every_struct_shape_serde_writes() {
    for (args, source) in DECODE_WITH_STRUCT_SHAPES {
        let expanded = expansion_under(args, source);
        assert!(
            !expanded.contains("compile_error"),
            "for {source}, got: {expanded}"
        );
        for emitted in [
            "pub fn from_value_with < F > (",
            "pub fn from_value_piped (",
            "fn decode_with_value_report (",
            "pub fn decode_with_value_issues < I > (",
            "pub fn decode_with_value_named (",
            "pub fn decode_with_value_fields < 'a , I > (",
            "pub use crate :: __tixschema_decode_with :: {",
        ] {
            assert!(
                expanded.contains(emitted),
                "for {source}, missing `{emitted}`, got: {expanded}"
            );
        }
        assert_eq!(
            expanded.contains("pub fn from_bson_with < F > ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("pub fn from_bson_piped ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("pub use crate :: __tixschema_mongodb :: {"),
            cfg!(feature = "mongodb"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("pub fn decode_with_bson_named ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("pub fn decode_with_bson_fields < 'a , I > ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        for under_mongodb in TYPED_PATHS.iter().chain(&OPERATIONS) {
            assert_eq!(
                expanded.contains(under_mongodb),
                cfg!(feature = "mongodb"),
                "for {source} and `{under_mongodb}`, got: {expanded}"
            );
        }
    }
}

/// A type that lists `"decode_with"` alone is read from BSON under `bson`, as every flagged type
/// is, and gets nothing of the MongoDB layer under any feature. An unflagged one gets neither.
#[cfg(feature = "serde")]
#[test]
fn a_type_that_lists_no_mongodb_gets_no_typed_path_and_no_operation() {
    for ((args, source), listed) in DECODE_WITH_STRUCT_SHAPES
        .iter()
        .chain(&DECODE_WITH_ENUM_FORMS)
        .flat_map(|shape| [(shape, false), (shape, true)])
    {
        let others = args
            .trim_start_matches("enable = [\"decode_with\", \"mongodb\"]")
            .trim_start_matches(", ");
        // Unflagged, or listing "decode_with" alone.
        let written = if listed && others.is_empty() {
            "enable = [\"decode_with\"]".to_owned()
        } else if listed {
            format!("enable = [\"decode_with\"], {others}")
        } else {
            others.to_owned()
        };
        let expanded = expansion_under(&written, source);
        for added in [
            "MongoFields",
            "MONGO_FIELDS",
            "pub trait Nests <",
            "OperationError",
            "pub struct Read <",
            "find_one",
            "pub fn find <",
            "find_with",
            "pub fn count <",
            "insert_one",
            "update_one",
            "update_many",
            "delete_one",
            "delete_many",
            "mongo_read_row",
            "mongo_written_row",
            "__tixschema_mongodb",
        ] {
            assert!(
                !expanded.contains(added),
                "for {source}, found `{added}` in: {expanded}"
            );
        }
        for bson_read in ["pub fn from_bson_with", "pub fn decode_with_bson_issues"] {
            assert_eq!(
                expanded.contains(bson_read),
                listed && cfg!(feature = "bson"),
                "for {written} {source}, `{bson_read}` in: {expanded}"
            );
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn an_item_is_recorded_as_one_value_where_serde_writes_it_as_one() {
    for (source, one_value) in [
        (
            "#[serde(transparent)] pub struct SeenBrand(pub String);",
            true,
        ),
        ("pub struct SeenSlot(pub u32);", true),
        ("pub struct SeenUnit;", true),
        ("pub enum SeenPlain { Draft, Paid }", true),
        ("pub struct SeenKeyed { pub held: String }", false),
        ("pub struct SeenPair(pub u32, pub u32);", false),
        (
            "#[serde(tag = \"kind\")] pub enum SeenTagged { Draft, Paid }",
            false,
        ),
    ] {
        let item: syn::Item = syn::parse_str(source).unwrap();
        let name = super::item_schema_ident(&item).unwrap().to_string();
        let _expanded = expansion_under("", source);
        assert_eq!(
            matches!(declared(&name), Some(Declared::OneValue(..))),
            one_value,
            "for {source}"
        );
        assert_eq!(
            matches!(declared(&name), Some(Declared::Model(..))),
            !one_value,
            "for {source}"
        );
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_type_declared_above_is_held_by_how_serde_writes_it() {
    let _brand = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"]",
        "#[serde(transparent)] pub struct HeldBrand(pub String);",
    );
    let _keyed = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct HeldKeyed { pub held: String }",
    );
    let expanded = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct HeldBy { pub brand: HeldBrand, pub brands: Vec<HeldBrand>, \
         pub keyed: HeldKeyed, pub keyeds: Vec<HeldKeyed> }",
    );
    for emitted in [
        "pub const fn brand (& self) -> held_by_schema :: Field < Root , HeldBrand > {",
        "pub const fn brands (& self) -> held_by_schema :: ListField < Root , HeldBrand > {",
        "pub const fn keyed (& self) -> held_by_schema :: Model < Root , HeldKeyed > {",
        "pub const fn keyeds (& self) -> held_by_schema :: ModelList < Root , HeldKeyed > {",
    ] {
        assert!(
            expanded.contains(emitted),
            "missing `{emitted}`, got: {expanded}"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn decode_with_reads_a_constrained_brand_with_the_brands_own_reader() {
    let expanded = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"], minLength = 3",
        "#[derive(Deserialize)] #[serde(transparent)] pub struct DecodeCode(pub String);",
    );
    for emitted in [
        "decode_code_schema :: value_leaf (found , Self :: decode_with_read_unjudged , \
         | read | serde_json :: to_value (read) . ok () , path . to_vec () , \
         & [(\"String\" , & [] , 0)] , issue)",
        // The read a brand is listed from is its unjudged read, held to the brand's own bound.
        "let read = < Self > :: model_schema_read_unjudged (deserializer) ? ; read . \
         model_schema_validate_bound () . map_err (",
        "pub fn model_schema_validate_bound (& self) -> :: core :: result :: Result < () , :: \
         std :: vec :: Vec < :: std :: string :: String >> { decode_code_schema :: \
         validate_value (& self . 0 . to_string ()) }",
        "pub fn model_schema_read_unjudged < 'de , D > (deserializer : D)",
    ] {
        assert!(
            expanded.contains(emitted),
            "missing `{emitted}`, got: {expanded}"
        );
    }
    // The slot carries no hook, and the brand's own read is the one the macro writes.
    assert!(
        expanded.starts_with("pub struct DecodeCode (pub String) ;"),
        "got: {expanded}"
    );
    for unwritten in ["deserialize_value", "deserialize_with"] {
        assert!(
            !expanded.contains(unwritten),
            "found `{unwritten}` in: {expanded}"
        );
    }
    assert_eq!(expanded.matches("pub mod decode_code_schema {").count(), 1);
    assert!(
        !expanded.contains("value_leaf (found , decode_code_schema"),
        "got: {expanded}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn decode_with_bounds_its_own_impls_and_nothing_else_of_a_generic_type() {
    let source = "pub struct DecodeEnvelope<T> { pub items: Vec<T> }";
    let expanded = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"], default_types(T = String)",
        source,
    );
    assert!(
        expanded.starts_with("pub struct DecodeEnvelope < T > {"),
        "got: {expanded}"
    );
    assert!(
        expanded.contains(
            "impl < T : serde :: de :: DeserializeOwned > DecodeEnvelope < T > \
             where Self : serde :: de :: DeserializeOwned { "
        ),
        "got: {expanded}"
    );
    assert_eq!(
        expanded.contains(
            "impl < T : serde :: de :: DeserializeOwned + serde :: Serialize > \
             DecodeEnvelope < T > \
             where Self : serde :: de :: DeserializeOwned + serde :: Serialize { "
        ),
        cfg!(feature = "bson"),
        "got: {expanded}"
    );
    let unflagged = expansion_under("default_types(T = String)", source);
    assert!(!unflagged.contains("DeserializeOwned"), "got: {unflagged}");
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert!(
        expanded.contains("impl < T > DecodeEnvelope < T > {"),
        "got: {expanded}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn decode_with_is_generated_on_a_type_with_a_flattened_field() {
    for (args, source) in DECODE_WITH_FLATTEN_FORMS {
        let expanded = expansion_under(args, source);
        assert!(
            !expanded.contains("compile_error"),
            "for {source}, got: {expanded}"
        );
        for emitted in [
            "pub fn from_value_with < F > (",
            "pub fn decode_with_value_issues < I > (",
            "pub fn decode_with_value_named (",
            "pub fn decode_with_value_fields < 'a , I > (",
        ] {
            assert!(
                expanded.contains(emitted),
                "for {source}, missing `{emitted}`, got: {expanded}"
            );
        }
        assert_eq!(
            expanded.contains("pub fn decode_with_bson_fields < 'a , I > ("),
            cfg!(feature = "bson"),
            "for {source}, got: {expanded}"
        );
        assert_eq!(
            expanded.contains("< DecodeAudit > :: decode_with_value_fields ("),
            source.contains("DecodeAudit"),
            "for {source}, got: {expanded}"
        );
    }
}

#[test]
fn decode_with_is_refused_for_good_on_a_type_alias() {
    let expanded = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub type DecodeVersions = Vec<DecodeVersion>;",
    );
    assert!(
        expanded.contains(
            "model_schema: type `DecodeVersions`: `enable = [\\\"decode_with\\\"]` cannot be written \
             on a type alias: `DecodeVersions` is another name for `Vec<DecodeVersion>`, and the \
             methods would have to be added to that type. List it on the model types the alias \
             reaches."
        ),
        "got: {expanded}"
    );
}

#[test]
fn decode_with_is_refused_for_good_on_a_type_that_borrows() {
    for source in [
        "pub struct DecodeNamed<'a> { pub name: &'a str }",
        "pub enum DecodeNamed<'a> { Named(&'a str), Unnamed }",
    ] {
        let expanded = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
        assert!(
            expanded.contains(
                "model_schema: type `DecodeNamed`: `enable = [\\\"decode_with\\\"]` cannot be \
                 written on a type that borrows: `from_value_with` owns the value it reads, so \
                 the decoded `DecodeNamed<'a>` cannot borrow from it for `'a`."
            ),
            "for {source}, got: {expanded}"
        );
    }
}

/// The message a field that borrows is refused with, as it reads on `DecodeLabelled`.
fn borrowing_field_refusal(field: &str, written: &str) -> String {
    format!(
        "model_schema: type `DecodeLabelled`: `enable = [\\\"decode_with\\\"]` cannot be written on \
         a type with a field that borrows: `{field}` is `{written}`, and `from_value_with` owns \
         the value it reads, so nothing decoded from it can be borrowed for `'static`. Write the \
         field as an owned type."
    )
}

/// A reference written in a field borrows from the owned value as a declared lifetime does.
#[test]
fn decode_with_is_refused_for_good_on_a_field_that_borrows() {
    for (source, field, written) in [
        (
            "pub struct DecodeLabelled { pub label: &'static str }",
            "label",
            "&'static str",
        ),
        (
            "pub struct DecodeLabelled { pub label: Option<&'static str> }",
            "label",
            "Option<&'static str>",
        ),
        (
            "pub struct DecodeLabelled { pub chunks: Vec<&'static [u8]>, pub name: String }",
            "chunks",
            "Vec<&'static [u8]>",
        ),
    ] {
        let expanded = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
        assert!(
            expanded.contains(&borrowing_field_refusal(field, written)),
            "for {source}, got: {expanded}"
        );
        assert!(
            !expanded.contains("fn from_value_with"),
            "for {source}, got: {expanded}"
        );
    }
}

/// A borrowing field is refused whatever shape declares it, an enum's variant included.
#[test]
fn decode_with_is_refused_on_a_borrowing_field_whatever_shape_declares_it() {
    for (source, field) in [
        ("pub struct DecodeLabelled(pub u32, pub &'static str);", "1"),
        (
            "#[serde(transparent)] pub struct DecodeLabelled(pub &'static str);",
            "0",
        ),
        (
            "#[serde(transparent)] pub struct DecodeLabelled { pub label: &'static str }",
            "label",
        ),
        (
            "pub enum DecodeLabelled { Named(&'static str), Unnamed }",
            "Named.0",
        ),
        (
            "pub enum DecodeLabelled { Named { label: &'static str }, Unnamed }",
            "Named.label",
        ),
    ] {
        let expanded = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
        assert!(
            expanded.contains(&borrowing_field_refusal(field, "&'static str")),
            "for {source}, got: {expanded}"
        );
        assert!(
            !expanded.contains("fn from_value_with"),
            "for {source}, got: {expanded}"
        );
    }
}

#[test]
fn decode_with_is_not_refused_on_a_field_that_borrows_nothing_from_the_value() {
    for source in [
        "pub struct DecodeLettered { pub label: Cow<'static, str> }",
        "pub struct DecodeLettered { #[serde(skip)] pub label: &'static str, pub name: String }",
        "pub struct DecodeLettered { #[serde(skip_deserializing)] pub label: &'static str }",
    ] {
        let expanded = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
        assert!(
            !expanded.contains("a field that borrows"),
            "for {source}, got: {expanded}"
        );
        #[cfg(feature = "serde")]
        assert!(
            expanded.contains("pub fn from_value_with"),
            "for {source}, got: {expanded}"
        );
    }
}

/// Three enums under a tag two functions of whose typed paths would take one name, each beside
/// what `decode_with` is refused on it for.
#[cfg(feature = "serde")]
fn enums_with_one_name_for_two_functions() -> [(&'static str, &'static str); 3] {
    [
        (
            "#[serde(tag = \"type\")] pub enum Transport { \
             By__Courier { name: String }, Collected }",
            "model_schema: type `Transport`: variant `By__Courier` holds `__` in its name. Under \
             a tag and `decode_with`, `__` parts the tag from the value in the function of each \
             arm, `type__<value>()`. Rename the variant",
        ),
        (
            "#[serde(tag = \"type\")] pub enum Transport { \
             #[serde(rename = \"by-courier\")] ByCourier { name: String }, \
             #[serde(rename = \"by_courier\")] Couriered }",
            "model_schema: type `Transport`: variants `ByCourier` and `Couriered` are written \
             `by-courier` and `by_courier` under the tag `type`, and both read `by_courier` in \
             snake case, so `type__by_courier()` would answer for both. Rename one of the two \
             values",
        ),
        (
            "#[serde(tag = \"is\")] pub enum Transport { Parcel, _Parcel }",
            "model_schema: type `Transport`: the arm function `is__parcel()` of variant `Parcel` \
             and `is__parcel()`, which asks for variant `_Parcel`, would be one function. Rename \
             the variant `_Parcel`",
        ),
    ]
}

/// The README quotes each refusal as it is printed.
#[cfg(feature = "serde")]
#[test]
fn decode_with_is_refused_where_two_functions_of_an_enums_paths_would_take_one_name() {
    for (source, refusal) in enums_with_one_name_for_two_functions() {
        let expanded = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
        assert!(expanded.contains(refusal), "for {source}, got: {expanded}");
        assert!(
            !expanded.contains("fn from_value_with"),
            "for {source}, got: {expanded}"
        );
        assert!(
            include_str!("../../README.md").contains(refusal),
            "the README no longer quotes this refusal:\n{refusal}"
        );
    }
}

/// The variant that holds `__`, the second of two values with one snake case, and the variant an
/// `is_` asks for.
#[cfg(feature = "serde")]
#[test]
fn each_refusal_of_a_name_two_functions_would_take_is_spanned_on_its_variant() {
    for ((source, _refusal), variant) in enums_with_one_name_for_two_functions().into_iter().zip([
        "By__Courier",
        "Couriered",
        "_Parcel",
    ]) {
        let item: syn::Item = syn::parse_str(source).unwrap();
        let args = super::parse_model_schema_args(
            syn::parse_str("enable = [\"decode_with\", \"mongodb\"]").unwrap(),
        );
        let located: Vec<String> = super::decode_with_guard_errors(&item, &args)
            .iter()
            .flat_map(located_source_texts)
            .collect();
        assert!(
            !located.is_empty() && located.iter().all(|text| text == variant),
            "for {source}, got: {located:?}"
        );
    }
}

/// An enum without the flag has no arm function, and is refused none of the three.
#[cfg(feature = "serde")]
#[test]
fn an_enum_without_decode_with_keeps_a_name_two_arm_functions_would_take() {
    for (source, _refusal) in enums_with_one_name_for_two_functions() {
        let expanded = expansion_under("", source);
        assert!(
            !expanded.contains("compile_error"),
            "for {source}, got: {expanded}"
        );
    }
}

/// What a type that lists `"mongodb"` is refused for the map `named` holds, keyed by `key`.
fn bson_refused_key_message(named: &str, key: &str) -> String {
    format!(
        "{named}: a map keyed by `{key}` cannot be stored in MongoDB — BSON document keys are \
         always text, and the `bson` library refuses to write this key and to read it back. Key \
         the map by a `String`, or do not list \"mongodb\" in `enable`."
    )
}

/// The refusals of each map `source` holds whose key the `bson` library refuses.
fn bson_refused_key_messages(source: &str) -> Vec<String> {
    let item: syn::Item = syn::parse_str(source).unwrap();
    super::mongodb_map_key_refusals(&item)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_map_keyed_by_a_number_or_a_bool_is_refused_wherever_a_field_holds_it() {
    for (written, key) in [
        ("HashMap<u32, String>", "u32"),
        ("HashMap<i64, String>", "i64"),
        ("HashMap<f64, String>", "f64"),
        ("BTreeMap<bool, u32>", "bool"),
        ("Option<HashMap<u8, String>>", "u8"),
        ("Vec<BTreeMap<bool, u32>>", "bool"),
        ("Option<Vec<BTreeMap<bool, u32>>>", "bool"),
        ("Box<HashMap<usize, String>>", "usize"),
        ("[HashMap<u16, String>; 2]", "u16"),
        ("(String, HashMap<i8, String>)", "i8"),
        ("HashMap<String, BTreeMap<i32, String>>", "i32"),
        ("HashMap<Box<u64>, String>", "Box<u64>"),
        ("std::collections::HashMap<u128, String>", "u128"),
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Roster {{ pub by_number: {written} }}")),
            [bson_refused_key_message("field `by_number`", key)],
            "for {written}"
        );
    }
}

#[test]
fn each_map_a_field_holds_under_a_refused_key_is_refused_once() {
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Roster { pub by_number: HashMap<u32, BTreeMap<bool, String>> }"
        ),
        [
            bson_refused_key_message("field `by_number`", "u32"),
            bson_refused_key_message("field `by_number`", "bool"),
        ]
    );
}

#[test]
fn a_map_in_a_slot_or_in_a_variants_member_is_refused_under_its_name() {
    assert_eq!(
        bson_refused_key_messages("pub struct Pair(pub String, pub HashMap<u32, String>);"),
        [bson_refused_key_message("field `1`", "u32")]
    );
    assert_eq!(
        bson_refused_key_messages(
            "pub enum Lineup { Numbered { by_number: HashMap<u32, String> }, \
             Flagged(String, BTreeMap<bool, String>), Empty }"
        ),
        [
            bson_refused_key_message("variant `Numbered`, field `by_number`", "u32"),
            bson_refused_key_message("variant `Flagged`, field `1`", "bool"),
        ]
    );
}

/// A map in either arm of a std `Result`, or in the bound of a std range, is refused as one the
/// field holds itself, behind whatever holds the `Result` or the range.
#[test]
fn a_map_in_either_arm_of_a_result_or_in_the_bound_of_a_range_is_refused() {
    for (written, key) in [
        ("Result<HashMap<u32, String>, String>", "u32"),
        ("Result<String, BTreeMap<bool, u8>>", "bool"),
        ("Option<Result<Vec<HashMap<i64, String>>, String>>", "i64"),
        ("std::result::Result<HashMap<u32, String>, String>", "u32"),
        (
            "core::result::Result<String, Box<HashMap<u8, String>>>",
            "u8",
        ),
        (
            "Vec<Result<HashMap<String, BTreeMap<i16, String>>, String>>",
            "i16",
        ),
        ("Range<HashMap<u32, String>>", "u32"),
        ("RangeInclusive<BTreeMap<bool, String>>", "bool"),
        ("RangeFrom<Vec<HashMap<u8, String>>>", "u8"),
        ("std::ops::RangeTo<HashMap<i32, String>>", "i32"),
        ("Result<Range<HashMap<u64, String>>, String>", "u64"),
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub outcome: {written} }}")),
            [bson_refused_key_message("field `outcome`", key)],
            "for {written}"
        );
    }
}

#[test]
fn each_arm_of_a_result_is_refused_for_its_own_map_the_oks_first() {
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Arena { pub outcome: Result<BTreeMap<bool, String>, HashMap<u32, String>> }"
        ),
        [
            bson_refused_key_message("field `outcome`", "bool"),
            bson_refused_key_message("field `outcome`", "u32"),
        ]
    );
}

/// A `Result` or a range that holds no map keyed by a number or a `bool` is not refused, a number
/// in an arm or as a bound being no map.
#[test]
fn a_result_or_a_range_that_holds_no_refused_map_is_not_refused() {
    for written in [
        "Result<HashMap<String, String>, String>",
        "Result<u32, bool>",
        "RangeInclusive<u32>",
        "Range<u64>",
        "Option<RangeFrom<HashMap<String, u32>>>",
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub outcome: {written} }}")),
            Vec::<String>::new(),
            "for {written}"
        );
    }
}

/// A type that lists `"mongodb"` and holds a map in an arm of a `Result` fails its expansion
/// naming the field and the key.
#[test]
fn a_map_in_an_arm_of_a_result_fails_the_expansion_of_a_type_that_lists_mongodb() {
    let source = "pub struct Arena { pub outcome: Result<HashMap<u32, String>, String> }";
    let refused = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
    assert!(
        refused.contains(&format!(
            "model_schema: type `Arena`: {}",
            bson_refused_key_message("field `outcome`", "u32").replace('"', "\\\"")
        )),
        "got: {refused}"
    );
}

/// A key serde writes as text, which both majors of the `bson` library store and read back, is
/// not refused: a `String`, a `char`, a `chrono` date or time, and a name the expansion knows no
/// number of.
#[test]
fn a_map_keyed_by_what_serde_writes_as_text_is_not_refused() {
    for written in [
        "HashMap<String, u32>",
        "HashMap<char, String>",
        "BTreeMap<chrono::NaiveDate, String>",
        "BTreeMap<NaiveTime, String>",
        "BTreeMap<NaiveDateTime, String>",
        "BTreeMap<DateTime<Utc>, String>",
        "HashMap<Slot, String>",
        "HashMap<KeyType, String>",
        "Vec<u32>",
        "Option<u64>",
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!(
                "pub struct Roster<KeyType> {{ pub by_number: {written} }}"
            )),
            Vec::<String>::new(),
            "for {written}"
        );
    }
}

#[test]
fn a_member_serde_neither_writes_nor_reads_is_not_refused() {
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Roster { #[serde(skip)] pub by_number: HashMap<u32, String>, \
             #[serde(skip_serializing, skip_deserializing)] pub by_flag: HashMap<bool, String> }"
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        bson_refused_key_messages(
            "pub enum Lineup { #[serde(skip)] Numbered { by_number: HashMap<u32, String> }, Empty }"
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Roster { #[serde(skip_serializing)] pub by_number: HashMap<u32, String> }"
        ),
        [bson_refused_key_message("field `by_number`", "u32")]
    );
}

/// A member its author's functions write and read, under `with` or under both `serialize_with`
/// and `deserialize_with`, is not refused, in a struct, in a slot and in a variant.
#[test]
fn a_member_written_and_read_by_its_authors_functions_is_not_refused() {
    for source in [
        "pub struct Roster { #[serde(with = \"keys_as_text\")] pub by_number: HashMap<u32, String> }",
        "pub struct Roster { #[serde(serialize_with = \"keys_written\", \
         deserialize_with = \"keys_read\")] pub by_number: HashMap<u32, String> }",
        "pub struct Roster { #[serde(serialize_with = \"keys_written\")] \
         #[serde(deserialize_with = \"keys_read\")] pub by_flag: BTreeMap<bool, String> }",
        "pub struct Pair(pub String, #[serde(with = \"keys_as_text\")] pub HashMap<u32, String>);",
        "pub enum Lineup { Numbered { #[serde(with = \"keys_as_text\")] by_number: \
         HashMap<u32, String> }, Empty }",
    ] {
        assert_eq!(
            bson_refused_key_messages(source),
            Vec::<String>::new(),
            "for {source}"
        );
    }
}

/// A member whose map serde still writes or reads itself, under `serialize_with` or
/// `deserialize_with` alone, is refused as before.
#[test]
fn a_member_one_of_whose_directions_is_serdes_own_is_refused() {
    for hook in [
        "serialize_with = \"keys_written\"",
        "deserialize_with = \"keys_read\"",
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!(
                "pub struct Roster {{ #[serde({hook})] pub by_number: HashMap<u32, String> }}"
            )),
            [bson_refused_key_message("field `by_number`", "u32")],
            "for {hook}"
        );
    }
}

/// A variant its author's functions write and read, under `with` or under both `serialize_with`
/// and `deserialize_with`, is not refused for what its members hold, and the variants beside it
/// still are.
#[test]
fn a_variant_written_and_read_by_its_authors_functions_is_not_refused() {
    for hooks in [
        "#[serde(with = \"numbered_as_text\")]",
        "#[serde(serialize_with = \"numbered_written\", deserialize_with = \"numbered_read\")]",
        "#[serde(serialize_with = \"numbered_written\")] \
         #[serde(deserialize_with = \"numbered_read\")]",
    ] {
        for variant in [
            "Numbered(HashMap<u32, String>)",
            "Numbered(String, BTreeMap<bool, String>)",
            "Numbered { by_number: HashMap<u32, String> }",
        ] {
            let source = format!("pub enum Lineup {{ {hooks} {variant}, Empty }}");
            assert_eq!(
                bson_refused_key_messages(&source),
                Vec::<String>::new(),
                "for {source}"
            );
        }
    }
    assert_eq!(
        bson_refused_key_messages(
            "pub enum Lineup { #[serde(with = \"numbered_as_text\")] \
             Numbered(HashMap<u32, String>), Flagged(BTreeMap<bool, String>) }"
        ),
        [bson_refused_key_message(
            "variant `Flagged`, field `0`",
            "bool"
        )]
    );
}

/// A variant whose members serde still writes or reads itself, under `serialize_with` or
/// `deserialize_with` alone, is refused as before.
#[test]
fn a_variant_one_of_whose_directions_is_serdes_own_is_refused() {
    for hook in [
        "serialize_with = \"numbered_written\"",
        "deserialize_with = \"numbered_read\"",
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!(
                "pub enum Lineup {{ #[serde({hook})] Numbered(HashMap<u32, String>), Empty }}"
            )),
            [bson_refused_key_message(
                "variant `Numbered`, field `0`",
                "u32"
            )],
            "for {hook}"
        );
    }
}

/// An enum whose one such map is in a variant its author's functions write and read is not
/// refused where it is held, and one whose variant has one of the two functions alone is.
#[test]
fn a_held_enum_whose_map_is_in_a_variant_behind_its_authors_functions_is_not_refused() {
    record_declared_above(&[
        "pub enum Lineup { #[serde(with = \"numbered_as_text\")] \
         Numbered(HashMap<u32, String>), Empty }",
        "pub enum Seating { #[serde(serialize_with = \"numbered_written\")] \
         Numbered(HashMap<u32, String>), Empty }",
    ]);
    assert_eq!(
        bson_refused_key_messages("pub struct Venue { pub lineup: Option<Lineup> }"),
        Vec::<String>::new()
    );
    assert_eq!(
        bson_refused_key_messages("pub struct Venue { pub seating: Seating }"),
        [held_bson_refused_key_message(
            "field `seating`",
            "Seating",
            "variant `Numbered`, field `0`, a map keyed by `u32`"
        )]
    );
}

/// A held model is read past where its author's functions write and read the member that holds
/// it, and a model whose one such map they write and read is not refused where it is held.
#[test]
fn a_held_model_behind_its_authors_functions_is_not_refused() {
    record_declared_above(&[
        "pub struct Seating { pub by_number: HashMap<u32, String> }",
        "pub struct Booked { #[serde(with = \"keys_as_text\")] pub by_number: HashMap<u32, String> }",
    ]);
    for source in [
        "pub struct Venue { #[serde(with = \"seating_as_text\")] pub seating: Seating }",
        "pub struct Venue { #[serde(serialize_with = \"seating_written\", \
         deserialize_with = \"seating_read\")] pub seating: Vec<Seating> }",
        "pub struct Venue { pub booked: Booked }",
    ] {
        assert_eq!(
            bson_refused_key_messages(source),
            Vec::<String>::new(),
            "for {source}"
        );
    }
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Venue { #[serde(serialize_with = \"seating_written\")] pub seating: Seating }"
        ),
        [held_bson_refused_key_message(
            "field `seating`",
            "Seating",
            "field `by_number`, a map keyed by `u32`"
        )]
    );
}

/// A brand or an alias declared above over a number or a `bool` is refused as the number is, and
/// one over a `String` is not.
#[test]
fn a_map_keyed_by_a_brand_or_an_alias_over_a_number_is_refused() {
    for declared in [
        "#[serde(transparent)] pub struct Count(pub u32);",
        "#[serde(transparent)] pub struct Ordinal { pub value: i64 }",
        "pub type Tally = Count;",
        "#[serde(transparent)] pub struct Flag(pub Box<bool>);",
        "#[serde(transparent)] pub struct Label(pub String);",
    ] {
        super::record_bson_refused_key_item(&syn::parse_str(declared).unwrap());
    }
    for (key, refused) in [
        ("Count", true),
        ("Ordinal", true),
        ("Tally", true),
        ("Flag", true),
        ("Label", false),
    ] {
        let expected: Vec<String> = refused
            .then(|| bson_refused_key_message("field `by_key`", key))
            .into_iter()
            .collect();
        assert_eq!(
            bson_refused_key_messages(&format!(
                "pub struct Roster {{ pub by_key: HashMap<{key}, String> }}"
            )),
            expected,
            "for {key}"
        );
    }
}

/// A generic brand or alias declared above whose value is its own parameter is refused where the
/// parameter is filled at a number or a `bool`, and not where it is filled at a `String` or
/// written with no argument.
#[test]
fn a_map_keyed_by_a_generic_brand_or_an_alias_filled_at_a_number_is_refused() {
    for declared in [
        "#[serde(transparent)] pub struct Ordinal<IdType>(pub IdType);",
        "pub type Slot<T> = T;",
        "#[serde(transparent)] pub struct Seat<Row, Place> { pub place: Place, \
         pub row: PhantomData<Row> }",
    ] {
        super::record_bson_refused_key_item(&syn::parse_str(declared).unwrap());
    }
    for (key, refused) in [
        ("Ordinal<u32>", true),
        ("Ordinal<bool>", true),
        ("Ordinal<Ordinal<u32>>", true),
        ("Ordinal<String>", false),
        ("Ordinal", false),
        ("Slot<u16>", true),
        ("Slot<String>", false),
        ("Seat<String, u8>", true),
        ("Seat<u8, String>", false),
    ] {
        let expected: Vec<String> = refused
            .then(|| bson_refused_key_message("field `by_key`", key))
            .into_iter()
            .collect();
        assert_eq!(
            bson_refused_key_messages(&format!(
                "pub struct Roster {{ pub by_key: HashMap<{key}, String> }}"
            )),
            expected,
            "for {key}"
        );
    }
}

/// A generic brand or alias declared above whose value reaches its own parameter under `Box`,
/// `Rc`, `Arc` or `Cow`, or through a brand declared above, is refused where the parameter is
/// filled at a number or a `bool`, and not where it is filled at a `String` or where the brand it
/// reaches it through is declared below it.
#[test]
fn a_map_keyed_by_a_generic_brand_over_a_wrapper_or_a_brand_of_its_parameter_is_refused() {
    record_declared_above(&[
        "#[serde(transparent)] pub struct Ordinal<IdType>(pub IdType);",
        "#[serde(transparent)] pub struct Boxed<T>(pub Box<T>);",
        "pub type Held<T> = Box<T>;",
        "#[serde(transparent)] pub struct Borrowed<'a, T>(pub Cow<'a, T>);",
        "#[serde(transparent)] pub struct Rank<T>(pub Ordinal<T>);",
        "pub type Placed<Row, Place> = Rc<Ordinal<Arc<Place>>>;",
        "#[serde(transparent)] pub struct Grade<T>(pub Tier<T>);",
        "#[serde(transparent)] pub struct Tier<IdType>(pub IdType);",
        "pub struct Keyed<KeyType> { pub by_key: HashMap<Rank<KeyType>, String> }",
    ]);
    for (key, refused) in [
        ("Boxed<u32>", true),
        ("Boxed<String>", false),
        ("Held<u32>", true),
        ("Held<String>", false),
        ("Borrowed<'static, i64>", true),
        ("Borrowed<'static, String>", false),
        ("Rank<u32>", true),
        ("Rank<bool>", true),
        ("Rank<String>", false),
        ("Placed<String, u8>", true),
        ("Placed<u8, String>", false),
        ("Tier<u32>", true),
        ("Grade<u32>", false),
    ] {
        let expected: Vec<String> = refused
            .then(|| bson_refused_key_message("field `by_key`", key))
            .into_iter()
            .collect();
        assert_eq!(
            bson_refused_key_messages(&format!(
                "pub struct Roster {{ pub by_key: HashMap<{key}, String> }}"
            )),
            expected,
            "for {key}"
        );
    }
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub keyed: Keyed<u32> }"),
        [held_bson_refused_key_message(
            "field `keyed`",
            "Keyed<u32>",
            "field `by_key`, a map keyed by `Rank<u32>`"
        )]
    );
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub keyed: Keyed<String> }"),
        Vec::<String>::new()
    );
}

/// What a type that lists `"mongodb"` is refused for the member `named`, which holds `model`, a
/// model declared above that holds the map `described` says.
fn held_bson_refused_key_message(named: &str, model: &str, described: &str) -> String {
    format!(
        "{named}: `{model}` holds a map that cannot be stored in MongoDB ({described}) — BSON \
         document keys are always text, and the `bson` library refuses to write this key and to \
         read it back. Key that map by a `String`, or do not list \"mongodb\" in `enable`."
    )
}

/// Records each of `declared` as its expansion records it for the items below it.
fn record_declared_above(declared: &[&str]) {
    for source in declared {
        super::record_wire_form(&syn::parse_str(source).unwrap());
    }
}

/// A model declared above that holds a map keyed by a number is refused where it is held, as
/// itself and behind an `Option`, a list, a map's value and a wrapper.
#[test]
fn a_model_declared_above_that_holds_a_map_keyed_by_a_number_is_refused_where_it_is_held() {
    record_declared_above(&[
        "pub struct Seating { pub label: String, pub by_number: HashMap<u32, String> }",
    ]);
    for written in [
        "Seating",
        "Option<Seating>",
        "Vec<Seating>",
        "HashMap<String, Seating>",
        "Box<Seating>",
        "Option<Vec<Seating>>",
        "(String, Seating)",
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Venue {{ pub seating: {written} }}")),
            [held_bson_refused_key_message(
                "field `seating`",
                "Seating",
                "field `by_number`, a map keyed by `u32`"
            )],
            "for {written}"
        );
    }
    assert_eq!(
        bson_refused_key_messages(
            "pub enum Booking { Open { seating: Seating }, Closed(String, Vec<Seating>) }"
        ),
        [
            held_bson_refused_key_message(
                "variant `Open`, field `seating`",
                "Seating",
                "field `by_number`, a map keyed by `u32`"
            ),
            held_bson_refused_key_message(
                "variant `Closed`, field `1`",
                "Seating",
                "field `by_number`, a map keyed by `u32`"
            ),
        ]
    );
}

/// A model held two levels down is refused at the holder, with each member on the way to the map.
#[test]
fn a_model_held_two_levels_down_is_refused_with_the_way_to_its_map() {
    record_declared_above(&[
        "pub struct Seating { pub by_number: HashMap<u32, String> }",
        "pub struct Floor { pub level: u8, pub seating: Seating }",
    ]);
    for written in ["Floor", "Option<Vec<Floor>>"] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Venue {{ pub floor: {written} }}")),
            [held_bson_refused_key_message(
                "field `floor`",
                "Floor",
                "field `seating`, which holds `Seating` with field `by_number`, a map keyed by `u32`"
            )],
            "for {written}"
        );
    }
}

/// An enum declared above is a held model as a struct is, its map named behind its variant.
#[test]
fn an_enum_declared_above_that_holds_a_map_keyed_by_a_bool_is_refused_where_it_is_held() {
    record_declared_above(&[
        "pub enum Lineup { Named { by_name: HashMap<String, String> }, \
         Flagged(BTreeMap<bool, String>), Empty }",
    ]);
    assert_eq!(
        bson_refused_key_messages("pub struct Venue { pub lineup: Option<Lineup> }"),
        [held_bson_refused_key_message(
            "field `lineup`",
            "Lineup",
            "variant `Flagged`, field `0`, a map keyed by `bool`"
        )]
    );
}

/// A held model is refused once, with the first map it holds under a refused key.
#[test]
fn a_held_model_is_refused_once_with_its_first_map() {
    record_declared_above(
        &["pub struct Seating { pub by_number: HashMap<u32, String>, \
         pub by_flag: BTreeMap<bool, String> }"],
    );
    assert_eq!(
        bson_refused_key_messages("pub struct Venue { pub seating: Seating }"),
        [held_bson_refused_key_message(
            "field `seating`",
            "Seating",
            "field `by_number`, a map keyed by `u32`"
        )]
    );
}

/// A model whose maps are keyed by text, one whose map serde neither writes nor reads, one not
/// declared above, and a member serde neither writes nor reads, are not refused. A model that
/// holds itself is refused for its own map alone.
#[test]
fn a_held_model_with_no_refused_map_is_not_refused() {
    record_declared_above(&[
        "pub struct Labels { pub by_name: HashMap<String, String> }",
        "pub struct Hidden { #[serde(skip)] pub by_number: HashMap<u32, String> }",
        "pub struct Seating { pub by_number: HashMap<u32, String> }",
        "pub struct Node { pub by_number: HashMap<u32, String>, pub children: Vec<Node> }",
    ]);
    for written in ["Labels", "Option<Hidden>", "Vec<Unseen>"] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Venue {{ pub held: {written} }}")),
            Vec::<String>::new(),
            "for {written}"
        );
    }
    assert_eq!(
        bson_refused_key_messages("pub struct Venue { #[serde(skip)] pub seating: Seating }"),
        Vec::<String>::new()
    );
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Node { pub by_number: HashMap<u32, String>, pub children: Vec<Node> }"
        ),
        [bson_refused_key_message("field `by_number`", "u32")]
    );
}

/// An alias declared above of a map keyed by a number is refused where it is held, as itself and
/// behind an `Option`, a list and a map's value, and the expansion of a holder that lists
/// `"mongodb"` fails naming the field, the alias and the map.
#[test]
fn an_alias_declared_above_of_a_map_keyed_by_a_number_is_refused_where_it_is_held() {
    let seats = "pub type Seats = HashMap<u32, String>;";
    let built = expansion_under("", seats);
    assert!(!built.contains("compile_error"), "got: {built}");
    for written in [
        "Seats",
        "Option<Seats>",
        "Vec<Seats>",
        "HashMap<String, Seats>",
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub seats: {written} }}")),
            [held_bson_refused_key_message(
                "field `seats`",
                "Seats",
                "a map keyed by `u32`"
            )],
            "for {written}"
        );
    }
    let refused = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct Arena { pub seats: Seats }",
    );
    assert!(
        refused.contains(&format!(
            "model_schema: type `Arena`: {}",
            held_bson_refused_key_message("field `seats`", "Seats", "a map keyed by `u32`")
                .replace('"', "\\\"")
        )),
        "got: {refused}"
    );
}

/// An alias declared above of a model that holds a map keyed by a number is refused where it is
/// held, naming the alias and the model's map.
#[test]
fn an_alias_declared_above_of_a_holder_model_is_refused_where_it_is_held() {
    record_declared_above(&[
        "pub struct Seating { pub by_number: HashMap<u32, String> }",
        "pub type Main = Seating;",
    ]);
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub main: Option<Main> }"),
        [held_bson_refused_key_message(
            "field `main`",
            "Main",
            "which holds `Seating` with field `by_number`, a map keyed by `u32`"
        )]
    );
}

/// An alias declared above of such an alias is refused where it is held, naming the alias it is
/// over.
#[test]
fn an_alias_declared_above_of_an_alias_of_a_map_keyed_by_a_number_is_refused_where_it_is_held() {
    record_declared_above(&[
        "pub type Seats = BTreeMap<bool, String>;",
        "pub type Stands = Seats;",
    ]);
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub stands: Stands }"),
        [held_bson_refused_key_message(
            "field `stands`",
            "Stands",
            "which holds `Seats` with a map keyed by `bool`"
        )]
    );
}

/// An alias declared above of a map keyed by a `String` holds nothing the `bson` library refuses.
#[test]
fn an_alias_declared_above_of_a_map_keyed_by_a_string_is_not_refused() {
    record_declared_above(&[
        "pub type Benches = HashMap<String, String>;",
        "pub type Stands = Vec<Benches>;",
    ]);
    for written in ["Benches", "Stands"] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub held: {written} }}")),
            Vec::<String>::new(),
            "for {written}"
        );
    }
}

/// An alias of a map keyed by a number declared below the type that holds it has not expanded
/// when the holder does, and is not refused there. Declared above, it is.
#[test]
fn an_alias_declared_below_of_a_map_keyed_by_a_number_is_not_refused() {
    let arena = "pub struct Arena { pub terraces: Terraces }";
    assert_eq!(bson_refused_key_messages(arena), Vec::<String>::new());
    record_declared_above(&["pub type Terraces = HashMap<u32, String>;"]);
    assert_eq!(
        bson_refused_key_messages(arena),
        [held_bson_refused_key_message(
            "field `terraces`",
            "Terraces",
            "a map keyed by `u32`"
        )]
    );
}

/// A model is recorded as its expansion runs, whatever its `enable` list, and refused only in a
/// holder that lists `"mongodb"`.
#[test]
fn a_held_model_is_refused_only_where_its_holder_lists_mongodb() {
    let seating = "pub struct Seating { pub by_number: HashMap<u32, String> }";
    let built = expansion_under("enable = [\"decode_with\"]", seating);
    assert!(!built.contains("compile_error"), "got: {built}");

    let venue = "pub struct Venue { pub seating: Seating }";
    let refused = expansion_under("enable = [\"decode_with\", \"mongodb\"]", venue);
    assert!(
        refused.contains(&format!(
            "model_schema: type `Venue`: {}",
            held_bson_refused_key_message(
                "field `seating`",
                "Seating",
                "field `by_number`, a map keyed by `u32`"
            )
            .replace('"', "\\\"")
        )),
        "got: {refused}"
    );
    for listed in ["enable = [\"decode_with\"]", ""] {
        let held = expansion_under(listed, venue);
        assert!(!held.contains("compile_error"), "for {listed}, got: {held}");
    }
}

/// Records each of `declared` as its expansion records it, and claims the name it publishes, for
/// the items below it.
fn declare_above(declared: &[&str]) {
    for source in declared {
        let item: syn::Item = syn::parse_str(source).unwrap();
        super::record_wire_form(&item);
        let ident = super::item_schema_ident(&item).unwrap().to_string();
        assert!(super::claim_published_name(&ident, &ident).is_none());
    }
}

/// A model declared above that keys a map by its own type parameter is refused where it is held
/// with a number or a `bool` in that place, as itself and behind an `Option` and a list, naming
/// the model as written and the map. Held with a `String` there, or with no argument, it is not.
#[test]
fn a_model_keyed_by_its_own_parameter_is_refused_where_it_is_held_filled_at_a_number() {
    declare_above(&[
        "pub struct Keyed<KeyType> { pub label: String, pub by_key: HashMap<KeyType, String> }",
    ]);
    for (written, model, key) in [
        ("Keyed<u32>", "Keyed<u32>", "u32"),
        ("Keyed<bool>", "Keyed<bool>", "bool"),
        ("Option<Vec<Keyed<i64>>>", "Keyed<i64>", "i64"),
        ("Box<Keyed<Box<u8>>>", "Keyed<Box<u8>>", "Box<u8>"),
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub keyed: {written} }}")),
            [held_bson_refused_key_message(
                "field `keyed`",
                model,
                &format!("field `by_key`, a map keyed by `{key}`")
            )],
            "for {written}"
        );
    }
    for written in ["Keyed<String>", "Vec<Keyed<char>>", "Keyed"] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub keyed: {written} }}")),
            Vec::<String>::new(),
            "for {written}"
        );
    }
}

/// A model declared above is refused where an argument it is held with holds a map keyed by a
/// number, naming the model as written and the map, and not where the map is keyed by a `String`.
#[test]
fn a_model_held_with_an_argument_that_holds_a_map_keyed_by_a_number_is_refused() {
    declare_above(&["pub struct Wrapper<Held> { pub held: Held }"]);
    for (written, model, key) in [
        (
            "Wrapper<HashMap<u32, String>>",
            "Wrapper<HashMap<u32, String>>",
            "u32",
        ),
        (
            "Option<Wrapper<Vec<BTreeMap<bool, String>>>>",
            "Wrapper<Vec<BTreeMap<bool, String>>>",
            "bool",
        ),
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub wrapped: {written} }}")),
            [held_bson_refused_key_message(
                "field `wrapped`",
                model,
                &format!("a map keyed by `{key}`")
            )],
            "for {written}"
        );
    }
    for written in ["Wrapper<HashMap<String, String>>", "Wrapper<Vec<u32>>"] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub wrapped: {written} }}")),
            Vec::<String>::new(),
            "for {written}"
        );
    }
}

/// A model or an alias declared above that holds a map in an arm of a `Result` is refused where it
/// is held, and so is a model that keys such a map by its own parameter, filled at a number.
#[test]
fn a_model_declared_above_that_holds_a_map_in_a_result_is_refused_where_it_is_held() {
    declare_above(&[
        "pub struct Verdict { pub outcome: Result<HashMap<u32, String>, String> }",
        "pub type Outcome = Result<String, BTreeMap<bool, String>>;",
        "pub struct Verdicts<Key> { pub outcome: Result<String, HashMap<Key, String>> }",
    ]);
    for (written, model, described) in [
        (
            "Verdict",
            "Verdict",
            "field `outcome`, a map keyed by `u32`",
        ),
        (
            "Option<Vec<Verdict>>",
            "Verdict",
            "field `outcome`, a map keyed by `u32`",
        ),
        ("Outcome", "Outcome", "a map keyed by `bool`"),
        (
            "Verdicts<u8>",
            "Verdicts<u8>",
            "field `outcome`, a map keyed by `u8`",
        ),
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub held: {written} }}")),
            [held_bson_refused_key_message(
                "field `held`",
                model,
                described
            )],
            "for {written}"
        );
    }
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub held: Verdicts<String> }"),
        Vec::<String>::new()
    );
}

/// A type of the crate's own named `Result` declared above is read as the model it is, at the
/// arguments it is held with, and the std `Result` written in full is still read as the std type.
#[test]
fn a_type_of_the_crates_own_named_result_is_read_as_the_model_it_is() {
    declare_above(&["pub struct Result<Key, Value> { pub by_key: HashMap<Key, Value> }"]);
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub held: Result<u32, String> }"),
        [held_bson_refused_key_message(
            "field `held`",
            "Result<u32, String>",
            "field `by_key`, a map keyed by `u32`"
        )]
    );
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Arena { pub held: std::result::Result<HashMap<u32, String>, String> }"
        ),
        [bson_refused_key_message("field `held`", "u32")]
    );
}

/// A type of the crate's own named `Result` with one type parameter it keys no map by builds
/// where it is held at a number.
#[test]
fn a_type_of_the_crates_own_named_result_keying_no_map_is_not_refused() {
    declare_above(&["pub struct Result<Held> { pub held: Held }"]);
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub held: Result<u32> }"),
        Vec::<String>::new()
    );
}

/// A type of the crate's own named after a range declared above is read as the model it is, and
/// the std range written in full is still read as the std type.
#[test]
fn a_type_of_the_crates_own_named_after_a_range_is_read_as_the_model_it_is() {
    declare_above(&["pub struct Range<Key> { pub by_key: HashMap<Key, String> }"]);
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub held: Range<u32> }"),
        [held_bson_refused_key_message(
            "field `held`",
            "Range<u32>",
            "field `by_key`, a map keyed by `u32`"
        )]
    );
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub held: Range<String> }"),
        Vec::<String>::new()
    );
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Arena { pub held: std::ops::Range<HashMap<u32, String>> }"
        ),
        [bson_refused_key_message("field `held`", "u32")]
    );
}

/// A model that keys a map by its own type parameter, and one whose argument holds a map keyed by
/// a number, declared below the type that holds them have not expanded when the holder does, and
/// are not refused there. Declared above, they are.
#[test]
fn a_generic_model_declared_below_is_not_refused() {
    let arena =
        "pub struct Arena { pub keyed: Keyed<u32>, pub wrapped: Wrapper<HashMap<u32, String>> }";
    assert_eq!(bson_refused_key_messages(arena), Vec::<String>::new());
    declare_above(&[
        "pub struct Keyed<KeyType> { pub by_key: HashMap<KeyType, String> }",
        "pub struct Wrapper<Held> { pub held: Held }",
    ]);
    assert_eq!(
        bson_refused_key_messages(arena),
        [
            held_bson_refused_key_message(
                "field `keyed`",
                "Keyed<u32>",
                "field `by_key`, a map keyed by `u32`"
            ),
            held_bson_refused_key_message(
                "field `wrapped`",
                "Wrapper<HashMap<u32, String>>",
                "a map keyed by `u32`"
            ),
        ]
    );
}

/// A model that hands its own parameter on to a model keyed by it keys that map by the parameter
/// too, and the map is named with what fills each parameter on the way, in a variant as in a
/// struct.
#[test]
fn a_parameter_handed_on_to_a_keyed_model_is_refused_where_a_number_fills_it() {
    declare_above(&[
        "pub struct Pair<First, Second> { pub by_first: HashMap<First, String>, \
         pub by_second: BTreeMap<Second, String> }",
        "pub struct Swapped<Left, Right> { pub label: String, pub pair: Option<Pair<Right, Left>> }",
        "pub enum Lineup<Seat> { Seated { pair: Pair<String, Seat> }, Empty }",
    ]);
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub swapped: Swapped<String, u8> }"),
        [held_bson_refused_key_message(
            "field `swapped`",
            "Swapped<String, u8>",
            "field `pair`, which holds `Pair<u8, String>` with field `by_first`, a map keyed by `u8`"
        )]
    );
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub lineup: Lineup<bool> }"),
        [held_bson_refused_key_message(
            "field `lineup`",
            "Lineup<bool>",
            "variant `Seated`, field `pair`, which holds `Pair<String, bool>` with field \
             `by_second`, a map keyed by `bool`"
        )]
    );
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Arena { pub swapped: Swapped<String, char>, pub lineup: Lineup<String> }"
        ),
        Vec::<String>::new()
    );
}

/// Every argument of a model declared above is read as one it writes into its row: one held only
/// in a `PhantomData` is refused as one held in a member.
#[test]
fn an_argument_held_only_in_phantom_data_is_read_as_written_into_the_row() {
    declare_above(&[
        "pub struct Tagged<Marked> { pub label: String, pub marker: PhantomData<Marked> }",
    ]);
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub tagged: Tagged<HashMap<u32, String>> }"),
        [held_bson_refused_key_message(
            "field `tagged`",
            "Tagged<HashMap<u32, String>>",
            "a map keyed by `u32`"
        )]
    );
}

/// A holder that lists `"mongodb"` fails naming the field, the model as written and the map, and
/// one that holds the same models filled at a `String` builds.
#[test]
fn a_holder_of_a_generic_model_filled_at_a_number_fails_where_it_lists_mongodb() {
    for (args, source) in [
        (
            "enable = [\"decode_with\"], default_types(KeyType = String)",
            "pub struct Keyed<KeyType> { pub by_key: HashMap<KeyType, String> }",
        ),
        (
            "enable = [\"decode_with\"], default_types(Held = String)",
            "pub struct Wrapper<Held> { pub held: Held }",
        ),
    ] {
        let built = expansion_under(args, source);
        assert!(!built.contains("compile_error"), "got: {built}");
    }
    let stored = "enable = [\"decode_with\", \"mongodb\"]";
    let refused = expansion_under(
        stored,
        "pub struct Arena { pub keyed: Keyed<u32>, pub wrapped: Wrapper<HashMap<u32, String>> }",
    );
    for (named, model, described) in [
        (
            "field `keyed`",
            "Keyed<u32>",
            "field `by_key`, a map keyed by `u32`",
        ),
        (
            "field `wrapped`",
            "Wrapper<HashMap<u32, String>>",
            "a map keyed by `u32`",
        ),
    ] {
        assert!(
            refused.contains(&format!(
                "model_schema: type `Arena`: {}",
                held_bson_refused_key_message(named, model, described).replace('"', "\\\"")
            )),
            "for {named}, got: {refused}"
        );
    }
    let built = expansion_under(
        stored,
        "pub struct Stands { pub keyed: Keyed<String>, \
         pub wrapped: Wrapper<HashMap<String, String>> }",
    );
    assert!(!built.contains("compile_error"), "got: {built}");
}

/// An alias declared above that keys a map by its own type parameter is refused where it is held
/// with a number or a `bool` in that place, as itself and behind an `Option` and a list, naming
/// the alias as written and the map. Held with a `String` or a `char` there, it is not.
#[test]
fn an_alias_keyed_by_its_own_parameter_is_refused_where_it_is_held_filled_at_a_number() {
    declare_above(&["pub type ByKey<KeyType> = HashMap<KeyType, String>;"]);
    for (written, alias, key) in [
        ("ByKey<u32>", "ByKey<u32>", "u32"),
        ("ByKey<bool>", "ByKey<bool>", "bool"),
        ("Option<Vec<ByKey<i64>>>", "ByKey<i64>", "i64"),
        ("ByKey<Box<u8>>", "ByKey<Box<u8>>", "Box<u8>"),
    ] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub by_key: {written} }}")),
            [held_bson_refused_key_message(
                "field `by_key`",
                alias,
                &format!("a map keyed by `{key}`")
            )],
            "for {written}"
        );
    }
    for written in ["ByKey<String>", "ByKey<char>"] {
        assert_eq!(
            bson_refused_key_messages(&format!("pub struct Arena {{ pub by_key: {written} }}")),
            Vec::<String>::new(),
            "for {written}"
        );
    }
}

/// An alias that hands its own parameter on to a model keyed by it is refused where a number
/// fills it, naming the model's map at that number. One of two parameters keyed by the second is
/// refused where the second is a number, and not where only the first is.
#[test]
fn an_alias_that_hands_its_parameter_on_to_a_keyed_model_is_refused_where_a_number_fills_it() {
    declare_above(&[
        "pub struct Keyed<KeyType> { pub by_key: HashMap<KeyType, String> }",
        "pub type Rekeyed<K> = Keyed<K>;",
        "pub type Second<A, B> = BTreeMap<B, A>;",
    ]);
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub rekeyed: Rekeyed<u32> }"),
        [held_bson_refused_key_message(
            "field `rekeyed`",
            "Rekeyed<u32>",
            "which holds `Keyed<u32>` with field `by_key`, a map keyed by `u32`"
        )]
    );
    assert_eq!(
        bson_refused_key_messages("pub struct Arena { pub second: Second<String, u32> }"),
        [held_bson_refused_key_message(
            "field `second`",
            "Second<String, u32>",
            "a map keyed by `u32`"
        )]
    );
    assert_eq!(
        bson_refused_key_messages(
            "pub struct Arena { pub rekeyed: Rekeyed<String>, pub second: Second<u32, String> }"
        ),
        Vec::<String>::new()
    );
}

/// An alias that keys a map by its own type parameter, declared below the type that holds it, has
/// not expanded when the holder does, and is not refused there. Declared above, it is.
#[test]
fn an_alias_keyed_by_its_own_parameter_declared_below_is_not_refused() {
    let arena = "pub struct Arena { pub by_key: ByKey<u32> }";
    assert_eq!(bson_refused_key_messages(arena), Vec::<String>::new());
    declare_above(&["pub type ByKey<KeyType> = HashMap<KeyType, String>;"]);
    assert_eq!(
        bson_refused_key_messages(arena),
        [held_bson_refused_key_message(
            "field `by_key`",
            "ByKey<u32>",
            "a map keyed by `u32`"
        )]
    );
}

/// An alias whose value is its own parameter is refused as a map key filled at a number, and not
/// as a member. One whose value reaches its parameter through a brand keeps that reading where an
/// argument it hands the brand holds a map keyed by its other parameter.
#[test]
fn an_alias_whose_value_is_its_own_parameter_is_read_as_a_key_and_holds_no_map() {
    declare_above(&[
        "pub type Slot<T> = T;",
        "#[serde(transparent)] pub struct Seat<Row, Place> { pub place: Place, \
         pub row: PhantomData<Row> }",
        "pub type Seated<Row, Place> = Seat<HashMap<Row, String>, Place>;",
    ]);
    for key in ["Slot<u32>", "Seated<String, u8>"] {
        assert_eq!(
            bson_refused_key_messages(&format!(
                "pub struct Roster {{ pub by_key: HashMap<{key}, String> }}"
            )),
            [bson_refused_key_message("field `by_key`", key)],
            "for {key}"
        );
    }
    assert_eq!(
        bson_refused_key_messages("pub struct Roster { pub slot: Slot<u32> }"),
        Vec::<String>::new()
    );
}

/// A holder that lists `"mongodb"` fails naming the field, the alias as written and the map, and
/// one that holds the alias filled at a `String` builds.
#[test]
fn a_holder_of_an_alias_keyed_by_its_parameter_filled_at_a_number_fails_where_it_lists_mongodb() {
    let declared = expansion_under(
        "default_types(KeyType = String)",
        "pub type ByKey<KeyType> = HashMap<KeyType, String>;",
    );
    assert!(!declared.contains("compile_error"), "got: {declared}");
    let stored = "enable = [\"decode_with\", \"mongodb\"]";
    let refused = expansion_under(stored, "pub struct Arena { pub by_key: ByKey<u32> }");
    assert!(
        refused.contains(&format!(
            "model_schema: type `Arena`: {}",
            held_bson_refused_key_message("field `by_key`", "ByKey<u32>", "a map keyed by `u32`")
                .replace('"', "\\\"")
        )),
        "got: {refused}"
    );
    let built = expansion_under(stored, "pub struct Stands { pub by_key: ByKey<String> }");
    assert!(!built.contains("compile_error"), "got: {built}");
}

/// The refusal is spanned on the field, and raised only where the type lists `"mongodb"`.
#[test]
fn a_map_keyed_by_a_number_is_refused_only_where_mongodb_is_listed() {
    let item: syn::Item =
        syn::parse_str("pub struct Roster { pub by_number: HashMap<u32, String> }").unwrap();
    let listed = |written: &str| {
        super::decode_with_guard_errors(
            &item,
            &super::parse_model_schema_args(syn::parse_str(written).unwrap()),
        )
    };
    let refusals = listed("enable = [\"decode_with\", \"mongodb\"]");
    let located: Vec<String> = refusals.iter().flat_map(located_source_texts).collect();
    assert_eq!(refusals.len(), 1);
    assert!(
        !located.is_empty() && located.iter().all(|text| text == "by_number"),
        "got: {located:?}"
    );
    assert!(listed("enable = [\"decode_with\"]").is_empty());
    assert!(listed("").is_empty());

    let source = "pub struct Roster { pub by_number: HashMap<u32, String> }";
    let refused = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
    assert!(
        refused.contains(&format!(
            "model_schema: type `Roster`: {}",
            bson_refused_key_message("field `by_number`", "u32").replace('"', "\\\"")
        )),
        "got: {refused}"
    );
    let built = expansion_under("enable = [\"decode_with\"]", source);
    assert!(!built.contains("compile_error"), "got: {built}");
}

#[test]
fn an_item_without_decode_with_expands_as_it_did() {
    for source in [
        "pub struct DecodePlain { pub name: String, pub tags: Vec<String> }",
        "pub struct DecodePlainLabel { pub label: &'static str }",
        "pub struct DecodePlainPair(pub String, pub u32);",
        "#[serde(transparent)] pub struct DecodePlainBrand(pub String);",
        "#[serde(transparent)] pub struct DecodePlainHolder { pub inner: String }",
        "pub struct DecodePlainUnit;",
        "pub enum DecodePlainChoice { One, Two }",
        "pub enum DecodePlainOutline { Circle { radius: f64 }, Empty }",
        "#[serde(tag = \"kind\")] pub enum DecodePlainFill { Clear, Solid { color: String } }",
        "#[serde(tag = \"kind\", content = \"data\")] pub enum DecodePlainStroke { Hairline, Width(u32) }",
        "#[serde(untagged)] pub enum DecodePlainContact { Email { address: String }, Phone { digits: u32 } }",
        "pub type DecodePlainAlias = String;",
    ] {
        let unflagged = expansion_under("", source);
        for added in [
            "decode_with",
            "from_value_with",
            "__tixschema",
            "Foundation",
        ] {
            assert!(
                !unflagged.contains(added),
                "for {source}, found `{added}` in: {unflagged}"
            );
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn decode_with_generates_the_entry_point_the_walker_and_the_callback_types() {
    let source = "pub struct DecodeFlagged { pub name: String }";
    let flagged = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
    for emitted in [
        "pub mod decode_flagged_schema {",
        "pub use crate :: __tixschema_decode_with :: {",
        "pub fn from_value_with < F > (",
        "pub fn from_value_piped (",
        "fn decode_with_value_report (",
        "pub fn decode_with_value_issues < I > (",
        "pub fn decode_with_value_named (",
        "pub fn decode_with_value_fields < 'a , I > (",
    ] {
        assert!(
            flagged.contains(emitted),
            "missing `{emitted}`, got: {flagged}"
        );
    }
    assert_eq!(
        flagged.contains("pub use crate :: __tixschema_mongodb :: {"),
        cfg!(feature = "mongodb"),
        "got: {flagged}"
    );
    // The callback's types are the crate's, written once by the initializers.
    let library = both_libraries();
    for declared in [
        "pub enum Segment {",
        "pub struct Path (pub :: std :: vec :: Vec < Segment >) ;",
        "pub enum Expected {",
        "pub enum Issue < V > {",
        "pub enum Verdict {",
        "pub enum Resolution {",
        "pub type Resolver < 'r , D , V > =",
        "pub fn unsettled < D , V : :: core :: clone :: Clone > (",
        "pub struct Unrecovered < V > {",
        "pub type ExpectedToken =",
        "pub type IssueFromParts < V , I > =",
        "pub fn issue_from_parts < V > (",
        "pub fn value_leaf < 'a , T , I , R , W > (",
    ] {
        assert!(
            library.contains(declared),
            "missing `{declared}`, got: {library}"
        );
        assert!(
            !flagged.contains(declared),
            "found `{declared}` in: {flagged}"
        );
    }
    for under_mongodb in [
        "pub enum OperationError {",
        "pub struct Read < 'c , T , O > {",
        "pub type WriteError =",
        "pub struct MongoPath {",
        "pub struct Filter < Root > {",
        "pub struct Update < Root > {",
        "pub struct AsFilter ;",
        "pub struct AsUpdate ;",
        "pub trait Nests < Root , N : ? :: core :: marker :: Sized , Kind > {",
        "pub struct Field < Root , V , N : ? :: core :: marker :: Sized = V > {",
        "pub struct OptionalField < Root , V , N : ? :: core :: marker :: Sized = V > {",
        "pub struct Element < V > {",
        "pub struct ListField < Root , V , N : ? :: core :: marker :: Sized = V > {",
        "pub struct Model < Root , M , N : ? :: core :: marker :: Sized = M > {",
        "pub struct OptionalModel < Root , M , N : ? :: core :: marker :: Sized = M > {",
        "pub struct ModelList < Root , M , N : ? :: core :: marker :: Sized = M > {",
        "pub struct Nested < Root , N : ? :: core :: marker :: Sized > {",
    ] {
        assert_eq!(
            library.contains(under_mongodb),
            cfg!(feature = "mongodb"),
            "for `{under_mongodb}`, got: {library}"
        );
        assert!(
            !flagged.contains(under_mongodb),
            "found `{under_mongodb}` in: {flagged}"
        );
    }
}

/// The two exceptions are the `clippy::pub_use` each re-export of a support library expects, and
/// the `doc(hidden)` on each library's module.
#[cfg(feature = "serde")]
#[test]
fn nothing_decode_with_emits_suppresses_a_lint_but_the_two_exceptions() {
    let named_fields = (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodeQuiet { pub name: String, pub spot: (String, u32), pub tags: Vec<String> }",
    );
    for (args, source) in DECODE_WITH_STRUCT_SHAPES
        .into_iter()
        .chain([named_fields])
        .chain(DECODE_WITH_ENUM_FORMS)
        .chain(DECODE_WITH_FLATTEN_FORMS)
    {
        let flagged = expansion_under(args, source);
        for suppression in ["allow", "hidden", "cfg_attr"] {
            assert!(
                !flagged.contains(&format!("# [{suppression}"))
                    && !flagged.contains(&format!("({suppression})")),
                "for {source}, found `{suppression}`, got: {flagged}"
            );
        }
        assert!(
            !flagged.contains("(expect)"),
            "for {source}, got: {flagged}"
        );
        let reexports = flagged.matches("pub use crate :: __tixschema_").count();
        assert!(reexports > 0, "for {source}, got: {flagged}");
        for expectation in ["# [expect", "# [expect (clippy :: pub_use , reason = \""] {
            assert_eq!(
                flagged.matches(expectation).count(),
                reexports,
                "for {source}, got: {flagged}"
            );
        }
    }
    let library = both_libraries();
    let hidden_modules = library.matches("# [doc (hidden)] mod __tixschema_").count();
    assert_eq!(
        hidden_modules,
        if cfg!(feature = "mongodb") { 2 } else { 1 },
        "got: {library}"
    );
    assert_eq!(
        library.matches("hidden").count(),
        hidden_modules,
        "got: {library}"
    );
    for suppression in ["allow", "expect", "cfg_attr"] {
        assert!(
            !library.contains(&format!("# [{suppression}")),
            "found `{suppression}`, got: {library}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn what_decode_with_emits_is_written_for_the_lints_a_consumer_denies() {
    let named_fields = (
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodeLinted { pub name: String }",
    );
    let library = both_libraries();
    assert!(!library.contains(": impl "), "got: {library}");
    for declared in [
        "pub enum Segment {",
        "pub struct Path (",
        "pub enum Expected {",
        "pub enum Issue < V > {",
        "pub enum Verdict {",
        "pub enum Resolution {",
        "pub struct Unrecovered < V > {",
        "pub struct Asked {",
        "pub struct TakenProbe < 'asked > (",
        #[cfg(feature = "mongodb")]
        "pub enum OperationError {",
        #[cfg(feature = "mongodb")]
        "pub struct Read < 'c , T , O > {",
        #[cfg(feature = "mongodb")]
        "pub struct MongoPath {",
        #[cfg(feature = "mongodb")]
        "pub struct Filter < Root > {",
        #[cfg(feature = "mongodb")]
        "pub struct Update < Root > {",
        #[cfg(feature = "mongodb")]
        "pub struct AsFilter ;",
        #[cfg(feature = "mongodb")]
        "pub struct AsUpdate ;",
        #[cfg(feature = "mongodb")]
        "pub struct Field < Root , V , N : ? :: core :: marker :: Sized = V > {",
        #[cfg(feature = "mongodb")]
        "pub struct OptionalField < Root , V , N : ? :: core :: marker :: Sized = V > {",
        #[cfg(feature = "mongodb")]
        "pub struct Element < V > {",
        #[cfg(feature = "mongodb")]
        "pub struct ListField < Root , V , N : ? :: core :: marker :: Sized = V > {",
        #[cfg(feature = "mongodb")]
        "pub struct Model < Root , M , N : ? :: core :: marker :: Sized = M > {",
        #[cfg(feature = "mongodb")]
        "pub struct OptionalModel < Root , M , N : ? :: core :: marker :: Sized = M > {",
        #[cfg(feature = "mongodb")]
        "pub struct ModelList < Root , M , N : ? :: core :: marker :: Sized = M > {",
        #[cfg(feature = "mongodb")]
        "pub struct Nested < Root , N : ? :: core :: marker :: Sized > {",
    ] {
        assert!(
            library.contains(&format!("# [non_exhaustive] {declared}")),
            "missing `{declared}`, got: {library}"
        );
    }
    for (args, source) in DECODE_WITH_STRUCT_SHAPES
        .into_iter()
        .chain([named_fields])
        .chain(DECODE_WITH_ENUM_FORMS)
        .chain(DECODE_WITH_FLATTEN_FORMS)
    {
        let flagged = expansion_under(args, source);
        assert!(!flagged.contains(": impl "), "for {source}, got: {flagged}");
        assert!(
            flagged.contains(
                "path : & [:: core :: result :: Result < :: std :: string :: String , usize >]"
            ) && !flagged.contains("& [Result <"),
            "for {source}, got: {flagged}"
        );
    }
}

/// A flagged type reaches another through its field by that type's own name.
#[cfg(feature = "serde")]
#[test]
fn a_flagged_type_names_the_type_it_reaches_and_never_its_module() {
    let flagged = expansion_under(
        "enable = [\"decode_with\", \"mongodb\"]",
        "pub struct DecodeOuter { pub inner: DecodeInner, pub many: Vec<DecodeInner> }",
    );
    let walker = flagged
        .split_once("pub fn decode_with_value_fields")
        .unwrap()
        .1;
    assert_eq!(
        walker
            .matches("< DecodeInner > :: decode_with_value_issues (")
            .count(),
        2,
        "got: {walker}"
    );
    assert!(!walker.contains("decode_inner_schema"), "got: {walker}");
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn what_a_schema_module_already_held_reads_the_authors_scope_past_the_added_names() {
    // A model in a shared slice and in a boxed one is one the type's unjudged read does not reach,
    // so its reader stays in the schema module.
    let source = "#[derive(Deserialize)] pub struct DecodeTicket { pub issue: Arc<[Issue]>, pub trail: Box<[Segment]> }";
    let flagged = expansion_under("enable = [\"decode_with\", \"mongodb\"]", source);
    for scoped in [
        "Result < Arc < [super :: Issue] > , D :: Error >",
        "< Arc < [super :: Issue] > > :: deserialize (deserializer)",
        "Result < Box < [super :: Segment] > , D :: Error >",
    ] {
        assert!(
            flagged.contains(scoped),
            "missing `{scoped}`, got: {flagged}"
        );
    }
    let unflagged = expansion_under("", source);
    assert!(!unflagged.contains("super :: Issue"), "got: {unflagged}");
}

#[test]
fn a_shape_the_parser_cannot_read_is_refused() {
    let probes: [proc_macro2::TokenStream; 9] = [
        quote::quote! { name = 3 },
        quote::quote! { name("Nested") },
        quote::quote! { name },
        quote::quote! { pattern = 3 },
        quote::quote! { minLength = "3" },
        quote::quote! { minLength = -1 },
        quote::quote! { maxLength = 99999999999999999999999999999999999999999 },
        quote::quote! { no_display = 3 },
        quote::quote! { bogus_flag },
    ];
    for probe in probes {
        let rendered = probe.to_string();
        assert!(args_rejection(probe).is_some(), "for {rendered}");
    }
}

/// An argument list `syn` itself cannot parse took the whole list down with it, silently.
#[test]
fn an_unparseable_argument_list_is_refused() {
    let rejection = args_rejection(quote::quote! { name = }).unwrap();
    assert_ne!(rejection, String::new());
}

#[test]
fn a_refusal_keeps_what_the_parser_had_already_read() {
    let args = super::parse_model_schema_args(quote::quote! { name = "Slug", nme = "Renamed" });
    assert_eq!(args.name_override.as_deref(), Some("Slug"));
    assert!(args.arg_rejection.is_some());
}

/// The refusal reaches the expansion as a `compile_error!` naming the type it was written on.
#[test]
fn a_refused_argument_reaches_the_expansion_as_a_named_compile_error() {
    let rejection = super::parse_model_schema_args(quote::quote! { nme = "Renamed" })
        .arg_rejection
        .unwrap();
    let item: syn::Item = syn::parse_quote! {
        struct TypeLevelUnknown {
            name: String,
        }
    };
    let error = super::attr_guard_error(&rejection, &super::item_label(&item)).to_string();
    for needle in ["compile_error", "type `TypeLevelUnknown`", "nme"] {
        assert!(error.contains(needle), "{needle} missing: {error}");
    }
}

/// The three shapes `model_schema` expands name themselves; anything else has no ident to name.
#[test]
fn every_expanded_shape_names_itself_in_a_guard_message() {
    for (item, label) in [
        (
            syn::parse_quote! { struct Carrier { name: String } },
            "type `Carrier`",
        ),
        (syn::parse_quote! { enum Carrier { One } }, "type `Carrier`"),
        (
            syn::parse_quote! { type Carrier = String; },
            "type `Carrier`",
        ),
        (syn::parse_quote! { fn carrier() {} }, "item"),
    ] {
        assert_eq!(super::item_label(&item), label);
    }
}

#[test]
fn default_types_reads_its_pairs_in_declaration_order() {
    let args = super::parse_model_schema_args(quote::quote! {
        default_types(IdType = String, DateType = f64, Nested = Vec<Option<u8>>)
    });
    assert_eq!(args.arg_rejection.as_ref().map(ToString::to_string), None);
    let read: Vec<(String, String)> = args
        .default_types
        .iter()
        .map(|(name, ty)| (name.to_string(), quote::quote!(#ty).to_string()))
        .collect();
    assert_eq!(
        read,
        vec![
            ("IdType".to_owned(), "String".to_owned()),
            ("DateType".to_owned(), "f64".to_owned()),
            ("Nested".to_owned(), "Vec < Option < u8 > >".to_owned()),
        ]
    );
}

#[test]
fn a_default_types_shape_the_parser_cannot_read_is_refused() {
    let probes: [proc_macro2::TokenStream; 6] = [
        quote::quote! { default_types = "IdType" },
        quote::quote! { default_types },
        quote::quote! { default_types() },
        quote::quote! { default_types(IdType) },
        quote::quote! { default_types("IdType" = String) },
        quote::quote! { default_types(IdType = String, , DateType = f64) },
    ];
    for probe in probes {
        let rendered = probe.to_string();
        assert!(args_rejection(probe).is_some(), "for {rendered}");
    }
}

#[test]
fn a_parameter_declared_twice_is_refused() {
    let rejection = args_rejection(quote::quote! {
        default_types(IdType = String, IdType = f64)
    })
    .unwrap();
    for needle in ["default_types", "IdType", "String", "f64"] {
        assert!(rejection.contains(needle), "{needle} missing: {rejection}");
    }
}

#[test]
fn a_repeated_parameter_is_refused_wherever_it_is_written() {
    let probes: [proc_macro2::TokenStream; 3] = [
        quote::quote! { default_types(IdType = String, IdType = String) },
        quote::quote! { default_types(IdType = String, DateType = f64, IdType = u8) },
        quote::quote! { default_types(IdType = String, DateType = f64, DateType = f64) },
    ];
    for probe in probes {
        let rendered = probe.to_string();
        assert!(args_rejection(probe).is_some(), "for {rendered}");
    }
}

#[test]
fn a_duplicate_entry_refusal_is_spanned_on_the_second_spelling() {
    let source = "default_types(IdType = String, DateType = f64, IdType = u8)";
    let args: proc_macro2::TokenStream = syn::parse_str(source).unwrap();
    let rejection = super::parse_model_schema_args(args).arg_rejection.unwrap();
    let span = rejection.span();
    assert_eq!(span.source_text().as_deref(), Some("IdType"));
    assert_eq!(span.start().column, source.rfind("IdType").unwrap());
    assert_ne!(span.start().column, source.find("IdType").unwrap());
}

#[test]
fn distinct_entries_are_read_unchanged() {
    let args = super::parse_model_schema_args(quote::quote! {
        default_types(AType = u32, BType = String, CType = u32, DType = Vec<Option<u8>>)
    });
    assert_eq!(args.arg_rejection.as_ref().map(ToString::to_string), None);
    let read: Vec<(String, String)> = args
        .default_types
        .iter()
        .map(|(name, ty)| (name.to_string(), quote::quote!(#ty).to_string()))
        .collect();
    assert_eq!(
        read,
        vec![
            ("AType".to_owned(), "u32".to_owned()),
            ("BType".to_owned(), "String".to_owned()),
            ("CType".to_owned(), "u32".to_owned()),
            ("DType".to_owned(), "Vec < Option < u8 > >".to_owned()),
        ]
    );
}

#[test]
fn default_types_coexists_with_every_other_argument() {
    let args = super::parse_model_schema_args(quote::quote! {
        name = "Slug", pattern = "^[a-z]+$", minLength = 1, maxLength = 8, no_display,
        default_types(IdType = String)
    });
    assert_eq!(args.arg_rejection.as_ref().map(ToString::to_string), None);
    assert_eq!(args.name_override.as_deref(), Some("Slug"));
    assert_eq!(args.pattern.as_deref(), Some("^[a-z]+$"));
    assert_eq!(args.min_length, Some(1));
    assert_eq!(args.max_length, Some(8));
    assert!(args.no_display);
    assert_eq!(args.default_types.len(), 1);
}

/// The `compile_error!` tokens `default_types` earns when `args` is written on `source`. Both are
/// parsed from text so their tokens carry file locations and each refusal's span can be read back
/// as the source it points at.
fn default_types_refusals(source: &str, args: &str) -> Vec<proc_macro2::TokenStream> {
    let item: syn::Item = syn::parse_str(source).unwrap();
    let parsed = super::parse_model_schema_args(syn::parse_str(args).unwrap());
    assert_eq!(
        parsed.arg_rejection.as_ref().map(ToString::to_string),
        None,
        "for {args}"
    );
    super::default_types_guard_errors(&item, &parsed)
}

/// The refusals of `args` on `source`, rendered.
fn default_types_messages(source: &str, args: &str) -> Vec<String> {
    default_types_refusals(source, args)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn an_entry_naming_no_declared_parameter_is_refused_in_every_build() {
    let messages = default_types_messages(
        "pub struct Renamed<IdType, DateType> { pub id: IdType, pub at: DateType }",
        "default_types(IdType = String, DateType = f64, WrongName = String)",
    );
    assert_eq!(messages.len(), 1, "got: {messages:?}");
    for needle in [
        "compile_error",
        "WrongName",
        "IdType",
        "DateType",
        "type `Renamed`",
    ] {
        assert!(
            messages[0].contains(needle),
            "{needle} missing: {}",
            messages[0]
        );
    }
}

#[test]
fn default_types_on_an_item_declaring_no_type_parameter_is_refused() {
    for source in [
        "pub struct Plain { pub id: String }",
        "pub enum Plain { One }",
        "pub type Plain = String;",
        "pub struct Borrowed<'label> { pub id: &'label str }",
        "pub struct Fixed<const WIDTH: usize> { pub id: [u8; WIDTH] }",
    ] {
        let messages = default_types_messages(source, "default_types(IdType = String)");
        assert_eq!(messages.len(), 1, "for {source}: {messages:?}");
        assert!(
            messages[0].contains("IdType"),
            "for {source}: {}",
            messages[0]
        );
    }
}

#[test]
fn an_item_declaring_a_default_for_every_parameter_earns_no_refusal() {
    for (source, args) in [
        (
            "pub struct Both<IdType, DateType> { pub id: IdType, pub at: DateType }",
            "default_types(IdType = String, DateType = f64)",
        ),
        (
            "pub enum Tagged<IdType> { Named { id: IdType } }",
            "default_types(IdType = String)",
        ),
        (
            "pub type Boxed<ValueType> = Vec<ValueType>;",
            "default_types(ValueType = String)",
        ),
        (
            "pub struct Mixed<'label, IdType, const WIDTH: usize> { pub id: IdType }",
            "default_types(IdType = String)",
        ),
        ("pub struct Plain { pub id: String }", ""),
    ] {
        let messages = default_types_messages(source, args);
        assert!(messages.is_empty(), "for {source}: {messages:?}");
    }
}

#[test]
fn a_default_types_refusal_is_spanned_on_what_earned_it() {
    let entry = default_types_refusals(
        "pub struct Renamed<IdType> { pub id: IdType }",
        "default_types(IdType = String, WrongName = String)",
    );
    assert_eq!(entry.len(), 1, "got: {}", entry.len());
    assert_eq!(entry[0].span().source_text().as_deref(), Some("WrongName"));

    #[cfg(feature = "jsonschema")]
    {
        let parameter = default_types_refusals(
            "pub struct EcmDocument<IdType, DateType> { pub id: IdType, pub at: DateType }",
            "default_types(IdType = String)",
        );
        assert_eq!(parameter.len(), 1, "got: {}", parameter.len());
        assert_eq!(
            parameter[0].span().source_text().as_deref(),
            Some("DateType")
        );
    }
}

#[cfg(any(feature = "zod", feature = "jsonschema"))]
#[test]
fn a_filling_no_document_can_be_built_from_is_refused_at_the_entry() {
    for written in ["i128", "u128", "f16", "f128"] {
        let messages = default_types_messages(
            "pub struct Probe<ValueType> { pub held: ValueType }",
            &format!("default_types(ValueType = {written})"),
        );
        assert_eq!(messages.len(), 1, "for {written}: {messages:?}");
        for needle in [
            "compile_error",
            written,
            "ValueType",
            &format!("{}_schema", written.to_lowercase()),
            &format!("{written}$Schema"),
            "the `zod` and `jsonschema` features",
            "type `Probe`",
        ] {
            assert!(
                messages[0].contains(needle),
                "{needle} missing for {written}: {}",
                messages[0]
            );
        }
    }
}

#[cfg(any(feature = "zod", feature = "jsonschema"))]
#[test]
fn a_filling_reaching_an_undescribable_std_type_is_refused_at_the_entry() {
    for (written, named, wire) in [
        ("OnceLock<u32>", "`OnceLock`", "Serialize"),
        ("OsString", "`OsString`", "externally"),
        ("Vec<OnceLock<u32>>", "`OnceLock`", "Serialize"),
        ("HashMap<String, OsString>", "`OsString`", "externally"),
        ("LinkedList<String>", "`LinkedList`", "Vec<T>"),
    ] {
        let messages = default_types_messages(
            "pub struct Probe<ValueType> { pub held: ValueType }",
            &format!("default_types(ValueType = {written})"),
        );
        assert_eq!(messages.len(), 1, "for {written}: {messages:?}");
        for needle in [
            "compile_error",
            "`default_types` entry `ValueType`",
            named,
            wire,
            "type `Probe`",
        ] {
            assert!(
                messages[0].contains(needle),
                "{needle} missing for {written}: {}",
                messages[0]
            );
        }
    }
}

/// The refusal points at the filling as written, the token the author can change.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
#[test]
fn an_undescribable_std_filling_refusal_is_spanned_on_the_filling() {
    let refusals = default_types_refusals(
        "pub struct Probe<IdType, HeldType> { pub id: IdType, pub held: HeldType }",
        "default_types(IdType = String, HeldType = OsString)",
    );
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    assert_eq!(
        refusals[0].span().source_text().as_deref(),
        Some("OsString")
    );
}

#[cfg(any(feature = "zod", feature = "jsonschema"))]
#[test]
fn a_filling_reaching_an_unwritable_map_key_is_refused_at_the_entry() {
    for (written, reason) in [
        ("HashMap<Vec<String>, u32>", "is a sequence of `String`"),
        ("HashMap<Option<String>, u32>", "an `Option<String>`"),
        ("HashMap<[String; 2], u32>", "is a sequence of `String`"),
        (
            "HashMap<String, HashMap<Vec<String>, u32>>",
            "is a sequence of `String`",
        ),
        (
            "Holder<HashMap<Vec<String>, u32>>",
            "is a sequence of `String`",
        ),
        (
            "HashMap<(u8, u8), u32>",
            "serde writes `(_, _)` as a JSON array",
        ),
        ("HashMap<(), u32>", "serde writes `()` as null, and refuses"),
    ] {
        let messages = default_types_messages(
            "pub struct Probe<ValueType> { pub held: ValueType }",
            &format!("default_types(ValueType = {written})"),
        );
        assert_eq!(messages.len(), 1, "for {written}: {messages:?}");
        for needle in [
            "compile_error",
            "`default_types` entry `ValueType`",
            "a map key must be",
            reason,
            "type `Probe`",
        ] {
            assert!(
                messages[0].contains(needle),
                "{needle} missing for {written}: {}",
                messages[0]
            );
        }
    }
}

#[cfg(any(feature = "zod", feature = "jsonschema"))]
#[test]
fn an_unwritable_map_key_filling_refusal_is_spanned_on_the_filling() {
    for written in [
        "HashMap<Vec<String>, u32>",
        "HashMap<Vec<Vec<String>>, u32>",
        "HashMap<Option<String>, u32>",
        "HashMap<[String; 2], u32>",
    ] {
        let refusals = default_types_refusals(
            "pub struct Probe<IdType, HeldType> { pub id: IdType, pub held: HeldType }",
            &format!("default_types(IdType = String, HeldType = {written})"),
        );
        assert_eq!(refusals.len(), 1, "for {written}: {refusals:?}");
        assert_eq!(
            refusals[0].span().source_text().as_deref(),
            Some(written),
            "for {written}"
        );
    }

    let undescribable = default_types_refusals(
        "pub struct Probe<IdType, HeldType> { pub id: IdType, pub held: HeldType }",
        "default_types(IdType = String, HeldType = HashMap<String, OsString>)",
    );
    assert_eq!(
        undescribable[0].span().source_text().as_deref(),
        Some("HashMap<String, OsString>")
    );
}

#[cfg(any(feature = "zod", feature = "jsonschema"))]
#[test]
fn a_filling_with_a_writable_map_key_clears_the_guard() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for written in [
        "HashMap<String, u32>",
        "HashMap<Slot, u32>",
        "Vec<HashMap<String, u32>>",
        "String",
    ] {
        let messages = default_types_messages(
            "pub struct Probe<ValueType> { pub held: ValueType }",
            &format!("default_types(ValueType = {written})"),
        );
        assert!(messages.is_empty(), "for {written}: {messages:?}");
    }
}

#[cfg(any(feature = "zod", feature = "jsonschema"))]
#[test]
fn an_undescribable_filling_refusal_is_spanned_on_the_filling() {
    let refusals = default_types_refusals(
        "pub struct Probe<WideType, NarrowType> { pub wide: WideType, pub narrow: NarrowType }",
        "default_types(WideType = String, NarrowType = i128)",
    );
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    assert_eq!(refusals[0].span().source_text().as_deref(), Some("i128"));
}

/// Only what is provably not a sibling is refused.
#[cfg(any(feature = "zod", feature = "jsonschema"))]
#[test]
fn a_renderable_or_sibling_named_filling_is_left_alone() {
    for written in [
        "Foo",
        "String",
        "bool",
        "char",
        "u8",
        "u64",
        "i64",
        "usize",
        "isize",
        "f32",
        "f64",
        "PathBuf",
        "Vec<char>",
        "some::path::char",
    ] {
        let messages = default_types_messages(
            "pub struct Probe<ValueType> { pub held: ValueType }",
            &format!("default_types(ValueType = {written})"),
        );
        assert!(messages.is_empty(), "for {written}: {messages:?}");
    }
}

#[cfg(not(any(feature = "zod", feature = "jsonschema")))]
#[test]
fn an_undescribable_filling_is_accepted_where_no_surface_reads_it() {
    for written in [
        "i128",
        "u128",
        "f16",
        "f128",
        "OsString",
        "OnceLock<u32>",
        "Vec<OnceLock<u32>>",
        "HashMap<String, OsString>",
        "HashMap<Vec<String>, u32>",
    ] {
        let messages = default_types_messages(
            "pub struct Probe<ValueType> { pub held: ValueType }",
            &format!("default_types(ValueType = {written})"),
        );
        assert!(messages.is_empty(), "for {written}: {messages:?}");
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_parameter_with_no_default_is_refused_where_the_json_document_is_built() {
    let messages = default_types_messages(
        "pub struct EcmDocument<IdType, DateType> { pub id: IdType, pub at: DateType }",
        "",
    );
    assert_eq!(messages.len(), 2, "got: {messages:?}");
    let joined = messages.join("\n");
    for needle in [
        "IdType",
        "DateType",
        "silently rejects valid payloads",
        "`jsonschema` feature",
        "default_types(IdType = String, DateType = String)",
        "type `EcmDocument`",
    ] {
        assert!(joined.contains(needle), "{needle} missing: {joined}");
    }
}

#[cfg(not(feature = "jsonschema"))]
#[test]
fn a_parameter_with_no_default_is_accepted_where_no_json_document_is_built() {
    for args in ["", "default_types(IdType = String)"] {
        let messages = default_types_messages(
            "pub struct EcmDocument<IdType, DateType> { pub id: IdType, pub at: DateType }",
            args,
        );
        assert!(messages.is_empty(), "for {args:?}: {messages:?}");
    }
}

/// The bound checks `args` earns on `source`. Both are parsed from text so the emitted tokens carry
/// file locations and the span each check hands the compiler can be read back as the source it was
/// written as.
fn filling_bound_checks(source: &str, args: &str) -> Vec<proc_macro2::TokenStream> {
    let item: syn::Item = syn::parse_str(source).unwrap();
    let parsed = super::parse_model_schema_args(syn::parse_str(args).unwrap());
    assert_eq!(
        parsed.arg_rejection.as_ref().map(ToString::to_string),
        None,
        "for {args}"
    );
    super::default_types_bound_checks(&item, &parsed)
}

/// The checks `args` earns on `source`, rendered.
fn filling_bound_check_text(source: &str, args: &str) -> Vec<String> {
    filling_bound_checks(source, args)
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Whether any token of `tokens`, at any depth, was written as `written` in the source it was
/// parsed from. A token the expansion synthesised was written nowhere and answers `None`.
fn some_token_was_written_as(tokens: &proc_macro2::TokenStream, written: &str) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Group(group) => some_token_was_written_as(&group.stream(), written),
        leaf @ (proc_macro2::TokenTree::Ident(_)
        | proc_macro2::TokenTree::Punct(_)
        | proc_macro2::TokenTree::Literal(_)) => {
            leaf.span().source_text().as_deref() == Some(written)
        }
    })
}

#[test]
fn a_bounded_parameters_filling_is_handed_to_a_function_carrying_that_bound() {
    let checks = filling_bound_check_text(
        "pub struct Counted<CountType: Copy> { pub count: CountType }",
        "default_types(CountType = String)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    for needle in [
        "fn default_type_filling < CountType : Copy > ()",
        "default_type_filling :: < String > ()",
    ] {
        assert!(
            checks[0].contains(needle),
            "{needle} missing: {}",
            checks[0]
        );
    }
    assert!(
        !checks[0].contains("compile_error"),
        "the compiler answers this, not the macro: {}",
        checks[0]
    );
}

#[test]
fn a_bound_check_carries_the_spans_the_entry_and_the_bound_were_written_at() {
    let checks = filling_bound_checks(
        "pub struct Counted<CountType: Copy> { pub count: CountType }",
        "default_types(CountType = String)",
    );
    assert_eq!(checks.len(), 1, "got: {}", checks.len());
    for written in ["String", "Copy", "CountType"] {
        assert!(
            some_token_was_written_as(&checks[0], written),
            "{written} was not carried at the span it was written at: {}",
            checks[0]
        );
    }
}

/// A parameter declares its bounds in either of two places, and a filling answers for both alike.
#[test]
fn a_bound_written_in_the_where_clause_is_read_like_one_written_beside_the_parameter() {
    let beside = filling_bound_check_text(
        "pub struct Counted<CountType: Copy> { pub count: CountType }",
        "default_types(CountType = String)",
    );
    let clause = filling_bound_check_text(
        "pub struct Counted<CountType> where CountType: Copy { pub count: CountType }",
        "default_types(CountType = String)",
    );
    assert_eq!(clause.len(), 1, "got: {clause:?}");
    assert_eq!(clause, beside);
}

#[test]
fn a_parameter_bounded_in_both_places_is_checked_against_every_bound() {
    let checks = filling_bound_check_text(
        "pub struct Counted<CountType: Copy> where CountType: Clone { pub count: CountType }",
        "default_types(CountType = String)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    assert!(
        checks[0].contains("fn default_type_filling < CountType : Copy + Clone > ()"),
        "got: {}",
        checks[0]
    );
}

#[test]
fn an_unbounded_parameter_earns_no_check() {
    for (source, args) in [
        (
            "pub struct Both<IdType, DateType> { pub id: IdType, pub at: DateType }",
            "default_types(IdType = String, DateType = f64)",
        ),
        (
            "pub enum Tagged<IdType> { Named { id: IdType } }",
            "default_types(IdType = String)",
        ),
        (
            "pub type Boxed<ValueType> = Vec<ValueType>;",
            "default_types(ValueType = String)",
        ),
        (
            "pub struct Mixed<'label, IdType, const WIDTH: usize> { pub id: IdType }",
            "default_types(IdType = String)",
        ),
        ("pub struct Plain { pub id: String }", ""),
    ] {
        let checks = filling_bound_check_text(source, args);
        assert!(checks.is_empty(), "for {source}: {checks:?}");
    }
}

#[test]
fn a_bound_naming_another_parameter_of_the_item_earns_no_check_of_its_own() {
    let checks = filling_bound_check_text(
        "pub struct Pair<AType: From<BType>, BType> { pub a: AType, pub b: BType }",
        "default_types(AType = String, BType = char)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    assert!(
        !checks[0].contains("fn default_type_filling <"),
        "the bound reads a neighbour, so no per-filling check carries it: {}",
        checks[0]
    );
}

#[test]
fn a_bound_naming_another_parameter_is_checked_at_the_whole_parameter_list_at_once() {
    let checks = filling_bound_check_text(
        "pub struct Pair<AType: From<BType>, BType> { pub a: AType, pub b: BType }",
        "default_types(AType = String, BType = char)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    for needle in [
        "fn default_type_fillings < AType , BType > ()",
        "where AType : From < BType >",
        "default_type_fillings :: < String , char > ()",
    ] {
        assert!(
            checks[0].contains(needle),
            "{needle} missing: {}",
            checks[0]
        );
    }
}

#[test]
fn the_joint_check_is_called_in_the_order_the_parameters_were_declared() {
    let checks = filling_bound_check_text(
        "pub struct Pair<AType: From<BType>, BType> { pub a: AType, pub b: BType }",
        "default_types(BType = char, AType = String)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    assert!(
        checks[0].contains("default_type_fillings :: < String , char > ()"),
        "got: {}",
        checks[0]
    );
}

#[test]
fn the_joint_check_declares_and_fills_the_parameters_no_bound_reads() {
    let checks = filling_bound_check_text(
        "pub struct Trio<AType: From<BType>, BType, CType> { pub a: AType, pub b: BType, pub c: CType }",
        "default_types(AType = String, BType = char, CType = u8)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    assert!(
        checks[0].contains("fn default_type_fillings < AType , BType , CType > ()")
            && checks[0].contains("default_type_fillings :: < String , char , u8 > ()"),
        "got: {}",
        checks[0]
    );
}

#[test]
fn a_lifetime_a_bound_reads_is_declared_as_written_and_left_out_of_the_call() {
    let checks = filling_bound_check_text(
        "pub struct Held<'label, ValueType: Into<&'label str>> { pub held: ValueType }",
        "default_types(ValueType = String)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    for needle in [
        "fn default_type_fillings < 'label , ValueType > ()",
        "where ValueType : Into < & 'label str >",
        "default_type_fillings :: < String > ()",
    ] {
        assert!(
            checks[0].contains(needle),
            "{needle} missing: {}",
            checks[0]
        );
    }
}

#[test]
fn a_bound_reading_a_const_parameter_earns_no_check() {
    let checks = filling_bound_check_text(
        "pub struct Bounded<ValueType: Fits<WIDTH>, const WIDTH: usize> { pub held: ValueType }",
        "default_types(ValueType = String)",
    );
    assert!(checks.is_empty(), "got: {checks:?}");
}

#[test]
fn a_const_no_bound_reads_leaves_the_joint_check_standing() {
    let checks = filling_bound_check_text(
        "pub struct Sized<AType: From<BType>, BType, const WIDTH: usize> { pub a: AType, pub b: BType }",
        "default_types(AType = String, BType = char)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    assert!(
        checks[0].contains("fn default_type_fillings < AType , BType > ()")
            && !checks[0].contains("WIDTH"),
        "got: {}",
        checks[0]
    );
}

#[test]
fn a_parameter_left_without_a_filling_withholds_the_joint_check() {
    let checks = filling_bound_check_text(
        "pub struct Pair<AType: From<BType>, BType> { pub a: AType, pub b: BType }",
        "default_types(AType = String)",
    );
    assert!(checks.is_empty(), "got: {checks:?}");
}

#[test]
fn a_parameter_bounded_both_ways_is_checked_once_against_each_half() {
    let checks = filling_bound_check_text(
        "pub struct Pair<AType: Copy + From<BType>, BType> { pub a: AType, pub b: BType }",
        "default_types(AType = u8, BType = char)",
    );
    assert_eq!(checks.len(), 2, "got: {checks:?}");
    assert!(
        checks[0].contains("fn default_type_filling < AType : Copy > ()")
            && !checks[0].contains("From"),
        "the per-filling check keeps only the neighbour-free half: {}",
        checks[0]
    );
    assert!(
        checks[1].contains("where AType : From < BType >") && !checks[1].contains("Copy"),
        "the joint check holds exactly the complement: {}",
        checks[1]
    );
}

#[test]
fn a_declaration_with_no_cross_parameter_bound_earns_no_joint_check() {
    for (source, args) in [
        (
            "pub struct Counted<CountType: Copy> { pub count: CountType }",
            "default_types(CountType = String)",
        ),
        (
            "pub struct Both<IdType, DateType> { pub id: IdType, pub at: DateType }",
            "default_types(IdType = String, DateType = f64)",
        ),
    ] {
        let checks = filling_bound_check_text(source, args);
        assert!(
            checks.iter().all(|check| !check.contains("fillings")),
            "for {source}: {checks:?}"
        );
    }
}

#[test]
fn an_alias_earns_no_check_for_a_bound_rust_leaves_unenforced() {
    for args in [
        "default_types(ValueType = String)",
        "default_types(ValueType = u32)",
    ] {
        let checks =
            filling_bound_check_text("pub type Boxed<ValueType: Copy> = Vec<ValueType>;", args);
        assert!(checks.is_empty(), "for {args}: {checks:?}");
    }
    let joint = filling_bound_check_text(
        "pub type Paired<AType: From<BType>, BType> = (AType, BType);",
        "default_types(AType = String, BType = char)",
    );
    assert!(joint.is_empty(), "got: {joint:?}");
}

#[test]
fn a_bound_naming_only_the_parameter_it_bounds_is_still_checked() {
    let checks = filling_bound_check_text(
        "pub struct Held<ValueType: Iterator<Item = ValueType>> { pub held: ValueType }",
        "default_types(ValueType = String)",
    );
    assert_eq!(checks.len(), 1, "got: {checks:?}");
    assert!(
        checks[0].contains("Iterator < Item = ValueType >"),
        "got: {}",
        checks[0]
    );
}

#[test]
fn every_bounded_filling_earns_its_own_check() {
    let checks = filling_bound_check_text(
        "pub struct Trio<AType: Copy, BType, CType: Clone> { pub a: AType, pub b: BType, pub c: CType }",
        "default_types(AType = u8, BType = String, CType = String)",
    );
    assert_eq!(checks.len(), 2, "got: {checks:?}");
    assert!(
        checks[0].contains("Copy") && checks[0].contains("u8"),
        "got: {}",
        checks[0]
    );
    assert!(
        checks[1].contains("Clone") && checks[1].contains("String"),
        "got: {}",
        checks[1]
    );
}

#[test]
fn every_expanded_shape_whose_bounds_rust_enforces_checks_its_fillings_alike() {
    let expected = filling_bound_check_text(
        "pub struct Held<ValueType: Copy> { pub held: ValueType }",
        "default_types(ValueType = String)",
    );
    assert_eq!(expected.len(), 1, "got: {expected:?}");
    assert_eq!(
        filling_bound_check_text(
            "pub enum Held<ValueType: Copy> { Named { held: ValueType } }",
            "default_types(ValueType = String)",
        ),
        expected
    );
}

/// The `compile_error!` tokens `source` earns for the example it carries against the parameters it
/// declares. Parsed from text so the tokens carry file locations and each refusal's span can be
/// read back as the source it points at.
#[cfg(feature = "zod")]
fn const_example_refusals(source: &str) -> Vec<proc_macro2::TokenStream> {
    super::const_parameter_example_errors(&syn::parse_str(source).unwrap())
}

/// The refusals `source` earns, rendered.
#[cfg(feature = "zod")]
fn const_example_messages(source: &str) -> Vec<String> {
    const_example_refusals(source)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(feature = "zod")]
#[test]
fn a_doc_example_on_a_const_declaring_item_is_refused() {
    for (source, label) in [
        (
            format!("{EXAMPLE_DOC_BLOCK}pub enum Probe<const WIDTH: usize> {{ Held }}"),
            "type `Probe`",
        ),
        (
            format!("{EXAMPLE_DOC_BLOCK}pub struct Probe<const WIDTH: usize>(pub String);"),
            "type `Probe`",
        ),
    ] {
        let messages = const_example_messages(&source);
        assert_eq!(messages.len(), 1, "for {source}: {messages:?}");
        for needle in [
            "compile_error",
            "WIDTH",
            "const parameter",
            "```rust example",
            "`zod` feature",
            label,
        ] {
            assert!(
                messages[0].contains(needle),
                "{needle} missing for {source}: {}",
                messages[0]
            );
        }
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_doc_example_is_refused_once_and_names_every_const_declared() {
    let source = format!(
        "{EXAMPLE_DOC_BLOCK}pub struct Probe<'label, ValueType, const WIDTH: usize, const DEPTH: \
         usize> {{ pub value: ValueType }}"
    );
    let refusals = const_example_refusals(&source);
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    assert_eq!(refusals[0].span().source_text().as_deref(), Some("WIDTH"));
    let rendered = refusals[0].to_string();
    for needle in ["WIDTH", "DEPTH"] {
        assert!(rendered.contains(needle), "{needle} missing: {rendered}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn several_doc_examples_on_a_const_declaring_item_earn_one_refusal() {
    let source = format!(
        "{EXAMPLE_DOC_BLOCK}{EXAMPLE_DOC_BLOCK}pub enum Probe<const WIDTH: usize> {{ Held }}"
    );
    let refusals = const_example_refusals(&source);
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    assert_eq!(refusals[0].span().source_text().as_deref(), Some("WIDTH"));
}

#[cfg(feature = "zod")]
#[test]
fn an_item_the_example_convention_covers_earns_no_refusal() {
    for source in [
        "pub enum Probe<const WIDTH: usize> { Held }".to_owned(),
        "pub struct Probe<const WIDTH: usize>(pub String);".to_owned(),
        format!("{EXAMPLE_DOC_BLOCK}pub struct Probe<'label> {{ pub label: &'label str }}"),
        format!("{EXAMPLE_DOC_BLOCK}pub struct Probe<ValueType> {{ pub value: ValueType }}"),
        format!("{EXAMPLE_DOC_BLOCK}pub enum Probe {{ Held }}"),
        format!("{EXAMPLE_DOC_BLOCK}pub struct Probe {{ pub value: String }}"),
    ] {
        let messages = const_example_messages(&source);
        assert!(messages.is_empty(), "for {source}: {messages:?}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_const_declaring_alias_is_left_alone() {
    let source = format!("{EXAMPLE_DOC_BLOCK}pub type Probe<const WIDTH: usize> = [u8; WIDTH];");
    let messages = const_example_messages(&source);
    assert!(messages.is_empty(), "got: {messages:?}");
}

/// The `compile_error!` tokens `source` earns for handing one of its own consts to a written type.
/// Parsed from text so the tokens carry file locations and each refusal's span can be read back as
/// the source it points at.
#[cfg(any(feature = "typescript", feature = "jsonschema"))]
fn const_argument_refusals(source: &str) -> Vec<proc_macro2::TokenStream> {
    super::const_parameter_argument_errors(&syn::parse_str(source).unwrap())
}

/// The refusals `source` earns, rendered.
#[cfg(any(feature = "typescript", feature = "jsonschema"))]
fn const_argument_messages(source: &str) -> Vec<String> {
    const_argument_refusals(source)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(any(feature = "typescript", feature = "jsonschema"))]
#[test]
fn a_const_handed_to_a_written_type_as_an_argument_is_refused() {
    for (source, label) in [
        (
            "pub struct Probe<const WIDTH: usize> { pub held: Leaf<WIDTH> }",
            "type `Probe`",
        ),
        (
            "pub enum Probe<const WIDTH: usize> { Held { held: Leaf<WIDTH> } }",
            "type `Probe`",
        ),
        (
            "pub type Probe<const WIDTH: usize> = Leaf<WIDTH>;",
            "type `Probe`",
        ),
    ] {
        let messages = const_argument_messages(source);
        assert_eq!(messages.len(), 1, "for {source}: {messages:?}");
        for needle in ["compile_error", "WIDTH", "const parameter", label] {
            assert!(
                messages[0].contains(needle),
                "{needle} missing for {source}: {}",
                messages[0]
            );
        }
    }
}

#[cfg(any(feature = "typescript", feature = "jsonschema"))]
#[test]
fn a_const_argument_refusal_is_spanned_on_the_argument() {
    let refusals =
        const_argument_refusals("pub struct Probe<const WIDTH: usize> { pub held: Leaf<WIDTH> }");
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    assert_eq!(refusals[0].span().source_text().as_deref(), Some("WIDTH"));
}

#[cfg(any(feature = "typescript", feature = "jsonschema"))]
#[test]
fn a_const_argument_is_found_under_every_wrapper_it_can_be_written_beneath() {
    for source in [
        "pub struct Probe<const WIDTH: usize> { pub held: Vec<Leaf<WIDTH>> }",
        "pub struct Probe<const WIDTH: usize> { pub held: Option<Box<Leaf<WIDTH>>> }",
        "pub struct Probe<const WIDTH: usize> { pub held: [Leaf<WIDTH>; 4] }",
        "pub struct Probe<const WIDTH: usize> { pub held: (String, Leaf<WIDTH>) }",
        "pub struct Probe<const WIDTH: usize> { pub held: HashMap<String, Leaf<WIDTH>> }",
    ] {
        let messages = const_argument_messages(source);
        assert_eq!(messages.len(), 1, "for {source}: {messages:?}");
    }
}

#[cfg(any(feature = "typescript", feature = "jsonschema"))]
#[test]
fn a_const_that_reaches_no_argument_list_earns_no_refusal() {
    for source in [
        "pub struct Probe<const WIDTH: usize> { pub held: [u8; WIDTH] }",
        "pub struct Probe<const WIDTH: usize> { pub held: Vec<[u8; WIDTH]> }",
        "pub struct Probe<const WIDTH: usize> { pub held: String }",
        "pub enum Probe<const WIDTH: usize> { Held }",
        "pub struct Probe<ValueType> { pub held: Leaf<ValueType> }",
        "pub struct Probe<'label> { pub held: Leaf<'label> }",
        "pub struct Probe { pub held: Leaf<String> }",
    ] {
        let messages = const_argument_messages(source);
        assert!(messages.is_empty(), "for {source}: {messages:?}");
    }
}

/// The name is claimed as the item is expanded, in every build: no surface has to be on for a
/// field written after it to hold the type, where `serde_json`'s map is refused.
#[test]
fn a_type_the_crate_declares_as_map_is_what_a_field_written_after_it_holds() {
    let holder = "pub struct Holder { pub held: Map<String, u32> }";
    let refused = expansion_under("", holder);
    assert!(refused.contains("`serde_json::Map`"), "got: {refused}");
    let written: syn::Type = syn::parse_str("Map<String, u32>").unwrap();
    let expansion = expansion_under(
        "default_types(K = String, V = u32)",
        "pub struct Map<K, V> { pub key: K, pub value: V }",
    );
    assert!(!expansion.contains("compile_error"), "got: {expansion}");
    let held = get_field_def("held", &written, "").field_type;
    assert!(
        matches!(
            &held,
            FieldDefType::SiblingType(name, arguments) if name == "Map" && arguments.len() == 2
        ),
        "got: {held:?}"
    );
    let after = expansion_under("", holder);
    assert!(!after.contains("serde_json::Map"), "got: {after}");
}

/// The `compile_error!` tokens `source` earns for the names the `indexmap` feature reads. Parsed
/// from text so each refusal's span can be read back as the source it points at.
#[cfg(not(feature = "indexmap"))]
fn indexmap_refusals(source: &str) -> Vec<proc_macro2::TokenStream> {
    super::gated_name_errors(&syn::parse_str(source).unwrap())
}

/// `sentence` as the `compile_error!` it is told through.
fn told_as_a_compile_error(sentence: &str) -> String {
    syn::Error::new(proc_macro2::Span::call_site(), sentence)
        .to_compile_error()
        .to_string()
}

#[cfg(not(feature = "indexmap"))]
#[test]
fn an_index_map_written_with_the_feature_off_is_refused_with_the_feature_named() {
    let refusals = indexmap_refusals("pub struct Document { pub data: IndexMap<String, String> }");
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    assert_eq!(
        refusals[0].to_string(),
        told_as_a_compile_error(
            "`IndexMap` is read as a map under tixschema's `indexmap` feature: add `features = \
             [\"indexmap\"]` to the `tixschema` dependency"
        )
    );
    assert_eq!(
        refusals[0].span().source_text().as_deref(),
        Some("IndexMap")
    );
}

/// The README prints the refusal as the compiler does, held here to what the guard says.
#[cfg(not(feature = "indexmap"))]
#[test]
fn the_readme_shows_the_refusal_of_an_index_map_written_with_the_feature_off() {
    let shown = format!(
        "error: {}\n --> src/main.rs:8:15",
        super::gated_name_message("IndexMap", "indexmap", "a map")
    );
    assert!(
        include_str!("../../README.md").contains(&shown),
        "the README no longer shows: {shown}"
    );
}

#[cfg(not(feature = "indexmap"))]
#[test]
fn an_index_set_written_with_the_feature_off_is_refused_with_the_feature_named() {
    let refusals = indexmap_refusals("pub struct Document { pub labels: IndexSet<String> }");
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    assert_eq!(
        refusals[0].to_string(),
        told_as_a_compile_error(
            "`IndexSet` is read as a list under tixschema's `indexmap` feature: add `features = \
             [\"indexmap\"]` to the `tixschema` dependency"
        )
    );
    assert_eq!(
        refusals[0].span().source_text().as_deref(),
        Some("IndexSet")
    );
}

#[cfg(not(feature = "indexmap"))]
#[test]
fn a_name_the_indexmap_feature_reads_is_found_wherever_an_item_writes_it() {
    for (source, found) in [
        (
            "pub struct Probe { pub held: Option<IndexMap<String, u32>> }",
            1,
        ),
        ("pub struct Probe { pub held: Vec<IndexSet<u32>> }", 1),
        (
            "pub struct Probe { pub held: HashMap<String, IndexSet<u32>> }",
            1,
        ),
        ("pub struct Probe { pub held: (String, IndexSet<u32>) }", 1),
        ("pub struct Probe { pub held: [IndexSet<u32>; 2] }", 1),
        (
            "pub struct Probe { pub held: indexmap::IndexMap<String, u32> }",
            1,
        ),
        (
            "pub struct Probe { pub held: IndexMap<String, u32, RandomState> }",
            1,
        ),
        (
            "pub struct Probe { pub held: IndexSet<u32, RandomState> }",
            1,
        ),
        ("pub struct Probe(pub IndexSet<u32>);", 1),
        (
            "pub enum Probe { Held { held: IndexMap<String, u32> }, Listed(IndexSet<u32>) }",
            2,
        ),
        ("pub type Probe = IndexMap<String, IndexSet<u32>>;", 2),
    ] {
        let refusals = indexmap_refusals(source);
        assert_eq!(refusals.len(), found, "for {source}: {refusals:?}");
    }
}

/// With the type arguments the feature reads it with, the name is the feature's; written any
/// other way it is a type of the author's, as it is with the feature on.
#[cfg(not(feature = "indexmap"))]
#[test]
fn a_name_the_indexmap_feature_would_not_read_earns_no_refusal() {
    for source in [
        "pub struct Probe { pub held: IndexMap }",
        "pub struct Probe { pub held: IndexMap<String> }",
        "pub struct Probe { pub held: IndexSet }",
        "pub struct Probe { pub held: HashMap<String, HashSet<u32>> }",
        "pub struct IndexMap<K, V> { pub next: Option<Box<IndexMap<K, V>>>, pub held: (K, V) }",
        "pub struct IndexSet<T> { pub next: Option<Box<IndexSet<T>>>, pub held: T }",
    ] {
        let refusals = indexmap_refusals(source);
        assert!(refusals.is_empty(), "for {source}: {refusals:?}");
    }
}

#[cfg(not(feature = "indexmap"))]
#[test]
fn a_type_of_the_crates_own_keeps_a_name_the_indexmap_feature_reads() {
    let source = "pub struct Probe { pub held: IndexMap<String, u32>, pub listed: IndexSet<u32> }";
    assert_eq!(indexmap_refusals(source).len(), 2);
    assert!(claim_published_name("Ordered", "IndexMap").is_none());
    assert_eq!(indexmap_refusals(source).len(), 1);
    assert!(claim_published_name("IndexSet", "IndexSet").is_none());
    assert!(indexmap_refusals(source).is_empty());
}

/// The sentence a name a feature reads earns with the feature off, told as the `compile_error!` it
/// is, and the source text its span points at.
#[cfg(any(not(feature = "chrono"), not(feature = "mongodb")))]
fn assert_refused_naming_the_feature(source: &str, refused: &str, sentence: &str) {
    let refusals = super::gated_name_errors(&syn::parse_str(source).unwrap());
    assert_eq!(refusals.len(), 1, "for {source}: {refusals:?}");
    assert_eq!(
        refusals[0].to_string(),
        told_as_a_compile_error(sentence),
        "for {source}"
    );
    assert_eq!(
        refusals[0].span().source_text().as_deref(),
        Some(refused),
        "for {source}"
    );
}

#[cfg(not(feature = "chrono"))]
#[test]
fn a_chrono_type_written_with_the_feature_off_is_refused_with_the_feature_named() {
    for (field, name) in [
        ("NaiveDate", "NaiveDate"),
        ("NaiveTime", "NaiveTime"),
        ("NaiveDateTime", "NaiveDateTime"),
        ("DateTime<Utc>", "DateTime"),
        ("Option<chrono::NaiveDate>", "NaiveDate"),
        ("Vec<DateTime<chrono::Utc>>", "DateTime"),
    ] {
        assert_refused_naming_the_feature(
            &format!("pub struct Visit {{ pub day: {field} }}"),
            name,
            &format!(
                "`{name}` is read as a date or a time under tixschema's `chrono` feature: add \
                 `features = [\"chrono\"]` to the `tixschema` dependency"
            ),
        );
    }
}

/// The README prints each refusal as the compiler does, held here to what the guard says.
#[cfg(not(feature = "chrono"))]
#[test]
fn the_readme_shows_the_refusal_of_a_chrono_type_written_with_the_feature_off() {
    let shown = format!(
        "error: {}\n --> src/main.rs:8:14",
        super::gated_name_message("NaiveDate", "chrono", "a date or a time")
    );
    assert!(
        include_str!("../../README.md").contains(&shown),
        "the README no longer shows: {shown}"
    );
}

#[cfg(not(feature = "mongodb"))]
#[test]
fn the_readme_shows_the_refusal_of_an_object_id_written_with_the_feature_off() {
    let shown = format!(
        "error: {}\n --> src/main.rs:8:13",
        super::gated_name_message("ObjectId", "mongodb", "a MongoDB ObjectId")
    );
    assert!(
        include_str!("../../README.md").contains(&shown),
        "the README no longer shows: {shown}"
    );
}

#[cfg(not(feature = "chrono"))]
#[test]
fn a_date_time_without_a_type_argument_is_no_chrono_type() {
    let source = "pub struct Visit { pub day: DateTime }";
    assert!(super::gated_name_errors(&syn::parse_str(source).unwrap()).is_empty());
}

#[cfg(not(feature = "chrono"))]
#[test]
fn a_type_of_the_crates_own_keeps_a_name_the_chrono_feature_reads() {
    let source = "pub struct Visit { pub day: NaiveDate }";
    assert_eq!(
        super::gated_name_errors(&syn::parse_str(source).unwrap()).len(),
        1
    );
    assert!(super::claim_published_name("NaiveDate", "NaiveDate").is_none());
    assert!(super::gated_name_errors(&syn::parse_str(source).unwrap()).is_empty());
}

#[cfg(not(feature = "mongodb"))]
#[test]
fn an_object_id_written_with_the_feature_off_is_refused_with_the_feature_named() {
    for field in ["ObjectId", "Option<bson::oid::ObjectId>", "Vec<ObjectId>"] {
        assert_refused_naming_the_feature(
            &format!("pub struct Record {{ pub id: {field} }}"),
            "ObjectId",
            "`ObjectId` is read as a MongoDB ObjectId under tixschema's `mongodb` feature: add \
             `features = [\"mongodb\"]` to the `tixschema` dependency",
        );
    }
}

#[cfg(not(feature = "mongodb"))]
#[test]
fn a_type_of_the_crates_own_keeps_a_name_the_mongodb_feature_reads() {
    let source = "pub struct Record { pub id: ObjectId }";
    assert_eq!(
        super::gated_name_errors(&syn::parse_str(source).unwrap()).len(),
        1
    );
    assert!(super::claim_published_name("ObjectId", "ObjectId").is_none());
    assert!(super::gated_name_errors(&syn::parse_str(source).unwrap()).is_empty());
}

/// Holds the table to a name of each shape the features gate, whatever features this build has on.
#[test]
fn a_table_of_gated_names_refuses_each_name_with_its_own_feature() {
    static TABLE: &[super::GatedName] = &[
        super::GatedName {
            feature: "dates",
            least_arguments: 0,
            name: "Stamp",
            read_as: "a date",
        },
        super::GatedName {
            feature: "zones",
            least_arguments: 1,
            name: "Zoned",
            read_as: "a zoned date",
        },
    ];
    let item: syn::Item = syn::parse_str(
        "pub struct Visit { pub day: Stamp, pub at: Option<Zoned<Utc>>, pub bare: Zoned }",
    )
    .unwrap();
    let refusals: Vec<String> = super::gated_name_errors_in(&item, TABLE)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        refusals,
        [
            told_as_a_compile_error(
                "`Stamp` is read as a date under tixschema's `dates` feature: add `features = \
                 [\"dates\"]` to the `tixschema` dependency"
            ),
            told_as_a_compile_error(
                "`Zoned` is read as a zoned date under tixschema's `zones` feature: add `features \
                 = [\"zones\"]` to the `tixschema` dependency"
            ),
        ]
    );
}

/// The refused item is emitted as it was written, and nothing beside it names a module a map
/// would have been looked up in.
#[cfg(not(feature = "indexmap"))]
#[test]
fn an_item_refused_for_an_indexmap_name_is_emitted_with_the_refusal_alone() {
    let expansion =
        expansion_over("pub struct Document { pub data: IndexMap<String, String> }").to_string();
    assert!(
        expansion.contains("`IndexMap` is read as a map under tixschema's `indexmap` feature"),
        "got: {expansion}"
    );
    assert!(
        expansion.contains("pub struct Document { pub data : IndexMap < String , String > }"),
        "got: {expansion}"
    );
    assert!(!expansion.contains("index_map_schema"), "got: {expansion}");
    assert_eq!(
        expansion.matches("compile_error").count(),
        1,
        "got: {expansion}"
    );
}

/// Builds the `Display` assertion for the sole field of `source`, parsed from text so its spans
/// carry file locations and `source_text()` can report what they point at.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn display_assertion(source: &str) -> proc_macro2::TokenStream {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let field = item.fields.iter().next().unwrap();
    super::build_branded_display_assertion(field, &item.generics)
}

/// Collects the source text each token in `tokens` points at, skipping the macro-synthesized
/// tokens that carry no location.
fn located_source_texts(tokens: &proc_macro2::TokenStream) -> Vec<String> {
    let mut texts = Vec::new();
    for tree in tokens.clone() {
        match &tree {
            proc_macro2::TokenTree::Group(group) => {
                texts.extend(located_source_texts(&group.stream()));
            }
            proc_macro2::TokenTree::Ident(_)
            | proc_macro2::TokenTree::Punct(_)
            | proc_macro2::TokenTree::Literal(_) => texts.extend(tree.span().source_text()),
        }
    }
    texts
}

/// Asserts that every located token in `tokens` points at `expected` and that at least one does —
/// what a diagnostic spanned on one written value looks like, and what one falling back to the
/// attribute does not.
#[cfg(feature = "jsonschema")]
fn assert_points_only_at(tokens: &proc_macro2::TokenStream, expected: &str, context: &str) {
    let located = located_source_texts(tokens);
    assert!(!located.is_empty(), "for {context}, nothing located");
    assert!(
        located.iter().all(|text| text == expected),
        "for {context}, got: {located:?}"
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn the_span_probe_sees_an_unlocated_token_stream() {
    let tokens = quote::quote! { const _: () = {}; };
    assert!(tokens.span().source_text().is_none());
    assert_eq!(located_source_texts(&tokens), Vec::<String>::new());
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn display_assertion_names_the_trait_and_points_at_the_inner_field() {
    let tokens = display_assertion("pub struct Tags(pub Vec<String>);");
    let rendered = tokens.to_string();
    assert!(
        rendered.contains(":: std :: fmt :: Display"),
        "got: {rendered}"
    );
    assert!(rendered.contains("Vec < String >"), "got: {rendered}");
    assert_eq!(tokens.span().source_text().as_deref(), Some("Vec<String>"));
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn display_assertion_is_skipped_when_the_inner_names_a_generic_param() {
    for source in [
        "pub struct DocumentId<IdType>(pub IdType);",
        "pub struct Wrapped<T>(pub Vec<T>);",
        "pub struct Borrowed<'a>(pub &'a str);",
        "pub struct Fixed<const N: usize>(pub [u8; N]);",
    ] {
        let tokens = display_assertion(source);
        assert!(tokens.is_empty(), "expected no assertion for {source}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn display_impl_delegates_from_the_inner_field_span() {
    let item: syn::ItemStruct = syn::parse_str("pub struct UserId(pub String);").unwrap();
    let field = item.fields.iter().next().unwrap();
    let tokens = super::build_branded_display_impl(
        &item.generics,
        &item.ident,
        field,
        &super::brand_member(0, field),
    );
    assert_eq!(
        tokens.to_string(),
        "impl :: std :: fmt :: Display for UserId where String : :: std :: fmt :: Display { fn fmt (& self , f : & mut :: std :: fmt :: Formatter < '_ >) -> :: std :: fmt :: Result { :: std :: fmt :: Display :: fmt (& self . 0 , f) } }"
    );
    assert_eq!(
        located_source_texts(&tokens).join(" "),
        "UserId String String String String String String String String String String String \
         String String String String String String String String String String String String \
         String String String String String String",
        "the interpolated type name, then every token of the where-clause predicate (the bound is \
         spanned on the field, so a non-`Display` inner is blamed there rather than at the \
         attribute), then the call that hands the formatting to the inner field"
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn generic_display_impl_bounds_every_type_parameter() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct DocumentId<IdType>(pub IdType);").unwrap();
    let field = item.fields.iter().next().unwrap();
    let tokens = super::build_branded_display_impl(
        &item.generics,
        &item.ident,
        field,
        &super::brand_member(0, field),
    );
    assert_eq!(
        tokens.to_string(),
        "impl < IdType > :: std :: fmt :: Display for DocumentId < IdType > where IdType : :: std :: fmt :: Display { fn fmt (& self , f : & mut :: std :: fmt :: Formatter < '_ >) -> :: std :: fmt :: Result { :: std :: fmt :: Display :: fmt (& self . 0 , f) } }"
    );
}

/// Builds the whole `Display` block — assertion plus impl — for the sole field of `source`.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn display_tokens(source: &str, args: &proc_macro2::TokenStream) -> String {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let field = item.fields.iter().next().unwrap();
    super::build_branded_display_tokens(
        &item.generics,
        &item.ident,
        field,
        &super::brand_member(0, field),
        &super::parse_model_schema_args(args.clone()),
    )
    .to_string()
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn constraints_keep_the_display_assertion_when_the_brand_opts_out_of_the_impl() {
    let tokens = display_tokens(
        "pub struct Slugs(pub Tags);",
        &quote::quote! { no_display, pattern = "^[a-z]+$" },
    );
    assert!(
        tokens.contains("assert_display :: < Tags > ()"),
        "got: {tokens}"
    );
    assert!(
        !tokens.contains("impl :: std :: fmt :: Display"),
        "got: {tokens}"
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn the_display_block_carries_only_the_impl_once_the_impl_is_emitted() {
    let impl_only = display_tokens("pub struct UserId(pub String);", &quote::quote! {});
    assert!(
        !impl_only.contains("assert_display")
            && impl_only.contains("impl :: std :: fmt :: Display for UserId")
            && impl_only.contains("where String : :: std :: fmt :: Display"),
        "got: {impl_only}"
    );
    assert_eq!(
        display_tokens(
            "pub struct SlugId(pub String);",
            &quote::quote! { pattern = "^[a-z]+$" }
        ),
        impl_only.replace("UserId", "SlugId"),
        "a constrained brand that kept its impl emits what an unconstrained one does"
    );
    assert_eq!(
        display_tokens(
            "pub struct Tags(pub Vec<String>);",
            &quote::quote! { no_display }
        ),
        "",
        "an unconstrained opt-out emits neither half"
    );
}

/// How many tokens of each `to_string()` call site a constrained brand expands to point back at
/// the inner field. Built from parsed source, so a token that carries the inner type's span has a
/// location to report and a macro-synthesized one does not.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn constrained_brand_inner_spanned_tokens(source: &str, inner: &str) -> (usize, usize) {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let args = super::parse_model_schema_args(quote::quote! { pattern = "^[a-z]+$" });
    let field = item.fields.iter().next().unwrap();
    let validation =
        super::build_branded_validation(&args, &[], field, &super::brand_member(0, field)).unwrap();
    let module_ident = syn::Ident::new("slug_id_schema", proc_macro2::Span::call_site());
    let bound = super::BrandBound::of(Some(&validation), &module_ident);
    let validate_method = super::with_branded_checks(bound.held.as_ref(), &[]);
    let count = |tokens| {
        located_source_texts(tokens)
            .iter()
            .filter(|text| text.as_str() == inner)
            .count()
    };
    (count(&bound.alone), count(&validate_method))
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn constrained_to_string_calls_carry_the_inner_fields_location() {
    let (bound_alone, validate_method) =
        constrained_brand_inner_spanned_tokens("pub struct SlugId(pub Tags);", "Tags");
    assert_eq!(
        bound_alone, 3,
        "the three respanned to_string() tokens alone"
    );
    assert_eq!(
        validate_method, 3,
        "the three respanned to_string() tokens alone"
    );
}

/// A constrained brand's emitted text for an inner spelled `inner`, as the three streams it is made
/// of: the validator, the method that answers for the bound alone, and the `validate()` method,
/// each of which calls into the first.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn constrained_brand_emission(inner: &str, module: &str) -> (String, String, String) {
    brand_emission_under(inner, module, quote::quote! { pattern = "^[a-z]+$" })
}

/// [`constrained_brand_emission`] under the checks `written` on the brand.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn brand_emission_under(
    inner: &str,
    module: &str,
    written: proc_macro2::TokenStream,
) -> (String, String, String) {
    let ty: syn::Type = syn::parse_str(inner).unwrap();
    let item: syn::ItemStruct = syn::parse_quote!(
        pub struct Branded(pub #ty);
    );
    let args = super::parse_model_schema_args(written);
    let field = item.fields.iter().next().unwrap();
    let validation =
        super::build_branded_validation(&args, &[], field, &super::brand_member(0, field)).unwrap();
    let module_ident = syn::Ident::new(module, proc_macro2::Span::call_site());
    let bound = super::BrandBound::of(Some(&validation), &module_ident);
    let validate_method = super::with_branded_checks(bound.held.as_ref(), &[]);
    (
        bound.module_items.to_string(),
        bound.alone.to_string(),
        validate_method.to_string(),
    )
}

/// A brand over a list is counted by the validator a list field gets, under the brand's own name
/// and reporting under no field.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_brand_over_a_list_counts_its_items_where_a_string_brand_measures_its_text() {
    let lengths = || quote::quote! { minLength = 1, maxLength = 3 };
    let (validate_fn, bound_alone, validate_method) =
        brand_emission_under("Vec<Part>", "name_config_schema", lengths());
    let field_module = emitted_list_module("Vec<Part>");
    assert_eq!(
        validate_fn,
        field_module[..field_module.find(" pub fn deserialize_field").unwrap()]
            .replace("validate_field_value", "validate_value")
            .replace("\"'{}': {}, got {}\" , \"field\" ,", "\"{}, got {}\" ,")
    );
    assert!(
        bound_alone.contains(
            "pub fn model_schema_validate_bound (& self) -> :: core :: result :: Result < () , :: \
             std :: vec :: Vec < :: std :: string :: String >> { name_config_schema :: \
             validate_value (& self . 0) }"
        ),
        "got: {bound_alone}"
    );
    assert!(
        validate_method.contains("name_config_schema :: validate_value (& self . 0)"),
        "got: {validate_method}"
    );

    let (_, boxed_bound, boxed_method) =
        brand_emission_under("Box<[String]>", "name_config_schema", lengths());
    assert!(
        boxed_bound.contains("{ name_config_schema :: validate_value (& * self . 0) }"),
        "got: {boxed_bound}"
    );
    assert!(
        boxed_method.contains("name_config_schema :: validate_value (& * self . 0)"),
        "got: {boxed_method}"
    );
}

/// The constrained path's generated text is what it has always been.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn the_constrained_path_renders_the_same_to_string_calls_it_always_has() {
    for spelling in ["String", "Option<PathBuf>", "Slug"] {
        let (validate_fn, bound_alone, validate_method) =
            constrained_brand_emission(spelling, "slug_id_schema");
        assert!(
            validate_fn.starts_with(
                "pub fn validate_value (value : & str) -> :: core :: result :: Result < () , :: std \
                 :: vec :: Vec < :: std :: string :: String >> {"
            ),
            "for {spelling}, got: {validate_fn}"
        );
        assert!(
            bound_alone
                .contains("{ slug_id_schema :: validate_value (& self . 0 . to_string ()) }"),
            "for {spelling}, got: {bound_alone}"
        );
        assert!(
            validate_method
                .contains("slug_id_schema :: validate_value (& self . 0 . to_string ())"),
            "for {spelling}, got: {validate_method}"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_path_brand_is_checked_through_its_lossy_rendering() {
    for spelling in [
        "PathBuf",
        "std::path::PathBuf",
        "Arc<Path>",
        "Box<Path>",
        "Cow<'static, Path>",
        "Rc<std::path::Path>",
        "Box<Arc<Path>>",
    ] {
        let (validate_fn, bound_alone, validate_method) =
            constrained_brand_emission(spelling, "asset_path_schema");
        assert!(
            validate_fn.starts_with(
                "pub fn validate_value (path : & :: std :: path :: Path) -> :: core :: result :: \
                 Result < () , :: std :: vec :: Vec < :: std :: string :: String >> { let rendered = \
                 path . to_string_lossy () ; let value : & str = & rendered ;"
            ),
            "for {spelling}, got: {validate_fn}"
        );
        assert!(
            bound_alone.contains("{ asset_path_schema :: validate_value (& self . 0) }"),
            "for {spelling}, got: {bound_alone}"
        );
        assert!(
            validate_method.contains("asset_path_schema :: validate_value (& self . 0)"),
            "for {spelling}, got: {validate_method}"
        );
    }
}

/// The JSON schema statements a map-typed field expands to, parsed from the map type's source and
/// dispatched through the field entry point, so a wrapper spelling reaches the map arm the way a
/// declared field reaches it.
#[cfg(feature = "jsonschema")]
fn map_field_schema(map_type: &str) -> proc_macro2::TokenStream {
    let ty: syn::Type = syn::parse_str(map_type).unwrap();
    super::build_field_type_schema(&super::get_field_def("m", &ty, ""), "m")
}

/// The value a field's `properties` insertion carries, lifted out of the statement so a wrapped
/// spelling can be held against the map it holds.
#[cfg(feature = "jsonschema")]
fn inserted_field_value(field_type: &str) -> String {
    const PREFIX: &str = r#"properties . insert ("m" . to_string () , "#;
    const SUFFIX: &str = ") ;";
    let tokens = map_field_schema(field_type).to_string();
    assert!(
        tokens.starts_with(PREFIX) && tokens.ends_with(SUFFIX),
        "for {field_type}, not a plain insertion: {tokens}"
    );
    tokens[PREFIX.len()..tokens.len() - SUFFIX.len()].to_owned()
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_unsupported_enum_keyed_map_value_emits_only_the_compile_error() {
    for map_type in [
        "HashMap<Slot, HashMap<(String, u32), u8>>",
        "HashMap<Slot, Vec<HashMap<(String, u32), u8>>>",
        "HashMap<Slot, HashMap<String, HashMap<(String, u32), u8>>>",
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.starts_with(":: core :: compile_error !"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains("enum_members"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains("properties . insert"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            tokens.contains(&format!("model_schema: field `m`: {TUPLE_KEY_REFUSAL}")),
            "for {map_type}, got: {tokens}"
        );
    }
}

/// The enum-key branch's rendering for a map value built by hand, for the value types no source type
/// produces.
#[cfg(feature = "jsonschema")]
fn enum_key_map_value_binding(field_type: FieldDefType) -> String {
    let ty: syn::Type = syn::parse_str("String").unwrap();
    let mut value = super::get_field_def("m", &ty, "");
    value.field_type = field_type;
    super::enum_key_map_json_schema_value("Slot", proc_macro2::Span::call_site(), &value)
        .unwrap()
        .to_string()
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_nested_enum_keyed_map_value_renders_its_inner_members() {
    for (map_type, expected) in [
        (
            "HashMap<Slot, HashMap<String, String>>",
            r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "string" } })"#,
        ),
        (
            "HashMap<Slot, HashMap<String, Vec<u64>>>",
            r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "array" , "items" : { "type" : "integer" } } })"#,
        ),
        (
            "HashMap<Slot, HashMap<String, HashMap<String, f64>>>",
            r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "object" , "additionalProperties" : { "type" : "number" } } })"#,
        ),
        (
            "HashMap<Slot, HashMap<String, Inner>>",
            r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs) })"#,
        ),
        (
            "HashMap<Slot, Vec<HashMap<String, String>>>",
            r#"serde_json :: json ! ({ "type" : "array" , "items" : serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "string" } }) })"#,
        ),
        (
            "HashMap<Slot, Option<HashMap<String, String>>>",
            r#"serde_json :: json ! ({ "anyOf" : [serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "string" } }) , { "type" : "null" }] })"#,
        ),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!("let value_schema = {expected} ;")),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            tokens.contains("Slot :: enum_members ()"),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_generic_sibling_enum_keyed_map_value_emits_the_sibling_schema() {
    let tokens = map_field_schema("HashMap<Slot, Wrapper<String>>").to_string();
    assert!(
        tokens.contains(
            "let arguments = [serde_json :: json ! ({ \"type\" : \"string\" })] ; wrapper_schema \
             :: Schema :: json_schema_within_with (in_flight , hoisted_defs , & arguments)"
        ),
        "got: {tokens}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_opaque_enum_keyed_map_value_stays_permissive() {
    let tokens = enum_key_map_value_binding(FieldDefType::Unknown);
    assert!(
        tokens.contains("let value_schema = serde_json :: json ! ({ }) ;"),
        "got: {tokens}"
    );
}

/// A string literal keeps the `const` it carries on the `String`-key path.
#[cfg(feature = "jsonschema")]
#[test]
fn a_string_literal_enum_keyed_map_value_keeps_its_const() {
    let tokens = enum_key_map_value_binding(FieldDefType::StringLiteral("Tixena".to_owned()));
    assert!(
        tokens.contains(
            r#"let value_schema = serde_json :: json ! ({ "type" : "string" , "const" : "Tixena" }) ;"#
        ),
        "got: {tokens}"
    );
}

/// What the text of the chrono type `value_type` names is held to, as the tokens of a JSON literal
/// write it: the `format` of a date and of a date and time with its zone, and the `pattern` of the
/// text serde writes for a time and a date and time with no zone.
#[cfg(all(feature = "chrono", feature = "jsonschema"))]
fn chrono_text_held(value_type: &str) -> String {
    match value_type {
        "NaiveDate" => r#""format" : "date""#.to_owned(),
        "NaiveTime" => {
            r#""pattern" : "^(?:(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\\.[0-9]+)?)$""#
                .to_owned()
        }
        "NaiveDateTime" => concat!(
            r#""pattern" : "^(?:(?:(?:[0-9][0-9][2468][048]|[0-9][0-9][13579][26]|"#,
            r"[0-9][0-9]0[48]|[02468][048]00|[13579][26]00)-02-29|[0-9]{4}-(?:(?:0[13578]|",
            r"1[02])-(?:0[1-9]|[12][0-9]|3[01])|(?:0[469]|11)-(?:0[1-9]|[12][0-9]|30)|",
            r"(?:02)-(?:0[1-9]|1[0-9]|2[0-8])))T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]",
            r#"(?:\\.[0-9]+)?)$""#
        )
        .to_owned(),
        _ => r#""format" : "date-time""#.to_owned(),
    }
}

/// A chrono value keeps the form it carries in field position, as it does under a `String` key.
#[cfg(all(feature = "chrono", feature = "jsonschema"))]
#[test]
fn a_chrono_enum_keyed_map_value_keeps_its_form() {
    for value_type in ["NaiveDate", "NaiveTime", "NaiveDateTime", "DateTime<Utc>"] {
        let map_type = format!("HashMap<Slot, {value_type}>");
        let tokens = map_field_schema(&map_type).to_string();
        assert!(
            tokens.contains(&format!(
                r#"let value_schema = serde_json :: json ! ({{ "type" : "string" , {} }}) ;"#,
                chrono_text_held(value_type)
            )),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(all(feature = "mongodb", feature = "jsonschema"))]
#[test]
fn an_object_id_enum_keyed_map_value_binds_the_one_oid_object() {
    let tokens = enum_key_map_value_binding(FieldDefType::ObjectId);
    assert!(
        tokens.contains(
            r#"let value_schema = serde_json :: json ! ({ "type" : "object" , "properties" : { "$oid" : (serde_json :: json ! ({ "type" : "string" , "pattern" : "^[a-f0-9]{24}$" })) } , "required" : ["$oid"] , "additionalProperties" : false }) ;"#
        ),
        "got: {tokens}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_scalar_enum_keyed_map_value_expands_to_the_per_member_loop() {
    let tokens = map_field_schema("HashMap<Slot, String>").to_string();
    assert!(!tokens.contains("compile_error"), "got: {tokens}");
    assert!(tokens.contains("Slot :: enum_members ()"), "got: {tokens}");
    assert!(
        tokens.contains(r#"serde_json :: json ! ({ "type" : "string" })"#),
        "got: {tokens}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_vec_sibling_enum_keyed_map_value_arrays_the_sibling_schema() {
    let arrayed = map_field_schema("HashMap<Slot, Vec<Inner>>").to_string();
    assert!(
        arrayed.contains(
            r#"let value_schema = serde_json :: json ! ({ "type" : "array" , "items" : inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs) }) ;"#
        ),
        "got: {arrayed}"
    );

    let single = map_field_schema("HashMap<Slot, Inner>").to_string();
    assert!(
        single.contains("let value_schema = inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs) ;"),
        "got: {single}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_optional_map_value_is_nullable_on_both_key_paths() {
    let enum_keyed = map_field_schema("HashMap<Slot, Option<String>>").to_string();
    assert!(
        enum_keyed.contains(
            r#"let value_schema = serde_json :: json ! ({ "anyOf" : [serde_json :: json ! ({ "type" : "string" }) , { "type" : "null" }] }) ;"#
        ),
        "got: {enum_keyed}"
    );

    let string_keyed = map_field_schema("HashMap<String, Option<String>>").to_string();
    assert!(
        string_keyed.contains(
            r#""additionalProperties" : { "anyOf" : [{ "type" : "string" } , { "type" : "null" }] }"#
        ),
        "got: {string_keyed}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_required_map_value_carries_no_nullable_wrap() {
    for map_type in [
        "HashMap<Slot, String>",
        "HashMap<Slot, Vec<Inner>>",
        "HashMap<String, String>",
        "HashMap<String, Vec<u64>>",
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(!tokens.contains("anyOf"), "for {map_type}, got: {tokens}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_registers_the_kind_of_what_it_targets() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    register_alias_info("Doc", "Doc", "doc_schema", AliasKind::NoEnumMembers);
    register_alias_info(
        "CorrelationId",
        "CorrelationId",
        "correlation_id_schema",
        AliasKind::StringWire,
    );
    register_alias_info("Tick", "Tick", "tick_schema", AliasKind::Stringified);
    for (target, expected) in [
        ("Slot", AliasKind::EnumMembers),
        ("Doc", AliasKind::NoEnumMembers),
        ("String", AliasKind::StringWire),
        ("PathBuf", AliasKind::StringWire),
        ("CorrelationId", AliasKind::StringWire),
        ("u32", AliasKind::Stringified),
        ("bool", AliasKind::Stringified),
        ("f64", AliasKind::Stringified),
        ("Tick", AliasKind::Stringified),
        ("Wrapper<Slot>", AliasKind::Stringified),
        ("Vec<String>", AliasKind::NoEnumMembers),
        ("Option<String>", AliasKind::NoEnumMembers),
        ("Vec<Slot>", AliasKind::NoEnumMembers),
        ("HashMap<Slot, String>", AliasKind::NoEnumMembers),
        ("(String, String)", AliasKind::NoEnumMembers),
        ("Ghost", AliasKind::Unknown),
    ] {
        let ty: syn::Type = syn::parse_str(target).unwrap();
        let kind = super::alias_target_kind(&super::get_field_def("AliasType", &ty, ""));
        assert_eq!(kind, expected, "for alias target {target}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_and_a_brand_answer_alike_for_the_same_target() {
    register_alias_info("Doc", "Doc", "doc_schema", AliasKind::NoEnumMembers);
    register_alias_info(
        "CorrelationId",
        "CorrelationId",
        "correlation_id_schema",
        AliasKind::StringWire,
    );
    register_alias_info("Tick", "Tick", "tick_schema", AliasKind::Stringified);
    for target in [
        "String",
        "PathBuf",
        "CorrelationId",
        "u32",
        "bool",
        "f64",
        "Tick",
        "Doc",
        "Vec<String>",
        "(String, String)",
        "HashMap<String, u32>",
    ] {
        let ty: syn::Type = syn::parse_str(target).unwrap();
        let alias = super::alias_target_kind(&super::get_field_def("AliasType", &ty, ""));
        assert_eq!(alias, brand_kind(target), "for target {target}");
    }
}

#[cfg(all(
    feature = "chrono",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn an_alias_of_a_chrono_target_is_stringified() {
    for target in ["NaiveDate", "NaiveTime", "NaiveDateTime", "DateTime<Utc>"] {
        let ty: syn::Type = syn::parse_str(target).unwrap();
        let kind = super::alias_target_kind(&super::get_field_def("AliasType", &ty, ""));
        assert_eq!(kind, AliasKind::Stringified, "for alias target {target}");
    }
}

#[cfg(all(
    feature = "mongodb",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn an_alias_of_an_object_id_stays_refused() {
    let ty: syn::Type = syn::parse_str("ObjectId").unwrap();
    let kind = super::alias_target_kind(&super::get_field_def("AliasType", &ty, ""));
    assert_eq!(kind, AliasKind::NoEnumMembers);
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_chain_carries_the_stringified_kind_to_its_end() {
    register_alias_info("Tick", "Tick", "tick_schema", AliasKind::Stringified);
    for end in ["u32", "Tick"] {
        let first_target: syn::Type = syn::parse_str(end).unwrap();
        let first = super::alias_target_kind(&super::get_field_def("FirstType", &first_target, ""));
        assert_eq!(first, AliasKind::Stringified, "for a chain ending at {end}");
        register_alias_info("First", "FirstType", "first_type_schema", first);

        let second_target: syn::Type = syn::parse_str("First").unwrap();
        let second =
            super::alias_target_kind(&super::get_field_def("SecondType", &second_target, ""));
        assert_eq!(
            second,
            AliasKind::Stringified,
            "for a chain ending at {end}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_of_a_refused_target_is_refused_under_its_own_name() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    for target in ["(String, String)", "Option<Slot>", "Vec<Slot>"] {
        register_alias_info(
            "RefusedKey",
            "RefusedKeyType",
            "refused_key_type_schema",
            super::alias_target_kind(&super::get_field_def(
                "RefusedKeyType",
                &syn::parse_str(target).unwrap(),
                "",
            )),
        );
        let error = field_map_key_error(&quote::quote! { HashMap<RefusedKey, u32> });
        assert!(
            error.contains("a map key must be a plain"),
            "for {target}, got: {error}"
        );
        assert!(error.contains("RefusedKey"), "for {target}, got: {error}");
        assert!(
            !error.contains("serde writes"),
            "for {target}, got: {error}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_chain_carries_the_string_wire_kind_to_its_end() {
    register_alias_info(
        "CorrelationId",
        "CorrelationId",
        "correlation_id_schema",
        AliasKind::StringWire,
    );
    for end in ["String", "CorrelationId"] {
        let first_target: syn::Type = syn::parse_str(end).unwrap();
        let first = super::alias_target_kind(&super::get_field_def("FirstType", &first_target, ""));
        assert_eq!(first, AliasKind::StringWire, "for a chain ending at {end}");
        register_alias_info("First", "FirstType", "first_type_schema", first);

        let second_target: syn::Type = syn::parse_str("First").unwrap();
        let second =
            super::alias_target_kind(&super::get_field_def("SecondType", &second_target, ""));
        assert_eq!(second, AliasKind::StringWire, "for a chain ending at {end}");
    }
}

/// [`super::branded_alias_kind`] read off the one field a brand is written with.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn brand_kind(inner: &str) -> AliasKind {
    let inner_ty: syn::Type = syn::parse_str(inner).unwrap();
    let item: syn::ItemStruct = syn::parse_quote! { struct Brand(#inner_ty); };
    super::branded_alias_kind(item.fields.iter().next().unwrap())
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_registers_what_serde_writes_for_its_inner() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    register_alias_info("Doc", "Doc", "doc_schema", AliasKind::NoEnumMembers);
    register_alias_info(
        "CorrelationId",
        "CorrelationId",
        "correlation_id_schema",
        AliasKind::StringWire,
    );
    register_alias_info("Tick", "Tick", "tick_schema", AliasKind::Stringified);
    for (inner, expected) in [
        ("String", AliasKind::StringWire),
        ("PathBuf", AliasKind::StringWire),
        ("CorrelationId", AliasKind::StringWire),
        ("Slot", AliasKind::StringWire),
        ("u32", AliasKind::Stringified),
        ("bool", AliasKind::Stringified),
        ("f64", AliasKind::Stringified),
        ("Tick", AliasKind::Stringified),
        ("Doc", AliasKind::NoEnumMembers),
        ("Vec<String>", AliasKind::NoEnumMembers),
        ("(String, String)", AliasKind::NoEnumMembers),
        ("HashMap<String, u32>", AliasKind::NoEnumMembers),
        ("Ghost", AliasKind::NoEnumMembers),
    ] {
        assert_eq!(brand_kind(inner), expected, "for brand inner {inner}");
    }
}

#[cfg(all(
    feature = "chrono",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_brand_over_a_chrono_inner_is_stringified() {
    for inner in ["NaiveDate", "NaiveTime", "NaiveDateTime", "DateTime<Utc>"] {
        assert_eq!(
            brand_kind(inner),
            AliasKind::Stringified,
            "for brand inner {inner}"
        );
    }
}

#[cfg(all(
    feature = "mongodb",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_brand_over_an_object_id_stays_refused() {
    assert_eq!(brand_kind("ObjectId"), AliasKind::NoEnumMembers);
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_stringified_key_is_left_alone_wherever_it_is_written() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    register_alias_info("Tick", "Tick", "tick_schema", AliasKind::Stringified);
    register_alias_info("Tags", "Tags", "tags_schema", AliasKind::NoEnumMembers);
    for field_type in [
        quote::quote! { HashMap<Tick, u32> },
        quote::quote! { HashMap<String, HashMap<Tick, u32>> },
        quote::quote! { HashMap<Slot, HashMap<Tick, u32>> },
        quote::quote! { HashMap<Tick, HashMap<Tick, u32>> },
        quote::quote! { Vec<HashMap<Tick, u32>> },
        quote::quote! { (String, HashMap<Tick, u32>) },
        quote::quote! { Wrapper<HashMap<Tick, u32>> },
    ] {
        let error = field_map_key_error(&field_type);
        assert!(error.is_empty(), "for {field_type}, got: {error}");
    }

    let refused = field_map_key_error(&quote::quote! { HashMap<Tags, u32> });
    assert!(
        refused.contains("a map key must be a plain"),
        "got: {refused}"
    );
    assert!(refused.contains("Tags"), "got: {refused}");
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_alias_chain_carries_the_enum_kind_to_its_end() {
    register_alias_info("Slot", "Slot", "slot_schema", AliasKind::EnumMembers);
    let first_target: syn::Type = syn::parse_str("Slot").unwrap();
    let first = super::alias_target_kind(&super::get_field_def("FirstType", &first_target, ""));
    register_alias_info("First", "FirstType", "first_type_schema", first);

    let second_target: syn::Type = syn::parse_str("First").unwrap();
    let second = super::alias_target_kind(&super::get_field_def("SecondType", &second_target, ""));
    assert_eq!(second, AliasKind::EnumMembers);
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_map_key_known_to_lack_enum_members_names_the_requirement() {
    register_alias_info(
        "KeyAlias",
        "KeyAliasType",
        "key_alias_type_schema",
        AliasKind::NoEnumMembers,
    );
    let tokens = map_field_schema("HashMap<KeyAlias, String>").to_string();
    assert!(
        !tokens.contains("KeyAlias :: enum_members"),
        "got: {tokens}"
    );
    assert!(
        tokens.starts_with(":: core :: compile_error !"),
        "got: {tokens}"
    );
    assert!(
        tokens.contains("a map key must be a plain"),
        "got: {tokens}"
    );
    assert!(tokens.contains("KeyAlias"), "got: {tokens}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_map_key_that_may_have_enum_members_expands_exactly_as_before() {
    let unregistered = map_field_schema("HashMap<Slot, String>").to_string();
    for kind in [AliasKind::EnumMembers, AliasKind::Unknown] {
        register_alias_info("Slot", "Slot", "slot_schema", kind);
        assert_eq!(
            map_field_schema("HashMap<Slot, String>").to_string(),
            unregistered
        );
    }
}

/// The `String`-key branch's output for a map value built by hand, for the value types no source
/// type produces: the `literal` override rewrites a *field's* type, never a map value's.
#[cfg(feature = "jsonschema")]
fn string_key_map_value_schema(field_type: FieldDefType) -> String {
    let ty: syn::Type = syn::parse_str("String").unwrap();
    let mut value = super::get_field_def("m", &ty, "");
    value.field_type = field_type;
    super::string_key_map_json_schema_value(&value)
        .unwrap()
        .to_string()
}

#[cfg(all(feature = "chrono", feature = "jsonschema"))]
#[test]
fn a_chrono_string_keyed_map_value_keeps_its_form() {
    for value_type in ["NaiveDate", "NaiveTime", "NaiveDateTime", "DateTime<Utc>"] {
        let map_type = format!("HashMap<String, {value_type}>");
        let tokens = map_field_schema(&map_type).to_string();
        assert!(
            tokens.contains(&format!(
                r#""additionalProperties" : {{ "type" : "string" , {} }}"#,
                chrono_text_held(value_type)
            )),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_string_literal_string_keyed_map_value_keeps_its_const() {
    let tokens = string_key_map_value_schema(FieldDefType::StringLiteral("Tixena".to_owned()));
    assert!(
        tokens.contains(r#""additionalProperties" : { "type" : "string" , "const" : "Tixena" }"#),
        "got: {tokens}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_opaque_string_keyed_map_value_stays_permissive() {
    let tokens = string_key_map_value_schema(FieldDefType::Unknown);
    assert!(
        tokens.contains(r#""additionalProperties" : { }"#),
        "got: {tokens}"
    );
}

/// A tuple as a map's value, under every key path, is the fixed array its own field position is.
#[cfg(feature = "jsonschema")]
#[test]
fn a_map_of_tuples_describes_each_value_as_its_array() {
    for map_type in [
        "HashMap<String, (String, u32)>",
        "HashMap<Slot, (String, u32)>",
        "HashMap<u32, (String, u32)>",
        "HashMap<String, Vec<(String, u32)>>",
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            !tokens.contains("compile_error"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            tokens.contains(r#""type" : "array" , "prefixItems""#),
            "for {map_type}, got: {tokens}"
        );
    }
}

/// The caret has to land on the tokens the author edits.
#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_map_value_points_at_the_written_value() {
    let tokens = sole_field_json_schema("pub struct Rows { pub m: HashMap<(u32, u32), u32> }");
    assert_points_only_at(&tokens, "(u32, u32)", "a map value");
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_tuple_element_map_value_points_at_the_written_value() {
    let tokens =
        sole_field_json_schema("pub struct Rows { pub t: (String, HashMap<(u32, u32), u32>) }");
    assert_points_only_at(&tokens, "(u32, u32)", "a tuple element map value");
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_scalar_string_keyed_map_value_expands_exactly_as_before() {
    for (map_type, expected) in [
        ("HashMap<String, String>", r#"{ "type" : "string" }"#),
        ("HashMap<String, u64>", r#"{ "type" : "integer" }"#),
        ("HashMap<String, i8>", r#"{ "type" : "integer" }"#),
        ("HashMap<String, f32>", r#"{ "type" : "number" }"#),
        ("HashMap<String, bool>", r#"{ "type" : "boolean" }"#),
        (
            "HashMap<String, Vec<String>>",
            r#"{ "type" : "array" , "items" : { "type" : "string" } }"#,
        ),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!(r#""additionalProperties" : {expected}"#)),
            "for {map_type}, got: {tokens}"
        );
    }
}

/// The `String`-key branch's output for a nested map value built by hand: a `String`-keyed inner
/// map holding `inner_type`, bare or behind a `Vec`, for the inner value types no source type
/// produces.
#[cfg(feature = "jsonschema")]
fn nested_string_key_map_value_schema(inner_type: FieldDefType, array_depth: u8) -> String {
    let ty: syn::Type = syn::parse_str("String").unwrap();
    let inner_key = super::get_field_def("m", &ty, "");
    let mut inner_value = inner_key.clone();
    inner_value.field_type = inner_type;
    let mut value = inner_key.clone();
    value.field_type = FieldDefType::Map(Box::new(inner_key), Box::new(inner_value));
    value.array_depth = array_depth;
    super::string_key_map_json_schema_value(&value)
        .unwrap()
        .to_string()
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_nested_string_keyed_map_value_renders_its_inner_members() {
    for (map_type, expected) in [
        (
            "HashMap<String, HashMap<String, String>>",
            r#"{ "type" : "object" , "additionalProperties" : { "type" : "string" } }"#,
        ),
        (
            "HashMap<String, HashMap<String, Vec<u64>>>",
            r#"{ "type" : "object" , "additionalProperties" : { "type" : "array" , "items" : { "type" : "integer" } } }"#,
        ),
        (
            "HashMap<String, HashMap<String, Option<bool>>>",
            r#"{ "type" : "object" , "additionalProperties" : { "anyOf" : [{ "type" : "boolean" } , { "type" : "null" }] } }"#,
        ),
        (
            "HashMap<String, HashMap<String, HashMap<String, f64>>>",
            r#"{ "type" : "object" , "additionalProperties" : { "type" : "object" , "additionalProperties" : { "type" : "number" } } }"#,
        ),
        (
            "HashMap<String, HashMap<String, Inner>>",
            r#"{ "type" : "object" , "additionalProperties" : inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs) }"#,
        ),
        (
            "HashMap<String, Option<HashMap<String, String>>>",
            r#"{ "anyOf" : [{ "type" : "object" , "additionalProperties" : { "type" : "string" } } , { "type" : "null" }] }"#,
        ),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!(r#""additionalProperties" : {expected}"#)),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains(r#""additionalProperties" : true"#),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_vec_of_maps_string_keyed_map_value_renders_its_inner_members() {
    for (map_type, expected) in [
        (
            "HashMap<String, Vec<HashMap<String, String>>>",
            r#"{ "type" : "array" , "items" : { "type" : "object" , "additionalProperties" : { "type" : "string" } } }"#,
        ),
        (
            "HashMap<String, Vec<HashMap<String, u64>>>",
            r#"{ "type" : "array" , "items" : { "type" : "object" , "additionalProperties" : { "type" : "integer" } } }"#,
        ),
        (
            "HashMap<String, Vec<HashMap<String, Inner>>>",
            r#"{ "type" : "array" , "items" : { "type" : "object" , "additionalProperties" : inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs) } }"#,
        ),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!(r#""additionalProperties" : {expected}"#)),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains(r#""additionalProperties" : true"#),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(all(feature = "chrono", feature = "jsonschema"))]
#[test]
fn a_nested_chrono_map_value_keeps_its_form() {
    for (map_type, expected) in [
        (
            "HashMap<String, HashMap<String, NaiveDate>>",
            r#"{ "type" : "object" , "additionalProperties" : { "type" : "string" , "format" : "date" } }"#.to_owned(),
        ),
        (
            "HashMap<String, HashMap<String, NaiveTime>>",
            format!(
                r#"{{ "type" : "object" , "additionalProperties" : {{ "type" : "string" , {} }} }}"#,
                chrono_text_held("NaiveTime")
            ),
        ),
        (
            "HashMap<String, Vec<HashMap<String, DateTime<Utc>>>>",
            r#"{ "type" : "array" , "items" : { "type" : "object" , "additionalProperties" : { "type" : "string" , "format" : "date-time" } } }"#.to_owned(),
        ),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!(r#""additionalProperties" : {expected}"#)),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains(r#""additionalProperties" : true"#),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_nested_string_literal_map_value_keeps_its_const() {
    let inner_member = r#"{ "type" : "object" , "additionalProperties" : { "type" : "string" , "const" : "Tixena" } }"#;
    let bare =
        nested_string_key_map_value_schema(FieldDefType::StringLiteral("Tixena".to_owned()), 0);
    assert!(
        bare.contains(&format!(r#""additionalProperties" : {inner_member}"#)),
        "got: {bare}"
    );

    let arrayed =
        nested_string_key_map_value_schema(FieldDefType::StringLiteral("Tixena".to_owned()), 1);
    assert!(
        arrayed.contains(&format!(
            r#""additionalProperties" : {{ "type" : "array" , "items" : {inner_member} }}"#
        )),
        "got: {arrayed}"
    );
}

/// A nested `ObjectId` member carries the `$oid` object the outer member carries.
#[cfg(all(feature = "mongodb", feature = "jsonschema"))]
#[test]
fn a_nested_object_id_map_value_keeps_its_oid_object() {
    let inner_member = r#"{ "type" : "object" , "additionalProperties" : { "type" : "object" , "properties" : { "$oid" : (serde_json :: json ! ({ "type" : "string" , "pattern" : "^[a-f0-9]{24}$" })) } , "required" : ["$oid"] , "additionalProperties" : false } }"#;
    let bare = nested_string_key_map_value_schema(FieldDefType::ObjectId, 0);
    assert!(
        bare.contains(&format!(r#""additionalProperties" : {inner_member}"#)),
        "got: {bare}"
    );

    let arrayed = nested_string_key_map_value_schema(FieldDefType::ObjectId, 1);
    assert!(
        arrayed.contains(&format!(
            r#""additionalProperties" : {{ "type" : "array" , "items" : {inner_member} }}"#
        )),
        "got: {arrayed}"
    );
}

/// The rendering an enum-keyed map carries in every position, spelled out for a key of the given
/// name and a member of the given tokens.
#[cfg(feature = "jsonschema")]
fn enum_key_map_rendering(key_type_name: &str, member: &str) -> String {
    format!(
        r#"serde_json :: json ! ({{ "type" : "object" , "properties" : ({{ let value_schema = {member} ; let mut map_properties = serde_json :: Map :: new () ; for enum_key in {key_type_name} :: enum_members () {{ map_properties . insert (enum_key . to_string () , value_schema . clone ()) ; }} map_properties }}) , "additionalProperties" : false }})"#
    )
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_nested_map_under_an_enumerating_key_expands_its_members() {
    let tokens = map_field_schema("HashMap<String, HashMap<Slot, String>>").to_string();
    let inner = enum_key_map_rendering("Slot", r#"serde_json :: json ! ({ "type" : "string" })"#);
    assert!(
        tokens.contains(&format!(r#""additionalProperties" : {inner}"#)),
        "got: {tokens}"
    );
    assert!(
        !tokens.contains(r#""additionalProperties" : true"#),
        "got: {tokens}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_nested_map_under_two_enumerating_keys_expands_both_member_sets() {
    let tokens = map_field_schema("HashMap<Slot, HashMap<Bucket, u64>>").to_string();
    let inner =
        enum_key_map_rendering("Bucket", r#"serde_json :: json ! ({ "type" : "integer" })"#);
    assert!(
        tokens.contains(&enum_key_map_rendering("Slot", &inner)),
        "got: {tokens}"
    );
    assert!(
        !tokens.contains(r#""additionalProperties" : true"#),
        "got: {tokens}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_wrapped_nested_enum_keyed_map_keeps_its_members_inside_the_slot_wrap() {
    let inner = enum_key_map_rendering("Slot", r#"serde_json :: json ! ({ "type" : "string" })"#);
    for (map_type, expected) in [
        (
            "HashMap<String, Vec<HashMap<Slot, String>>>",
            format!(r#"serde_json :: json ! ({{ "type" : "array" , "items" : {inner} }})"#),
        ),
        (
            "HashMap<String, Option<HashMap<Slot, String>>>",
            format!(r#"serde_json :: json ! ({{ "anyOf" : [{inner} , {{ "type" : "null" }}] }})"#),
        ),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!(r#""additionalProperties" : {expected}"#)),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_enum_keyed_map_tuple_element_expands_its_members() {
    let expected =
        enum_key_map_rendering("Slot", r#"serde_json :: json ! ({ "type" : "integer" })"#);
    for field_type in [
        "(String, HashMap<Slot, u32>)",
        "(String, HashMap<String, HashMap<Slot, u32>>)",
    ] {
        let tokens = tuple_field_schema(field_type);
        assert!(
            tokens.contains(&expected),
            "for {field_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains(r#""additionalProperties" : true"#),
            "for {field_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_nested_map_key_known_to_lack_enum_members_names_the_requirement() {
    register_alias_info(
        "KeyAlias",
        "KeyAliasType",
        "key_alias_type_schema",
        AliasKind::NoEnumMembers,
    );
    for map_type in [
        "HashMap<String, HashMap<KeyAlias, String>>",
        "HashMap<Slot, HashMap<KeyAlias, String>>",
        "HashMap<String, Vec<HashMap<KeyAlias, String>>>",
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            !tokens.contains("KeyAlias :: enum_members"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            tokens.starts_with(":: core :: compile_error !"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains("properties . insert"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            tokens.contains("a map key must be a plain"),
            "for {map_type}, got: {tokens}"
        );
        assert!(tokens.contains("KeyAlias"), "for {map_type}, got: {tokens}");
    }
}

/// The guard reaches a tuple slot too, that being another position the map is dispatched through.
#[cfg(feature = "jsonschema")]
#[test]
fn a_tuple_element_map_key_known_to_lack_enum_members_names_the_requirement() {
    register_alias_info(
        "KeyAlias",
        "KeyAliasType",
        "key_alias_type_schema",
        AliasKind::NoEnumMembers,
    );
    let tokens = tuple_field_schema("(String, HashMap<KeyAlias, String>)");
    assert!(
        tokens.starts_with(":: core :: compile_error !"),
        "got: {tokens}"
    );
    assert!(
        tokens.contains("a map key must be a plain"),
        "got: {tokens}"
    );
    assert!(tokens.contains("field `t`"), "got: {tokens}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_enum_keyed_map_renders_the_same_in_every_position() {
    let expected =
        enum_key_map_rendering("Slot", r#"serde_json :: json ! ({ "type" : "string" })"#);
    assert!(
        map_field_schema("HashMap<Slot, String>")
            .to_string()
            .contains(&expected),
        "field position lost its enumeration"
    );
    for position in [
        map_field_schema("HashMap<String, HashMap<Slot, String>>").to_string(),
        map_field_schema("HashMap<Slot, HashMap<Slot, String>>").to_string(),
        tuple_field_schema("(String, HashMap<Slot, String>)"),
    ] {
        assert!(position.contains(&expected), "got: {position}");
    }
}

/// A map under a key read from its text describes its values, nested in another map's value too,
/// and names its keys by that text.
#[cfg(feature = "jsonschema")]
#[test]
fn a_nested_map_under_a_key_read_from_its_text_describes_its_values() {
    for (map_type, names) in [
        (
            "HashMap<String, HashMap<u32, String>>",
            INTEGER_KEY_NAMES.to_owned(),
        ),
        (
            "HashMap<String, HashMap<Wrapper<Slot>, String>>",
            "({ let named_key : serde_json :: Value = ({ let arguments".to_owned(),
        ),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        let described = format!(
            r#""additionalProperties" : serde_json :: json ! ({{ "type" : "object" , "additionalProperties" : {{ "type" : "string" }} , "propertyNames" : ({names}"#
        );
        assert!(tokens.contains(&described), "for {map_type}, got: {tokens}");
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_unsupported_nested_map_value_emits_only_the_compile_error() {
    for map_type in [
        "HashMap<String, HashMap<(String, u32), u32>>",
        "HashMap<String, Vec<HashMap<(String, u32), u32>>>",
        "HashMap<u32, HashMap<(String, u32), u32>>",
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.starts_with(":: core :: compile_error !"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains("properties . insert"),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            tokens.contains(&format!("model_schema: field `m`: {TUPLE_KEY_REFUSAL}")),
            "for {map_type}, got: {tokens}"
        );
    }
}

/// The value half of a map type, parsed from the map type's source.
#[cfg(feature = "jsonschema")]
fn map_value_def(map_type: &str) -> super::FieldDef {
    let ty: syn::Type = syn::parse_str(map_type).unwrap();
    let field = super::get_field_def("m", &ty, "");
    let value = if let FieldDefType::Map(_, value) = &field.field_type {
        Some(value.as_ref().clone())
    } else {
        None
    };
    value.unwrap()
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_vec_sibling_map_value_parses_as_the_sibling_at_the_depth_it_is_written() {
    for (map_type, array_depth) in [
        ("HashMap<String, Inner>", 0_u8),
        ("HashMap<String, Vec<Inner>>", 1),
        ("HashMap<String, Vec<Vec<Inner>>>", 2),
    ] {
        let value = map_value_def(map_type);
        assert_eq!(value.array_depth, array_depth, "for {map_type}");
        assert!(
            matches!(&value.field_type, FieldDefType::SiblingType(name, args) if name == "Inner" && args.is_empty()),
            "for {map_type}, got: {:?}",
            value.field_type
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_sibling_string_keyed_map_value_emits_the_sibling_schema() {
    for (map_type, expected) in [
        (
            "HashMap<String, Inner>",
            "inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs)",
        ),
        (
            "HashMap<String, Vec<Inner>>",
            r#"serde_json :: json ! ({ "type" : "array" , "items" : inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs) })"#,
        ),
        (
            "HashMap<String, Option<Inner>>",
            r#"serde_json :: json ! ({ "anyOf" : [inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs) , { "type" : "null" }] })"#,
        ),
        (
            "HashMap<String, Option<Vec<Inner>>>",
            r#"serde_json :: json ! ({ "anyOf" : [serde_json :: json ! ({ "type" : "array" , "items" : inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs) }) , { "type" : "null" }] })"#,
        ),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!(r#""additionalProperties" : {expected}"#)),
            "for {map_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains(r#""additionalProperties" : true"#),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_generic_sibling_string_keyed_map_value_emits_the_sibling_schema() {
    let tokens = map_field_schema("HashMap<String, Wrapper<String>>").to_string();
    assert!(
        tokens.contains(
            "let arguments = [serde_json :: json ! ({ \"type\" : \"string\" })] ; wrapper_schema \
             :: Schema :: json_schema_within_with (in_flight , hoisted_defs , & arguments)"
        ),
        "got: {tokens}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_reference_site_argument_the_dispatch_cannot_render_is_refused() {
    let argument = get_field_def(
        "",
        &syn::parse_str("HashMap<String, HashMap<(u32, u32), u32>>").unwrap(),
        "",
    );
    let rendered = super::argument_json_schema_value("IdType", &argument).to_string();
    assert!(
        rendered.starts_with(":: core :: compile_error !"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains(&format!("model_schema: `IdType`: {TUPLE_KEY_REFUSAL}")),
        "got: {rendered}"
    );
}

/// The arguments a reference site writes, read off the sole field of `source` so each one carries
/// the span of the tokens it was written with.
#[cfg(feature = "jsonschema")]
fn sole_field_arguments(source: &str) -> Vec<super::FieldDef> {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let field = item.fields.iter().next().unwrap();
    let FieldDefType::SiblingType(_, arguments) = get_field_def("", &field.ty, "").field_type
    else {
        return Vec::new();
    };
    arguments
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_reference_site_argument_points_at_the_written_argument() {
    let arguments =
        sole_field_arguments("pub struct Holder { pub boxed: Boxed<HashMap<(u32, u32), u32>> }");
    assert_points_only_at(
        &super::argument_json_schema_value("Boxed", &arguments[0]),
        "(u32, u32)",
        "a reference-site argument",
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_declared_filling_points_at_the_written_filling() {
    let parameter: syn::Ident = syn::parse_str("IdType").unwrap();
    let filling: syn::Type = syn::parse_str("HashMap<(u32, u32), u32>").unwrap();
    assert_points_only_at(
        &super::declared_filling_json_schema_value("IdType", &[(parameter, filling)]),
        "(u32, u32)",
        "a declared filling",
    );
}

#[test]
fn a_map_never_parses_as_a_sibling_named_after_its_container() {
    for spelling in [
        "HashMap<String, u32>",
        "BTreeMap<String, u32>",
        "std::collections::BTreeMap<String, u32>",
        "Option<HashMap<String, u32>>",
        "Box<HashMap<String, u32>>",
        "Vec<BTreeMap<String, u32>>",
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let field_type = get_field_def("m", &ty, "").field_type;
        assert!(
            matches!(field_type, FieldDefType::Map(..)),
            "for {spelling}, got: {field_type:?}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_aliased_string_keyed_map_value_resolves_its_module_through_the_registry() {
    register_alias_info(
        "Inner",
        "InnerType",
        "inner_type_schema",
        AliasKind::NoEnumMembers,
    );
    for map_type in ["HashMap<String, Inner>", "HashMap<String, Vec<Inner>>"] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(
                "inner_type_schema :: Schema :: json_schema_within (in_flight , hoisted_defs)"
            ),
            "for {map_type}, got: {tokens}"
        );
    }
}

/// A value spelled as a wrapper around `u32`, built rather than parsed: the parser collapses a
/// `Vec` onto its element before any wrapper name is read, so the `Vec` spelling of a wrapper name
/// can only be put in front of a dispatch this way.
#[cfg(feature = "jsonschema")]
fn wrapped_u32_value(wrapper: &str) -> super::FieldDef {
    let element = super::get_field_def("", &syn::parse_quote!(u32), "");
    super::FieldDef {
        array_lengths: Vec::new(),
        docs: String::new(),
        field_type: FieldDefType::SiblingType(wrapper.to_owned(), vec![element]),
        array_depth: 0,
        model_schema_prop_meta: None,
        nullable_levels: Vec::new(),
        name: "items".to_owned(),
        absent_from_wire: false,
        omits_value: false,
        #[cfg(any(feature = "zod", feature = "jsonschema"))]
        serde_default: None,
        bytes: false,
        set_levels: Vec::new(),
        type_span: proc_macro2::Span::call_site(),
    }
}

/// The `Vec<u32>` value every wrapper spelling of the same thing is held against, its one level
/// written as a set where `wrapper` is one.
#[cfg(feature = "jsonschema")]
fn parsed_u32_vec_value_as(wrapper: &str) -> super::FieldDef {
    let mut parsed = super::get_field_def("items", &syn::parse_quote!(Vec<u32>), "");
    if is_set_wrapper(wrapper) {
        parsed.set_levels = vec![0];
    }
    parsed
}

#[cfg(feature = "jsonschema")]
#[test]
fn every_sequence_wrapper_describes_as_the_vec_of_its_element() {
    for wrapper in SEQUENCE_WRAPPERS {
        let expected =
            super::build_field_type_schema(&parsed_u32_vec_value_as(wrapper), "items").to_string();
        assert_eq!(
            super::build_field_type_schema(&wrapped_u32_value(wrapper), "items").to_string(),
            expected,
            "for: {wrapper}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn every_sequence_wrapper_describes_as_the_vec_of_its_element_in_a_slot() {
    for wrapper in SEQUENCE_WRAPPERS {
        let parsed = parsed_u32_vec_value_as(wrapper);
        let value = wrapped_u32_value(wrapper);
        assert_eq!(
            super::build_map_member_schema(&value).unwrap().to_string(),
            super::build_map_member_schema(&parsed).unwrap().to_string(),
            "for: {wrapper}"
        );
        assert_eq!(
            super::build_tuple_element_json_schema(&value)
                .unwrap()
                .to_string(),
            super::build_tuple_element_json_schema(&parsed)
                .unwrap()
                .to_string(),
            "for: {wrapper}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_set_writes_unique_items_at_its_own_level_alone() {
    let distinct = r#""uniqueItems" : true"#;
    for wrapper in SEQUENCE_WRAPPERS {
        let written =
            super::build_field_type_schema(&wrapped_u32_value(wrapper), "items").to_string();
        assert_eq!(
            written.matches(distinct).count(),
            usize::from(is_set_wrapper(wrapper)),
            "for: {wrapper}, got: {written}"
        );
    }
}

#[cfg(all(feature = "jsonschema", feature = "serde"))]
#[test]
fn every_sequence_wrapper_describes_as_the_vec_of_its_element_in_an_untagged_member() {
    for wrapper in SEQUENCE_WRAPPERS {
        assert_eq!(
            super::field_json_schema_value(&wrapped_u32_value(wrapper)).to_string(),
            super::field_json_schema_value(&parsed_u32_vec_value_as(wrapper)).to_string(),
            "for: {wrapper}"
        );
    }
}

/// One value per arm of the untagged-member dispatch, labelled as it was written. Built at no array
/// level, so each holds exactly the tokens its arm emits before the array wrap sees them.
#[cfg(all(feature = "jsonschema", feature = "serde"))]
fn untagged_member_dispatch_values() -> Vec<(&'static str, super::FieldDef)> {
    let parsed: [(&'static str, syn::Type); 5] = [
        ("MetricTag", syn::parse_quote!(MetricTag)),
        ("String", syn::parse_quote!(String)),
        ("u32", syn::parse_quote!(u32)),
        ("f64", syn::parse_quote!(f64)),
        ("bool", syn::parse_quote!(bool)),
    ];
    let mut values: Vec<(&'static str, super::FieldDef)> = parsed
        .iter()
        .map(|(label, ty)| (*label, super::get_field_def("items", ty, "")))
        .collect();

    let mut bounded = super::get_field_def("items", &syn::parse_quote!(String), "");
    bounded.model_schema_prop_meta = Some(ModelSchemaPropMeta {
        min_length: Some(2),
        pattern: Some("^[a-z]+$".to_owned()),
        ..Default::default()
    });
    values.push(("String under a bound", bounded));

    let mut literal = super::get_field_def("items", &syn::parse_quote!(String), "");
    literal.field_type = FieldDefType::StringLiteral("north".to_owned());
    values.push(("a string literal", literal));

    values.push((
        "HashMap<String, u32>",
        super::get_field_def("items", &syn::parse_quote!(HashMap<String, u32>), ""),
    ));
    values.push((
        "(i64, String)",
        super::get_field_def("items", &syn::parse_quote!((i64, String)), ""),
    ));
    #[cfg(feature = "mongodb")]
    values.push((
        "ObjectId",
        super::get_field_def("items", &syn::parse_quote!(ObjectId), ""),
    ));
    #[cfg(feature = "chrono")]
    values.push((
        "NaiveDate",
        super::get_field_def("items", &syn::parse_quote!(NaiveDate), ""),
    ));

    values
}

/// Every arm of the untagged-member dispatch hands the array wrap a value the wrap can carry.
#[cfg(all(feature = "jsonschema", feature = "serde"))]
#[test]
fn every_untagged_member_value_is_one_the_array_wrap_can_carry() {
    for (label, value) in untagged_member_dispatch_values() {
        let tokens = super::field_json_schema_value(&value);
        let opens_a_block = matches!(
            tokens.clone().into_iter().next(),
            Some(proc_macro2::TokenTree::Group(group))
                if group.delimiter() == proc_macro2::Delimiter::Brace
        );
        assert!(!opens_a_block, "for: {label}, got: {tokens}");
    }
}

#[cfg(all(feature = "jsonschema", feature = "serde"))]
#[test]
fn the_array_wrap_carries_each_untagged_member_arms_own_tokens() {
    for (label, value) in untagged_member_dispatch_values() {
        let item = super::field_json_schema_value(&value).to_string();
        let mut arrayed = value;
        arrayed.array_depth = 1;
        assert_eq!(
            super::field_json_schema_value(&arrayed).to_string(),
            format!("serde_json :: json ! ({{ \"type\" : \"array\" , \"items\" : {item} }})"),
            "for: {label}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_sibling_slot_carries_the_schema_module_reference() {
    let spellings: [syn::Type; 4] = [
        syn::parse_quote!(MetricTag),
        syn::parse_quote!(Vec<MetricTag>),
        syn::parse_quote!(HashSet<MetricTag>),
        syn::parse_quote!(Option<BTreeSet<MetricTag>>),
    ];
    for value in &spellings {
        let parsed = super::get_field_def("tag", value, "");
        let element = super::build_tuple_element_json_schema(&parsed)
            .unwrap()
            .to_string();
        assert!(element.contains("metric_tag_schema"), "Got: {element}");
        assert_eq!(
            element,
            super::build_map_member_schema(&parsed).unwrap().to_string()
        );
    }
}

/// The `json_schema()` body a brand over `inner_ty` carries, read off the same dispatch the
/// branded expansion runs.
#[cfg(feature = "jsonschema")]
fn brand_json_schema_over(inner_ty: &syn::Type) -> String {
    super::build_branded_json_schema_method(
        &super::ModelSchemaArgs::default(),
        &super::branded_json_inner(&super::get_field_def("_inner", inner_ty, "")),
        "Wrapped",
        &[],
    )
    .to_string()
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_named_type_resolves_to_the_same_module_in_field_and_brand_position() {
    register_alias_info(
        "Renamed",
        "RenamedType",
        "renamed_type_schema",
        AliasKind::NoEnumMembers,
    );
    for (name, module) in [
        ("Foreign", "foreign_schema"),
        ("Renamed", "renamed_type_schema"),
    ] {
        let inner_ty: syn::Type = syn::parse_str(name).unwrap();
        let reference =
            format!("{module} :: Schema :: json_schema_within (in_flight , hoisted_defs)");

        let field =
            super::build_field_type_schema(&super::get_field_def("id", &inner_ty, ""), "id")
                .to_string();
        assert!(field.contains(&reference), "for {name}, got: {field}");

        let brand = brand_json_schema_over(&inner_ty);
        assert!(brand.contains(&reference), "for {name}, got: {brand}");
    }
}

/// The JSON-schema insertion for the sole field of `source`, parsed from text so its spans carry
/// file locations and `source_text()` can report what they point at.
#[cfg(feature = "jsonschema")]
fn sole_field_json_schema(source: &str) -> proc_macro2::TokenStream {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let field = item.fields.iter().next().unwrap();
    let field_name = field.ident.as_ref().unwrap().to_string();
    let def = get_field_def(&field_name, &field.ty, "");
    super::build_field_type_schema(&def, &field_name)
}

/// The source text each occurrence of the ident `name` points at, `None` for an occurrence carrying
/// no location.
#[cfg(feature = "jsonschema")]
fn ident_source_texts(tokens: &proc_macro2::TokenStream, name: &str) -> Vec<Option<String>> {
    let mut found = Vec::new();
    for tree in tokens.clone() {
        match &tree {
            proc_macro2::TokenTree::Group(group) => {
                found.extend(ident_source_texts(&group.stream(), name));
            }
            proc_macro2::TokenTree::Ident(ident) if ident == name => {
                found.push(tree.span().source_text());
            }
            proc_macro2::TokenTree::Ident(_)
            | proc_macro2::TokenTree::Punct(_)
            | proc_macro2::TokenTree::Literal(_) => {}
        }
    }
    found
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_sibling_reference_points_at_the_type_the_field_names() {
    for source in [
        "struct Outer { inner: Inner }",
        "struct Outer { inner: Vec<Inner> }",
        "struct Outer { inner: Box<Inner> }",
        "struct Outer { inner: HashMap<String, Inner> }",
        "struct Outer { inner: (Inner, u32) }",
    ] {
        let tokens = sole_field_json_schema(source);
        for named in ["inner_schema", "Schema", "json_schema_within"] {
            assert_eq!(
                ident_source_texts(&tokens, named),
                vec![Some("Inner".to_owned())],
                "for {source}, at `{named}`, got: {tokens}"
            );
        }
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_sibling_reference_emits_the_tokens_it_always_has() {
    assert_eq!(
        sole_field_json_schema("struct Outer { inner: Inner }").to_string(),
        "properties . insert (\"inner\" . to_string () , inner_schema :: Schema :: json_schema_within (in_flight , hoisted_defs)) ;"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_enum_keyed_map_points_its_members_call_at_the_key_the_field_names() {
    for source in [
        "struct Outer { m: HashMap<Slot, String> }",
        "struct Outer { m: Vec<HashMap<Slot, String>> }",
        "struct Outer { m: HashMap<String, HashMap<Slot, String>> }",
        "struct Outer { m: (HashMap<Slot, String>, u32) }",
    ] {
        let tokens = sole_field_json_schema(source);
        for named in ["Slot", "enum_members"] {
            assert_eq!(
                ident_source_texts(&tokens, named),
                vec![Some("Slot".to_owned())],
                "for {source}, at `{named}`, got: {tokens}"
            );
        }
    }
}

/// The tuple-field insertion for a field type built by hand.
#[cfg(feature = "jsonschema")]
fn tuple_field_schema(field_type: &str) -> String {
    let ty: syn::Type = syn::parse_str(field_type).unwrap();
    super::build_field_type_schema(&super::get_field_def("t", &ty, ""), "t").to_string()
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_tuple_element_holding_an_unrenderable_map_emits_only_the_compile_error() {
    for field_type in [
        "(String, HashMap<(u32, u32), u32>)",
        "(String, Vec<HashMap<(u32, u32), u32>>)",
        "(String, (u32, HashMap<(u32, u32), u32>))",
    ] {
        let tokens = tuple_field_schema(field_type);
        assert!(
            tokens.starts_with(":: core :: compile_error !"),
            "for {field_type}, got: {tokens}"
        );
        assert!(
            !tokens.contains("properties . insert"),
            "for {field_type}, got: {tokens}"
        );
        assert!(
            tokens.contains(&format!("model_schema: field `t`: {TUPLE_KEY_REFUSAL}")),
            "for {field_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_sequence_wrapped_map_field_describes_as_the_array_of_the_map_it_holds() {
    for (map_type, wrapped_type) in [
        ("HashMap<String, u64>", "Vec<HashMap<String, u64>>"),
        ("HashMap<String, u64>", "VecDeque<HashMap<String, u64>>"),
        ("HashMap<Slot, u64>", "Vec<HashMap<Slot, u64>>"),
        (
            "HashMap<Slot, Vec<u64>>",
            "BTreeSet<HashMap<Slot, Vec<u64>>>",
        ),
        ("HashMap<u32, u64>", "Vec<HashMap<u32, u64>>"),
        (
            "HashMap<String, HashMap<Slot, u64>>",
            "Vec<HashMap<String, HashMap<Slot, u64>>>",
        ),
    ] {
        let item = inserted_field_value(map_type);
        let distinct = if wrapped_type.starts_with("BTreeSet") {
            r#" , "uniqueItems" : true"#
        } else {
            ""
        };
        assert_eq!(
            inserted_field_value(wrapped_type),
            format!(
                r#"serde_json :: json ! ({{ "type" : "array" , "items" : {item}{distinct} }})"#
            ),
            "for {wrapped_type}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_optional_sequence_wrapped_map_field_widens_the_array_with_null() {
    let array = format!(
        r#"serde_json :: json ! ({{ "type" : "array" , "items" : {} }})"#,
        inserted_field_value("HashMap<String, u64>")
    );
    assert_eq!(
        inserted_field_value("Option<Vec<HashMap<String, u64>>>"),
        format!(r#"serde_json :: json ! ({{ "anyOf" : [{array} , {{ "type" : "null" }}] }})"#)
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_sequence_of_optional_maps_admits_the_null_among_its_items() {
    let item = inserted_field_value("HashMap<String, u64>");
    assert_eq!(
        inserted_field_value("Vec<Option<HashMap<String, u64>>>"),
        format!(
            r#"serde_json :: json ! ({{ "type" : "array" , "items" : serde_json :: json ! ({{ "anyOf" : [{item} , {{ "type" : "null" }}] }}) }})"#
        )
    );
}

/// The description of a map of `u64` values keyed by an integer.
#[cfg(feature = "jsonschema")]
fn integer_keyed() -> String {
    format!(
        r#"serde_json :: json ! ({{ "type" : "object" , "additionalProperties" : {{ "type" : "integer" }} , "propertyNames" : ({INTEGER_KEY_NAMES} }})"#
    )
}

/// A map named without a sequence wrapper describes its values on every key path.
#[cfg(feature = "jsonschema")]
#[test]
fn an_unwrapped_map_field_describes_its_values_on_every_key_path() {
    let string_keyed = r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "integer" } })"#;
    for (map_type, expected) in [
        ("HashMap<String, u64>", string_keyed.to_owned()),
        ("BTreeMap<String, u64>", string_keyed.to_owned()),
        ("HashMap<u32, u64>", integer_keyed()),
    ] {
        assert_eq!(inserted_field_value(map_type), expected, "for {map_type}");
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_optional_unwrapped_map_field_widens_the_object_with_null() {
    for (map_type, base) in [
        (
            "Option<HashMap<String, u64>>",
            r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "integer" } })"#.to_owned(),
        ),
        ("Option<HashMap<u32, u64>>", integer_keyed()),
    ] {
        assert_eq!(
            inserted_field_value(map_type),
            format!(r#"serde_json :: json ! ({{ "anyOf" : [{base} , {{ "type" : "null" }}] }})"#),
            "for {map_type}"
        );
    }
    assert_eq!(
        inserted_field_value("Option<HashMap<Slot, u64>>"),
        format!(
            r#"serde_json :: json ! ({{ "anyOf" : [{} , {{ "type" : "null" }}] }})"#,
            inserted_field_value("HashMap<Slot, u64>")
        )
    );
}

#[cfg(all(feature = "mongodb", feature = "jsonschema"))]
#[test]
fn an_object_id_tuple_element_spells_the_one_oid_object() {
    let parsed = super::get_field_def("id", &syn::parse_quote!(ObjectId), "");
    assert_eq!(
        super::build_tuple_element_json_schema(&parsed)
            .unwrap()
            .to_string(),
        r#"serde_json :: json ! ({ "type" : "object" , "properties" : { "$oid" : (serde_json :: json ! ({ "type" : "string" , "pattern" : "^[a-f0-9]{24}$" })) } , "required" : ["$oid"] , "additionalProperties" : false })"#
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_optional_sequence_wrapper_member_stays_nullable() {
    let mut optional_set = wrapped_u32_value("HashSet");
    optional_set.nullable_levels = vec![optional_set.array_depth];
    let mut optional_vec = parsed_u32_vec_value_as("HashSet");
    optional_vec.nullable_levels = vec![optional_vec.array_depth];
    assert_eq!(
        super::build_map_member_schema(&optional_set)
            .unwrap()
            .to_string(),
        super::build_map_member_schema(&optional_vec)
            .unwrap()
            .to_string()
    );
}

/// Runs a fixed discriminated enum through the real collect/render pipeline, returning its
/// TypeScript member fragments, Zod member fragments, and JSON-schema member fragments. Rebuilt
/// from scratch on every call so repeated calls exercise whatever ordering the collection stage
/// imposes, not a single cached traversal.
fn rendered_discriminated_union() -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Action {
            Upload { path: String },
            Generate,
            Delete(String),
            Rename { from: String, to: String },
            Move(String, String),
            Archive,
        }
    };
    let variants = collect_discriminated_variants(
        &mut item,
        UNCASED,
        Some("action_schema"),
        &super::ModelSchemaArgs::default(),
    );
    let rendered = render_discriminated_variants("type", "value", "Action", &variants.0);
    (
        rendered.0,
        rendered.1,
        rendered.2.iter().map(ToString::to_string).collect(),
    )
}

#[test]
fn discriminated_union_members_follow_declaration_order() {
    let (ts_members, zod_members, json_members) = rendered_discriminated_union();
    assert_eq!(ts_members.len(), DECLARED_VARIANTS.len());
    assert_eq!(zod_members.len(), DECLARED_VARIANTS.len());
    assert_eq!(json_members.len(), DECLARED_VARIANTS.len());

    for (position, declared) in DECLARED_VARIANTS.iter().enumerate() {
        assert!(
            ts_members[position].contains(&format!("type: \"{declared}\";")),
            "TypeScript member {position} is not `{declared}`: {}",
            ts_members[position]
        );
        assert!(
            zod_members[position].contains(&format!("type: z.literal(\"{declared}\")")),
            "Zod member {position} is not `{declared}`: {}",
            zod_members[position]
        );
        #[cfg(feature = "jsonschema")]
        assert!(
            json_members[position].contains(&format!("\"const\" : \"{declared}\"")),
            "JSON-schema member {position} is not `{declared}`: {}",
            json_members[position]
        );
    }
}

/// Every per-variant collection feeding emission must be order-preserving.
#[test]
fn discriminated_union_rendering_is_stable_across_runs() {
    const RUNS: usize = 32;

    let first = rendered_discriminated_union();
    for run in 1..RUNS {
        assert_eq!(
            rendered_discriminated_union(),
            first,
            "run {run} rendered a different union than run 0"
        );
    }
}

#[test]
fn adjacently_tagged_empty_named_variant_nests_an_empty_content_object() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Probe {
            Empty {},
            Unit,
        }
    };
    let variants = collect_discriminated_variants(
        &mut item,
        UNCASED,
        Some("probe_schema"),
        &super::ModelSchemaArgs::default(),
    );
    let rendered = render_discriminated_variants("kind", "data", "Probe", &variants.0);

    let ts = &rendered.0[0];
    assert!(ts.contains("data: {\n};"), "Got: {ts}");

    #[cfg(feature = "zod")]
    let zod = &rendered.1[0];
    #[cfg(feature = "zod")]
    assert!(zod.contains("data: z.strictObject({\n}),"), "Got: {zod}");

    #[cfg(feature = "jsonschema")]
    let json = rendered.2[0].to_string();
    #[cfg(feature = "jsonschema")]
    assert!(json.contains("\"data\""), "Got: {json}");
}

/// The `validate()` contribution for a field spelled `spelling`, reached from `access`, as tokens.
#[cfg(feature = "serde")]
fn emitted_validation_from(spelling: &str, access: MemberAccess) -> String {
    let ty: syn::Type = syn::parse_str(spelling).unwrap();
    let shape = constrained_shape(&ty).unwrap();
    let field = proc_macro2::Ident::new("field", proc_macro2::Span::call_site());
    let checker = proc_macro2::Ident::new("check", proc_macro2::Span::call_site());
    build_field_validation(&shape.wraps, &member_access_expr(access, &field), &checker).to_string()
}

/// The `validate()` contribution for a struct's field spelled `spelling`, as tokens.
#[cfg(feature = "serde")]
fn emitted_validation(spelling: &str) -> String {
    emitted_validation_from(spelling, MemberAccess::SelfField)
}

/// The two positions differ in one place and nowhere else: where the checked value is reached.
#[cfg(feature = "serde")]
#[test]
fn a_variant_member_is_reached_through_the_binding_its_arm_made() {
    for spelling in [
        "String",
        "u32",
        "Option<String>",
        "Vec<String>",
        "Option<Arc<[String]>>",
        "Box<Option<String>>",
    ] {
        assert_eq!(
            emitted_validation_from(spelling, MemberAccess::VariantBinding),
            emitted_validation(spelling).replace("& self . field", "member_field"),
            "spelling {spelling}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_bare_field_is_checked_in_place() {
    assert_eq!(
        emitted_validation("String"),
        "if let :: core :: result :: Result :: Err (reported) = check (& self . field) { errors . \
         extend (reported) ; }"
    );
    assert_eq!(emitted_validation("u32"), emitted_validation("String"));
}

/// A constraint describes the value on the wire, and a `None` puts none there.
#[cfg(feature = "serde")]
#[test]
fn an_option_is_checked_inside_its_some() {
    assert_eq!(
        emitted_validation("Option<String>"),
        "{ let value_0 = & self . field ; if let :: core :: option :: Option :: Some (value_1) = \
         value_0 { if let :: core :: result :: Result :: Err (reported) = check (value_1) { errors \
         . extend (reported) ; } } }"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_transparent_wrapper_is_dereferenced_through() {
    let expected = "{ let value_0 = & self . field ; let value_1 = & * * value_0 ; if let :: core \
                    :: result :: Result :: Err (reported) = check (value_1) { errors . extend \
                    (reported) ; } }";
    for spelling in ["Arc<str>", "Box<String>", "Cow<'a, str>", "Rc<str>"] {
        assert_eq!(
            emitted_validation(spelling),
            expected,
            "spelling {spelling}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_borrowed_str_is_dereferenced_through() {
    for spelling in ["&'a str", "&'static str", "&'a mut str"] {
        assert_eq!(
            emitted_validation(spelling),
            emitted_validation("Box<str>"),
            "spelling {spelling}"
        );
    }
    assert_eq!(
        emitted_validation("Option<&'a str>"),
        emitted_validation("Option<Box<str>>")
    );
    assert_eq!(
        emitted_validation("&'a [u8]"),
        emitted_validation("Box<[u8]>")
    );
}

/// A list is checked itself, in place, as a bare string is: nothing it holds is walked.
#[cfg(feature = "serde")]
#[test]
fn a_sequence_is_checked_itself_and_none_of_its_elements() {
    let expected = emitted_validation("String");
    for wrapper in SEQUENCE_WRAPPERS {
        assert_eq!(
            emitted_validation(&format!("{wrapper}<String>")),
            expected,
            "wrapper {wrapper}"
        );
    }
    assert_eq!(
        emitted_validation("[:: std :: string :: String ; 2]"),
        expected
    );
    assert_eq!(emitted_validation("Vec<Vec<String>>"), expected);
    assert_eq!(emitted_validation("Vec<Part>"), expected);
    assert_eq!(
        emitted_validation("Option<Vec<u8>>"),
        emitted_validation("Option<String>")
    );
}

/// The wrappers compose in the order they were written, each reaching exactly one level, and
/// the walk stops at the list they hold.
#[cfg(feature = "serde")]
#[test]
fn mixed_wrappers_compose_in_written_order() {
    assert_eq!(
        emitted_validation("Option<Arc<[String]>>"),
        "{ let value_0 = & self . field ; if let :: core :: option :: Option :: Some (value_1) = \
         value_0 { let value_2 = & * * value_1 ; if let :: core :: result :: Result :: Err \
         (reported) = check (value_2) { errors . extend (reported) ; } } }"
    );
}

/// A struct's field named `field`, as its helpers name and reach it.
#[cfg(feature = "serde")]
fn checked_struct_field() -> CheckedMember {
    let item: syn::ItemStruct = syn::parse_quote! { struct Probe { field: String } };
    let field = item.fields.iter().next().unwrap();
    checked_member(field, 0, None, &["field".to_owned()])
}

/// The schema-module items emitted for a `minLength` field spelled `spelling`, as tokens.
#[cfg(feature = "serde")]
fn emitted_string_module(spelling: &str) -> String {
    let ty: syn::Type = syn::parse_str(spelling).unwrap();
    let shape = constrained_shape(&ty).unwrap();
    let meta = ModelSchemaPropMeta {
        min_length: Some(3),
        ..ModelSchemaPropMeta::default()
    };
    generate_string_validation_code(&checked_struct_field(), &meta, &shape, &ty)
        .module_items
        .to_string()
}

/// The validator emitted for a `String` field constrained by `pattern`, as tokens — everything the
/// schema module holds ahead of the deserializer.
#[cfg(feature = "serde")]
fn emitted_pattern_validator(pattern: &str) -> String {
    let ty: syn::Type = syn::parse_str("String").unwrap();
    let meta = ModelSchemaPropMeta {
        pattern: Some(pattern.to_owned()),
        ..ModelSchemaPropMeta::default()
    };
    let module = generate_string_validation_code(
        &checked_struct_field(),
        &meta,
        &constrained_shape(&ty).unwrap(),
        &ty,
    )
    .module_items
    .to_string();
    module[..module.find("pub fn deserialize_field").unwrap()].to_owned()
}

/// The schema-module items emitted for a list field spelled `spelling` under `minLength = 1,
/// maxLength = 3`, as tokens.
#[cfg(feature = "serde")]
fn emitted_list_module(spelling: &str) -> String {
    let ty: syn::Type = syn::parse_str(spelling).unwrap();
    let shape = constrained_shape(&ty).unwrap();
    let meta = ModelSchemaPropMeta {
        max_length: Some(3),
        min_length: Some(1),
        ..ModelSchemaPropMeta::default()
    };
    generate_list_validation_code(&checked_struct_field(), &meta, &shape.wraps)
        .module_items
        .to_string()
}

/// A list's validator and hook name no type the list holds, so one text serves every list.
#[cfg(feature = "serde")]
#[test]
fn a_list_is_counted_by_a_validator_and_a_hook_written_over_any_list() {
    let module = emitted_list_module("Vec<Part>");
    assert_eq!(
        module,
        "pub fn validate_field_value < L > (list : & L) -> :: core :: result :: Result < () , :: \
         std :: vec :: Vec < :: std :: string :: String >> where L : ? :: core :: marker :: Sized \
         , for < 'held > & 'held L : :: core :: iter :: IntoIterator , { let held = list . \
         into_iter () . count () ; let mut errors : :: std :: vec :: Vec < :: std :: string :: \
         String > = :: std :: vec :: Vec :: new () ; if held < 1usize { errors . push (format ! \
         (\"'{}': {}, got {}\" , \"field\" , \"too short: minimum length is 1\" , held)) ; } if \
         held > 3usize { errors . push (format ! (\"'{}': {}, got {}\" , \"field\" , \"too long: \
         maximum length is 3\" , held)) ; } if errors . is_empty () { :: core :: result :: Result \
         :: Ok (()) } else { :: core :: result :: Result :: Err (errors) } } pub fn \
         deserialize_field < 'de , D , L > (deserializer : D) -> :: core :: result :: Result < L \
         , D :: Error > where D : serde :: Deserializer < 'de > , L : serde :: Deserialize < 'de \
         > , for < 'held > & 'held L : :: core :: iter :: IntoIterator , { let read = < L as \
         serde :: Deserialize > :: deserialize (deserializer) ? ; let check = | value_0 : & L | \
         -> :: core :: result :: Result < () , :: std :: vec :: Vec < :: std :: string :: String \
         >> { validate_field_value (value_0) ? ; :: core :: result :: Result :: Ok (()) } ; check \
         (& read) . map_err (| violations : :: std :: vec :: Vec < :: std :: string :: String > | \
         serde :: de :: Error :: custom (violations . join (\"; \"))) ? ; :: core :: result :: \
         Result :: Ok (read) }"
    );
    for spelling in [
        "Vec<T>",
        "Vec<Self>",
        "Vec<&'a str>",
        "HashSet<String>",
        "BTreeSet<u8>",
        "BinaryHeap<u8>",
        "VecDeque<Part>",
        "[String; 2]",
        "Vec<Vec<String>>",
    ] {
        assert_eq!(emitted_list_module(spelling), module, "for {spelling}");
    }
}

/// A wrapped list's hook names each transparent wrapper by what it derefs to.
#[cfg(feature = "serde")]
#[test]
fn a_wrapped_lists_hook_reads_through_its_wrappers() {
    let module = emitted_list_module("Option<Arc<[String]>>");
    assert_eq!(emitted_list_module("Option<Box<[T]>>"), module);
    for written in [
        "pub fn deserialize_field < 'de , D , H1 , L > (deserializer : D) -> :: core :: result :: \
         Result < :: core :: option :: Option < H1 > , D :: Error > where D : serde :: \
         Deserializer < 'de > , :: core :: option :: Option < H1 > : serde :: Deserialize < 'de > \
         , H1 : :: core :: ops :: Deref < Target = L > , L : ? :: core :: marker :: Sized , for < \
         'held > & 'held L : :: core :: iter :: IntoIterator , {",
        "if let :: core :: option :: Option :: Some (value_1) = value_0 { let value_2 = & * * \
         value_1 ; validate_field_value (value_2) ? ; }",
    ] {
        assert!(module.contains(written), "{written} is not in: {module}");
    }

    let nested = emitted_list_module("Box<Option<Rc<Vec<u8>>>>");
    for written in [
        "pub fn deserialize_field < 'de , D , H2 , H0 , L > (deserializer : D) -> :: core :: \
         result :: Result < H0 , D :: Error > where D : serde :: Deserializer < 'de > , H0 : \
         serde :: Deserialize < 'de > , H2 : :: core :: ops :: Deref < Target = L > , H0 : :: \
         core :: ops :: Deref < Target = :: core :: option :: Option < H2 > > , L : ? :: core :: \
         marker :: Sized , for < 'held > & 'held L : :: core :: iter :: IntoIterator , {",
        "let value_1 = & * * value_0 ; if let :: core :: option :: Option :: Some (value_2) = \
         value_1 { let value_3 = & * * value_2 ; validate_field_value (value_3) ? ; }",
    ] {
        assert!(nested.contains(written), "{written} is not in: {nested}");
    }
}

/// A borrowed list is counted by `validate()` alone.
#[cfg(feature = "serde")]
#[test]
fn a_borrowed_list_publishes_its_validator_and_no_hook() {
    for spelling in ["&'a [String]", "&'static Vec<u8>", "Option<&'a [Part]>"] {
        let module = emitted_list_module(spelling);
        assert!(
            module.starts_with("pub fn validate_field_value < L > (list : & L)"),
            "for {spelling}: {module}"
        );
        assert!(
            !module.contains("deserialize_field"),
            "a hook was emitted for {spelling}: {module}"
        );
    }
}

/// Just the deserializer of [`emitted_string_module`], which is everything after the validator.
#[cfg(feature = "serde")]
fn emitted_string_deserializer(spelling: &str) -> String {
    let module = emitted_string_module(spelling);
    assert!(
        module.contains("pub fn deserialize_field"),
        "no deserializer emitted for {spelling}: {module}"
    );
    module[module.find("pub fn deserialize_field").unwrap()..].to_owned()
}

#[cfg(feature = "serde")]
#[test]
fn a_bare_field_deserializes_the_constrained_value_itself() {
    assert_eq!(
        emitted_string_deserializer("String"),
        "pub fn deserialize_field < 'de , D > (deserializer : D) -> :: core :: result :: Result < \
         :: std :: string :: String , D :: Error > where D : serde :: Deserializer < 'de > , { use \
         serde :: Deserialize ; let s = :: std :: string :: String :: deserialize (deserializer) ? \
         ; validate_field_value (& s) . map_err (| violations : :: std :: vec :: Vec < :: std :: \
         string :: String > | serde :: de :: Error :: custom (violations . join (\"; \"))) ? ; \
         :: core :: result :: Result :: Ok (s) }"
    );

    let numeric_ty: syn::Type = syn::parse_str("u32").unwrap();
    let numeric_meta = ModelSchemaPropMeta {
        minimum: Some(5.0_f64),
        ..ModelSchemaPropMeta::default()
    };
    let numeric = generate_numeric_validation_code(
        &checked_struct_field(),
        "u32",
        &numeric_meta,
        &constrained_shape(&numeric_ty).unwrap(),
        &numeric_ty,
    )
    .module_items
    .to_string();
    assert!(
        numeric.ends_with(
            "pub fn deserialize_field < 'de , D > (deserializer : D) -> :: core :: result :: \
             Result < u32 , D :: Error > where D : serde :: Deserializer < 'de > , { use serde \
             :: Deserialize ; let v = u32 :: deserialize (deserializer) ? ; \
             validate_field_value (& v) . map_err (| violations : :: std :: vec :: Vec < :: std :: \
             string :: String > | serde :: de :: Error :: custom (violations . join (\"; \"))) \
             ? ; :: core :: result :: Result :: Ok (v) }"
        ),
        "bare numeric deserializer moved: {numeric}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_variant_field_names_its_helpers_for_its_variant() {
    assert_eq!(helper_name_stem("note", None), "note");
    assert_eq!(helper_name_stem("note", Some("Upload")), "upload_note");
    assert_eq!(
        helper_name_stem("note", Some("DeleteForever")),
        "delete_forever_note"
    );
}

/// The sole field of `item`, run through the constraint generator under `constraint` as the field
/// of variant `One`, gated as `gate` says, returning (`module_items`, `validate_body`,
/// `guard_error`, injected attribute count).
#[cfg(feature = "serde")]
fn generated_field_validation(
    item: &syn::ItemStruct,
    constraint: &ModelSchemaPropMeta,
    gate: ConstraintGate,
) -> (bool, bool, Option<String>, usize) {
    generated_validation_under(item, constraint, gate, Some("One"))
}

/// [`generated_field_validation`] for the field of `variant`, or of a struct where there is none.
#[cfg(feature = "serde")]
fn generated_validation_under(
    item: &syn::ItemStruct,
    constraint: &ModelSchemaPropMeta,
    gate: ConstraintGate,
    variant: Option<&str>,
) -> (bool, bool, Option<String>, usize) {
    let field = item.fields.iter().next().unwrap();
    let mut new_attrs = Vec::new();
    let steps = [vec!["member".to_owned()]];
    let ctx = FieldContext {
        container_defaulted: false,
        container_read_back: false,
        container_wire: super::ContainerWire::OneWay,
        enum_args: None,
        member_steps: &steps,
        on_the_wire: true,
        rename_all: None,
        schema_module_name: Some("probe_schema"),
        type_name: "Probe",
        type_parameters: &[],
        variant_ident: variant,
    };
    let (module_items, validate_body, guard_error) =
        generate_field_validation(&ctx, field, 0, constraint, gate, &mut new_attrs);
    (
        module_items.is_some(),
        validate_body.is_some(),
        guard_error.map(|tokens| tokens.to_string()),
        new_attrs.len(),
    )
}

#[cfg(feature = "serde")]
#[test]
fn a_tuple_structs_slot_is_checked_where_a_named_field_of_the_struct_is() {
    let min_length = ModelSchemaPropMeta {
        min_length: Some(3),
        ..ModelSchemaPropMeta::default()
    };
    let maximum = ModelSchemaPropMeta {
        maximum: Some(5.0_f64),
        ..ModelSchemaPropMeta::default()
    };
    let text: (syn::ItemStruct, syn::ItemStruct) = (
        syn::parse_quote! { struct Probe(String); },
        syn::parse_quote! { struct Probe { note: String } },
    );
    let number: (syn::ItemStruct, syn::ItemStruct) = (
        syn::parse_quote! { struct Probe(u32); },
        syn::parse_quote! { struct Probe { count: u32 } },
    );

    for ((positional, named), constraint) in [(&text, &min_length), (&number, &maximum)] {
        let slot =
            generated_validation_under(positional, constraint, ConstraintGate::Validator, None);
        assert_eq!(slot, (true, true, None, 0), "a validator and no serde hook");
        assert_eq!(
            slot,
            generated_validation_under(named, constraint, ConstraintGate::Validator, None)
        );
    }

    let unconstrained = generated_validation_under(
        &text.0,
        &ModelSchemaPropMeta::default(),
        ConstraintGate::Validator,
        None,
    );
    assert_eq!(unconstrained, (false, false, None, 0));
}

#[cfg(feature = "serde")]
#[test]
fn a_variants_slot_is_checked_where_a_named_member_of_the_variant_is() {
    let positional: syn::ItemStruct = syn::parse_quote! { struct Probe(String); };
    let named: syn::ItemStruct = syn::parse_quote! { struct Probe { note: String } };
    let borrowed: syn::ItemStruct = syn::parse_quote! { struct Probe<'text>(&'text str); };
    let min_length = ModelSchemaPropMeta {
        min_length: Some(3),
        ..ModelSchemaPropMeta::default()
    };

    for gate in [ConstraintGate::Deserializer, ConstraintGate::Validator] {
        assert_eq!(
            generated_field_validation(&positional, &min_length, gate),
            generated_field_validation(&named, &min_length, gate),
            "gate {gate:?}"
        );
    }
    assert_eq!(
        generated_field_validation(&borrowed, &min_length, ConstraintGate::Deserializer),
        (true, true, None, 0),
        "a borrowed slot publishes its validator and no hook"
    );
    assert_eq!(
        generated_field_validation(
            &positional,
            &ModelSchemaPropMeta::default(),
            ConstraintGate::Deserializer
        ),
        (false, false, None, 0)
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_constraint_under_an_interior_mutability_wrapper_names_the_wrapper() {
    for (spelling, wrapper) in [
        ("RefCell<String>", "RefCell"),
        ("Cell<u32>", "Cell"),
        ("Mutex<String>", "Mutex"),
        ("RwLock<String>", "RwLock"),
        ("&'a RefCell<String>", "RefCell"),
        ("Option<&'a Mutex<String>>", "Mutex"),
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let item: syn::ItemStruct = syn::parse_quote! {
            struct Probe { guarded: #ty }
        };

        let (module_items, validate_body, guard_error, injected_attrs) = generated_field_validation(
            &item,
            &ModelSchemaPropMeta {
                min_length: Some(3),
                ..ModelSchemaPropMeta::default()
            },
            ConstraintGate::Deserializer,
        );
        let error = guard_error.unwrap();
        assert!(error.contains("compile_error"), "for {spelling}: {error}");
        assert!(error.contains(wrapper), "for {spelling}: {error}");
        assert!(error.contains("Deref"), "for {spelling}: {error}");
        assert!(!module_items, "a refused field generated helpers: {error}");
        assert!(
            !validate_body,
            "a refused field reached validate(): {error}"
        );
        assert_eq!(injected_attrs, 0, "a refused field was given a serde hook");

        let unconstrained = generated_field_validation(
            &item,
            &ModelSchemaPropMeta::default(),
            ConstraintGate::Deserializer,
        );
        assert_eq!(unconstrained, (false, false, None, 0), "for {spelling}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_constrained_tuple_variant_names_its_helpers_by_the_slots_position() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Probe {
            One(#[model_schema_prop(minLength = 3)] String),
            Two(String, #[model_schema_prop(minimum = 5)] u32),
            Three(String),
        }
    };
    let variants = collect_discriminated_variants(
        &mut item,
        UNCASED,
        Some("probe_schema"),
        &super::ModelSchemaArgs::default(),
    );

    let errors: Vec<String> = variants.2.iter().map(ToString::to_string).collect();
    assert!(errors.is_empty(), "got: {errors:?}");

    let validation_fns: Vec<String> = variants.1.iter().map(ToString::to_string).collect();
    assert_eq!(validation_fns.len(), 2, "got: {validation_fns:?}");
    for (rendered, validator, reported) in [
        (
            &validation_fns[0],
            "fn validate_one_0_value",
            r#"format ! ("'{}': {}, got {}" , "One""#,
        ),
        (
            &validation_fns[1],
            "fn validate_two_1_value",
            r#"format ! ("'{}': {}, got {}" , "Two.1""#,
        ),
    ] {
        assert!(rendered.contains(validator), "got: {rendered}");
        assert!(rendered.contains(reported), "got: {rendered}");
    }

    let arms = &variants.3;
    let rendered_arms = quote::quote!(#(#arms),*).to_string();
    for arm in [
        "Self :: One (member_slot_0) => { if let :: core :: result :: Result :: Err (reported) = \
         validate_one_0_value (member_slot_0)",
        "Self :: Two (_ , member_slot_1) => { if let :: core :: result :: Result :: Err (reported) \
         = validate_two_1_value (member_slot_1)",
        "Self :: Three (..) => { }",
    ] {
        assert!(rendered_arms.contains(arm), "got: {rendered_arms}");
    }

    let rendered_attrs = walked_field_attrs(enum_fields(&item));
    assert_eq!(rendered_attrs, "", "a tagged slot was given a serde hook");
}

#[cfg(feature = "serde")]
#[test]
fn a_wrapped_field_deserializes_its_declared_type() {
    assert_eq!(
        emitted_string_deserializer("Option<String>"),
        "pub fn deserialize_field < 'de , D > (deserializer : D) -> :: core :: result :: Result < \
         Option < String > , D :: Error > where D : serde :: Deserializer < 'de > , { fn \
         deserialize_validated < 'de , D , T , F > (deserializer : D , check : F) -> :: core :: \
         result :: Result < T , D :: Error > where D : serde :: Deserializer < 'de > , T : \
         serde :: Deserialize < 'de > , F : :: core :: ops :: FnOnce (& T) -> :: core :: result :: Result < () , :: std \
         :: vec :: Vec < :: std :: string :: String >> , { use serde :: Deserialize ; let value = \
         T :: deserialize (deserializer) ? ; check (& value) . map_err (| violations : :: std :: \
         vec :: Vec < :: std :: string :: String > | serde :: de :: Error :: custom (violations . \
         join (\"; \"))) ? ; :: core :: result :: Result :: Ok (value) } deserialize_validated \
         (deserializer , | value_0 : & Option < String > | { if let :: core :: option :: Option :: \
         Some (value_1) = value_0 { validate_field_value (value_1) ? ; } :: core :: result :: \
         Result :: Ok (()) }) }"
    );
}

/// The `validate()` walk for `spelling` rewritten into the ending the wire needs: the collected
/// violation becomes the answered one, and nothing else about the walk is touched.
#[cfg(feature = "serde")]
fn wire_walk_of(spelling: &str) -> String {
    let ty: syn::Type = syn::parse_str(spelling).unwrap();
    let shape = constrained_shape(&ty).unwrap();
    let field = proc_macro2::Ident::new("field", proc_macro2::Span::call_site());
    let checker = proc_macro2::Ident::new("validate_field_value", proc_macro2::Span::call_site());
    build_field_validation(
        &shape.wraps,
        &member_access_expr(MemberAccess::SelfField, &field),
        &checker,
    )
    .to_string()
    .replace(
        "if let :: core :: result :: Result :: Err (reported) = validate_field_value (",
        "validate_field_value (",
    )
    .replace(") { errors . extend (reported) ; }", ") ? ;")
    .trim_start_matches("{ let value_0 = & self . field ; ")
    .trim_end_matches(" }")
    .to_owned()
}

#[cfg(feature = "serde")]
#[test]
fn the_wire_walk_is_the_validate_walk_shape_for_shape() {
    for spelling in [
        "Box<String>",
        "Cow<'static, str>",
        "Option<Box<str>>",
        "Box<Option<String>>",
    ] {
        let walk = wire_walk_of(spelling);
        let deserializer = emitted_string_deserializer(spelling);
        assert!(
            deserializer.contains(&walk),
            "spelling {spelling} walks differently on the wire than in validate(): \
             expected {walk} within {deserializer}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_borrowed_field_type_carries_its_lifetime_into_the_hook() {
    assert!(
        emitted_string_deserializer("Cow<'a, str>").starts_with(
            "pub fn deserialize_field < 'de , 'a , D > (deserializer : D) -> :: core :: result :: Result < Cow < 'a , str > , D :: Error >"
        ),
        "{}",
        emitted_string_deserializer("Cow<'a, str>")
    );
    assert!(
        emitted_string_deserializer("Cow<'static, str>").starts_with(
            "pub fn deserialize_field < 'de , D > (deserializer : D) -> :: core :: result :: Result < Cow < 'static , str > , D :: Error >"
        ),
        "{}",
        emitted_string_deserializer("Cow<'static, str>")
    );

    let ty: syn::Type = syn::parse_str("Option<Cow<'a, Cow<'a, str>>>").unwrap();
    let lifetimes = constrained_shape(&ty).unwrap().lifetimes;
    assert_eq!(
        lifetimes.len(),
        1,
        "a lifetime spelled twice must still be declared once"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_borrowed_str_publishes_its_validator_and_no_hook() {
    for spelling in ["&'a str", "&'static str", "Option<&'a str>"] {
        let module = emitted_string_module(spelling);
        assert!(
            module.starts_with("pub fn validate_field_value (value : & str)"),
            "for {spelling}: {module}"
        );
        assert!(
            !module.contains("deserialize_field"),
            "a hook was emitted for {spelling}: {module}"
        );
    }
    for spelling in [
        "String",
        "Option<String>",
        "Box<Option<String>>",
        "Cow<'a, str>",
    ] {
        assert!(
            emitted_string_module(spelling).contains("pub fn deserialize_field"),
            "the hook of {spelling} went missing"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_borrowed_str_member_is_hung_with_no_hook() {
    let min_length = ModelSchemaPropMeta {
        min_length: Some(3),
        ..ModelSchemaPropMeta::default()
    };

    for field in [
        quote::quote! { note: &'a str },
        quote::quote! { note: &'static str },
        quote::quote! { #[serde(borrow)] note: Option<&'a str> },
        quote::quote! { note: &'a [String] },
        quote::quote! { note: Option<&'a Vec<u8>> },
        quote::quote! { #[serde(deserialize_with = "trimmed")] note: &'a str },
        quote::quote! { #[serde(with = "trimmed")] note: &'a str },
    ] {
        let borrowed: syn::ItemStruct = syn::parse_quote! { struct Probe<'a> { #field } };
        for gate in [ConstraintGate::Deserializer, ConstraintGate::Validator] {
            assert_eq!(
                generated_field_validation(&borrowed, &min_length, gate),
                (true, true, None, 0),
                "for {field} under {gate:?}"
            );
        }
    }

    let owned: syn::ItemStruct = syn::parse_quote! { struct Probe { note: String } };
    assert_eq!(
        generated_field_validation(&owned, &min_length, ConstraintGate::Deserializer),
        (true, true, None, 1)
    );
    let owned_optional: syn::ItemStruct =
        syn::parse_quote! { struct Probe { note: Option<String> } };
    assert_eq!(
        generated_field_validation(&owned_optional, &min_length, ConstraintGate::Deserializer),
        (true, true, None, 2),
        "the hook, and the default that keeps a missing key a `None` under it"
    );
}

/// A member read through a reference, whatever it borrows, publishes its validator and is hung
/// with no hook.
#[cfg(feature = "serde")]
#[test]
fn a_member_read_through_any_reference_is_hung_with_no_hook() {
    let min_length = ModelSchemaPropMeta {
        min_length: Some(3),
        ..ModelSchemaPropMeta::default()
    };
    let minimum = ModelSchemaPropMeta {
        minimum: Some(5.0_f64),
        ..ModelSchemaPropMeta::default()
    };

    for (field, constraint) in [
        (quote::quote! { note: &'a String }, &min_length),
        (quote::quote! { note: &'a Option<String> }, &min_length),
        (quote::quote! { note: Option<&'a Box<String>> }, &min_length),
        (quote::quote! { note: &'a &'a str }, &min_length),
        (quote::quote! { note: &'a Path }, &min_length),
        (quote::quote! { count: &'a u32 }, &minimum),
        (quote::quote! { count: &'a Option<u32> }, &minimum),
        (quote::quote! { count: Option<&'a f64> }, &minimum),
    ] {
        let borrowed: syn::ItemStruct = syn::parse_quote! { struct Probe<'a> { #field } };
        for gate in [ConstraintGate::Deserializer, ConstraintGate::Validator] {
            assert_eq!(
                generated_field_validation(&borrowed, constraint, gate),
                (true, true, None, 0),
                "for {field} under {gate:?}"
            );
        }
    }
}

/// The validator of a value read through a reference is the owned value's, and no hook is
/// written beside it.
#[cfg(feature = "serde")]
#[test]
fn a_value_read_through_any_reference_publishes_its_validator_and_no_hook() {
    for spelling in [
        "&'a String",
        "&'static String",
        "&'a Option<String>",
        "Option<&'a Box<String>>",
    ] {
        let module = emitted_string_module(spelling);
        assert!(
            module.starts_with("pub fn validate_field_value (value : & str)"),
            "for {spelling}: {module}"
        );
        assert!(
            !module.contains("deserialize_field"),
            "a hook was emitted for {spelling}: {module}"
        );
    }

    let minimum = ModelSchemaPropMeta {
        minimum: Some(5.0_f64),
        ..ModelSchemaPropMeta::default()
    };
    for spelling in ["&'a u32", "&'static u32", "&'a Option<u32>"] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let module = generate_numeric_validation_code(
            &checked_struct_field(),
            "u32",
            &minimum,
            &constrained_shape(&ty).unwrap(),
            &ty,
        )
        .module_items
        .to_string();
        assert!(
            module.starts_with("pub fn validate_field_value (value : & u32)"),
            "for {spelling}: {module}"
        );
        assert!(
            !module.contains("deserialize_field"),
            "a hook was emitted for {spelling}: {module}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn only_an_outermost_option_without_a_default_gets_one_injected() {
    let defaulted = true;
    let plain = false;

    for spelling in [
        "Option<String>",
        "Option<Vec<String>>",
        "Box<Option<String>>",
        "Arc<Rc<Option<String>>>",
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let wraps = constrained_shape(&ty).unwrap().wraps;
        assert!(
            needs_injected_default(&wraps, plain),
            "{spelling} would answer a missing key with an error"
        );
        assert!(
            !needs_injected_default(&wraps, defaulted),
            "{spelling} already has a default of its own"
        );
    }

    for spelling in [
        "String",
        "Vec<Option<String>>",
        "Box<String>",
        "Box<Vec<Option<String>>>",
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let wraps = constrained_shape(&ty).unwrap().wraps;
        assert!(
            !needs_injected_default(&wraps, plain),
            "{spelling} has no optional key to restore"
        );
    }
}

/// A field with no value for a length or a range to describe emits nothing at all.
#[cfg(feature = "serde")]
#[test]
fn a_field_without_a_constrainable_value_has_no_shape() {
    for spelling in [
        "Tagged<String>",
        "HashMap<String, String>",
        "(String, String)",
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        assert!(
            constrained_shape(&ty).is_none(),
            "spelling {spelling} should reach no constrainable value"
        );
    }
}

/// A bound is read through every reference, to whatever it borrows.
#[cfg(feature = "serde")]
#[test]
fn every_reference_is_read_through_to_what_it_borrows() {
    use super::ConstraintWrap::{Borrowed, Optional};

    for (spelling, leaf, wraps) in [
        ("&'a String", "str", vec![Borrowed]),
        ("&'a Option<Vec<String>>", "list", vec![Borrowed, Optional]),
        ("&'a u32", "u32", vec![Borrowed]),
        ("&'a Path", "path", vec![Borrowed]),
        ("&'a Slug", "Slug", vec![Borrowed]),
        ("&'a &'a str", "str", vec![Borrowed, Borrowed]),
        ("Option<&'a String>", "str", vec![Optional, Borrowed]),
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let shape = constrained_shape(&ty).unwrap();
        let reached = match shape.leaf {
            ConstraintLeaf::List => "list".to_owned(),
            ConstraintLeaf::Named(named) => quote::ToTokens::to_token_stream(&named).to_string(),
            ConstraintLeaf::Number(number) => number.to_owned(),
            ConstraintLeaf::Path => "path".to_owned(),
            ConstraintLeaf::Str => "str".to_owned(),
        };
        assert_eq!(reached, leaf, "for {spelling}");
        assert!(shape.wraps == wraps, "for {spelling}");
    }
}

/// A held model is walked through every reference, by both walks, to what the owned spelling
/// walks to.
#[cfg(feature = "serde")]
#[test]
fn a_held_model_is_walked_through_every_reference() {
    use super::ConstraintWrap::{Borrowed, Optional, Sequence};

    for (spelling, wraps) in [
        ("&'a Stamp", vec![Borrowed]),
        ("Option<&'a Stamp>", vec![Optional, Borrowed]),
        ("&'a [Stamp]", vec![Borrowed, Sequence]),
        ("&'a Option<Stamp>", vec![Borrowed, Optional]),
        ("&'a &'a Vec<Stamp>", vec![Borrowed, Borrowed, Sequence]),
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let def = get_field_def("", &ty, "");
        assert!(
            super::nested_shape(&ty).unwrap() == wraps,
            "the nested walk, for {spelling}"
        );
        let held = super::held_shape(&ty, &def).unwrap();
        assert!(
            matches!(held.end, super::HeldEnd::Declared),
            "for {spelling}"
        );
        assert!(held.wraps == wraps, "the held walk, for {spelling}");
    }
}

/// A field read through a reference is given no hook: serde reads no reference to a model.
#[cfg(feature = "serde")]
#[test]
fn a_field_read_through_a_reference_is_given_no_read_hook() {
    let steps = [vec!["stamp".to_owned()]];
    let ctx = FieldContext {
        container_defaulted: false,
        container_read_back: true,
        container_wire: super::ContainerWire::OneWay,
        enum_args: None,
        member_steps: &steps,
        on_the_wire: true,
        rename_all: None,
        schema_module_name: Some("probe_schema"),
        type_name: "Probe",
        type_parameters: &[],
        variant_ident: None,
    };
    let named_below = |spelling: &str| {
        let field: syn::Field =
            syn::parse_str::<syn::FieldsNamed>(&format!("{{ stamp: {spelling} }}"))
                .unwrap()
                .named
                .into_iter()
                .next()
                .unwrap();
        let def = get_field_def("stamp", &field.ty, "");
        super::named_below(&field, &def, &ctx)
    };
    assert!(named_below("Stamp").is_some());
    assert!(named_below("Option<Stamp>").is_some());
    // A type that names a lifetime is refused on that alone; the spellings with none are refused
    // for the reference.
    for spelling in ["&'a Stamp", "Option<&'a Stamp>", "&Stamp", "Option<&Stamp>"] {
        assert!(named_below(spelling).is_none(), "for {spelling}");
    }

    let mut item: syn::ItemStruct = syn::parse_quote! {
        #[derive(serde::Deserialize, serde::Serialize)]
        struct Probe<'a> {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            maybe: Option<&'a Stamp>,
            stamp: &'a Stamp,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("probe_schema"),
        "Probe",
        &item.generics.clone(),
        &super::StructContainer {
            defaulted: false,
            read_back: true,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);
    let attrs = walked_field_attrs(item.fields.iter());
    assert!(!attrs.contains("deserialize_with"), "got: {attrs}");
    assert!(
        collected.2.is_empty(),
        "got: {:?}",
        collected
            .2
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    let walked = collected
        .3
        .iter()
        .map(ToString::to_string)
        .collect::<String>();
    assert_eq!(walked.matches(". validate ()").count(), 2, "got: {walked}");
    let read = super::judged_read::of_struct(&mut item, true).to_string();
    assert!(!read.contains("read_unjudged_"), "got: {read}");
}

#[cfg(feature = "serde")]
#[test]
fn a_field_typed_with_a_named_type_ends_on_that_type() {
    for (spelling, leaf, depth) in [
        ("Tag", "Tag", 0),
        ("Option<Tag>", "Tag", 1),
        ("Option<Box<tags::Tag>>", "tags :: Tag", 2),
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let shape = constrained_shape(&ty).unwrap();
        let written = match shape.leaf {
            ConstraintLeaf::Named(named) => quote::ToTokens::to_token_stream(&named).to_string(),
            ConstraintLeaf::List
            | ConstraintLeaf::Number(_)
            | ConstraintLeaf::Path
            | ConstraintLeaf::Str => String::new(),
        };
        assert_eq!(written, leaf, "for {spelling}");
        assert_eq!(shape.wraps.len(), depth, "for {spelling}");
    }
}

/// A list is a value of its own: the walk ends on it and reads nothing it holds.
#[cfg(feature = "serde")]
#[test]
fn a_list_is_the_value_the_walk_ends_on() {
    for (spelling, depth, _held) in LIST_REACH {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let shape = constrained_shape(&ty).unwrap();
        assert!(matches!(shape.leaf, ConstraintLeaf::List), "for {spelling}");
        assert_eq!(shape.wraps.len(), depth, "for {spelling}");
    }
}

/// `held_as_a_list` is the constraint walk held to a list behind transparent wrappers alone.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn the_list_a_brand_holds_is_the_constraint_walks() {
    let no_list = [
        "String",
        "Option<String>",
        "Box<str>",
        "Tag",
        "HashMap<String, u8>",
    ];
    for (spelling, held) in LIST_REACH
        .iter()
        .map(|&(spelling, _depth, held)| (spelling, held))
        .chain(no_list.map(|spelling| (spelling, false)))
    {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let walked = constrained_shape(&ty).is_some_and(|shape| {
            matches!(shape.leaf, ConstraintLeaf::List)
                && shape
                    .wraps
                    .iter()
                    .all(|wrap| matches!(wrap, super::ConstraintWrap::Transparent))
        });
        assert_eq!(walked, held, "the walk, for {spelling}");
        assert_eq!(super::held_as_a_list(&ty), held, "for {spelling}");
    }
}

/// The slots a tuple struct is read from, at the three arities the dispatch answers for. The empty
/// one has no declaration in this crate — `struct Nothing();` is refused by the lint table — so it
/// is read here, where the slots are built rather than parsed off an item.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn tuple_slots(spellings: &[&str]) -> Vec<super::FieldDef> {
    spellings
        .iter()
        .map(|spelling| get_field_def("", &syn::parse_str(spelling).unwrap(), ""))
        .collect()
}

/// The shape a struct declaring exactly the given slots publishes, none of them off the wire.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn whole_tuple(spellings: &[&str]) -> super::TupleStructShape {
    tuple_struct_shape(spellings.len(), tuple_slots(spellings))
}

#[cfg(feature = "typescript")]
#[test]
fn a_tuple_struct_describes_as_its_arity_in_typescript() {
    assert_eq!(tuple_struct_ts_body(&whole_tuple(&[])), "[]");
    assert_eq!(tuple_struct_ts_body(&whole_tuple(&["String"])), "string");
    assert_eq!(
        tuple_struct_ts_body(&whole_tuple(&["String", "u32"])),
        "[string, number]"
    );
}

/// [`a_tuple_struct_describes_as_its_arity_in_typescript`] for the Zod surface.
#[cfg(feature = "zod")]
#[test]
fn a_tuple_struct_describes_as_its_arity_in_zod() {
    let owner = super::SlotOwner {
        item_name: "Whole",
        #[cfg(feature = "typescript")]
        parameters: &[],
        rust_ident: "Whole",
    };
    assert_eq!(
        tuple_struct_zod_body(&whole_tuple(&[]), &owner),
        "z.tuple([])"
    );
    assert_eq!(
        tuple_struct_zod_body(&whole_tuple(&["String"]), &owner),
        "z.string()"
    );
    assert_eq!(
        tuple_struct_zod_body(&whole_tuple(&["String", "u32"]), &owner),
        "z.tuple([z.string(), z.number().int().min(0).max(4294967295)])"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_tuple_struct_describes_as_its_arity_in_json_schema() {
    let empty = tuple_struct_json_body("Nothing", &whole_tuple(&[])).to_string();
    assert!(empty.contains("prefixItems"), "Got: {empty}");
    assert!(empty.contains("minItems"), "Got: {empty}");

    let single = tuple_struct_json_body("Plain", &whole_tuple(&["String"])).to_string();
    assert!(single.contains("string"), "Got: {single}");
    assert!(!single.contains("prefixItems"), "Got: {single}");

    let pair = tuple_struct_json_body("Pair", &whole_tuple(&["String", "u32"])).to_string();
    assert!(pair.contains("prefixItems"), "Got: {pair}");
    assert!(pair.contains("maxItems"), "Got: {pair}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_tuple_struct_slot_the_dispatch_cannot_render_is_refused() {
    for shape in [
        whole_tuple(&["HashMap<(u32, u32), u32>"]),
        whole_tuple(&["String", "HashMap<(u32, u32), u32>"]),
    ] {
        let rendered = tuple_struct_json_body("Pair", &shape).to_string();
        assert!(
            rendered.starts_with(":: core :: compile_error !"),
            "Got: {rendered}"
        );
        assert!(
            rendered.contains(&format!("model_schema: `Pair`: {TUPLE_KEY_REFUSAL}")),
            "Got: {rendered}"
        );
        assert!(!rendered.contains("prefixItems"), "Got: {rendered}");
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_tuple_struct_slot_points_at_that_slot_alone() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Pair(pub String, pub HashMap<(u32, u32), u32>);").unwrap();
    let slots: Vec<super::FieldDef> = item
        .fields
        .iter()
        .map(|field| get_field_def("", &field.ty, ""))
        .collect();
    let shape = tuple_struct_shape(slots.len(), slots);
    assert_points_only_at(
        &tuple_struct_json_body("Pair", &shape),
        "(u32, u32)",
        "a tuple struct's second slot",
    );
}

/// The bare value is the *declared* arity's, not the described list's.
#[cfg(feature = "typescript")]
#[test]
fn a_slot_dropped_off_the_wire_leaves_the_tuple_an_array() {
    let shrunk = tuple_struct_shape(2, tuple_slots(&["String"]));
    assert_eq!(tuple_struct_ts_body(&shrunk), "[string]");
    assert_eq!(
        tuple_struct_ts_body(&tuple_struct_shape(1, tuple_slots(&["String"]))),
        "string"
    );
}

/// The same reading on the JSON surface, where the arity is written twice as its own bounds.
#[cfg(feature = "jsonschema")]
#[test]
fn a_slot_dropped_off_the_wire_shrinks_the_described_arity() {
    let shrunk = tuple_struct_json_body("Pair", &tuple_struct_shape(2, tuple_slots(&["u32"])));
    let rendered = shrunk.to_string();
    assert!(rendered.contains("prefixItems"), "Got: {rendered}");
    assert!(rendered.contains("1usize"), "Got: {rendered}");
    assert!(!rendered.contains("2usize"), "Got: {rendered}");
}

/// A two-slot struct whose second slot is written at the given spelling.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn slot_pair(spelling: &str) -> syn::ItemStruct {
    syn::parse_str(&format!(
        "struct Pair(String, #[serde({spelling})] Option<String>);"
    ))
    .unwrap()
}

/// Runs the slot refusal over that struct's second slot, with the declared arity supplied rather
/// than counted, so the lone-slot exemption can be read off the same declaration.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn slot_guard_result(item: &syn::ItemStruct, declared_slots: usize) -> Result<(), syn::Error> {
    let field = item.fields.iter().nth(1).unwrap();
    check_slot_wire_is_readable(
        field,
        1,
        declared_slots,
        "Pair",
        parse_serde_key_omission(&field.attrs),
    )
}

/// The refusal message for the slot at that spelling, and `None` where it is left alone.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
fn slot_refusal(spelling: &str, declared_slots: usize) -> Option<String> {
    slot_guard_result(&slot_pair(spelling), declared_slots)
        .err()
        .map(|err| err.to_string())
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_slot_dropped_from_one_direction_only_is_refused() {
    for (spelling, refused) in SLOT_OMISSION_SPELLINGS {
        let refusal = slot_refusal(spelling, 2);
        assert_eq!(refusal.is_some(), refused, "{spelling}: {refusal:?}");
        if let Some(message) = refusal {
            assert!(message.contains("slot 1"), "{spelling}: {message}");
            assert!(message.contains("`Pair`"), "{spelling}: {message}");
        }
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_lone_slot_is_refused_for_no_spelling() {
    for (spelling, _) in SLOT_OMISSION_SPELLINGS {
        assert_eq!(slot_refusal(spelling, 1), None, "for: {spelling}");
    }
}

/// The variant a declaration's first (and here only) variant parses to.
fn declared_variant(declaration: &str) -> syn::Variant {
    let item: syn::ItemEnum = syn::parse_str(declaration).unwrap();
    item.variants.into_iter().next().unwrap()
}

/// Every refusal that declaration's variant earns, one per slot that earns one.
fn variant_slot_refusals(declaration: &str) -> Vec<String> {
    let variant = declared_variant(declaration);
    let variant_name = variant.ident.to_string();
    variant
        .fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
            check_variant_slot_wire_is_readable(
                field,
                index,
                &variant_name,
                "Wire",
                parse_serde_key_omission(&field.attrs),
            )
            .err()
            .map(|err| err.to_string())
        })
        .collect()
}

#[test]
fn a_variant_slot_dropped_from_one_direction_only_is_refused() {
    for (spelling, refused) in SLOT_OMISSION_SPELLINGS {
        let refusals = variant_slot_refusals(&format!(
            "enum Wire {{ One(#[serde({spelling})] Option<String>, u32) }}"
        ));
        assert_eq!(
            refusals.len(),
            usize::from(refused),
            "{spelling}: {refusals:?}"
        );
        for message in refusals {
            assert!(message.contains("slot 0"), "{spelling}: {message}");
            assert!(message.contains("`One`"), "{spelling}: {message}");
            assert!(message.contains("`Wire`"), "{spelling}: {message}");
        }
    }
}

#[test]
fn a_named_variant_member_is_refused_for_no_spelling() {
    for (spelling, _) in SLOT_OMISSION_SPELLINGS {
        let refusals = variant_slot_refusals(&format!(
            "enum Wire {{ One {{ #[serde({spelling})] a: Option<String> }} }}"
        ));
        assert!(refusals.is_empty(), "{spelling}: {refusals:?}");
    }
}

#[test]
fn a_lone_variant_slot_is_refused_for_the_same_spellings() {
    for (spelling, refused) in SLOT_OMISSION_SPELLINGS {
        let refusals = variant_slot_refusals(&format!(
            "enum Wire {{ One(#[serde({spelling})] Option<String>) }}"
        ));
        assert_eq!(
            refusals.len(),
            usize::from(refused),
            "{spelling}: {refusals:?}"
        );
    }
}

#[test]
fn a_variant_taking_its_lone_slot_off_the_wire_publishes_a_unit() {
    for spelling in ["skip", "skip_serializing, skip_deserializing"] {
        let variant =
            declared_variant(&format!("enum Wire {{ One(#[serde({spelling})] String) }}"));
        assert_eq!(variant_wire_kind(&variant), VariantKind::Unit, "{spelling}");
    }
}

#[test]
fn every_other_declared_arity_keeps_the_kind_it_declared() {
    for (declaration, expected) in [
        (
            "enum Wire { One(#[serde(skip)] String, u32) }",
            VariantKind::TupleMultiple,
        ),
        (
            "enum Wire { One(#[serde(skip)] String, #[serde(skip)] u32) }",
            VariantKind::TupleMultiple,
        ),
        ("enum Wire { One(String) }", VariantKind::TupleSingle),
        (
            "enum Wire { One { #[serde(skip)] a: String } }",
            VariantKind::Named,
        ),
        ("enum Wire { One }", VariantKind::Unit),
    ] {
        assert_eq!(
            variant_wire_kind(&declared_variant(declaration)),
            expected,
            "for: {declaration}"
        );
    }
}

/// The adjacent-form refusals a declaration earns, one per variant that earns one, as rendered
/// `compile_error!` token strings.
#[cfg(feature = "serde")]
fn adjacent_refusals(declaration: &str) -> Vec<String> {
    let item: syn::ItemEnum = syn::parse_str(declaration).unwrap();
    adjacent_collapsed_slot_guard_errors(&item, "type", "value")
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[cfg(feature = "serde")]
#[test]
fn an_adjacent_variant_whose_lone_slot_is_dropped_is_refused() {
    let refusals = adjacent_refusals("enum Wire { One(#[serde(skip)] String) }");
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    for needle in [
        "`One`",
        "`Wire`",
        r#"{\"type\":\"One\"}"#,
        r#"{\"type\":\"One\",\"value\":null}"#,
        "Declare `One` as a unit variant",
    ] {
        assert!(
            refusals[0].contains(needle),
            "{needle} missing from: {}",
            refusals[0]
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn every_other_adjacent_arity_is_left_alone() {
    for declaration in [
        "enum Wire { One(#[serde(skip)] String, u32) }",
        "enum Wire { One(#[serde(skip)] String, #[serde(skip)] u32) }",
        "enum Wire { One(String) }",
        "enum Wire { One { #[serde(skip)] a: String } }",
        "enum Wire { One }",
        "enum Wire { One() }",
    ] {
        let refusals = adjacent_refusals(declaration);
        assert!(refusals.is_empty(), "{declaration}: {refusals:?}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn the_adjacent_collapse_refusal_points_at_the_variant() {
    let item: syn::ItemEnum =
        syn::parse_str("enum Wire { Kept(u8, bool), Lone(#[serde(skip)] String) }").unwrap();
    let errors = adjacent_collapsed_slot_guard_errors(&item, "type", "value");
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert_eq!(
        errors[0].span().source_text().as_deref(),
        Some("Lone(#[serde(skip)] String)")
    );
}

#[cfg(feature = "serde")]
#[test]
fn the_adjacent_collapse_refusal_quotes_the_declared_keys() {
    let item: syn::ItemEnum = syn::parse_str("enum Wire { One(#[serde(skip)] String) }").unwrap();
    let refusal = adjacent_collapsed_slot_guard_errors(&item, "kind", "payload")[0].to_string();
    assert!(refusal.contains(r#"{\"kind\":\"One\"}"#), "Got: {refusal}");
    assert!(
        refusal.contains(r#"{\"kind\":\"One\",\"payload\":null}"#),
        "Got: {refusal}"
    );
}

/// The collected walk a renderer probe hands one variant, holding just the members it writes.
#[cfg(feature = "serde")]
fn walked_members(field_defs: &[super::FieldDef]) -> super::UntaggedVariantMembers {
    super::UntaggedVariantMembers {
        bound: Vec::new(),
        checks: Vec::new(),
        deferred_attrs: Vec::new(),
        field_defs: field_defs.to_vec(),
        flattened_fields: Vec::new(),
        guard_errors: Vec::new(),
        validation_fns: Vec::new(),
    }
}

/// The member spelling an untagged variant is refused with, run over the kind it publishes.
#[cfg(feature = "serde")]
fn untagged_refusal(declaration: &str, members: &[super::FieldDef]) -> String {
    let variant = declared_variant(declaration);
    render_untagged_variant(
        &variant_wire_kind(&variant),
        &variant,
        &walked_members(members),
        "Wire",
    )
    .map_or_else(|err| err.to_string(), |_| String::new())
}

#[cfg(feature = "serde")]
#[test]
fn an_untagged_variant_whose_lone_slot_is_dropped_is_refused_for_the_collapse() {
    let refusal = untagged_refusal("enum Wire { One(#[serde(skip)] String) }", &[]);
    for needle in [
        "`One`",
        "off the wire in both directions",
        "`null`",
        "Keep the slot on the wire",
        "remove `One` from the union",
    ] {
        assert!(refusal.contains(needle), "{needle} missing from: {refusal}");
    }
    assert!(
        !refusal.contains("supports newtype"),
        "the collapse must not be told what the union supports: {refusal}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn the_untagged_collapse_refusal_points_at_the_variant() {
    let variant = declared_variant("enum Wire { One(#[serde(skip)] String) }");
    let error = render_untagged_variant(
        &variant_wire_kind(&variant),
        &variant,
        &walked_members(&[]),
        "Wire",
    )
    .unwrap_err();
    assert_eq!(
        error.span().source_text().as_deref(),
        Some("One(#[serde(skip)] String)")
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_declared_untagged_unit_variant_keeps_its_own_refusal() {
    // The ellipsis and em dash are spelled by escape so the pin stays byte-exact without writing a
    // non-ASCII literal into the source.
    let expected = "model_schema: variant `One`: `#[serde(untagged)]` supports newtype (`V(T)`) \
                    and struct (`V { \u{2026} }`) variants only \u{2014} `One` is a unit variant, \
                    which the union has no member spelling for: a member is written as the inner \
                    type or as an object of named fields, and a variant that is neither has \
                    nothing to be written as. Give it a single inner type or named fields, or drop \
                    `#[serde(untagged)]`.";
    for declaration in ["enum Wire { One }", "enum Wire { One() }"] {
        assert_eq!(
            untagged_refusal(declaration, &[]),
            expected,
            "{declaration}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_refused_untagged_tuple_variant_is_named_by_its_declared_arity() {
    let carried = get_field_def("_1", &syn::parse_str::<syn::Type>("u32").unwrap(), "");
    let refusal = untagged_refusal("enum Wire { One(#[serde(skip)] String, u32) }", &[carried]);
    assert!(
        refusal.contains("a tuple variant with 2 fields"),
        "Got: {refusal}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn every_path_spelling_reaches_a_constrainable_value() {
    for spelling in [
        "PathBuf",
        "std::path::PathBuf",
        "Box<Path>",
        "Cow<'a, Path>",
        "Option<PathBuf>",
        "Option<Box<Path>>",
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        let shape = constrained_shape(&ty).unwrap();
        assert!(
            matches!(shape.leaf, ConstraintLeaf::Path),
            "spelling {spelling} should reach the path leaf"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_path_is_checked_through_its_lossy_rendering() {
    let module = emitted_string_module("PathBuf");
    assert!(
        module.starts_with(
            "pub fn validate_field_value (path : & :: std :: path :: Path) -> :: core :: result :: \
             Result < () , :: std :: vec :: Vec < :: std :: string :: String >> { let rendered = path \
             . to_string_lossy () ; let value : & str = & rendered ;"
        ),
        "got: {module}"
    );
    assert!(
        module.contains("if value . len () < 3usize"),
        "the checks are the ones a string field is held to: {module}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_bare_path_field_deserializes_the_owned_path() {
    assert_eq!(
        emitted_string_deserializer("PathBuf"),
        "pub fn deserialize_field < 'de , D > (deserializer : D) -> :: core :: result :: Result < \
         :: std :: path :: PathBuf , D :: Error > where D : serde :: Deserializer < 'de > , { use \
         serde :: Deserialize ; let s = :: std :: path :: PathBuf :: deserialize (deserializer) ? \
         ; validate_field_value (& s) . map_err (| violations : :: std :: vec :: Vec < :: std :: \
         string :: String > | serde :: de :: Error :: custom (violations . join (\"; \"))) ? ; \
         :: core :: result :: Result :: Ok (s) }"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_wrapped_path_field_deserializes_its_declared_type() {
    assert_eq!(
        emitted_string_deserializer("Box<Path>"),
        emitted_string_deserializer("Box<str>").replace("Box < str >", "Box < Path >")
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_trivial_pattern_is_emitted_as_the_call_it_says_the_same_thing_as() {
    assert_eq!(
        emitted_pattern_validator("^/"),
        "pub fn validate_field_value (value : & str) -> :: core :: result :: Result < () , :: std :: \
         vec :: Vec < :: std :: string :: String >> { let mut errors : :: std :: vec :: Vec < :: std :: \
         string :: String > = :: std :: vec :: Vec :: new () ; if ! value . starts_with ('/') { \
         errors . push (format ! (\"'{}': {}\" , \"field\" , \"does not match pattern '^/'\")) \
         ; } if errors . is_empty () { :: core :: result :: Result :: Ok (()) } else { :: core :: \
         result :: Result :: Err (errors) } } "
    );
    assert_eq!(
        emitted_pattern_validator("^abc$"),
        "pub fn validate_field_value (value : & str) -> :: core :: result :: Result < () , :: std :: \
         vec :: Vec < :: std :: string :: String >> { let mut errors : :: std :: vec :: Vec < :: std :: \
         string :: String > = :: std :: vec :: Vec :: new () ; if value != \"abc\" { errors . push \
         (format ! (\"'{}': {}\" , \"field\" , \"does not match pattern '^abc$'\")) ; } if \
         errors . is_empty () { :: core :: result :: Result :: Ok (()) } else { :: core :: result :: \
         Result :: Err (errors) } } "
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_pattern_of_any_real_shape_keeps_its_regex() {
    assert_eq!(
        emitted_pattern_validator("^[a-z]+$"),
        "pub fn validate_field_value (value : & str) -> :: core :: result :: Result < () , :: std :: \
         vec :: Vec < :: std :: string :: String >> { let mut errors : :: std :: vec :: Vec < :: std :: \
         string :: String > = :: std :: vec :: Vec :: new () ; { use :: std :: sync :: LazyLock ; \
         static RE : LazyLock < regex :: Regex > = LazyLock :: new (|| { regex :: Regex :: new \
         (\"^[a-z]+$\") . unwrap () }) ; if ! RE . is_match (value) { errors . push (format ! \
         (\"'{}': {}\" , \"field\" , \"does not match pattern '^[a-z]+$'\")) ; } } if errors . \
         is_empty () { :: core :: result :: Result :: Ok (()) } else { :: core :: result :: Result :: \
         Err (errors) } } "
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_scalar_union_member_is_recorded_as_the_type_serde_writes_it_as() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Choice {
            Obj(Holder),
            Text(String),
            Count(u32),
            Many(Vec<Holder>),
        }
    };
    let (_, _, _, merge_parts, _, errors, _, _, _, _) = collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    let recorded: Vec<(String, Option<&str>)> = merge_parts
        .iter()
        .map(|member| (member.branch_path(), member.non_object))
        .collect();
    assert_eq!(
        recorded,
        vec![
            ("1".to_owned(), None),
            ("2".to_owned(), Some("string")),
            ("3".to_owned(), Some("integer")),
            ("4".to_owned(), Some("array")),
        ]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_with_a_scalar_member_is_refused_naming_the_branch() {
    let error = recorded_union_flatten_error(
        "ScalarChoice",
        syn::parse_quote! {
            enum ScalarChoice {
                Obj(Holder),
                Text(String),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: ScalarChoice },
    )
    .unwrap();
    assert!(error.contains("compile_error"), "got: {error}");
    assert!(
        error.contains(
            "`#[serde(flatten)]` of `ScalarChoice` writes a union member that is not an object"
        ),
        "got: {error}"
    );
    assert!(
        error.contains("its branch 2 describes a `string`"),
        "got: {error}"
    );
    assert!(
        error.contains("write the field as a named member so the value gets a key of its own"),
        "got: {error}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_nested_scalar_union_member_is_refused_by_its_trail() {
    recorded_union_flatten_error(
        "NestedInner",
        syn::parse_quote! {
            enum NestedInner {
                Obj(Holder),
                Text(String),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: NestedInner },
    );
    let error = recorded_union_flatten_error(
        "NestedOuter",
        syn::parse_quote! {
            enum NestedOuter {
                Inner(NestedInner),
                Other(Holder),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: NestedOuter },
    )
    .unwrap();
    assert!(
        error.contains("its branch 1.2 describes a `string`"),
        "got: {error}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_of_objects_is_not_refused() {
    assert!(
        recorded_union_flatten_error(
            "ObjectChoice",
            syn::parse_quote! {
                enum ObjectChoice {
                    First(Holder),
                    Second(Other),
                }
            },
            &syn::parse_quote! { #[serde(flatten)] either: ObjectChoice },
        )
        .is_none()
    );
    let unrecorded: syn::Field = syn::parse_quote! { #[serde(flatten)] base: NeverRecorded };
    assert!(flatten_edge_guard_error(&unrecorded, "Host").is_none());
}

/// Records an untagged enum's members the way its own expansion does, then asks the flatten guard
/// what an object naming it would be told. Returns the rendered `compile_error!`, or `None` where
/// the merge has nothing to refuse.
#[cfg(all(feature = "serde", feature = "zod"))]
fn recorded_union_flatten_error(
    rust_ident: &str,
    mut item: syn::ItemEnum,
    field: &syn::Field,
) -> Option<String> {
    let (_, _, _, merge_parts, _, errors, _, _, _, _) = collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    register_alias_info(
        rust_ident,
        rust_ident,
        &ident_schema_module_name(rust_ident),
        AliasKind::NoEnumMembers,
    );
    record_zod_union_members(rust_ident, &merge_parts);
    flatten_edge_guard_error(field, "Host").map(|error| error.to_string())
}

/// The branch trails one untagged enum's members are recorded at, beside what each is proved to
/// write, in declaration order.
#[cfg(all(feature = "serde", feature = "zod"))]
fn recorded_member_trails(mut item: syn::ItemEnum) -> Vec<(String, Option<&'static str>)> {
    let (_, _, _, merge_parts, _, errors, _, _, _, _) = collect_untagged_members(
        &mut item,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    merge_parts
        .iter()
        .map(|member| (member.branch_path(), member.non_object))
        .collect()
}

/// Registers a name carrying both answers a `#[model_schema()]` item's own expansion records for
/// it: what serde writes it as, and the JSON type keyword its published wire describes as where
/// that wire is proved to be no object.
#[cfg(all(feature = "serde", feature = "zod"))]
fn seed_registered_wire(rust_ident: &str, kind: AliasKind, wire: Option<&'static str>) {
    register_alias_info(
        rust_ident,
        rust_ident,
        &ident_schema_module_name(rust_ident),
        kind,
    );
    record_wire_leaves(
        rust_ident,
        &[WireLeaf {
            branch: Vec::new(),
            non_object: wire,
            written_as: None,
        }],
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn an_optional_union_member_is_recorded_as_its_value_beside_the_absence() {
    assert_eq!(
        recorded_member_trails(syn::parse_quote! {
            enum Choice {
                Obj(Holder),
                Maybe(Option<Holder>),
                Text(Option<String>),
            }
        }),
        vec![
            ("1".to_owned(), None),
            ("2.1".to_owned(), None),
            ("2.2".to_owned(), Some("null")),
            ("3.1".to_owned(), Some("string")),
            ("3.2".to_owned(), Some("null")),
        ]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_member_written_without_an_option_keeps_the_one_position_it_had() {
    assert_eq!(
        recorded_member_trails(syn::parse_quote! {
            enum Choice {
                Obj(Holder),
                Other(Second),
            }
        }),
        vec![("1".to_owned(), None), ("2".to_owned(), None)]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_with_an_optional_member_is_refused_naming_the_null_leaf() {
    let error = recorded_union_flatten_error(
        "NullableChoice",
        syn::parse_quote! {
            enum NullableChoice {
                Obj(Holder),
                Maybe(Option<Holder>),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: NullableChoice },
    )
    .unwrap();
    assert!(
        error.contains(
            "`#[serde(flatten)]` of `NullableChoice` writes a union member that is not an object"
        ),
        "got: {error}"
    );
    assert!(
        error.contains("its branch 2.2 describes a `null`"),
        "got: {error}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn an_optional_scalar_union_member_is_refused_below_the_option_it_was_written_under() {
    let error = recorded_union_flatten_error(
        "NullableScalarChoice",
        syn::parse_quote! {
            enum NullableScalarChoice {
                Obj(Holder),
                Maybe(Option<String>),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: NullableScalarChoice },
    )
    .unwrap();
    assert!(
        error.contains("its branch 2.1 describes a `string`"),
        "got: {error}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_union_member_naming_a_registered_non_object_wire_is_recorded_as_that_wire() {
    seed_registered_wire("StringBrand", AliasKind::StringWire, Some("string"));
    seed_registered_wire("SwitchBrand", AliasKind::Stringified, Some("boolean"));
    seed_registered_wire("PlainEnum", AliasKind::EnumMembers, Some("string"));
    seed_registered_wire("NamedStruct", AliasKind::NoEnumMembers, None);
    seed_registered_wire("TaggedEnum", AliasKind::NoEnumMembers, None);
    seed_registered_wire("CountBrand", AliasKind::Stringified, Some("integer"));
    assert_eq!(
        recorded_member_trails(syn::parse_quote! {
            enum Choice {
                Named(NamedStruct),
                Slug(StringBrand),
                Switch(SwitchBrand),
                Hue(PlainEnum),
                Tagged(TaggedEnum),
                Count(CountBrand),
                Foreign(NeverRegistered),
            }
        }),
        vec![
            ("1".to_owned(), None),
            ("2".to_owned(), Some("string")),
            ("3".to_owned(), Some("boolean")),
            ("4".to_owned(), Some("string")),
            ("5".to_owned(), None),
            ("6".to_owned(), Some("integer")),
            ("7".to_owned(), None),
        ]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_with_a_named_string_wire_member_is_refused_naming_the_branch() {
    seed_registered_wire("Slug", AliasKind::StringWire, Some("string"));
    let error = recorded_union_flatten_error(
        "SlugChoice",
        syn::parse_quote! {
            enum SlugChoice {
                Obj(Holder),
                Slug(Slug),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: SlugChoice },
    )
    .unwrap();
    assert!(
        error.contains("its branch 2 describes a `string`"),
        "got: {error}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_with_a_named_stringified_or_enumerated_member_is_refused() {
    seed_registered_wire("Switch", AliasKind::Stringified, Some("boolean"));
    let switch = recorded_union_flatten_error(
        "SwitchChoice",
        syn::parse_quote! {
            enum SwitchChoice {
                Obj(Holder),
                Switch(Switch),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: SwitchChoice },
    )
    .unwrap();
    assert!(
        switch.contains("its branch 2 describes a `boolean`"),
        "got: {switch}"
    );

    seed_registered_wire("Hue", AliasKind::EnumMembers, Some("string"));
    let hue = recorded_union_flatten_error(
        "HueChoice",
        syn::parse_quote! {
            enum HueChoice {
                Obj(Holder),
                Hue(Hue),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: HueChoice },
    )
    .unwrap();
    assert!(
        hue.contains("its branch 2 describes a `string`"),
        "got: {hue}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_union_member_naming_an_object_or_an_unregistered_type_stays_admitted() {
    seed_registered_wire("Doc", AliasKind::NoEnumMembers, None);
    assert!(
        recorded_union_flatten_error(
            "NamedChoice",
            syn::parse_quote! {
                enum NamedChoice {
                    Obj(Holder),
                    Doc(Doc),
                    Foreign(NeverRegistered),
                }
            },
            &syn::parse_quote! { #[serde(flatten)] either: NamedChoice },
        )
        .is_none()
    );
}

/// What the flatten guard is asked about a field naming one item directly, with no union between.
#[cfg(all(feature = "serde", feature = "zod"))]
fn direct_flatten_error(field: &syn::Field) -> Option<String> {
    flatten_edge_guard_error(field, "Host").map(|error| error.to_string())
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_registered_scalar_wire_is_refused_where_the_field_was_written() {
    seed_registered_wire("Counted", AliasKind::NoEnumMembers, Some("integer"));
    let error = direct_flatten_error(&syn::parse_quote! { #[serde(flatten)] c: Counted }).unwrap();
    assert!(
        error.contains("`#[serde(flatten)]` of `Counted` is not written as an object"),
        "got: {error}"
    );
    assert!(
        error.contains("its schema describes a `integer`"),
        "got: {error}"
    );
    assert!(
        error.contains("write the field as a named member so the value gets a key of its own"),
        "got: {error}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_registered_string_boolean_or_array_wire_is_refused_by_its_own_keyword() {
    for (rust_ident, kind, keyword) in [
        ("DirectSlug", AliasKind::StringWire, "string"),
        ("DirectSwitch", AliasKind::Stringified, "boolean"),
        ("DirectPair", AliasKind::NoEnumMembers, "array"),
    ] {
        seed_registered_wire(rust_ident, kind, Some(keyword));
        let named: syn::Type = syn::parse_str(rust_ident).unwrap();
        let error =
            direct_flatten_error(&syn::parse_quote! { #[serde(flatten)] v: #named }).unwrap();
        assert!(
            error.contains(&format!("its schema describes a `{keyword}`")),
            "got: {error}"
        );
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_direct_flatten_of_an_object_or_an_unregistered_name_stays_admitted() {
    seed_registered_wire("DirectDoc", AliasKind::NoEnumMembers, None);
    for admitted in [
        syn::parse_quote! { #[serde(flatten)] base: DirectDoc },
        syn::parse_quote! { #[serde(flatten)] base: NeverRegisteredDirectly },
        syn::parse_quote! { #[serde(flatten)] counts: HashMap<String, u32> },
        syn::parse_quote! { #[serde(flatten)] counts: Option<BTreeMap<String, u32>> },
    ] {
        let field: syn::Field = admitted;
        assert!(
            direct_flatten_error(&field).is_none(),
            "got a rejection for {}",
            quote::ToTokens::to_token_stream(&field)
        );
    }
}

/// A flattened value written in place, which no name stands for: serde refuses every write of a
/// list, a tuple, text and a number flattened, and writes `()` as no key.
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_direct_flatten_of_a_value_written_in_place_is_refused_by_its_own_keyword() {
    seed_registered_wire("DirectCount", AliasKind::NoEnumMembers, Some("integer"));
    for (written, spelled, keyword) in [
        (
            syn::parse_quote! { #[serde(flatten)] counts: Vec<DirectCount> },
            "Vec<DirectCount>",
            "array",
        ),
        (
            syn::parse_quote! { #[serde(flatten)] counts: VecDeque<u32> },
            "VecDeque<u32>",
            "array",
        ),
        (
            syn::parse_quote! { #[serde(flatten)] pair: (String, u32) },
            "(String, u32)",
            "array",
        ),
        (
            syn::parse_quote! { #[serde(flatten)] count: Option<u32> },
            "Option<u32>",
            "integer",
        ),
        (
            syn::parse_quote! { #[serde(flatten)] label: String },
            "String",
            "string",
        ),
    ] {
        let field: syn::Field = written;
        let refused = format!(
            "model_schema: `Host`: `#[serde(flatten)]` of `{spelled}` is not written as an object \
             \u{2014} its schema describes a `{keyword}`, which has no members to merge, and what \
             serde writes for it does not join the object being written; write the field as a \
             named member so the value gets a key of its own. {}",
            super::FLATTENED_IN_PLACE_SCOPE
        );
        let error = direct_flatten_error(&field).unwrap_or_default();
        assert!(error.contains(&refused), "for {spelled}, got: {error}");
        if spelled == "(String, u32)" {
            let answered = format!("error: {refused}");
            assert!(
                include_str!("../../README.md").contains(&answered),
                "the README no longer quotes: {answered}"
            );
        }
    }
    let refused = format!(
        "model_schema: `Host`: `#[serde(flatten)]` of `()` is written as `()` \u{2014} its schema \
         describes a `null`: serde writes no key for it, and reads the object back, while the \
         TypeScript type and the Zod schema intersect the object with `null`, which admits no \
         value; write the field as a named member so the value gets a key of its own. {}",
        super::FLATTENED_IN_PLACE_SCOPE
    );
    let error = direct_flatten_error(&syn::parse_quote! { #[serde(flatten)] nothing: () })
        .unwrap_or_default();
    assert!(error.contains(&refused), "got: {error}");
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_direct_flatten_of_a_plain_enum_is_left_to_the_guard_written_for_it() {
    seed_registered_wire("DirectHue", AliasKind::EnumMembers, Some("string"));
    record_plain_enum("DirectHue", true);
    let field: syn::Field = syn::parse_quote! { #[serde(flatten)] tone: DirectHue };
    assert!(direct_flatten_error(&field).is_none());
    let written = super::flattened_field_guard_error(&field, "Host")
        .map(|error| error.to_string())
        .unwrap();
    assert!(
        written.contains("a plain enum writes its"),
        "got: {written}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_direct_flatten_of_a_nullable_scalar_registration_is_refused_at_its_value_branch() {
    seed_slot_registration(&syn::parse_quote! { struct DirectMaybeCount(Option<i64>); });
    let field: syn::Field = syn::parse_quote! { #[serde(flatten)] c: DirectMaybeCount };
    let error = direct_flatten_error(&field).unwrap();
    assert!(
        error.contains(
            "`#[serde(flatten)]` of `DirectMaybeCount` writes a union member that is not an object"
        ),
        "got: {error}"
    );
    assert!(
        error.contains("its branch 1 describes a `integer`"),
        "got: {error}"
    );
    assert!(
        error.contains("write the field as a named member so the value gets a key of its own"),
        "got: {error}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_nullable_registration_is_refused_by_the_keyword_its_value_side_proves() {
    seed_slot_registration(&syn::parse_quote! { struct DirectMaybeName(Option<String>); });
    seed_slot_registration(&syn::parse_quote! { struct DirectMaybeFlag(Option<bool>); });
    seed_slot_registration(&syn::parse_quote! { struct DirectMaybeList(Option<Vec<String>>); });
    for (rust_ident, keyword) in [
        ("DirectMaybeName", "string"),
        ("DirectMaybeFlag", "boolean"),
        ("DirectMaybeList", "array"),
    ] {
        let named: syn::Type = syn::parse_str(rust_ident).unwrap();
        let error =
            direct_flatten_error(&syn::parse_quote! { #[serde(flatten)] v: #named }).unwrap();
        assert!(
            error.contains(&format!("its branch 1 describes a `{keyword}`")),
            "got: {error}"
        );
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_direct_flatten_of_a_nullable_object_registration_stays_admitted() {
    seed_registered_wire("DirectPlainDoc", AliasKind::NoEnumMembers, None);
    seed_slot_registration(&syn::parse_quote! { struct DirectMaybeDoc(Option<DirectPlainDoc>); });
    let field: syn::Field = syn::parse_quote! { #[serde(flatten)] base: DirectMaybeDoc };
    assert!(
        direct_flatten_error(&field).is_none(),
        "got a rejection for {}",
        quote::ToTokens::to_token_stream(&field)
    );
}

/// Runs the registration a tuple struct's own expansion runs, so what the registry answers for the
/// name is what a declaration put there rather than a word written by hand.
#[cfg(all(feature = "serde", feature = "zod"))]
fn seed_slot_registration(item: &syn::ItemStruct) {
    let _: (String, String, syn::Ident) = super::struct_module_idents(
        &item.ident,
        None,
        super::tuple_struct_surface(
            &item.fields,
            &super::type_parameters_in_scope(&item.generics),
        ),
    );
}

/// The same for a branded newtype, whose wire is its inner's.
#[cfg(all(feature = "serde", feature = "zod"))]
fn seed_brand_registration(item: &syn::ItemStruct) {
    let rust_ident = item.ident.to_string();
    super::register_branded_newtype(
        item,
        &rust_ident,
        &rust_ident,
        &ident_schema_module_name(&rust_ident),
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn every_spelling_of_one_value_publishes_one_json_type_keyword() {
    for (inner, keyword) in [
        ("u32", "integer"),
        ("i64", "integer"),
        ("usize", "integer"),
        ("f32", "number"),
        ("f64", "number"),
        ("bool", "boolean"),
        ("String", "string"),
    ] {
        let ty: syn::Type = syn::parse_str(inner).unwrap();
        let written = super::get_field_def("v", &ty, "");
        let named = format!("\"{keyword}\"");
        let field = super::build_field_type_schema(&written, "v").to_string();
        let slot = super::scalar_field_json_schema_item(&written)
            .unwrap()
            .to_string();
        let brand = brand_json_schema_over(&ty);
        assert!(field.contains(&named), "field {inner}, got: {field}");
        assert!(slot.contains(&named), "slot {inner}, got: {slot}");
        assert!(brand.contains(&named), "brand {inner}, got: {brand}");
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_union_member_naming_a_numeric_registration_is_recorded_by_its_own_keyword() {
    seed_brand_registration(&syn::parse_quote! {
        #[serde(transparent)]
        struct WireTicks(u32);
    });
    seed_brand_registration(&syn::parse_quote! {
        #[serde(transparent)]
        struct WireRatio(f64);
    });
    seed_slot_registration(&syn::parse_quote! { struct WireSlotTicks(u32); });
    assert_eq!(
        recorded_member_trails(syn::parse_quote! {
            enum WireNumericChoice {
                Obj(Holder),
                Ticks(WireTicks),
                Ratio(WireRatio),
                Slot(WireSlotTicks),
            }
        }),
        vec![
            ("1".to_owned(), None),
            ("2".to_owned(), Some("integer")),
            ("3".to_owned(), Some("number")),
            ("4".to_owned(), Some("integer")),
        ]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_with_a_named_integer_wire_member_is_refused_naming_the_keyword() {
    seed_brand_registration(&syn::parse_quote! {
        #[serde(transparent)]
        struct FlatWireTicks(u32);
    });
    let error = recorded_union_flatten_error(
        "WireTickChoice",
        syn::parse_quote! {
            enum WireTickChoice {
                Obj(Holder),
                Ticks(FlatWireTicks),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: WireTickChoice },
    )
    .unwrap();
    assert!(
        error.contains("its branch 2 describes a `integer`"),
        "got: {error}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn an_array_shaped_registration_is_recorded_apart_from_a_map_shaped_one() {
    seed_slot_registration(&syn::parse_quote! { struct WireBag(Vec<String>); });
    seed_slot_registration(
        &syn::parse_quote! { struct WireBucket(std::collections::HashMap<String, String>); },
    );
    seed_slot_registration(&syn::parse_quote! { struct WirePair(String, u32); });
    assert_eq!(
        recorded_member_trails(syn::parse_quote! {
            enum WireContainerChoice {
                Obj(Holder),
                Bag(WireBag),
                Bucket(WireBucket),
                Pair(WirePair),
            }
        }),
        vec![
            ("1".to_owned(), None),
            ("2".to_owned(), Some("array")),
            ("3".to_owned(), None),
            ("4".to_owned(), Some("array")),
        ]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_with_a_named_array_wire_member_is_refused_while_a_map_stays_admitted() {
    seed_slot_registration(&syn::parse_quote! { struct FlatWireBag(Vec<String>); });
    seed_slot_registration(
        &syn::parse_quote! { struct FlatWireBucket(std::collections::HashMap<String, String>); },
    );
    let bag = recorded_union_flatten_error(
        "WireBagChoice",
        syn::parse_quote! {
            enum WireBagChoice {
                Obj(Holder),
                Bag(FlatWireBag),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: WireBagChoice },
    )
    .unwrap();
    assert!(
        bag.contains("its branch 2 describes a `array`"),
        "got: {bag}"
    );
    assert!(
        recorded_union_flatten_error(
            "WireBucketChoice",
            syn::parse_quote! {
                enum WireBucketChoice {
                    Obj(Holder),
                    Bucket(FlatWireBucket),
                }
            },
            &syn::parse_quote! { #[serde(flatten)] either: WireBucketChoice },
        )
        .is_none()
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_union_member_naming_a_nullable_registration_carries_its_null_leaf() {
    seed_slot_registration(&syn::parse_quote! { struct WireMaybeDoc(Option<Holder>); });
    seed_slot_registration(&syn::parse_quote! { struct WireMaybeCount(Option<u32>); });
    assert_eq!(
        recorded_member_trails(syn::parse_quote! {
            enum WireNullableChoice {
                Obj(Holder),
                Maybe(WireMaybeDoc),
                Count(WireMaybeCount),
            }
        }),
        vec![
            ("1".to_owned(), None),
            ("2.1".to_owned(), None),
            ("2.2".to_owned(), Some("null")),
            ("3.1".to_owned(), Some("integer")),
            ("3.2".to_owned(), Some("null")),
        ]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_with_a_named_nullable_member_is_refused_naming_the_trail() {
    seed_slot_registration(&syn::parse_quote! { struct FlatWireMaybeDoc(Option<Holder>); });
    let named = recorded_union_flatten_error(
        "WireNamedNullableChoice",
        syn::parse_quote! {
            enum WireNamedNullableChoice {
                Obj(Holder),
                Maybe(FlatWireMaybeDoc),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: WireNamedNullableChoice },
    )
    .unwrap();
    let written = recorded_union_flatten_error(
        "WireWrittenNullableChoice",
        syn::parse_quote! {
            enum WireWrittenNullableChoice {
                Obj(Holder),
                Maybe(Option<Holder>),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: WireWrittenNullableChoice },
    )
    .unwrap();
    assert!(
        named.contains("its branch 2.2 describes a `null`"),
        "got: {named}"
    );
    assert_eq!(
        named.replace("WireNamedNullableChoice", "CHOICE"),
        written.replace("WireWrittenNullableChoice", "CHOICE")
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_union_member_naming_a_tagged_enum_carries_one_leaf_per_variant() {
    seed_external_registration(&syn::parse_quote! {
        enum WireExtBare {
            Bare,
            Wrapped(Holder),
        }
    });
    assert_eq!(
        recorded_member_trails(syn::parse_quote! {
            enum WireExtBareChoice {
                Obj(Holder),
                Ext(WireExtBare),
            }
        }),
        vec![
            ("1".to_owned(), None),
            ("2.1".to_owned(), Some("string")),
            ("2.2".to_owned(), None),
        ]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_union_member_naming_an_all_object_tagged_enum_keeps_its_one_leaf() {
    seed_external_registration(&syn::parse_quote! {
        enum WireExtObjects {
            One(Holder),
            Two(Other),
        }
    });
    assert_eq!(
        recorded_member_trails(syn::parse_quote! {
            enum WireExtObjChoice {
                Obj(Holder),
                Ext(WireExtObjects),
            }
        }),
        vec![("1".to_owned(), None), ("2".to_owned(), None)]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn flattening_a_union_with_a_tagged_enum_member_is_refused_naming_the_trail() {
    seed_external_registration(&syn::parse_quote! {
        enum FlatWireExtBare {
            Bare,
            Wrapped(Holder),
        }
    });
    let refusal = recorded_union_flatten_error(
        "WireExtFlatChoice",
        syn::parse_quote! {
            enum WireExtFlatChoice {
                Obj(Holder),
                Ext(FlatWireExtBare),
            }
        },
        &syn::parse_quote! { #[serde(flatten)] either: WireExtFlatChoice },
    )
    .unwrap();
    assert!(
        refusal.contains(
            "`#[serde(flatten)]` of `WireExtFlatChoice` writes a union member as the name of an \
             externally tagged enum's unit variant \u{2014} its branch 2.1 describes a `string`, \
             and serde writes that name as a key holding null"
        ),
        "got: {refusal}"
    );
    seed_external_registration(&syn::parse_quote! {
        enum FlatWireExtObjects {
            One(Holder),
            Two(Other),
        }
    });
    assert!(
        recorded_union_flatten_error(
            "WireExtObjFlatChoice",
            syn::parse_quote! {
                enum WireExtObjFlatChoice {
                    Obj(Holder),
                    Ext(FlatWireExtObjects),
                }
            },
            &syn::parse_quote! { #[serde(flatten)] either: WireExtObjFlatChoice },
        )
        .is_none()
    );
}

/// Runs the registration an externally tagged enum's own expansion runs, so the leaves the registry
/// answers with for the name are ones a declaration put there rather than words written by hand.
#[cfg(all(feature = "serde", feature = "zod"))]
fn seed_external_registration(item: &syn::ItemEnum) {
    let rust_ident = item.ident.to_string();
    let _: (String, syn::Ident) = super::enum_module_idents(
        &item.ident,
        &rust_ident,
        AliasKind::NoEnumMembers,
        super::Surface::externally_tagged(&item.variants),
    );
}

#[cfg(feature = "serde")]
#[test]
fn the_empty_string_pattern_keeps_the_call_it_was_already_emitted_as() {
    assert_eq!(
        emitted_pattern_validator("^$"),
        "pub fn validate_field_value (value : & str) -> :: core :: result :: Result < () , :: std :: \
         vec :: Vec < :: std :: string :: String >> { let mut errors : :: std :: vec :: Vec < :: std :: \
         string :: String > = :: std :: vec :: Vec :: new () ; if ! value . is_empty () { errors . \
         push (format ! (\"'{}': {}\" , \"field\" , \"does not match pattern '^$'\")) ; } if \
         errors . is_empty () { :: core :: result :: Result :: Ok (()) } else { :: core :: result :: \
         Result :: Err (errors) } } "
    );
}

/// A boundary with something beside it is the rewrite the lone-assertion refusal names.
#[cfg(feature = "serde")]
#[test]
fn a_word_boundary_pattern_keeps_its_regex() {
    assert_eq!(
        emitted_pattern_validator(r"\b[0-9A-Za-z_]+"),
        "pub fn validate_field_value (value : & str) -> :: core :: result :: Result < () , :: std :: \
         vec :: Vec < :: std :: string :: String >> { let mut errors : :: std :: vec :: Vec < :: std :: \
         string :: String > = :: std :: vec :: Vec :: new () ; { use :: std :: sync :: LazyLock ; \
         static RE : LazyLock < regex :: Regex > = LazyLock :: new (|| { regex :: Regex :: new \
         (\"\\\\b[0-9A-Za-z_]+\") . unwrap () }) ; if ! RE . is_match (value) { errors . push \
         (format ! (\"'{}': {}\" , \"field\" , \"does not match pattern \
         '\\\\b[0-9A-Za-z_]+'\")) ; } } if errors . is_empty () { :: core :: result :: Result :: \
         Ok (()) } else { :: core :: result :: Result :: Err (errors) } } "
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_flattened_type_parameter_is_merged_at_the_document_its_filling_binds() {
    let ty: syn::Type = syn::parse_str("HeldType").unwrap();
    let mut held = super::get_field_def("held", &ty, "");
    held.erase_type_parameters(&["HeldType".to_owned()]);

    let source = super::flatten_merged_source(&held);

    assert_eq!(source.label, "held");
    assert_eq!(
        source.value.to_string(),
        "_arg_held_type . clone ()",
        "the placeholder still stands where the filling belongs"
    );
}

#[test]
fn an_identifier_member_key_is_written_bare() {
    for key in ["id", "userId", "_private", "$ref", "a1", "A", "_", "$"] {
        assert_eq!(
            super::ts_member_key(key),
            key,
            "expected `{key}` written bare"
        );
    }
}

#[test]
fn a_non_identifier_member_key_is_written_as_a_string() {
    assert_eq!(super::ts_member_key("reply-to"), "\"reply-to\"");
    assert_eq!(super::ts_member_key("content-type"), "\"content-type\"");
    assert_eq!(super::ts_member_key("2fa"), "\"2fa\"");
    assert_eq!(super::ts_member_key(""), "\"\"");
    assert_eq!(super::ts_member_key("a b"), "\"a b\"");
    assert_eq!(super::ts_member_key("caf\u{e9}"), "\"caf\u{e9}\"");
}

#[test]
fn a_member_key_carrying_a_quote_or_a_backslash_is_escaped() {
    assert_eq!(super::ts_member_key("a\"b"), "\"a\\\"b\"");
    assert_eq!(super::ts_member_key("a\\b"), "\"a\\\\b\"");
}

#[cfg(all(feature = "serde", feature = "typescript"))]
#[test]
fn an_untagged_sibling_exclusion_writes_a_non_identifier_key_as_a_string() {
    let closed = super::close_untagged_flatten_member(
        "{ subject: string }",
        &["reply-to".to_owned(), "sent_at".to_owned()],
    );

    assert_eq!(
        closed,
        "{ subject: string; \"reply-to\"?: never; sent_at?: never }"
    );
}

/// Collects a discriminated enum's guard failures as rendered `compile_error!` token strings.
#[cfg(feature = "serde")]
fn discriminated_guard_errors(mut item: syn::ItemEnum) -> Vec<String> {
    collect_discriminated_variants(
        &mut item,
        UNCASED,
        Some("probe_schema"),
        &super::ModelSchemaArgs::default(),
    )
    .2
    .iter()
    .map(ToString::to_string)
    .collect()
}

/// The field defs one variant of a discriminated enum collected, beside the ones it flattens.
#[cfg(feature = "serde")]
fn collected_variant_fields(mut item: syn::ItemEnum) -> (Vec<String>, Vec<String>) {
    let variants = collect_discriminated_variants(
        &mut item,
        UNCASED,
        Some("probe_schema"),
        &super::ModelSchemaArgs::default(),
    )
    .0;
    let variant = variants.first().unwrap();
    (
        variant.field_defs.iter().map(|f| f.name.clone()).collect(),
        variant
            .flattened_fields
            .iter()
            .map(|f| f.name.clone())
            .collect(),
    )
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_variant_field_is_split_out_of_the_variants_members() {
    let (written, flattened) = collected_variant_fields(syn::parse_quote! {
        enum Probe {
            Named {
                #[serde(flatten)]
                extra: PlainBase,
                x: String,
            },
        }
    });
    assert_eq!(written, vec!["x".to_owned()]);
    assert_eq!(flattened, vec!["extra".to_owned()]);
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_variant_field_over_an_unrecorded_name_is_not_refused() {
    assert_eq!(
        discriminated_guard_errors(syn::parse_quote! {
            enum Probe {
                Named {
                    #[serde(flatten)]
                    extra: NeverRecordedBase,
                    x: String,
                },
            }
        }),
        Vec::<String>::new()
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_variant_field_over_a_plain_enum_is_refused() {
    record_plain_enum("VariantHue", true);
    let errors = discriminated_guard_errors(syn::parse_quote! {
        enum Probe {
            Named {
                #[serde(flatten)]
                tone: VariantHue,
                x: String,
            },
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(
        errors[0].contains("carries `#[serde(flatten)]` over `VariantHue`"),
        "got: {}",
        errors[0]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_flattened_variant_field_over_a_multi_branch_source_is_accepted() {
    let mut choice: syn::ItemEnum = syn::parse_quote! {
        enum VariantChoice {
            First(Holder),
            Second(Other),
        }
    };
    let (_, _, _, merge_parts, _, errors, _, _, _, _) = collect_untagged_members(
        &mut choice,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    register_alias_info(
        "VariantChoice",
        "VariantChoice",
        &ident_schema_module_name("VariantChoice"),
        AliasKind::NoEnumMembers,
    );
    record_zod_union_members("VariantChoice", &merge_parts);

    let refusals = discriminated_guard_errors(syn::parse_quote! {
        enum Probe {
            Named {
                #[serde(flatten)]
                either: VariantChoice,
                x: String,
            },
        }
    });
    assert!(refusals.is_empty(), "got: {refusals:?}");
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn an_optional_flattened_variant_field_is_accepted() {
    let refusals = discriminated_guard_errors(syn::parse_quote! {
        enum Probe {
            Named {
                #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
                extra: Option<PlainBase>,
                x: String,
            },
        }
    });
    assert!(refusals.is_empty(), "got: {refusals:?}");
}

/// What the walk of the members of `variant` collects, the variant of an untagged enum named
/// `Probe` with no casing, no parameter and no default.
#[cfg(feature = "serde")]
fn walked_untagged_variant(variant: &mut syn::Variant) -> super::UntaggedVariantMembers {
    super::collect_untagged_variant_members(
        variant,
        &super::UntaggedEnum {
            args: &super::ModelSchemaArgs::default(),
            name: "Probe",
            rename_all_fields: None,
            schema_module_name: UNTAGGED_MODULE,
            serde_default: &|_, _| None,
            type_parameters: &[],
            wire: super::ContainerWire::OneWay,
        },
        &[],
    )
}

/// The members one untagged variant collected, beside the ones it flattens.
#[cfg(feature = "serde")]
fn collected_untagged_variant_fields(mut item: syn::ItemEnum) -> (Vec<String>, Vec<String>) {
    let variant = item.variants.first_mut().unwrap();
    let walked = walked_untagged_variant(variant);
    (
        walked.field_defs.iter().map(|f| f.name.clone()).collect(),
        walked
            .flattened_fields
            .iter()
            .map(|f| f.name.clone())
            .collect(),
    )
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_untagged_variant_field_is_split_out_of_the_variants_members() {
    let (written, flattened) = collected_untagged_variant_fields(syn::parse_quote! {
        enum Probe {
            Named {
                #[serde(flatten)]
                extra: PlainBase,
                x: String,
            },
        }
    });
    assert_eq!(written, vec!["x".to_owned()]);
    assert_eq!(flattened, vec!["extra".to_owned()]);
}

#[cfg(all(feature = "serde", feature = "typescript"))]
#[test]
fn a_flattening_untagged_member_proves_no_key_list() {
    let mut item: syn::ItemEnum = syn::parse_quote! {
        enum Probe {
            Named {
                #[serde(flatten)]
                extra: PlainBase,
                x: String,
            },
        }
    };
    let variant = item.variants.first_mut().unwrap();
    let walked = walked_untagged_variant(variant);
    assert_eq!(
        super::untagged_member_keys(
            &VariantKind::Named,
            &walked.field_defs,
            &walked.flattened_fields
        ),
        None
    );
    assert_eq!(
        super::untagged_member_keys(&VariantKind::Named, &walked.field_defs, &[]),
        Some(vec!["x".to_owned()])
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_untagged_variant_field_over_a_plain_enum_is_refused() {
    record_plain_enum("UntaggedVariantHue", true);
    let errors = untagged_guard_errors(syn::parse_quote! {
        enum Probe {
            Named {
                #[serde(flatten)]
                tone: UntaggedVariantHue,
                x: String,
            },
        }
    });
    assert_eq!(errors.len(), 1, "got: {errors:?}");
    assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
    assert!(
        errors[0].contains("carries `#[serde(flatten)]` over `UntaggedVariantHue`"),
        "got: {}",
        errors[0]
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_flattened_untagged_variant_field_over_a_multi_branch_source_is_accepted() {
    let mut choice: syn::ItemEnum = syn::parse_quote! {
        enum UntaggedVariantChoice {
            First(Holder),
            Second(Other),
        }
    };
    let (_, _, _, merge_parts, _, errors, _, _, _, _) = collect_untagged_members(
        &mut choice,
        UNTAGGED_MODULE,
        &super::ModelSchemaArgs::default(),
    );
    assert!(errors.is_empty(), "got: {errors:?}");
    register_alias_info(
        "UntaggedVariantChoice",
        "UntaggedVariantChoice",
        &ident_schema_module_name("UntaggedVariantChoice"),
        AliasKind::NoEnumMembers,
    );
    record_zod_union_members("UntaggedVariantChoice", &merge_parts);

    let refusals = untagged_guard_errors(syn::parse_quote! {
        enum Probe {
            Named {
                #[serde(flatten)]
                either: UntaggedVariantChoice,
                x: String,
            },
        }
    });
    assert!(refusals.is_empty(), "got: {refusals:?}");
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_untagged_variant_field_over_a_single_key_set_source_is_not_refused() {
    assert_eq!(
        untagged_guard_errors(syn::parse_quote! {
            enum Probe {
                Named {
                    #[serde(flatten)]
                    extra: PlainBase,
                    x: String,
                },
            }
        }),
        Vec::<String>::new()
    );
}

/// Runs the whole attribute expansion over `source`, the way the `#[proc_macro_attribute]` does
/// either side of its `proc_macro` conversion.
fn expansion_over(source: &str) -> proc_macro2::TokenStream {
    super::exec_model_schema(
        proc_macro2::TokenStream::new(),
        syn::parse_str(source).unwrap(),
    )
}

/// [`expansion_over`], carrying the `#[model_schema(...)]` arguments a bare source cannot write.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn expansion_with_args_over(args: &str, source: &str) -> proc_macro2::TokenStream {
    super::exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    )
}

/// A brand over a string or a path publishes the text a field's bound is measured on.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_brand_publishes_its_text_only_over_a_string_or_a_path() {
    for (args, source, text) in [
        (
            "",
            "#[serde(transparent)] pub struct TextSlug(pub String);",
            Some(":: std :: borrow :: Cow :: Borrowed (& * self . 0)"),
        ),
        (
            "",
            "#[serde(transparent)] pub struct TextShared(pub Arc<str>);",
            Some(":: std :: borrow :: Cow :: Borrowed (& * self . 0)"),
        ),
        (
            "",
            "#[serde(transparent)] pub struct TextNamed { pub text: String }",
            Some(":: std :: borrow :: Cow :: Borrowed (& * self . text)"),
        ),
        (
            "no_display",
            "#[serde(transparent)] pub struct TextDir(pub PathBuf);",
            Some("self . 0 . to_string_lossy ()"),
        ),
        (
            "",
            "#[serde(transparent)] pub struct TextCount(pub u32);",
            None,
        ),
        (
            "no_display",
            "#[serde(transparent)] pub struct TextMany(pub Vec<String>);",
            None,
        ),
        (
            "default_types(IdType = String)",
            "#[serde(transparent)] pub struct TextId<IdType>(pub IdType);",
            None,
        ),
    ] {
        let tokens = expansion_with_args_over(args, source).to_string();
        assert!(!tokens.contains("compile_error"), "for {source}: {tokens}");
        let published =
            "pub fn model_schema_bound_text (& self) -> :: std :: borrow :: Cow < '_ , str >";
        match text {
            Some(written) => assert!(
                tokens.contains(&format!("{published} {{ {written} }}")),
                "for {source}: {tokens}"
            ),
            None => assert!(!tokens.contains("model_schema_bound_text"), "for {source}"),
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_field_under_a_structs_own_tag_key_is_refused() {
    for source in [
        "#[serde(tag = \"kind\")] pub struct TagNamed { pub kind: String, pub name: String }",
        "#[serde(tag = \"kind\")] pub struct TagRenamed { #[serde(rename = \"kind\")] \
         pub other: String }",
    ] {
        let tokens = expansion_over(source).to_string();
        assert!(
            tokens.contains("writes the struct's name under `kind`"),
            "for {source}, got: {tokens}"
        );
    }
    let accepted =
        expansion_over("#[serde(tag = \"kind\")] pub struct TagAlone { pub name: String }")
            .to_string();
    assert!(!accepted.contains("compile_error"), "got: {accepted}");
}

#[test]
fn an_item_with_no_shape_is_refused_on_the_item() {
    for (source, first_token) in [
        ("pub fn unsupported_shape() {}", "pub"),
        ("impl Unsupported {}", "impl"),
        ("pub trait Unsupported {}", "pub"),
        ("pub mod unsupported {}", "pub"),
        ("pub const UNSUPPORTED: u8 = 0;", "pub"),
        ("pub static UNSUPPORTED: u8 = 0;", "pub"),
        ("pub union Unsupported { pub a: u8 }", "pub"),
    ] {
        let tokens = expansion_over(source);
        assert!(
            tokens
                .to_string()
                .contains("model_schema: unsupported target for this attribute"),
            "for {source}, got: {tokens}"
        );
        let located = located_source_texts(&tokens);
        assert_eq!(
            located.first().map(String::as_str),
            Some(first_token),
            "for {source}, got: {located:?}"
        );
    }
}

#[test]
fn the_shapes_the_macro_expands_are_not_refused() {
    for source in [
        "pub struct DispatchedShape { pub name: String }",
        "pub enum DispatchedChoice { One, Two }",
        "pub type DispatchedAlias = String;",
    ] {
        let tokens = expansion_over(source);
        assert!(
            !tokens.to_string().contains("unsupported target"),
            "for {source}, got: {tokens}"
        );
    }
}

/// `Foo()` is refused on the variant under the default, the adjacent and the untagged tagging
/// alike.
#[test]
fn an_empty_tuple_variant_is_refused_on_the_variant() {
    for source in [
        "pub enum Choice { Unit, Empty(), Data(i32) }",
        "#[serde(tag = \"kind\", content = \"body\")] pub enum Choice { Unit, Empty(), Data(i32) }",
        "#[serde(untagged)] pub enum Choice { Empty(), Data(i32) }",
    ] {
        let expanded = expansion_over(source).to_string();
        assert!(
            expanded.contains(
                "model_schema: variant `Empty`: `Empty()` is a tuple variant with no field, which \
                 serde writes with an empty `[]` payload, while every surface describes it as the \
                 unit variant `Empty`, so a value of it never crosses. Write `Empty` for a unit \
                 variant, or give it a field."
            ),
            "for {source}, got: {expanded}"
        );
        let refusals = super::empty_tuple_variant_errors(&syn::parse_str(source).unwrap());
        assert_eq!(refusals.len(), 1, "for {source}, got: {refusals:?}");
        let located = located_source_texts(&refusals[0]);
        assert!(
            !located.is_empty()
                && located
                    .iter()
                    .all(|text| text.starts_with("Empty") || text == "()"),
            "for {source}, got: {located:?}"
        );
    }
}

/// A map whose value is `()` builds and describes its values as `null`: serde writes
/// `{"keyed":{"a":null}}` and refuses `[]` as a value, and Zod under node reads the first and
/// refuses the second.
#[cfg(all(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_map_of_units_describes_its_values_as_null() {
    let expanded = expansion_over(
        "#[derive(Serialize, Deserialize)] pub struct ProbeUnitsKeyed { pub keyed: \
         std::collections::BTreeMap<String, ()> }",
    );
    let written = expanded.to_string();
    assert!(!written.contains("compile_error"), "got: {written}");
    let texts = literal_texts(&expanded).join("\n");
    for rendered in [
        "keyed: Partial<Record<string, null>>;",
        "keyed: z.record(z.string(), z.null()),",
    ] {
        assert!(texts.contains(rendered), "no `{rendered}` in: {texts}");
    }
    assert!(
        written.contains(
            r#""keyed" . to_string () , serde_json :: json ! ({ "type" : "object" , "additionalProperties" : serde_json :: json ! ({ "type" : "null" }) }))"#
        ),
        "got: {written}"
    );
}

/// A unit variant, a one-slot and a many-slot tuple variant are not refused.
#[test]
fn a_unit_variant_and_a_tuple_variant_with_fields_are_not_refused() {
    let refusals = super::empty_tuple_variant_errors(
        &syn::parse_str("pub enum Choice { Unit, One(i32), Two(i32, String) }").unwrap(),
    );
    assert!(refusals.is_empty(), "got: {refusals:?}");
    assert!(
        !expansion_over("pub enum Choice { Unit, One(i32) }")
            .to_string()
            .contains("tuple variant with no field")
    );
}

/// How many `compile_error!` invocations `tokens` writes, counted off the token trees so a name a
/// message quotes in its own text is not mistaken for a second diagnostic.
#[cfg(feature = "serde")]
fn compile_error_count(tokens: &proc_macro2::TokenStream) -> usize {
    let mut count = 0;
    for tree in tokens.clone() {
        match &tree {
            proc_macro2::TokenTree::Group(group) => count += compile_error_count(&group.stream()),
            proc_macro2::TokenTree::Ident(ident) => {
                if ident == "compile_error" {
                    count += 1;
                }
            }
            proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {}
        }
    }
    count
}

/// The diagnostics `tokens` carries, told from the attribute values and refusal stubs that are
/// string literals too by the prefix every diagnostic this crate writes opens with.
#[cfg(feature = "serde")]
fn refusal_texts(tokens: &proc_macro2::TokenStream) -> Vec<String> {
    literal_texts(tokens)
        .into_iter()
        .filter(|text| text.starts_with("model_schema: "))
        .collect()
}

/// The all-unit enum of the untagged case, written under `attributes`.
#[cfg(feature = "serde")]
fn all_unit_source(attributes: &str) -> String {
    format!("{attributes} pub enum BalanceError {{ DbError, InsufficientBalance }}")
}

#[cfg(feature = "serde")]
#[test]
fn an_all_unit_untagged_enum_earns_the_unit_variant_refusal_on_every_variant() {
    let tokens = expansion_over(&all_unit_source(
        "#[serde(rename_all = \"kebab-case\", untagged)]",
    ));
    assert_eq!(compile_error_count(&tokens), 2, "got: {tokens}");
    let refusals = refusal_texts(&tokens);
    assert_eq!(refusals.len(), 2, "got: {refusals:?}");
    for (text, variant) in refusals.iter().zip(["DbError", "InsufficientBalance"]) {
        assert!(
            text.contains(&format!("variant `{variant}`")),
            "got: {text}"
        );
        assert!(text.contains("is a unit variant"), "got: {text}");
        assert!(
            text.contains("supports newtype (`V(T)`) and struct"),
            "got: {text}"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_refused_all_unit_untagged_enum_publishes_no_string_union() {
    let tokens = expansion_over(&all_unit_source(
        "#[serde(rename_all = \"kebab-case\", untagged)]",
    ))
    .to_string();
    assert!(
        tokens.contains("refused by `#[model_schema()]`"),
        "got: {tokens}"
    );
    for absent in ["db-error", "insufficient-balance", "z.enum"] {
        assert!(!tokens.contains(absent), "{absent} present: {tokens}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn the_same_all_unit_enum_without_untagged_is_not_refused() {
    let tokens = expansion_over(&all_unit_source("#[serde(rename_all = \"kebab-case\")]"));
    assert_eq!(compile_error_count(&tokens), 0, "got: {tokens}");
    assert!(refusal_texts(&tokens).is_empty(), "got: {tokens}");
}

#[cfg(feature = "serde")]
#[test]
fn the_same_untagged_attribute_over_carrying_variants_is_not_refused() {
    let tokens = expansion_over(
        "#[serde(rename_all = \"kebab-case\", untagged)] pub enum BalanceError { DbError(String), \
         InsufficientBalance { shortfall: u32 } }",
    );
    assert_eq!(compile_error_count(&tokens), 0, "got: {tokens}");
}

/// The text of every string literal in `tokens`, which is where a `compile_error!` carries its
/// message — read back unescaped, as against the escaped literal `to_string` renders.
fn literal_texts(tokens: &proc_macro2::TokenStream) -> Vec<String> {
    let mut texts = Vec::new();
    for tree in tokens.clone() {
        match &tree {
            proc_macro2::TokenTree::Group(group) => {
                texts.extend(literal_texts(&group.stream()));
            }
            proc_macro2::TokenTree::Literal(literal) => {
                if let syn::Lit::Str(text) = syn::Lit::new(literal.clone()) {
                    texts.push(text.value());
                }
            }
            proc_macro2::TokenTree::Ident(_) | proc_macro2::TokenTree::Punct(_) => {}
        }
    }
    texts
}

/// Whether `tokens` writes `#[model_schema_prop ...]` as an attribute. Read off the token trees
/// rather than the rendered string, which carries the name inside the refusal's own message text.
fn writes_a_prop_attribute(tokens: &proc_macro2::TokenStream) -> bool {
    let mut hashed = false;
    for tree in tokens.clone() {
        match &tree {
            proc_macro2::TokenTree::Group(group) => {
                let named = group
                    .stream()
                    .into_iter()
                    .next()
                    .is_some_and(|first| matches!(&first, proc_macro2::TokenTree::Ident(ident) if ident == "model_schema_prop"));
                if (hashed && named) || writes_a_prop_attribute(&group.stream()) {
                    return true;
                }
                hashed = false;
            }
            proc_macro2::TokenTree::Punct(punct) => hashed = punct.as_char() == '#',
            proc_macro2::TokenTree::Ident(_) | proc_macro2::TokenTree::Literal(_) => hashed = false,
        }
    }
    false
}

/// A brand's slot carrying `keys`, as the declaration a consumer writes it.
fn branded_slot_source(keys: &str) -> String {
    format!("#[serde(transparent)] pub struct Branded(#[model_schema_prop({keys})] pub String);")
}

#[test]
fn a_prop_on_a_branded_newtypes_slot_is_refused() {
    for keys in SLOT_PROP_KEYS {
        let tokens = expansion_over(&branded_slot_source(keys));
        assert!(
            tokens.to_string().contains("compile_error"),
            "for `{keys}`, got: {tokens}"
        );
        assert!(
            literal_texts(&tokens).contains(&format!(
                "model_schema: type `Branded`: {BRANDED_SLOT_PROP_REFUSAL}"
            )),
            "for `{keys}`, got: {tokens}"
        );
    }
}

#[test]
fn an_unknown_key_on_a_brand_slot_earns_the_position_refusal() {
    let tokens = expansion_over(&branded_slot_source("bogus_key = 3"));
    assert!(!tokens.to_string().contains("unknown"), "got: {tokens}");
}

#[test]
fn a_refused_brand_slot_keeps_no_prop_attribute_on_the_emitted_item() {
    for keys in SLOT_PROP_KEYS {
        let tokens = expansion_over(&branded_slot_source(keys));
        assert!(
            !writes_a_prop_attribute(&tokens),
            "for `{keys}`, got: {tokens}"
        );
    }
}

#[test]
fn a_prop_on_a_brand_field_serde_does_not_read_is_refused_once() {
    let tokens = expansion_over(UNREAD_BRAND_FIELD_SOURCE);
    let refusals = literal_texts(&tokens)
        .into_iter()
        .filter(|text| text.starts_with("model_schema: "))
        .collect::<Vec<_>>();
    assert_eq!(
        refusals,
        vec![format!(
            "model_schema: type `Label`: {UNREAD_BRAND_FIELD_PROP_REFUSAL}"
        )],
        "got: {tokens}"
    );
    assert!(!writes_a_prop_attribute(&tokens), "got: {tokens}");
}

#[test]
fn a_named_brand_with_no_prop_on_any_field_earns_no_refusal() {
    let source = UNREAD_BRAND_FIELD_SOURCE.replace("#[model_schema_prop(maxLength = 9)] ", "");
    assert!(branded_slot_refusals(&source).is_empty());
    let tokens = expansion_over(&source);
    assert!(
        !tokens.to_string().contains("compile_error"),
        "got: {tokens}"
    );
}

#[test]
fn each_prop_attribute_on_a_brand_slot_earns_its_own_refusal() {
    let refusals = branded_slot_refusals(
        "#[serde(transparent)] pub struct Branded(\
         #[model_schema_prop(minLength = 2)] #[model_schema_prop(maxLength = 8)] pub String);",
    );
    assert_eq!(refusals.len(), 2, "got: {refusals:?}");
}

/// The `compile_error!` tokens `source` earns for a `#[model_schema_prop]` on a brand's slot.
/// Parsed from text so the tokens carry file locations and the refusal's span can be read back as
/// the source it points at.
fn branded_slot_refusals(source: &str) -> Vec<proc_macro2::TokenStream> {
    super::branded_slot_prop_errors(&syn::parse_str(source).unwrap())
}

#[test]
fn a_brand_slot_refusal_is_spanned_on_the_attribute() {
    let refusals = branded_slot_refusals(&branded_slot_source("pattern = \"^[a-z]+$\""));
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    let located = located_source_texts(&refusals[0]);
    assert_eq!(located.first().map(String::as_str), Some("#"));
    assert_eq!(
        located.last().map(String::as_str),
        Some("[model_schema_prop(pattern = \"^[a-z]+$\")]")
    );
}

#[test]
fn a_prop_on_a_named_brand_field_earns_the_refusal() {
    let refusals = branded_slot_refusals(
        "#[serde(transparent)] pub struct Named { #[model_schema_prop(pattern = \"^a$\")] \
         pub inner: String, #[serde(skip)] #[model_schema_prop(minLength = 2)] pub note: String }",
    );
    assert_eq!(refusals.len(), 2, "got: {refusals:?}");
    for (refusal, (message, attribute)) in refusals.iter().zip([
        (
            BRANDED_SLOT_PROP_REFUSAL,
            "[model_schema_prop(pattern = \"^a$\")]",
        ),
        (
            UNREAD_BRAND_FIELD_PROP_REFUSAL,
            "[model_schema_prop(minLength = 2)]",
        ),
    ]) {
        assert!(
            literal_texts(refusal).contains(&format!("model_schema: type `Named`: {message}")),
            "got: {refusal}"
        );
        assert_eq!(
            located_source_texts(refusal).last().map(String::as_str),
            Some(attribute)
        );
    }
}

#[test]
fn a_prop_on_a_tuple_brand_earns_the_refusal_of_the_slot_it_is_written_on() {
    let (read, marker) = (
        (
            BRANDED_SLOT_PROP_REFUSAL,
            "[model_schema_prop(pattern = \"^a$\")]",
        ),
        (
            UNREAD_BRAND_FIELD_PROP_REFUSAL,
            "[model_schema_prop(minLength = 2)]",
        ),
    );
    for (source, expected) in [
        (
            "#[serde(transparent)] pub struct Marked(#[model_schema_prop(pattern = \"^a$\")] \
             pub String, #[serde(skip)] #[model_schema_prop(minLength = 2)] PhantomData<u8>);",
            [read, marker],
        ),
        (
            "#[serde(transparent)] pub struct Marked(#[serde(skip)] \
             #[model_schema_prop(minLength = 2)] PhantomData<u8>, \
             #[model_schema_prop(pattern = \"^a$\")] pub String);",
            [marker, read],
        ),
    ] {
        let refusals = branded_slot_refusals(source);
        assert_eq!(refusals.len(), 2, "for {source}, got: {refusals:?}");
        for (refusal, (message, attribute)) in refusals.iter().zip(expected) {
            assert!(
                literal_texts(refusal).contains(&format!("model_schema: type `Marked`: {message}")),
                "for {source}, got: {refusal}"
            );
            assert_eq!(
                located_source_texts(refusal).last().map(String::as_str),
                Some(attribute),
                "for {source}"
            );
        }
    }
}

#[test]
fn a_prop_on_a_tuple_brands_marker_is_refused_once() {
    let tokens = expansion_over(
        "#[derive(Debug, Deserialize, Serialize)] #[serde(transparent)] pub struct Label(\
         #[model_schema_prop(maxLength = 9)] #[serde(skip)] PhantomData<u8>, pub String);",
    );
    let refusals = literal_texts(&tokens)
        .into_iter()
        .filter(|text| text.starts_with("model_schema: "))
        .collect::<Vec<_>>();
    assert_eq!(
        refusals,
        vec![format!(
            "model_schema: type `Label`: {UNREAD_BRAND_FIELD_PROP_REFUSAL}"
        )],
        "got: {tokens}"
    );
    assert!(!writes_a_prop_attribute(&tokens), "got: {tokens}");
}

#[test]
fn a_prop_outside_a_brand_slot_earns_no_refusal() {
    for source in [
        "pub struct Plain(#[model_schema_prop(preprocess = [\"trim\"])] pub String);",
        "pub struct Plain(#[model_schema_prop(literal = \"fixed\")] pub String);",
        "#[serde(transparent)] pub struct Wide(\
         #[model_schema_prop(minLength = 2)] pub String, pub u32);",
        "#[serde(transparent)] pub struct Bare(pub String);",
        "pub enum Choice { Slug(#[model_schema_prop(minLength = 2)] String) }",
        "pub type Alias = String;",
    ] {
        let refusals = branded_slot_refusals(source);
        assert!(refusals.is_empty(), "for {source}, got: {refusals:?}");
    }
}

/// The refusal a transparent struct written as `written` and read as `read` earns, less the prefix
/// `item_label` supplies in front of it.
fn transparent_direction_refusal(written: &str, read: &str) -> String {
    format!(
        "`#[serde(transparent)]` writes the struct as the value of {written} and reads it as the \
         value of {read}, so the payload serde writes is not one serde reads. A schema and an \
         emitted client describe one value, so neither can be written for the pair. Make one \
         field the value in both directions, and write `#[serde(skip)]` on every other field."
    )
}

#[test]
fn a_transparent_struct_written_and_read_as_two_fields_is_refused_naming_both() {
    for (source, written, read) in SPLIT_TRANSPARENT_SOURCES {
        let tokens = expansion_over(source);
        let refusals = literal_texts(&tokens)
            .into_iter()
            .filter(|text| text.starts_with("model_schema: "))
            .collect::<Vec<_>>();
        assert_eq!(
            refusals,
            vec![format!(
                "model_schema: type `Split`: {}",
                transparent_direction_refusal(written, read)
            )],
            "for {source}, got: {tokens}"
        );
    }
}

#[test]
fn the_readme_shows_the_transparent_direction_refusal() {
    let shown = format!(
        "error: model_schema: type `Split`: {}",
        transparent_direction_refusal("field `count`", "field `text`")
    );
    assert!(include_str!("../../README.md").contains(&shown));
}

#[test]
fn a_transparent_direction_refusal_is_spanned_on_the_struct_name() {
    for (source, _, _) in SPLIT_TRANSPARENT_SOURCES {
        let refusals = super::transparent_direction_errors(&syn::parse_str(source).unwrap());
        assert_eq!(refusals.len(), 1, "for {source}, got: {refusals:?}");
        let located = located_source_texts(&refusals[0]);
        assert!(
            !located.is_empty() && located.iter().all(|text| text == "Split"),
            "for {source}, got: {located:?}"
        );
    }
}

#[test]
fn a_transparent_struct_each_derive_takes_one_field_of_is_not_refused() {
    let attributes = "#[serde(transparent)] pub struct Split { #[serde(default)] pub count: u32, \
         #[serde(skip_serializing)] pub text: String }";
    for source in [
        format!("#[derive(Debug, Serialize)] {attributes}"),
        format!("#[derive(Debug, Deserialize)] {attributes}"),
        attributes.to_owned(),
        UNREAD_BRAND_FIELD_SOURCE.replace("#[model_schema_prop(maxLength = 9)] ", ""),
        "#[derive(Deserialize, Serialize)] #[serde(transparent)] pub struct Slug(pub String);"
            .to_owned(),
        "#[derive(Deserialize, Serialize)] pub struct Split { #[serde(default)] pub count: u32, \
         #[serde(skip_serializing)] pub text: String }"
            .to_owned(),
    ] {
        let refusals = super::transparent_direction_errors(&syn::parse_str(&source).unwrap());
        assert!(refusals.is_empty(), "for {source}, got: {refusals:?}");
    }
}

/// One merged source as [`super::zod_merged_joins`] reads it: what it is written as, the key sets
/// it writes where it names a choice, and whether it offers its own absence beside them.
#[cfg(feature = "zod")]
fn merged_source(spelling: &str, branches: &[&str], absence: SourceAbsence) -> MergedOperand {
    MergedOperand {
        absence,
        branches: branches
            .iter()
            .map(|branch| (*branch).to_owned())
            .collect::<Vec<_>>(),
        entries: false,
        spelling: spelling.to_owned(),
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_single_key_set_source_closes_the_object_once() {
    let operands = [merged_source("Base$Schema", &[], SourceAbsence::Never)];
    assert_eq!(
        super::zod_merged_joins(&operands),
        vec![".and(z.lazy(() => Base$Schema))".to_owned()]
    );
    assert_eq!(
        super::zod_merged_object("OWN", &operands),
        "OWN.and(z.lazy(() => Base$Schema))"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_single_branch_source_collapses_to_the_branchs_own_object() {
    let operands = [merged_source(
        "Choice$Schema",
        &["Only$Schema"],
        SourceAbsence::Never,
    )];
    assert_eq!(
        super::zod_merged_object("OWN", &operands),
        "OWN.and(z.lazy(() => Only$Schema))"
    );
}

/// Flattening nothing writes the object as it stands.
#[cfg(feature = "zod")]
#[test]
fn a_source_less_object_is_written_as_it_stands() {
    assert_eq!(super::zod_merged_joins(&[]), vec![String::new()]);
    assert_eq!(super::zod_merged_object("OWN", &[]), "OWN");
}

#[cfg(feature = "zod")]
#[test]
fn a_two_branch_source_writes_the_object_once_per_branch() {
    let operands = [merged_source(
        "Choice$Schema",
        &["Left$Schema", "Right$Schema"],
        SourceAbsence::Never,
    )];
    assert_eq!(
        super::zod_merged_joins(&operands),
        vec![
            ".and(z.lazy(() => Left$Schema))".to_owned(),
            ".and(z.lazy(() => Right$Schema))".to_owned(),
        ]
    );
    assert_eq!(
        super::zod_merged_object("OWN", &operands),
        "z.union([\n  OWN.and(z.lazy(() => Left$Schema)),\n  OWN.and(z.lazy(() => Right$Schema)),\n])"
    );
}

/// A third branch is a third combination, in the order the choice records its members.
#[cfg(feature = "zod")]
#[test]
fn a_three_branch_source_writes_the_object_once_per_branch() {
    let operands = [merged_source(
        "Choice$Schema",
        &["Left$Schema", "Middle$Schema", "Right$Schema"],
        SourceAbsence::Never,
    )];
    assert_eq!(
        super::zod_merged_joins(&operands),
        vec![
            ".and(z.lazy(() => Left$Schema))".to_owned(),
            ".and(z.lazy(() => Middle$Schema))".to_owned(),
            ".and(z.lazy(() => Right$Schema))".to_owned(),
        ]
    );
    assert_eq!(
        super::zod_merged_object("OWN", &operands),
        "z.union([\n  OWN.and(z.lazy(() => Left$Schema)),\n  \
         OWN.and(z.lazy(() => Middle$Schema)),\n  OWN.and(z.lazy(() => Right$Schema)),\n])"
    );
}

#[cfg(feature = "zod")]
#[test]
fn an_absent_source_writes_the_object_with_it_and_without_it() {
    for absence in [SourceAbsence::Field, SourceAbsence::Published] {
        let operands = [merged_source("Base$Schema", &[], absence)];
        assert_eq!(
            super::zod_merged_joins(&operands),
            vec![".and(z.lazy(() => Base$Schema))".to_owned(), String::new()]
        );
        assert_eq!(
            super::zod_merged_object("OWN", &operands),
            "z.union([\n  OWN.and(z.lazy(() => Base$Schema)),\n  OWN,\n])"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn the_absence_of_a_source_read_as_entries_holds_every_key_it_was_handed() {
    for absence in [SourceAbsence::Field, SourceAbsence::Published] {
        let operands = [MergedOperand {
            entries: true,
            ..merged_source("Counts$Schema", &[], absence)
        }];
        assert_eq!(
            super::zod_merged_joins(&operands),
            vec![
                ".and(z.lazy(() => Counts$Schema))".to_owned(),
                ".and(z.record(z.string(), z.unknown()).transform(() => ({})))".to_owned()
            ]
        );
    }
    let always = [MergedOperand {
        entries: true,
        ..merged_source("Counts$Schema", &[], SourceAbsence::Never)
    }];
    assert_eq!(
        super::zod_merged_joins(&always),
        vec![".and(z.lazy(() => Counts$Schema))".to_owned()]
    );
}

#[cfg(feature = "zod")]
#[test]
fn two_branching_sources_write_their_cross_product() {
    let operands = [
        merged_source(
            "First$Schema",
            &["Left$Schema", "Right$Schema"],
            SourceAbsence::Never,
        ),
        merged_source(
            "Second$Schema",
            &["Up$Schema", "Down$Schema"],
            SourceAbsence::Never,
        ),
    ];
    assert_eq!(
        super::zod_merged_joins(&operands),
        vec![
            ".and(z.lazy(() => Left$Schema)).and(z.lazy(() => Up$Schema))".to_owned(),
            ".and(z.lazy(() => Left$Schema)).and(z.lazy(() => Down$Schema))".to_owned(),
            ".and(z.lazy(() => Right$Schema)).and(z.lazy(() => Up$Schema))".to_owned(),
            ".and(z.lazy(() => Right$Schema)).and(z.lazy(() => Down$Schema))".to_owned(),
        ]
    );
}

#[cfg(feature = "zod")]
#[test]
fn an_absent_branching_source_offers_its_branches_and_the_absence() {
    let operands = [merged_source(
        "Choice$Schema",
        &["Left$Schema", "Right$Schema"],
        SourceAbsence::Field,
    )];
    assert_eq!(
        super::zod_merged_joins(&operands),
        vec![
            ".and(z.lazy(() => Left$Schema))".to_owned(),
            ".and(z.lazy(() => Right$Schema))".to_owned(),
            String::new(),
        ]
    );
    assert_eq!(
        super::zod_merged_object("OWN", &operands),
        "z.union([\n  OWN.and(z.lazy(() => Left$Schema)),\n  \
         OWN.and(z.lazy(() => Right$Schema)),\n  OWN,\n])"
    );
}

/// The same source beside an absent plain one: three combinations against two, six in all.
#[cfg(feature = "zod")]
#[test]
fn a_branching_source_multiplies_against_an_absent_one() {
    let operands = [
        merged_source(
            "Choice$Schema",
            &["Left$Schema", "Right$Schema"],
            SourceAbsence::Never,
        ),
        merged_source("Base$Schema", &[], SourceAbsence::Field),
    ];
    assert_eq!(
        super::zod_merged_joins(&operands),
        vec![
            ".and(z.lazy(() => Left$Schema)).and(z.lazy(() => Base$Schema))".to_owned(),
            ".and(z.lazy(() => Left$Schema))".to_owned(),
            ".and(z.lazy(() => Right$Schema)).and(z.lazy(() => Base$Schema))".to_owned(),
            ".and(z.lazy(() => Right$Schema))".to_owned(),
        ]
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_hoist_and_the_inlining_write_the_same_combinations() {
    let operands = [merged_source(
        "Choice$Schema",
        &["Left$Schema", "Right$Schema"],
        SourceAbsence::Never,
    )];
    let (preamble, expression) = super::zod_merged_statements("Host", "OWN", &operands);
    assert_eq!(preamble, "const Host$OwnSchema = OWN;\n\n");
    assert_eq!(
        expression,
        super::zod_merged_object("Host$OwnSchema", &operands)
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_single_combination_binds_no_name_on_either_path() {
    let operands = [merged_source("Base$Schema", &[], SourceAbsence::Never)];
    assert_eq!(
        super::zod_merged_statements("Host", "OWN", &operands),
        (String::new(), super::zod_merged_object("OWN", &operands))
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_field_bottoming_out_in_a_declared_type_is_walked_and_a_primitive_one_is_not() {
    let mut item: syn::ItemStruct = syn::parse_quote! {
        struct Envelope {
            account: Account,
            count: u32,
            flagged: bool,
            name: String,
            tags: Vec<Tag>,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        None,
        Some("envelope_schema"),
        "Envelope",
        &syn::Generics::default(),
        &super::StructContainer {
            defaulted: false,
            read_back: false,
            serde_default: &|_| None,
            wire: super::ContainerWire::OneWay,
        },
    );
    assert!(collected.4.is_empty(), "got: {:?}", collected.4);

    let bodies = collected
        .3
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        bodies.len(),
        2,
        "a walk for each of the two fields that could hold a bound, and none for the three that \
         could not. Got: {bodies:?}"
    );
    assert!(
        bodies
            .iter()
            .all(|walk| walk.contains("trait UnpublishedValidate")),
        "each walk carries the fallback it asks its value through. Got: {bodies:?}"
    );
    let walks = bodies.join("");
    assert!(
        walks.contains("& self . account") && walks.contains("& self . tags"),
        "got: {walks}"
    );
    for primitive in ["count", "flagged", "name"] {
        assert!(
            !walks.contains(&format!("& self . {primitive}")),
            "`{primitive}` bottoms out in a primitive, whose bounds are declared on the field and \
             run there. Got: {walks}"
        );
    }
}

#[test]
fn untagged_enum_registry_answers_only_after_the_enum_is_recorded() {
    let declared_below: syn::Type = syn::parse_quote!(UntaggedEnumRegistryProbeBelow);
    assert!(!is_recorded_untagged_enum(&declared_below));

    record_untagged_enum("UntaggedEnumRegistryProbeAbove");
    let declared_above: syn::Type = syn::parse_quote!(UntaggedEnumRegistryProbeAbove);
    assert!(is_recorded_untagged_enum(&declared_above));
}

/// serde reads `untagged` wherever the list writes it, after a key with a value included.
#[test]
fn an_enum_is_recorded_untagged_wherever_its_list_writes_the_key() {
    for (attribute, name) in [
        (
            "#[serde(untagged, rename_all = \"camelCase\")]",
            "UntaggedWrittenFirstProbe",
        ),
        (
            "#[serde(rename_all = \"camelCase\", untagged)]",
            "UntaggedWrittenLastProbe",
        ),
    ] {
        expansion_under(
            "",
            &format!("{attribute} pub enum {name} {{ Gone(String), Missing(u32) }}"),
        );
        let named: syn::Type = syn::parse_str(name).unwrap();
        assert!(
            is_recorded_untagged_enum(&named),
            "`{name}` under {attribute} is untagged"
        );
    }
}

/// A brand over `Box<IdType>` expands without panicking, checked against `Box<String>`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_constrained_brand_over_a_wrapped_parameter_expands_against_the_wrapped_default() {
    let tokens = expansion_with_args_over(
        "minLength = 3, default_types(IdType = String)",
        "
        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct BoxedId<IdType>(pub Box<IdType>);
        ",
    );
    let rendered = tokens.to_string();
    assert!(!rendered.contains("compile_error"), "got: {rendered}");
    assert!(
        rendered.contains("type_identity :: < Box < String > >"),
        "got: {rendered}"
    );
}

/// A constrained generic brand's hook carries its own type identity and names no outside crate.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_constrained_generic_brand_defines_its_own_type_identity() {
    let generic = expansion_with_args_over(
        "minLength = 3, default_types(IdType = String)",
        "
        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct GenericSlug<IdType>(pub IdType);
        ",
    )
    .to_string();
    assert!(generic.contains("fn type_identity"), "got: {generic}");
    assert!(
        generic.contains("type_identity :: < String >"),
        "got: {generic}"
    );
    assert!(!generic.contains("typeid ::"), "got: {generic}");

    let concrete = expansion_with_args_over(
        "minLength = 3",
        "
        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct ConcreteSlug(pub String);
        ",
    )
    .to_string();
    assert!(!concrete.contains("type_identity"), "got: {concrete}");
}

/// A parameter the brand's inner names is read at its declared default through the type
/// identity, and a parameter beside a concrete inner changes nothing: the inner is judged at every
/// filling by `validate_value` alone.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn only_a_brand_whose_inner_names_a_parameter_reads_at_the_declared_default() {
    let over_parameter = expansion_with_args_over(
        "minLength = 3, default_types(Held = PathBuf)",
        "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct HeldPath<Held>(pub Held);",
    )
    .to_string();
    assert!(
        over_parameter.contains("fn type_identity"),
        "got: {over_parameter}"
    );
    assert!(
        over_parameter.contains("fn validate_at_declared_default"),
        "got: {over_parameter}"
    );
    assert!(
        !over_parameter.contains("held . to_string"),
        "got: {over_parameter}"
    );

    for inner in ["PathBuf", "String"] {
        let beside_marker = expansion_with_args_over(
            "minLength = 3, no_display, default_types(Mark = String)",
            &format!(
                "#[derive(Serialize, Deserialize)] #[serde(transparent)] \
                 pub struct Marked{inner}<Mark> {{ #[serde(skip)] pub mark: PhantomData<Mark>, \
                 pub text: {inner} }}"
            ),
        )
        .to_string();
        assert!(
            !beside_marker.contains("type_identity"),
            "{inner}: {beside_marker}"
        );
        assert!(
            !beside_marker.contains("validate_at_declared_default"),
            "{inner}: {beside_marker}"
        );
        assert!(
            beside_marker.contains("fn validate_value"),
            "{inner}: {beside_marker}"
        );
    }
}

/// The default a parameter takes where `default_types` leaves its entry out is the standard
/// `String`, written at its full path.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod"),
    not(feature = "jsonschema")
))]
#[test]
fn the_fallback_default_of_a_generic_brand_is_the_standard_string() {
    let rendered = expansion_with_args_over(
        "minLength = 2",
        "#[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct FallbackCode<T>(pub T);",
    )
    .to_string();
    assert!(
        rendered.contains("type_identity :: < :: std :: string :: String > ()"),
        "got: {rendered}"
    );
    assert!(
        !rendered.contains("type_identity :: < String > ()"),
        "got: {rendered}"
    );
}

/// Declares what the table of declared defaults reads, above the brands that name them.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
fn declare_the_names_a_default_may_be() {
    for (args, source) in [
        (
            "no_display",
            "#[derive(Serialize)] #[serde(transparent)] pub struct TableTag<'text>(pub &'text str);",
        ),
        ("", "pub type TableKept = TableTag<'static>;"),
        ("", "pub type TableText = String;"),
        (
            "no_display",
            "#[derive(Serialize)] #[serde(transparent)] pub struct TableLabel(pub String);",
        ),
    ] {
        let declared = expansion_with_args_over(args, source).to_string();
        assert!(!declared.contains("compile_error"), "{source}: {declared}");
    }
}

/// A generic brand with a string check reads its value as the declared default only where the
/// expansion can prove the default writes every lifetime it has.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_declared_default_is_read_only_where_every_lifetime_it_has_is_written() {
    declare_the_names_a_default_may_be();
    let brand = |default: &str, args: &str| {
        expansion_with_args_over(
            &format!("{args}default_types(Text = {default})"),
            "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct TableRow<Text>(pub Text);",
        )
        .to_string()
    };
    for built in [
        "String",
        "&'static str",
        "Cow<'_, str>",
        "PathBuf",
        "Box<str>",
        "TableLabel",
        "TableText",
    ] {
        let expanded = brand(built, "minLength = 2, ");
        assert!(!expanded.contains("compile_error"), "{built}: {expanded}");
    }
    for (refused, said) in [
        ("TableTag<'static>", None),
        ("TableKept", Some("is an alias that hides a lifetime")),
        ("PlainKept", Some("is a type this expansion has not read")),
        (
            "other_crate::Kept",
            Some("is a type this expansion has not read"),
        ),
        ("TableTag", Some("write `TableTag<'_>`")),
        (
            "Vec<PlainKept>",
            Some("is a type this expansion has not read"),
        ),
    ] {
        let expanded = brand(refused, "minLength = 2, ");
        match said {
            None => assert!(!expanded.contains("compile_error"), "{refused}: {expanded}"),
            Some(clause) => {
                assert!(expanded.contains("compile_error"), "{refused}: {expanded}");
                assert!(expanded.contains(clause), "{refused}: {expanded}");
                assert!(
                    expanded.contains("a string check on the generic brand `TableRow`"),
                    "{refused}: {expanded}"
                );
            }
        }
    }
}

/// A generic brand with no string check is refused nothing, whatever its default.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_generic_brand_with_no_string_check_is_refused_no_default() {
    let expanded = expansion_with_args_over(
        "default_types(Text = UnreadKept)",
        "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct UncheckedRow<Text>(pub Text);",
    )
    .to_string();
    assert!(!expanded.contains("compile_error"), "got: {expanded}");
}

/// `validate()` is written once, in the `impl` that holds the type's delegates, under the
/// generics the type declares: a type with a parameter gets no second `impl` at one filling.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn validate_is_written_once_in_the_types_own_impl() {
    for (args, source, own_impl) in [
        (
            "",
            "#[derive(Serialize, Deserialize)]
            pub struct FixedCrate {
                #[model_schema_prop(minLength = 1)]
                pub label: String,
            }",
            "impl FixedCrate {",
        ),
        (
            "default_types(CodeType = String)",
            "#[derive(Serialize, Deserialize)]
            pub struct FilledCrate<CodeType> {
                pub codes: Vec<CodeType>,
                #[model_schema_prop(minLength = 1)]
                pub label: String,
            }",
            "impl < CodeType > FilledCrate < CodeType > {",
        ),
        (
            "default_types(CodeType = String)",
            "#[derive(Serialize, Deserialize)]
            pub struct FilledSack<CodeType>(
                #[model_schema_prop(minLength = 1)] pub String,
                pub CodeType,
            );",
            "impl < CodeType > FilledSack < CodeType > {",
        ),
        (
            "default_types(CodeType = String)",
            "#[derive(Serialize, Deserialize)]
            #[serde(tag = \"kind\")]
            pub enum FilledLoad<CodeType> {
                Packed {
                    codes: Vec<CodeType>,
                    #[model_schema_prop(minLength = 1)]
                    label: String,
                },
                Unpacked,
            }",
            "impl < CodeType > FilledLoad < CodeType > {",
        ),
        (
            "minLength = 2",
            "#[derive(Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct FixedCode(pub String);",
            "impl FixedCode {",
        ),
        (
            "minLength = 2, default_types(CodeType = String)",
            "#[derive(Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct FilledCode<CodeType>(pub CodeType);",
            "impl < CodeType > FilledCode < CodeType > {",
        ),
        (
            "no_display, minLength = 1, default_types(T = String)",
            "#[derive(Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct FilledCodes<T>(pub Vec<T>);",
            "impl < T > FilledCodes < T > {",
        ),
    ] {
        let rendered = expansion_with_args_over(args, source).to_string();
        assert!(!rendered.contains("compile_error"), "got: {rendered}");
        assert!(
            rendered.contains(own_impl),
            "no `{own_impl}` in: {rendered}"
        );
        let (_, held) = rendered.split_once(own_impl).unwrap();
        assert!(held.contains("pub fn validate (& self)"), "got: {rendered}");
        assert_eq!(
            rendered.matches("pub fn validate (& self)").count(),
            1,
            "got: {rendered}"
        );
        assert!(
            !rendered.contains("< String > { pub fn validate"),
            "got: {rendered}"
        );
    }
}

/// A generic brand's `validate()` hands the value to the function that reads it as its declared
/// default, under no bound the brand's author did not write, and no function answers a reference
/// to that default. The read asks of the parameter what serde's derive asks, and no `Display`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_generic_brands_checks_are_run_by_the_function_that_asks_which_filling_it_is() {
    let generic = expansion_with_args_over(
        "minLength = 2, default_types(CodeType = String)",
        "
        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct AskedCode<CodeType>(pub CodeType);
        ",
    )
    .to_string();
    for written in [
        "pub struct AskedCode < CodeType > (",
        "pub fn validate_at_declared_default < 'held , T > (held : & 'held T) -> :: core :: \
         result :: Result < () , :: std :: vec :: Vec < :: std :: string :: String >> { if \
         type_identity :: < T > () == type_identity :: < String > () { let held = unsafe { & * :: \
         core :: ptr :: from_ref (held) . cast :: < String > () } ; validate_value (& held . \
         to_string ()) } else { :: core :: result :: Result :: Ok (()) } }",
        "if let :: core :: result :: Result :: Err (reported) = asked_code_schema :: \
         validate_at_declared_default (& self . 0) { errors . extend (reported) ; }",
        "pub fn model_schema_read_unjudged < 'de , D > (deserializer : D) -> :: core :: result \
         :: Result < Self , D :: Error > where D : serde :: Deserializer < 'de > , CodeType : \
         serde :: Deserialize < 'de > , {",
        "# [serde (remote = \"AskedCode\")] # [serde (bound (deserialize = \"CodeType : serde :: \
         Deserialize < 'de >\"))] # [serde (transparent)] struct ModelSchemaRemote < CodeType > \
         (CodeType) ;",
    ] {
        assert!(generic.contains(written), "no `{written}` in: {generic}");
    }
    for unwritten in [
        "Option < & String >",
        "DeserializeOwned",
        "deserialize_value",
        "deserialize_with",
    ] {
        assert!(
            !generic.contains(unwritten),
            "found `{unwritten}` in: {generic}"
        );
    }

    for (args, source) in [
        (
            "minLength = 2",
            "#[derive(Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct UnaskedCode(pub String);",
        ),
        (
            "no_display, minLength = 1, default_types(T = String)",
            "#[derive(Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct UnaskedCodes<T>(pub Vec<T>);",
        ),
    ] {
        let rendered = expansion_with_args_over(args, source).to_string();
        assert!(
            !rendered.contains("validate_at_declared_default"),
            "got: {rendered}"
        );
    }
}

/// A declared default that writes a lifetime is compared and read at the lifetime of the borrow,
/// so a value is never read at a lifetime longer than its own. Nothing else asks which filling
/// the brand holds.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_default_that_writes_a_lifetime_is_read_at_the_borrows_own() {
    let rendered = expansion_with_args_over(
        "minLength = 2, default_types(Text = &'static str)",
        "
        #[derive(Serialize)]
        #[serde(transparent)]
        pub struct BorrowedStamp<Text>(pub Text);
        ",
    )
    .to_string();
    assert!(!rendered.contains("compile_error"), "got: {rendered}");
    let written = "pub fn validate_at_declared_default < 'held , T > (held : & 'held T) -> :: \
                   core :: result :: Result < () , :: std :: vec :: Vec < :: std :: string :: \
                   String >> { if type_identity :: < T > () == type_identity :: < & 'held str > \
                   () { let held = unsafe { & * :: core :: ptr :: from_ref (held) . cast :: < & \
                   'held str > () } ; validate_value (& held . to_string ()) } else { :: core :: \
                   result :: Result :: Ok (()) } }";
    assert!(rendered.contains(written), "no `{written}` in: {rendered}");
    assert_eq!(rendered.matches("== type_identity ::").count(), 1);
    assert!(!rendered.contains("deserialize_value"), "got: {rendered}");
}

/// Every lifetime a type writes is replaced, and one a `for<...>` or a signature binds is not.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_type_is_written_at_one_lifetime() {
    for (written, at_one) in [
        ("String", "String"),
        ("&str", "& 'held str"),
        ("&'static str", "& 'held str"),
        ("&'_ mut str", "& 'held mut str"),
        ("Cow<'static, str>", "Cow < 'held , str >"),
        ("Tag<'_>", "Tag < 'held >"),
        (
            "(&'text str, Box<&str>, [&str; 2], *const &str)",
            "(& 'held str , Box < & 'held str > , [& 'held str ; 2] , * const & 'held str)",
        ),
        (
            "<&'static str as ToOwned>::Owned",
            "< & 'held str as ToOwned > :: Owned",
        ),
        (
            "Box<dyn Iterator<Item = &'static str> + 'static>",
            "Box < dyn Iterator < Item = & 'held str > + 'held >",
        ),
        ("fn(&str) -> &str", "fn (& str) -> & str"),
        ("fn(&'_ str, &'static str)", "fn (& '_ str , & 'held str)"),
        (
            "for<'each> fn(&'each str, &'text str)",
            "for < 'each > fn (& 'each str , & 'held str)",
        ),
        (
            "Box<dyn for<'each> Fn(&'each str, &str) -> Tag<'static> + 'text>",
            "Box < dyn for < 'each > Fn (& 'each str , & str) -> Tag < 'held > + 'held >",
        ),
    ] {
        let ty: syn::Type = syn::parse_str(written).unwrap();
        let read = super::at_one_lifetime(&ty, &syn::parse_quote!('held));
        assert_eq!(quote::quote!(#read).to_string(), at_one, "for `{written}`");
    }
}

/// The arguments a two-parameter `Pair` binds.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn pair_parameters() -> Vec<String> {
    vec!["Left".to_owned(), "Right".to_owned()]
}

#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn an_items_own_call_inside_a_map_becomes_its_self_view_and_no_other_call_does() {
    let viewed = maps_of_itself_through_the_self_view(
        "Pair",
        &pair_parameters(),
        "z.strictObject({\n  \
         get held() { return z.array(z.record(z.string(), Pair$SchemaFactory(left, right))); },\n  \
         get list() { return z.array(Pair$SchemaFactory(left, right)); },\n  \
         get other() { return z.record(z.string(), SpotPair$SchemaFactory(left, right)); },\n  \
         get turned() { return z.record(z.string(), Pair$SchemaFactory(right, left)); },\n\
         })",
    );
    assert_eq!(
        viewed,
        "z.strictObject({\n  \
         get held() { return z.array(z.record(z.string(), Pair$SchemaSelf(left, right))); },\n  \
         get list() { return z.array(Pair$SchemaFactory(left, right)); },\n  \
         get other() { return z.record(z.string(), SpotPair$SchemaFactory(left, right)); },\n  \
         get turned() { return z.record(z.string(), Pair$SchemaFactory(right, left)); },\n\
         })"
    );
}

#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn a_unions_arrow_is_handed_the_self_view_for_the_argument_it_writes_inside_a_map() {
    let arrow = "(<Reached$ extends SomeType, Reached2$ extends SomeType>\
                 (reached$: Reached$, reached2$: Reached2$) => \
                 z.union([z.tuple([reached$, z.record(z.string(), reached2$)]), z.null()]))";
    let viewed = maps_of_itself_through_the_self_view(
        "Pair",
        &pair_parameters(),
        &format!("{arrow}(Pair$SchemaFactory(left, right), Pair$SchemaFactory(left, right))"),
    );
    assert_eq!(
        viewed,
        format!("{arrow}(Pair$SchemaFactory(left, right), Pair$SchemaSelf(left, right))"),
        "the first argument is written outside every map and keeps the factory's own type"
    );
}

#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn a_parenthesis_inside_a_quoted_text_does_not_end_a_map() {
    let viewed = maps_of_itself_through_the_self_view(
        "Pair",
        &pair_parameters(),
        "z.record(z.string().meta({ description: \"a ) b\" }), Pair$SchemaFactory(left, right))",
    );
    assert_eq!(
        viewed,
        "z.record(z.string().meta({ description: \"a ) b\" }), Pair$SchemaSelf(left, right))"
    );
}

/// A brand that carries no bound and names no function keeps the read serde's derive writes it:
/// its item is emitted as it was written, and nothing beside it is a read of the macro's.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_brand_with_no_rule_gains_no_read_of_the_macros() {
    for (args, source, emitted) in [
        (
            "",
            "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct UnruledBrand(pub \
             String);",
            "# [derive (Serialize , Deserialize)] # [serde (transparent)] pub struct UnruledBrand \
             (pub String) ;",
        ),
        (
            "no_display",
            "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct UnruledNamed { pub \
             rows: Vec<String> }",
            "# [derive (Serialize , Deserialize)] # [serde (transparent)] pub struct UnruledNamed \
             { pub rows : Vec < String > }",
        ),
        (
            "default_types(IdType = String)",
            "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct \
             UnruledGeneric<IdType>(pub IdType);",
            "# [derive (Serialize , Deserialize)] # [serde (transparent)] pub struct \
             UnruledGeneric < IdType > (pub IdType) ;",
        ),
        (
            "enable = [\"decode_with\", \"mongodb\"]",
            "#[derive(Serialize, Deserialize)] #[serde(transparent)] pub struct UnruledDecoded(pub \
             String);",
            "# [derive (Serialize , Deserialize)] # [serde (transparent)] pub struct \
             UnruledDecoded (pub String) ;",
        ),
    ] {
        let expanded = expansion_under(args, source);
        assert!(
            expanded.starts_with(emitted),
            "for {source}, got: {expanded}"
        );
        for unwritten in [
            "pub fn model_schema_read_unjudged",
            "model_schema_validate_bound",
            "ModelSchemaRemote",
            "deserialize_with",
            "validate_value",
        ] {
            assert!(
                !expanded.contains(unwritten),
                "for {source}, found `{unwritten}` in: {expanded}"
            );
        }
    }
}

/// An untagged enum with a member over `()` publishes the record the JSON Schema merge reads on
/// its schema module's `Schema`, under the one name the merge asks for, and an alias, a brand and
/// a struct of one slot over a name, written bare, forward that name's; nothing else carries it.
#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn the_record_of_a_member_over_unit_is_published_under_its_one_name() {
    assert_eq!(super::UNIT_MEMBER_RECORD, "WRITES_A_UNIT_MEMBER");
    let published = |source: &str| {
        expansion_over(&format!("#[derive(Serialize, Deserialize)] {source}"))
            .to_string()
            .contains("pub const WRITES_A_UNIT_MEMBER : bool =")
    };
    let declared = |source: &str| {
        expansion_over(source)
            .to_string()
            .contains("pub const WRITES_A_UNIT_MEMBER : bool =")
    };
    assert!(
        !published("pub struct ProbeRecordFields { pub a: String }"),
        "a struct publishes the record"
    );
    assert!(
        published(
            "#[serde(untagged)] pub enum ProbeRecordUnit { Obj(ProbeRecordFields), Nothing(()) }"
        ),
        "an untagged enum with a member over `()` publishes no record"
    );
    assert!(
        !published(
            "#[serde(untagged)] pub enum ProbeRecordObjects { Obj(ProbeRecordFields), \
             Named { b: String } }"
        ),
        "an untagged enum of objects publishes the record"
    );
    assert!(
        declared("pub type ProbeRecordAlias = ProbeRecordUnit;"),
        "an alias over a name forwards no record"
    );
    assert!(
        published("pub struct ProbeRecordSlot(ProbeRecordUnit);"),
        "a struct of one slot over a name forwards no record"
    );
    assert!(
        !declared("pub type ProbeRecordMaybe = Option<ProbeRecordUnit>;"),
        "an alias over an `Option` forwards the record"
    );
    assert!(
        !published("pub struct ProbeRecordBoxed(Box<ProbeRecordUnit>);"),
        "a struct of one slot behind a wrapper forwards the record"
    );
}

/// An enum whose every variant serde neither writes nor reads is refused, its members' keys taken
/// off, and so is an enum of no variant, under a tag or not; an enum with a variant serde reads or
/// writes is not.
#[test]
fn an_enum_with_no_variant_serde_writes_or_reads_is_refused() {
    assert_refused_without_member_keys(
        "",
        "pub enum ProbeBecalmed { #[serde(skip)] Drifting { #[model_schema_prop(minLength = 1)] \
         note: String }, #[serde(skip_serializing, skip_deserializing)] Still }",
        "model_schema: type `ProbeBecalmed`: every variant is under `#[serde(skip)]`, or under \
         `skip_serializing` and `skip_deserializing` together, so serde writes no value of it and \
         reads none, and no surface can describe it. Let serde write or read one of its variants.",
    );
    for (source, name) in [
        ("pub enum Becalm {}", "Becalm"),
        (
            "#[serde(tag = \"kind\")] pub enum BecalmTagged {}",
            "BecalmTagged",
        ),
    ] {
        let refusal = format!(
            "model_schema: type `{name}`: declares no variant, so serde writes no value of it and \
             reads none, and no surface can describe it. Declare a variant serde writes or reads."
        );
        assert_refused_without_member_keys("", source, &refusal);
        assert!(
            name != "Becalm" || include_str!("../../README.md").contains(&refusal),
            "the README no longer quotes this refusal:\n{refusal}"
        );
    }
    for source in [
        "pub enum ProbeHalfWritten { #[serde(skip_serializing)] Read, #[serde(skip)] Neither }",
        "pub enum ProbeHalfRead { #[serde(skip_deserializing)] Written, #[serde(skip)] Neither }",
    ] {
        let refusals = super::no_variant_on_the_wire_errors(&syn::parse_str(source).unwrap());
        assert!(refusals.is_empty(), "for {source}, got: {refusals:?}");
    }
}

/// What a variant under `#[serde(untagged)]` in an enum serde writes with names is refused with.
fn untagged_variant_refusal(enum_name: &str, variant: &str) -> String {
    format!(
        "model_schema: type `{enum_name}`: variant `{variant}` is under `#[serde(untagged)]`, so \
         serde writes it with no name, as its content alone, inside an enum it writes with names, \
         and no surface can describe it. Take `#[serde(untagged)]` off the variant, or put it on \
         the whole enum."
    )
}

/// A variant under `#[serde(untagged)]` that serde writes or reads, in an enum it writes with
/// names, is refused under every tagging, its members' keys taken off, ahead of every refusal that
/// reads it beside the enum's tag; every such variant is refused in one expansion, and one serde's
/// derive refuses for its place is refused in serde's words.
#[test]
fn a_variant_under_untagged_in_an_enum_written_with_names_is_refused() {
    for (source, enum_name) in [
        (
            "pub enum Mixed { Named { #[model_schema_prop(minLength = 1)] label: String }, \
             #[serde(untagged)] Loose(String) }",
            "Mixed",
        ),
        (
            "#[serde(tag = \"kind\")] pub enum ProbeTaggedLoose { Named { label: String }, \
             #[serde(untagged)] Loose(String) }",
            "ProbeTaggedLoose",
        ),
        (
            "#[serde(tag = \"kind\", content = \"body\")] pub enum ProbeNextLoose { Named { label: \
             String }, #[serde(untagged)] Loose(String) }",
            "ProbeNextLoose",
        ),
        (
            "#[serde(tag = \"kind\")] pub enum ProbeLooseEntries { Named { label: String }, \
             #[serde(untagged)] Loose { #[serde(flatten)] extra: HashMap<String, String> } }",
            "ProbeLooseEntries",
        ),
        (
            "pub enum ProbeReadLoose { Named { label: String }, #[serde(untagged, \
             skip_serializing)] Loose(String) }",
            "ProbeReadLoose",
        ),
        (
            "pub enum ProbeWrittenLoose { Named { label: String }, #[serde(untagged, \
             skip_deserializing)] Loose(String) }",
            "ProbeWrittenLoose",
        ),
    ] {
        let refusal = untagged_variant_refusal(enum_name, "Loose");
        assert_refused_without_member_keys("", source, &refusal);
        assert!(
            enum_name != "Mixed" || include_str!("../../README.md").contains(&refusal),
            "the README no longer quotes this refusal:\n{refusal}"
        );
    }
    assert_refused_without_member_keys(
        "",
        "pub enum ProbeHue { Red, #[serde(untagged)] Other }",
        &untagged_variant_refusal("ProbeHue", "Other"),
    );

    let both = super::exec_model_schema(
        syn::parse_str("").unwrap(),
        syn::parse_str(
            "#[serde(tag = \"kind\")] pub enum ProbeTwoLoose { Named { label: String }, \
             #[serde(untagged)] Text(String), #[serde(untagged)] Count(u32) }",
        )
        .unwrap(),
    );
    let written = both.to_string();
    assert_eq!(
        identifiers_named(both, "compile_error"),
        2,
        "got: {written}"
    );
    for variant in ["Text", "Count"] {
        let refusal = untagged_variant_refusal("ProbeTwoLoose", variant);
        assert!(
            written.contains(&refusal),
            "missing `{refusal}` in: {written}"
        );
    }

    for source in [
        "pub enum Ahead { #[serde(untagged)] Loose(String), Named { label: String } }",
        "pub enum Ahead { #[serde(untagged, skip)] Loose(String), Named { label: String } }",
    ] {
        let refusal = "model_schema: type `Ahead`: variant `Loose` is under `#[serde(untagged)]` \
                       ahead of a variant that is not, which serde's derive refuses: `all \
                       variants with the #[serde(untagged)] attribute must be placed at the end \
                       of the enum`. Take `#[serde(untagged)]` off the variant, or put it on the \
                       whole enum.";
        assert_refused_without_member_keys("", source, refusal);
        assert!(
            include_str!("../../README.md").contains(refusal),
            "the README no longer quotes this refusal:\n{refusal}"
        );
    }
}

/// An enum untagged as a whole builds with a variant under `#[serde(untagged)]` as well, and so
/// does a variant under it serde neither writes nor reads; under a tag alone, a tuple variant of no
/// slot or of several is left to the refusals in the words of serde's derive, which refuses it
/// untagged or not.
#[test]
fn an_untagged_variant_of_an_untagged_enum_or_off_the_wire_is_not_refused() {
    for source in [
        "#[serde(untagged)] pub enum ProbeBothLoose { Named { label: String }, \
         #[serde(untagged)] Loose(String) }",
        "pub enum ProbeSkippedLooseVariant { Named { label: String }, #[serde(untagged, skip)] \
         Loose(String) }",
        "pub enum ProbeAbsentLoose { Named { label: String }, #[serde(untagged, \
         skip_serializing, skip_deserializing)] Loose(String) }",
        "#[serde(tag = \"kind\")] pub enum ProbeTaggedPair { Named { label: String }, \
         #[serde(untagged)] Pair(u8, u8) }",
        "#[serde(tag = \"kind\")] pub enum ProbeTaggedNothing { Named { label: String }, \
         #[serde(untagged)] Nothing() }",
    ] {
        let refusals = super::untagged_variant_errors(&syn::parse_str(source).unwrap());
        assert!(refusals.is_empty(), "for {source}, got: {refusals:?}");
    }
    for source in [
        "#[serde(untagged)] pub enum ProbeBothLoose { Named { label: String }, \
         #[serde(untagged)] Loose(String) }",
        "pub enum ProbeSkippedLooseVariant { Named { label: String }, #[serde(untagged, skip)] \
         Loose(String) }",
    ] {
        let expanded =
            super::exec_model_schema(syn::parse_str("").unwrap(), syn::parse_str(source).unwrap());
        let written = expanded.to_string();
        assert_eq!(
            identifiers_named(expanded, "compile_error"),
            0,
            "for {source}, got: {written}"
        );
    }
    let nothing = super::empty_tuple_variant_errors(
        &syn::parse_str(
            "#[serde(tag = \"kind\")] pub enum ProbeTaggedNothing { Named { label: String }, \
             #[serde(untagged)] Nothing() }",
        )
        .unwrap(),
    );
    assert_eq!(nothing.len(), 1, "got: {nothing:?}");
}

/// No guard of what serde writes or reads refuses a variant serde neither writes nor reads: a
/// tuple variant of no slot, a lone slot taken off the wire under a content key, a bare `Option`
/// member, a unit variant's name that breaks the enum's pattern, an untagged unit variant and
/// tuple variant of several slots, and an untagged member over text a holder flattens.
#[cfg(feature = "serde")]
#[test]
fn the_guards_of_the_wire_read_past_a_variant_serde_neither_writes_nor_reads() {
    for (args, source) in [
        (
            "",
            "pub enum ProbeSkippedEmpty { Open { note: String }, #[serde(skip)] Nothing() }",
        ),
        (
            "",
            "#[serde(tag = \"kind\", content = \"body\")] pub enum ProbeSkippedNextEmpty { Open { \
             note: String }, #[serde(skip)] Nothing() }",
        ),
        (
            "",
            "#[serde(untagged)] pub enum ProbeSkippedLooseEmpty { Open { note: String }, \
             #[serde(skip)] Nothing() }",
        ),
        (
            "",
            "#[serde(tag = \"kind\", content = \"body\")] pub enum ProbeSkippedLone { Open { note: \
             String }, #[serde(skip)] Lone(#[serde(skip)] u8) }",
        ),
        (
            "",
            "#[serde(tag = \"kind\")] pub enum ProbeSkippedOption { Open { note: String }, \
             #[serde(skip)] Closed { reason: Option<String> } }",
        ),
        (
            "",
            "#[serde(untagged)] pub enum ProbeSkippedLoose { Open { note: String }, \
             #[serde(skip)] Closed { reason: Option<String> }, #[serde(skip)] Gone, \
             #[serde(skip)] Pair(u8, u8) }",
        ),
        (
            "",
            "#[serde(untagged)] pub enum ProbeSkippedText { Fields { note: String }, \
             #[serde(skip)] Text(String) }",
        ),
        (
            "",
            "pub struct ProbeSkippedHolder { #[serde(flatten)] pub held: ProbeSkippedText, pub \
             label: String }",
        ),
        (
            "pattern = \"^[a-z]+$\"",
            "#[serde(rename_all = \"lowercase\")] pub enum ProbeSkippedName { Red, \
             #[serde(skip, rename = \"GREEN\")] Green }",
        ),
    ] {
        let expanded = super::exec_model_schema(
            syn::parse_str(args).unwrap(),
            syn::parse_str(&format!("#[derive(Serialize, Deserialize)] {source}")).unwrap(),
        );
        assert_eq!(
            compile_error_count(&expanded),
            0,
            "for {source}, got: {expanded}"
        );
    }
}

/// The guards [`the_guards_of_the_wire_read_past_a_variant_serde_neither_writes_nor_reads`] reads
/// past refuse a variant serde drops in one direction alone, as before.
#[cfg(feature = "serde")]
#[test]
fn the_guards_of_the_wire_refuse_a_variant_serde_drops_in_one_direction() {
    // Declared ahead of the holder that flattens it, which reads what it records.
    let declared = expansion_under(
        "",
        "#[derive(Serialize, Deserialize)] #[serde(untagged)] pub enum ProbeHalfText { Fields { \
         note: String }, #[serde(skip_deserializing)] Text(String) }",
    );
    assert!(!declared.contains("compile_error"), "got: {declared}");
    for (args, source, refusal) in [
        (
            "",
            "pub enum ProbeHalfEmpty { Open { note: String }, #[serde(skip_serializing)] \
             Nothing() }",
            "`Nothing()` is a tuple variant with no field",
        ),
        (
            "",
            "#[serde(tag = \"kind\")] pub enum ProbeHalfOption { Open { note: String }, \
             #[serde(skip_deserializing)] Closed { reason: Option<String> } }",
            "field `reason` is `Option`",
        ),
        (
            "pattern = \"^[a-z]+$\"",
            "#[serde(rename_all = \"lowercase\")] pub enum ProbeHalfName { Red, \
             #[serde(skip_serializing, rename = \"GREEN\")] Green }",
            "variant `Green` is written `GREEN`",
        ),
        (
            "",
            "#[serde(untagged)] pub enum ProbeHalfLoose { Open { note: String }, \
             #[serde(skip_deserializing)] Gone }",
            "`Gone` is a unit variant",
        ),
        (
            "",
            "pub struct ProbeHalfHolder { #[serde(flatten)] pub held: ProbeHalfText, pub label: \
             String }",
            "its branch 2 describes a `string`",
        ),
    ] {
        let expanded = super::exec_model_schema(
            syn::parse_str(args).unwrap(),
            syn::parse_str(&format!("#[derive(Serialize, Deserialize)] {source}")).unwrap(),
        )
        .to_string();
        assert!(expanded.contains(refusal), "for {source}, got: {expanded}");
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[test]
fn a_transparent_tuple_struct_with_a_one_way_slot_expands_and_a_plain_one_is_refused() {
    let transparent = refusal_texts(&expansion_over(
        "#[derive(Deserialize, Serialize)] #[serde(transparent)] \
         struct Marked(PhantomData<u8>, String, #[serde(default, skip_serializing)] u32);",
    ));
    assert_eq!(transparent, Vec::<String>::new());
    let plain = refusal_texts(&expansion_over(
        "#[derive(Deserialize, Serialize)] \
         struct Pair(#[serde(skip_serializing)] String, u32);",
    ));
    assert_eq!(plain.len(), 1, "got: {plain:?}");
    assert!(
        plain[0].contains("slot 0 of tuple struct `Pair`"),
        "got: {plain:?}"
    );
}

#[cfg(feature = "serde")]
fn map_key_refusal(parameter: &str, source: &str) -> Vec<String> {
    refusal_texts(&super::exec_model_schema(
        syn::parse_str(&format!("default_types({parameter} = String)")).unwrap(),
        syn::parse_str(&format!("#[derive(Deserialize, Serialize)] {source}")).unwrap(),
    ))
}

#[cfg(feature = "serde")]
#[test]
fn a_map_of_models_keyed_by_a_parameter_with_no_serialize_is_refused() {
    assert_eq!(
        map_key_refusal(
            "K",
            "pub struct Shelf<K: Ord> { pub by_key: BTreeMap<K, Parcel> }"
        ),
        [
            "model_schema: field `by_key`: `validate()` names each entry of this map by its key, \
             as serde writes it, and `K` declares no `Serialize` on `Shelf`. Write `K: Serialize` \
             on the type."
        ]
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_map_key_that_wraps_a_parameter_or_sits_in_an_enum_variant_is_refused() {
    let wrapped = map_key_refusal(
        "K",
        "struct Shelf<K: Ord> { by_bay: BTreeMap<Bay<K>, Parcel> }",
    );
    let variant = map_key_refusal(
        "K",
        "enum Stock<K: Ord> { Keyed { by_key: BTreeMap<K, Parcel> } }",
    );
    let nested = map_key_refusal(
        "K",
        "struct Shelf<K: Ord> { rows: Vec<(u8, BTreeMap<K, Parcel>)>, lone: BTreeMap<K, Parcel> }",
    );
    assert_eq!(
        [wrapped.len(), variant.len(), nested.len()],
        [1, 1, 2],
        "got: {wrapped:?} {variant:?} {nested:?}"
    );
    assert!(variant[0].contains("`Stock`"), "got: {variant:?}");
}

#[cfg(feature = "serde")]
#[test]
fn a_map_key_is_accepted_where_the_parameter_declares_serialize_or_the_walk_reads_no_key() {
    let accepted = [
        (
            "Key",
            "struct Rack<Key: Ord + Serialize> { by_key: BTreeMap<Key, Parcel> }",
        ),
        (
            "Key",
            "struct Rack<Key: Ord + serde::Serialize> { by_key: BTreeMap<Key, Parcel> }",
        ),
        (
            "Key",
            "struct Rack<Key> where Key: Ord + Serialize { by_key: BTreeMap<Key, Parcel> }",
        ),
        (
            "K",
            "struct Ledger<K: Ord> { counts_by_key: BTreeMap<K, u32> }",
        ),
        (
            "K",
            "struct Docks<K: Ord> { by_dock: BTreeMap<String, Parcel>, other: K }",
        ),
        (
            "K",
            "struct Skipped<K: Ord> { #[serde(skip)] by_key: BTreeMap<K, Parcel> }",
        ),
    ];
    for (parameter, source) in accepted {
        assert_eq!(
            map_key_refusal(parameter, source),
            Vec::<String>::new(),
            "for {source}"
        );
    }
}
