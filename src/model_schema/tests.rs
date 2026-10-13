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
use super::{WireLeaf, flatten_edge_guard_error, record_wire_leaves, record_zod_union_members};

#[cfg(feature = "zod")]
use super::{MergedOperand, SourceAbsence};

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
use crate::utils::{Declared, declared};
use crate::utils::{is_recorded_untagged_enum, record_untagged_enum};

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
    ("decode_with", "pub struct DecodePair(pub String, pub u32);"),
    ("decode_with", "pub struct DecodeWrapper(pub String);"),
    ("decode_with", "pub struct DecodeTags(pub Vec<String>);"),
    (
        "decode_with",
        "#[serde(transparent)] pub struct DecodeBrand(pub String);",
    ),
    (
        "decode_with, no_display",
        "#[serde(transparent)] pub struct DecodePinned(pub DecodeInner);",
    ),
    ("decode_with", "pub struct DecodeUnit;"),
    (
        "decode_with, default_types(T = String)",
        "pub struct DecodePage<T> { pub items: Vec<T>, pub total: u32 }",
    ),
    (
        "decode_with, default_types(T = String)",
        "#[serde(transparent)] pub struct DecodeWrapped<T>(pub T);",
    ),
    (
        "decode_with",
        "#[serde(transparent)] pub struct DecodeHolder { pub inner: String }",
    ),
    (
        "decode_with",
        "#[serde(transparent)] pub struct DecodeCurrent { pub inner: DecodeInner }",
    ),
    (
        "decode_with, default_types(T = String)",
        "#[serde(transparent)] pub struct DecodeSleeve<T> { pub inner: T }",
    ),
];

/// What `mongodb` adds for a flagged type's typed paths: the struct of them in its module, and on
/// the type the const that holds them.
#[cfg(feature = "serde")]
const TYPED_PATHS: [&str; 2] = ["pub struct MongoFields <", "pub const MONGO_FIELDS :"];

/// What `mongodb` adds for a flagged type's operations: in its module the error and the read they
/// answer, and on the type the reads, the count and the writes.
#[cfg(feature = "serde")]
const OPERATIONS: [&str; 14] = [
    "pub enum OperationError {",
    "pub struct Read < 'c , T , O > {",
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
    ("decode_with", "pub enum DecodeStatus { Draft, Published }"),
    (
        "decode_with",
        "pub enum DecodeOutline { Circle { radius: f64 }, Empty, Label(String), To(i32, i32) }",
    ),
    (
        "decode_with",
        "#[serde(tag = \"kind\")] pub enum DecodeFill { Clear, Solid { color: String }, Versioned(DecodeInner) }",
    ),
    (
        "decode_with",
        "#[serde(tag = \"kind\", content = \"data\")] pub enum DecodeStroke { Dashed { gap: u32 }, Hairline, Width(u32) }",
    ),
    (
        "decode_with",
        "#[derive(Deserialize)] #[serde(untagged)] pub enum DecodeContact { Email { address: String }, Versioned(DecodeInner) }",
    ),
    (
        "decode_with, default_types(T = String)",
        "pub enum DecodeAnswer<T> { Empty, Value(T) }",
    ),
    (
        "decode_with",
        "pub enum DecodeContour { #[serde(alias = \"Round\")] Circle { radius: f64 }, #[serde(alias = \"Blank\")] Empty, #[serde(skip)] Lost }",
    ),
    (
        "decode_with",
        "#[serde(tag = \"kind\")] pub enum DecodeCoating { #[serde(alias = \"Blank\")] Clear, #[serde(skip_deserializing)] Retired, Solid { color: String } }",
    ),
    (
        "decode_with",
        "#[serde(tag = \"kind\", content = \"data\")] pub enum DecodeDash { #[serde(alias = \"Dotted\")] Dashed { gap: u32 }, #[serde(skip_deserializing)] Faded(u32), Hairline }",
    ),
    (
        "decode_with",
        "#[derive(Deserialize)] #[serde(untagged)] pub enum DecodeChannel { Email { address: String }, #[serde(skip_deserializing)] Pager { number: i32 } }",
    ),
];

/// One type per source serde flattens into a struct, then one enum per form serde writes an enum
/// in whose struct variant flattens a field, each with the arguments it is declared under.
#[cfg(feature = "serde")]
const DECODE_WITH_FLATTEN_FORMS: [(&str, &str); 7] = [
    (
        "decode_with",
        "pub struct DecodeFlatEntry { #[serde(flatten)] pub audit: DecodeAudit, \
         #[serde(flatten, default, skip_serializing_if = \"Option::is_none\")] pub extra: Option<DecodeExtra>, \
         #[serde(flatten)] pub fill: DecodeFill, pub title: String }",
    ),
    (
        "decode_with",
        "pub struct DecodeFlatBag { #[serde(flatten)] pub extra: HashMap<String, i32>, pub title: String }",
    ),
    (
        "decode_with, default_types(T = DecodeBody)",
        "pub struct DecodeFlatEnvelope<T> { #[serde(flatten)] pub body: T, pub id: String }",
    ),
    (
        "decode_with",
        "pub enum DecodeFlatOutline { Made { #[serde(flatten)] audit: DecodeAudit, title: String }, Gone }",
    ),
    (
        "decode_with",
        "#[serde(tag = \"kind\")] pub enum DecodeFlatFill { Made { #[serde(flatten)] audit: DecodeAudit }, Gone }",
    ),
    (
        "decode_with",
        "#[serde(tag = \"kind\", content = \"data\")] pub enum DecodeFlatStroke { Made { #[serde(flatten)] audit: DecodeAudit }, Gone }",
    ),
    (
        "decode_with",
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
    ("&'a String", false),
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

/// Runs the guard over the sole field of `item`, deriving `is_optional` and `is_nullable` the way
/// the generator does rather than re-sniffing the type.
#[cfg(feature = "serde")]
fn guard_result(item: &syn::ItemStruct) -> Result<(), syn::Error> {
    let field = item.fields.iter().next().unwrap();
    let field_name = field
        .ident
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let field_def = get_field_def(&field_name, &field.ty, "");
    let is_nullable = super::parse_model_schema_prop_attributes(&field.attrs).nullable;
    check_optional_field_serialization(field, field_def.is_optional(), is_nullable)
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
    let field_name = field
        .ident
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let field_def = get_field_def(&field_name, &field.ty, "");
    check_omitted_key_is_readable(
        field,
        field_def.is_optional(),
        has_serde_default(&item.attrs),
    )
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
    let (_, _, zod_parts, _, _, errors, _, _) = collect_untagged_members(
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
    let (_, _, _, _, _, errors, validation_fns, _) = collect_untagged_members(
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
    let (_, _, _, _, _, errors, validation_fns, _) =
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
    let (_, _, _, _, _, errors, validation_fns, arms) = collect_untagged_members(
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
fn untagged_member_holding_an_opaque_value_stays_permissive() {
    let values = untagged_member_values(syn::parse_quote! {
        enum Untagged {
            Raw { raw: serde_json::Value },
        }
    });
    assert!(
        values[0].contains("serde_json :: json ! ({ })"),
        "got: {}",
        values[0]
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn untagged_member_holding_an_unsupported_map_value_emits_only_the_compile_error() {
    let values = untagged_member_values(syn::parse_quote! {
        enum Untagged {
            Rows { rows: HashMap<String, (String, u32)> },
        }
    });
    assert_eq!(
        values[0].matches("compile_error !").count(),
        1,
        "got: {}",
        values[0]
    );
    assert!(
        values[0].contains("model_schema: field `rows`: a tuple is not supported as a map value"),
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
        &super::MapMemberRejection::Tuple(proc_macro2::Span::call_site()),
    )
    .to_string();
    assert!(
        rendered.starts_with(":: core :: compile_error !"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains("model_schema: variant `rows`: a tuple is not supported as a map value"),
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
            let name = field
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            get_field_def(&name, &field.ty, "")
        })
        .collect()
}

/// The content sink's caret, at both contents an externally tagged variant can carry.
#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn a_refused_external_content_points_at_the_written_content() {
    const SOURCE: &str = "enum Tagged { \
        Rows(HashMap<String, (u32, u32)>), \
        Multi(String, HashMap<String, (u32, u32)>) }";

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
        Rows(HashMap<String, (u32, u32)>), \
        Multi(String, HashMap<String, (u32, u32)>) }";

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
        ("HashMap<String, (u32, u32)>", "an untagged map member"),
        (
            "(String, HashMap<String, (u32, u32)>)",
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

#[cfg(feature = "serde")]
#[test]
fn internally_tagged_newtype_over_a_scalar_is_rejected() {
    for (source, shape) in [
        ("enum E { V(String) }", "a string"),
        ("enum E { V(bool) }", "a boolean"),
        ("enum E { V(i64) }", "an integer"),
        ("enum E { V(f64) }", "a float"),
        ("enum E { V((u32, u32)) }", "a tuple"),
        ("enum E { V(Vec<Payload>) }", "a sequence"),
        ("enum E { V(Option<Payload>) }", "an optional"),
    ] {
        let errors = internal_guard_errors(&syn::parse_str(source).unwrap());
        assert_eq!(errors.len(), 1, "got: {errors:?} for {source}");
        assert!(errors[0].contains("compile_error"), "got: {}", errors[0]);
        assert!(errors[0].contains("variant `V`"), "got: {}", errors[0]);
        assert!(
            errors[0].contains(&format!("containing {shape}")),
            "expected serde's own wording for {source}. Got: {}",
            errors[0]
        );
    }
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

/// An empty tuple variant carries nothing: serde writes the tag alone, which is the unit arm.
#[cfg(feature = "serde")]
#[test]
fn internally_tagged_empty_tuple_variant_is_accepted() {
    let errors = internal_guard_errors(&syn::parse_quote! {
        enum E { V() }
    });
    assert!(errors.is_empty(), "got: {errors:?}");
}

/// A name is not the criterion — what serde writes for it is.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn internally_tagged_newtype_over_a_registered_plain_enum_is_rejected() {
    register_alias_info("Hue", "Hue", "hue_schema", AliasKind::EnumMembers);
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

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn internally_tagged_newtype_over_a_non_enum_or_unknown_name_is_accepted() {
    register_alias_info(
        "Payload",
        "Payload",
        "payload_schema",
        AliasKind::NoEnumMembers,
    );
    for source in ["enum E { V(Payload) }", "enum E { V(NeverRegistered) }"] {
        let errors = internal_guard_errors(&syn::parse_str(source).unwrap());
        assert!(errors.is_empty(), "got: {errors:?} for {source}");
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn flattening_a_registered_plain_enum_is_rejected() {
    register_alias_info("Hue", "Hue", "hue_schema", AliasKind::EnumMembers);
    register_alias_info(
        "Payload",
        "Payload",
        "payload_schema",
        AliasKind::NoEnumMembers,
    );

    let rejected: syn::Field = syn::parse_quote! { pub tone: Hue };
    let error = super::flattened_field_guard_error(&rejected, "Holder")
        .map(|tokens| tokens.to_string())
        .unwrap_or_default();
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

/// Each `cfg` written above the function `named` of `source`, or above its import of it.
fn gates_above(source: &str, named: &str) -> Vec<String> {
    let mut gates = Vec::new();
    for item in &syn::parse_file(source).unwrap().items {
        let attrs = if let syn::Item::Fn(function) = item
            && function.sig.ident == named
        {
            &function.attrs
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
    let name = field
        .ident
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
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
    let name = field
        .ident
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
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
        false,
        false,
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
        false,
        false,
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
        false,
        false,
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
        false,
        false,
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
        false,
        false,
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
        false,
        false,
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
        false,
        false,
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
        false,
        true,
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
        false,
        true,
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
        false,
        true,
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
            organization_id: Arc<Slug>,
        }
    };
    let collected = super::collect_struct_fields(
        &mut item.fields,
        Some("camelCase"),
        Some("enrolment_schema"),
        "Enrolment",
        &syn::Generics::default(),
        false,
        true,
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
        false,
        true,
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
        false,
        super::derives_deserialize(&item.attrs),
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
        false,
        false,
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
        false,
        false,
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
    let (_, _, _, _, _, errors, _, _) = collect_untagged_members(
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
    let name = field
        .ident
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let written = get_field_def(&name, &field.ty, "");
    let mut rendered = written.clone();
    rendered.erase_type_parameters(parameters);
    apply_serde_key_omission(&mut rendered, field);
    let meta = super::parse_model_schema_prop_attributes(&field.attrs);
    super::collect_field_guard_errors(field, &rendered, &written, &name, &meta, Vec::new())
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn field_prop_guard_errors(item: &syn::ItemStruct) -> Vec<String> {
    field_prop_guard_errors_in_scope(item, &[])
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
    let name = field
        .ident
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
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
            delegate.contains(
                "let examples_json = serde_json :: to_string (& Self :: schema_examples ()) . \
                 unwrap () ;"
            ),
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
            "z.lazy(() => (<Reached$ extends SomeType>(reached$: Reached$) => \
             z.union([z.null().transform(() => undefined), reached$, z.undefined()]).prefault(undefined))(DocumentId$SchemaFactory(z.string())))",
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
        "z.lazy(() => DocumentId$SchemaFactory(z.number().int()))"
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
        &syn::parse_str("HashMap<String, (u32, u32)>").unwrap(),
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
        rendered.contains("model_schema: `DocumentId`: a tuple is not supported as a map value"),
        "got: {rendered}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_brand_inner_points_at_the_written_inner() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct DocumentId(pub HashMap<String, (u32, u32)>);").unwrap();
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
        super::BrandedJsonInner::Chrono("date-time"),
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
        "pub type Rows = HashMap<String, (u32, u32)>;",
        "pub type Rows = Vec<HashMap<String, (u32, u32)>>;",
        "pub type Rows = (String, HashMap<String, (u32, u32)>);",
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
            tokens.contains(
                "model_schema: type alias `Rows`: a tuple is not supported as a map value"
            ),
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
    let alias: syn::ItemType =
        syn::parse_str("pub type Rows = HashMap<String, (u32, u32)>;").unwrap();
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
        pub type RowsData = HashMap<String, (u32, u32)>;
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

#[cfg(feature = "jsonschema")]
#[test]
fn a_brand_over_a_map_with_a_tuple_value_is_refused() {
    for inner in [
        quote::quote! { HashMap<String, (u32, u32)> },
        quote::quote! { Vec<HashMap<String, (u32, u32)>> },
        quote::quote! { (String, HashMap<String, (u32, u32)>) },
    ] {
        let errors = branded_errors(&syn::parse_quote! {
            #[serde(transparent)]
            struct Wrap(pub #inner);
        });
        assert_eq!(errors.len(), 1, "for {inner}, got: {errors:?}");
        for needle in [
            "compile_error",
            "model_schema: type `Wrap`",
            "a tuple is not supported as a map value",
        ] {
            assert!(
                errors[0].contains(needle),
                "{needle} missing for {inner}: {}",
                errors[0]
            );
        }
    }
}

/// The refusal points at the tuple that earned it, not at the map or the whole inner.
#[cfg(feature = "jsonschema")]
#[test]
fn a_brand_slot_value_refusal_is_spanned_on_the_tuple() {
    let item: syn::ItemStruct =
        syn::parse_str("#[serde(transparent)] struct Wrap(pub HashMap<String, (u32, u32)>);")
            .unwrap();
    let refusals = branded_guard_errors(&item, &super::ModelSchemaArgs::default());
    assert_eq!(refusals.len(), 1, "got: {}", refusals.len());
    assert_eq!(
        refusals[0].span().source_text().as_deref(),
        Some("(u32, u32)")
    );
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
        ("serde_json::Value", "opaque"),
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
        ("serde_json::Value", Some("opaque")),
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
        ("PublishesItsParameter<serde_json::Value>", "opaque"),
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
        "serde_json::Value",
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
        quote::quote! { decode_with },
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

/// `decode_with` reads as a flag: written alone, or with the boolean that says which way it is set.
#[test]
fn decode_with_is_read_as_a_flag() {
    for (written, set) in [
        ("decode_with", true),
        ("decode_with = true", true),
        ("decode_with = false", false),
        ("", false),
    ] {
        let args = super::parse_model_schema_args(syn::parse_str(written).unwrap());
        assert!(args.arg_rejection.is_none(), "for {written}");
        assert_eq!(args.decode_with, set, "for {written}");
    }
    assert!(args_rejection(quote::quote! { decode_with = 3 }).is_some());
}

/// The whole expansion of `source` under `#[model_schema(...)]` written with `args`.
fn expansion_under(args: &str, source: &str) -> String {
    super::exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    )
    .to_string()
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
                "type `Named`: `{check}` is supported only on a branded newtype"
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

/// An untagged enum writes a unit variant as `null`, and refuses it for that.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_unit_variant_has_no_name_to_measure() {
    let expanded = expansion_under(
        "minLength = 9",
        "#[serde(untagged)] pub enum Loose { Nothing, Text(String) }",
    );
    assert!(!expanded.contains("which breaks"), "got: {expanded}");
    assert!(
        expanded.contains("`Nothing` is a unit variant"),
        "got: {expanded}"
    );
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
                     variant written with no data and each field written as a string or a path, \
                     and `Shape` has neither"
                )),
                "got: {expanded}"
            );
        }
    }
    assert!(
        include_str!("../../README.md").contains(
            "error: model_schema: type `Shape`: `minLength` reaches nothing here: an enum's check \
             holds each variant written with no data and each field written as a string or a \
             path, and `Shape` has neither\n"
        ),
        "the README no longer shows the refusal"
    );
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
        "decode_with",
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
            "pub enum Issue < V > {",
            "pub enum Resolution {",
            "pub fn value_leaf < 'a , T , I , R , W > (",
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
        "decode_with",
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
            "pub enum Issue < V > {",
            "pub enum Resolution {",
            "pub fn value_leaf < 'a , T , I , R , W > (",
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

#[cfg(feature = "serde")]
#[test]
fn a_type_without_decode_with_gets_no_typed_path_and_no_operation() {
    for (args, source) in DECODE_WITH_STRUCT_SHAPES
        .iter()
        .chain(&DECODE_WITH_ENUM_FORMS)
    {
        let unflagged = args
            .trim_start_matches("decode_with")
            .trim_start_matches(", ");
        let expanded = expansion_under(unflagged, source);
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
        ] {
            assert!(
                !expanded.contains(added),
                "for {source}, found `{added}` in: {expanded}"
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
            matches!(declared(&name), Some(Declared::OneValue)),
            one_value,
            "for {source}"
        );
        assert_eq!(
            matches!(declared(&name), Some(Declared::Model)),
            !one_value,
            "for {source}"
        );
    }
}

#[cfg(feature = "mongodb")]
#[test]
fn a_type_declared_above_is_held_by_how_serde_writes_it() {
    let _brand = expansion_under(
        "decode_with",
        "#[serde(transparent)] pub struct HeldBrand(pub String);",
    );
    let _keyed = expansion_under("decode_with", "pub struct HeldKeyed { pub held: String }");
    let expanded = expansion_under(
        "decode_with",
        "pub struct HeldBy { pub brand: HeldBrand, pub brands: Vec<HeldBrand>, \
         pub keyed: HeldKeyed, pub keyeds: Vec<HeldKeyed> }",
    );
    for emitted in [
        "pub const fn brand (& self) -> self :: Field < Root , super :: HeldBrand > {",
        "pub const fn brands (& self) -> self :: ListField < Root , super :: HeldBrand > {",
        "pub const fn keyed (& self) -> self :: Model < Root , super :: HeldKeyed > {",
        "pub const fn keyeds (& self) -> self :: ModelList < Root , super :: HeldKeyed > {",
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
        "decode_with, minLength = 3",
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
    let expanded = expansion_under("decode_with, default_types(T = String)", source);
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
        "decode_with",
        "pub type DecodeVersions = Vec<DecodeVersion>;",
    );
    assert!(
        expanded.contains(
            "model_schema: type `DecodeVersions`: `#[model_schema(decode_with)]` cannot be written \
             on a type alias: `DecodeVersions` is another name for `Vec<DecodeVersion>`, and the \
             methods would have to be added to that type. Write the flag on the model types the \
             alias reaches."
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
        let expanded = expansion_under("decode_with", source);
        assert!(
            expanded.contains(
                "model_schema: type `DecodeNamed`: `#[model_schema(decode_with)]` cannot be \
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
        "model_schema: type `DecodeLabelled`: `#[model_schema(decode_with)]` cannot be written on \
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
        let expanded = expansion_under("decode_with", source);
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
        let expanded = expansion_under("decode_with", source);
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
        let expanded = expansion_under("decode_with", source);
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
        let expanded = expansion_under("decode_with", source);
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
        let args = super::parse_model_schema_args(syn::parse_str("decode_with").unwrap());
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
        assert_eq!(
            expansion_under("decode_with = false", source),
            unflagged,
            "for {source}"
        );
        assert!(
            !unflagged.contains("decode_with") && !unflagged.contains("from_value_with"),
            "for {source}, got: {unflagged}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn decode_with_generates_the_entry_point_the_walker_and_the_callback_types() {
    let source = "pub struct DecodeFlagged { pub name: String }";
    let flagged = expansion_under("decode_with", source);
    assert_eq!(expansion_under("decode_with = true", source), flagged);
    for emitted in [
        "pub mod decode_flagged_schema {",
        "pub fn from_value_with < F > (",
        "pub fn from_value_piped (",
        "fn decode_with_value_report (",
        "pub fn decode_with_value_issues < I > (",
        "pub fn decode_with_value_named (",
        "pub fn decode_with_value_fields < 'a , I > (",
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
            flagged.contains(emitted),
            "missing `{emitted}`, got: {flagged}"
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
            flagged.contains(under_mongodb),
            cfg!(feature = "mongodb"),
            "for `{under_mongodb}`, got: {flagged}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn nothing_decode_with_emits_suppresses_a_lint() {
    let named_fields = (
        "decode_with",
        "pub struct DecodeQuiet { pub name: String, pub spot: (String, u32), pub tags: Vec<String> }",
    );
    for (args, source) in DECODE_WITH_STRUCT_SHAPES
        .into_iter()
        .chain([named_fields])
        .chain(DECODE_WITH_ENUM_FORMS)
        .chain(DECODE_WITH_FLATTEN_FORMS)
    {
        let flagged = expansion_under(args, source);
        for suppression in ["allow", "expect", "hidden", "cfg_attr"] {
            assert!(
                !flagged.contains(&format!("# [{suppression}"))
                    && !flagged.contains(&format!("({suppression})")),
                "for {source}, found `{suppression}`, got: {flagged}"
            );
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn what_decode_with_emits_is_written_for_the_lints_a_consumer_denies() {
    let named_fields = (
        "decode_with",
        "pub struct DecodeLinted { pub name: String }",
    );
    for (args, source) in DECODE_WITH_STRUCT_SHAPES
        .into_iter()
        .chain([named_fields])
        .chain(DECODE_WITH_ENUM_FORMS)
        .chain(DECODE_WITH_FLATTEN_FORMS)
    {
        let flagged = expansion_under(args, source);
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
                flagged.contains(&format!("# [non_exhaustive] {declared}")),
                "for {source}, missing `{declared}`, got: {flagged}"
            );
        }
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
        "decode_with",
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
    // A model in an `Arc` and in a fixed array is one the type's unjudged read does not reach, so
    // its reader stays in the schema module.
    let source = "#[derive(Deserialize)] pub struct DecodeTicket { pub issue: Arc<Issue>, pub trail: [Segment; 2] }";
    let flagged = expansion_under("decode_with", source);
    for scoped in [
        "Result < Arc < super :: Issue > , D :: Error >",
        "< Arc < super :: Issue > > :: deserialize (deserializer)",
        "Result < [super :: Segment ; 2] , D :: Error >",
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
        ("HashMap<(u8, u8), u32>", "serde writes `(_, _)`"),
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
/// field written after it to hold the type.
#[test]
fn a_type_the_crate_declares_as_map_is_what_a_field_written_after_it_holds() {
    let written: syn::Type = syn::parse_str("Map<String, u32>").unwrap();
    assert!(matches!(
        get_field_def("held", &written, "").field_type,
        FieldDefType::Map(..)
    ));
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
}

/// The `compile_error!` tokens `source` earns for the names the `indexmap` feature reads. Parsed
/// from text so each refusal's span can be read back as the source it points at.
#[cfg(not(feature = "indexmap"))]
fn indexmap_refusals(source: &str) -> Vec<proc_macro2::TokenStream> {
    super::indexmap_feature_errors(&syn::parse_str(source).unwrap())
}

/// `sentence` as the `compile_error!` it is told through.
#[cfg(not(feature = "indexmap"))]
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
        super::indexmap_feature_message("IndexMap", "a map")
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
    let tokens = super::build_branded_display_impl(&item.generics, &item.ident, field);
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
    let tokens = super::build_branded_display_impl(&item.generics, &item.ident, field);
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
    let validation =
        super::build_branded_validation(&args, &[], item.fields.iter().next().unwrap()).unwrap();
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
    let validation =
        super::build_branded_validation(&args, &[], item.fields.iter().next().unwrap()).unwrap();
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
        "HashMap<Slot, (String, u32)>",
        "HashMap<Slot, HashMap<String, (String, u32)>>",
        "HashMap<Slot, Vec<HashMap<String, (String, u32)>>>",
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
            tokens.contains("model_schema: field `m`: a tuple is not supported as a map value"),
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

/// A chrono value keeps the format it carries in field position, as it does under a `String` key.
#[cfg(all(feature = "chrono", feature = "jsonschema"))]
#[test]
fn a_chrono_enum_keyed_map_value_keeps_its_format() {
    for (map_type, format) in [
        ("HashMap<Slot, NaiveDate>", "date"),
        ("HashMap<Slot, NaiveTime>", "time"),
        ("HashMap<Slot, NaiveDateTime>", "date-time"),
        ("HashMap<Slot, DateTime<Utc>>", "date-time"),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!(
                r#"let value_schema = serde_json :: json ! ({{ "type" : "string" , "format" : "{format}" }}) ;"#
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
fn a_chrono_string_keyed_map_value_keeps_its_format() {
    for (map_type, format) in [
        ("HashMap<String, NaiveDate>", "date"),
        ("HashMap<String, NaiveTime>", "time"),
        ("HashMap<String, NaiveDateTime>", "date-time"),
        ("HashMap<String, DateTime<Utc>>", "date-time"),
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(&format!(
                r#""additionalProperties" : {{ "type" : "string" , "format" : "{format}" }}"#
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

#[cfg(feature = "jsonschema")]
#[test]
fn an_unsupported_string_keyed_map_value_emits_only_the_compile_error() {
    let tokens = map_field_schema("HashMap<String, (String, u32)>").to_string();
    assert!(
        tokens.starts_with(":: core :: compile_error !"),
        "got: {tokens}"
    );
    assert!(!tokens.contains("properties . insert"), "got: {tokens}");
    assert!(
        tokens.contains("model_schema: field `m`: a tuple is not supported as a map value"),
        "got: {tokens}"
    );
}

/// The caret has to land on the tokens the author edits.
#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_map_value_points_at_the_written_value() {
    let tokens = sole_field_json_schema("pub struct Rows { pub m: HashMap<String, (u32, u32)> }");
    assert_points_only_at(&tokens, "(u32, u32)", "a map value");
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_tuple_element_map_value_points_at_the_written_value() {
    let tokens =
        sole_field_json_schema("pub struct Rows { pub t: (String, HashMap<String, (u32, u32)>) }");
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
fn a_nested_chrono_map_value_keeps_its_format() {
    for (map_type, expected) in [
        (
            "HashMap<String, HashMap<String, NaiveDate>>",
            r#"{ "type" : "object" , "additionalProperties" : { "type" : "string" , "format" : "date" } }"#,
        ),
        (
            "HashMap<String, HashMap<String, NaiveTime>>",
            r#"{ "type" : "object" , "additionalProperties" : { "type" : "string" , "format" : "time" } }"#,
        ),
        (
            "HashMap<String, Vec<HashMap<String, DateTime<Utc>>>>",
            r#"{ "type" : "array" , "items" : { "type" : "object" , "additionalProperties" : { "type" : "string" , "format" : "date-time" } } }"#,
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

#[cfg(feature = "jsonschema")]
#[test]
fn a_nested_map_under_an_unenumerable_key_still_renders_as_an_object() {
    for map_type in [
        "HashMap<String, HashMap<u32, String>>",
        "HashMap<String, HashMap<Wrapper<Slot>, String>>",
    ] {
        let tokens = map_field_schema(map_type).to_string();
        assert!(
            tokens.contains(
                r#""additionalProperties" : { "type" : "object" , "additionalProperties" : true }"#
            ),
            "for {map_type}, got: {tokens}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_unsupported_nested_map_value_emits_only_the_compile_error() {
    for map_type in [
        "HashMap<String, HashMap<String, (String, u32)>>",
        "HashMap<String, Vec<HashMap<String, (String, u32)>>>",
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
            tokens.contains("model_schema: field `m`: a tuple is not supported as a map value"),
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
        &syn::parse_str("HashMap<String, (u32, u32)>").unwrap(),
        "",
    );
    let rendered = super::argument_json_schema_value("IdType", &argument).to_string();
    assert!(
        rendered.starts_with(":: core :: compile_error !"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains("model_schema: `IdType`: a tuple is not supported as a map value"),
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
        sole_field_arguments("pub struct Holder { pub boxed: Boxed<HashMap<String, (u32, u32)>> }");
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
    let filling: syn::Type = syn::parse_str("HashMap<String, (u32, u32)>").unwrap();
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
        type_span: proc_macro2::Span::call_site(),
    }
}

/// The `Vec<u32>` value every wrapper spelling of the same thing is held against.
#[cfg(feature = "jsonschema")]
fn parsed_u32_vec_value() -> super::FieldDef {
    super::get_field_def("items", &syn::parse_quote!(Vec<u32>), "")
}

#[cfg(feature = "jsonschema")]
#[test]
fn every_sequence_wrapper_describes_as_the_vec_of_its_element() {
    let expected = super::build_field_type_schema(&parsed_u32_vec_value(), "items").to_string();
    for wrapper in SEQUENCE_WRAPPERS {
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
    let parsed = parsed_u32_vec_value();
    let expected_member = super::build_map_member_schema(&parsed).unwrap().to_string();
    let expected_element = super::build_tuple_element_json_schema(&parsed)
        .unwrap()
        .to_string();
    for wrapper in SEQUENCE_WRAPPERS {
        let value = wrapped_u32_value(wrapper);
        assert_eq!(
            super::build_map_member_schema(&value).unwrap().to_string(),
            expected_member,
            "for: {wrapper}"
        );
        assert_eq!(
            super::build_tuple_element_json_schema(&value)
                .unwrap()
                .to_string(),
            expected_element,
            "for: {wrapper}"
        );
    }
}

#[cfg(all(feature = "jsonschema", feature = "serde"))]
#[test]
fn every_sequence_wrapper_describes_as_the_vec_of_its_element_in_an_untagged_member() {
    let expected = super::field_json_schema_value(&parsed_u32_vec_value()).to_string();
    for wrapper in SEQUENCE_WRAPPERS {
        assert_eq!(
            super::field_json_schema_value(&wrapped_u32_value(wrapper)).to_string(),
            expected,
            "for: {wrapper}"
        );
    }
}

/// One value per arm of the untagged-member dispatch, labelled as it was written. Built at no array
/// level, so each holds exactly the tokens its arm emits before the array wrap sees them.
#[cfg(all(feature = "jsonschema", feature = "serde"))]
fn untagged_member_dispatch_values() -> Vec<(&'static str, super::FieldDef)> {
    let parsed: [(&'static str, syn::Type); 6] = [
        ("MetricTag", syn::parse_quote!(MetricTag)),
        ("String", syn::parse_quote!(String)),
        ("u32", syn::parse_quote!(u32)),
        ("f64", syn::parse_quote!(f64)),
        ("bool", syn::parse_quote!(bool)),
        ("serde_json::Value", syn::parse_quote!(serde_json::Value)),
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
        let mut arrayed = value.clone();
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
        "(String, HashMap<String, (u32, u32)>)",
        "(String, Vec<HashMap<String, (u32, u32)>>)",
        "(String, (u32, HashMap<String, (u32, u32)>))",
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
            tokens.contains("model_schema: field `t`: a tuple is not supported as a map value"),
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
        assert_eq!(
            inserted_field_value(wrapped_type),
            format!(r#"serde_json :: json ! ({{ "type" : "array" , "items" : {item} }})"#),
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

/// A map named without a sequence wrapper describes exactly as it always has, on every key path.
#[cfg(feature = "jsonschema")]
#[test]
fn an_unwrapped_map_field_keeps_the_object_it_has_always_described_as() {
    let string_keyed = r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "integer" } })"#;
    for (map_type, expected) in [
        ("HashMap<String, u64>", string_keyed),
        ("BTreeMap<String, u64>", string_keyed),
        (
            "HashMap<u32, u64>",
            r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : true })"#,
        ),
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
            r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : { "type" : "integer" } })"#,
        ),
        (
            "Option<HashMap<u32, u64>>",
            r#"serde_json :: json ! ({ "type" : "object" , "additionalProperties" : true })"#,
        ),
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
    let mut optional_vec = parsed_u32_vec_value();
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
        enum_args: None,
        member_steps: &steps,
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

/// `str` and a list are the borrows a bound is read through.
#[cfg(feature = "serde")]
#[test]
fn a_reference_to_anything_but_str_or_a_list_has_no_shape() {
    for spelling in [
        "&'a String",
        "&'a Option<Vec<String>>",
        "&'a u32",
        "&'a Path",
        "&'a Slug",
        "&'a &'a str",
        "Option<&'a String>",
    ] {
        let ty: syn::Type = syn::parse_str(spelling).unwrap();
        assert!(
            constrained_shape(&ty).is_none(),
            "spelling {spelling} should reach no constrainable value"
        );
    }
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
        "z.tuple([z.string(), z.number().int()])"
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
        whole_tuple(&["HashMap<String, (u32, u32)>"]),
        whole_tuple(&["String", "HashMap<String, (u32, u32)>"]),
    ] {
        let rendered = tuple_struct_json_body("Pair", &shape).to_string();
        assert!(
            rendered.starts_with(":: core :: compile_error !"),
            "Got: {rendered}"
        );
        assert!(
            rendered.contains("model_schema: `Pair`: a tuple is not supported as a map value"),
            "Got: {rendered}"
        );
        assert!(!rendered.contains("prefixItems"), "Got: {rendered}");
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_refused_tuple_struct_slot_points_at_that_slot_alone() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Pair(pub String, pub HashMap<String, (u32, u32)>);").unwrap();
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
    let (_, _, _, merge_parts, _, errors, _, _) = collect_untagged_members(
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
    let (_, _, _, merge_parts, _, errors, _, _) = collect_untagged_members(
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
    let (_, _, _, merge_parts, _, errors, _, _) = collect_untagged_members(
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
fn a_direct_flatten_of_an_object_an_unregistered_name_or_an_array_stays_admitted() {
    seed_registered_wire("DirectDoc", AliasKind::NoEnumMembers, None);
    seed_registered_wire("DirectCount", AliasKind::NoEnumMembers, Some("integer"));
    for admitted in [
        syn::parse_quote! { #[serde(flatten)] base: DirectDoc },
        syn::parse_quote! { #[serde(flatten)] base: NeverRegisteredDirectly },
        syn::parse_quote! { #[serde(flatten)] counts: Vec<DirectCount> },
    ] {
        let field: syn::Field = admitted;
        assert!(
            direct_flatten_error(&field).is_none(),
            "got a rejection for {}",
            quote::ToTokens::to_token_stream(&field)
        );
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_direct_flatten_of_a_plain_enum_is_left_to_the_guard_written_for_it() {
    seed_registered_wire("DirectHue", AliasKind::EnumMembers, Some("string"));
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
        refusal
            .contains("`#[serde(flatten)]` of `WireExtFlatChoice` writes a union member that is"),
        "got: {refusal}"
    );
    assert!(
        refusal.contains("its branch 2.1 describes a `string`, which has no members to merge"),
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

#[cfg(feature = "jsonschema")]
#[test]
fn a_flatten_source_with_no_name_of_its_own_keeps_the_placeholder() {
    let ty: syn::Type = syn::parse_str("serde_json::Value").unwrap();
    let held = super::get_field_def("held", &ty, "");

    let source = super::flatten_merged_source(&held);

    assert_eq!(source.label, "held");
    assert_eq!(
        source.value.to_string(),
        "serde_json :: json ! ({ \"type\" : \"object\" })"
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

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_flattened_variant_field_over_a_plain_enum_is_refused() {
    register_alias_info(
        "VariantHue",
        "VariantHue",
        &ident_schema_module_name("VariantHue"),
        AliasKind::EnumMembers,
    );
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
    let (_, _, _, merge_parts, _, errors, _, _) = collect_untagged_members(
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

/// The members one untagged variant collected, beside the ones it flattens.
#[cfg(feature = "serde")]
fn collected_untagged_variant_fields(mut item: syn::ItemEnum) -> (Vec<String>, Vec<String>) {
    let variant = item.variants.first_mut().unwrap();
    let walked = super::collect_untagged_variant_members(
        variant,
        "Probe",
        UNTAGGED_MODULE,
        &[],
        None,
        &super::ModelSchemaArgs::default(),
        &[],
    );
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
    let walked = super::collect_untagged_variant_members(
        variant,
        "Probe",
        UNTAGGED_MODULE,
        &[],
        None,
        &super::ModelSchemaArgs::default(),
        &[],
    );
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

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_flattened_untagged_variant_field_over_a_plain_enum_is_refused() {
    register_alias_info(
        "UntaggedVariantHue",
        "UntaggedVariantHue",
        &ident_schema_module_name("UntaggedVariantHue"),
        AliasKind::EnumMembers,
    );
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
    let (_, _, _, merge_parts, _, errors, _, _) = collect_untagged_members(
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

/// `Foo()` is refused on the variant under the default and the adjacent tagging alike.
#[test]
fn an_empty_tuple_variant_is_refused_on_the_variant() {
    for source in [
        "pub enum Choice { Unit, Empty(), Data(i32) }",
        "#[serde(tag = \"kind\", content = \"body\")] pub enum Choice { Unit, Empty(), Data(i32) }",
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
    assert_eq!(refusals.len(), 1, "got: {refusals:?}");
    assert_eq!(
        located_source_texts(&refusals[0])
            .last()
            .map(String::as_str),
        Some("[model_schema_prop(pattern = \"^a$\")]")
    );
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
        false,
        false,
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
            "decode_with",
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
