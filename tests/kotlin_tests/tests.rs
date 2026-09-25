//! Tests for the Kotlin type and `kotlinx.serialization` codec backend (`kotlin` feature).
//!
//! Each `#[model_schema]` item earns `kotlin_definition()` inside a `{snake_case}_kotlin` module
//! beside it (never a direct inherent `impl` — see `features::kotlin::kotlin_module_tokens`).

use std::collections::HashMap;

#[cfg(feature = "object_id")]
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use tixschema::model_schema;

// ---------------------------------------------------------------------------------------------
// Struct: a renamed key and an optional field — the design's own worked example.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowRequest {
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

// ---------------------------------------------------------------------------------------------
// The width table.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Widths {
    pub arch_signed: isize,
    pub double: f64,
    pub flag: bool,
    pub items: Vec<i32>,
    pub large_signed: i64,
    pub letter: char,
    pub map: HashMap<String, i32>,
    pub medium_signed: i32,
    pub medium_unsigned: u32,
    pub single: f32,
    pub small_signed: i16,
    pub small_unsigned: u16,
    pub text: String,
    pub tiny_signed: i8,
    pub tiny_unsigned: u8,
}

// ---------------------------------------------------------------------------------------------
// Plain enum.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Active,
    Inactive,
    Pending,
}

// ---------------------------------------------------------------------------------------------
// Internally tagged enum.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum WindowError {
    NotFound,
    RateLimited { retry_after_ms: i64 },
}

// ---------------------------------------------------------------------------------------------
// Adjacently tagged enum (`tag` + `content`).
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DynamicValue {
    Flag(bool),
    Nothing,
    Number(i64),
}

// ---------------------------------------------------------------------------------------------
// Externally tagged enum (serde's own default).
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExternalTagged {
    Ping { nonce: i32 },
    Pong { nonce: i32 },
}

// A unit variant beside a data-carrying one: the externally tagged shape's bare-string wire form
// for the unit case, round-tripped through the same reader as the data variant's own object.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimpleChoice {
    None,
    Value(String),
}

// ---------------------------------------------------------------------------------------------
// Untagged enum.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DateValue {
    Epoch(i64),
    Iso(String),
}

// ---------------------------------------------------------------------------------------------
// Tuple field.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldsTuple {
    pub coords: (String, i64),
}

// ---------------------------------------------------------------------------------------------
// A unit struct: no data, `{}` on the wire, `object` in Kotlin — see `test_unit_struct_is_an_object`.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping;

// ---------------------------------------------------------------------------------------------
// Non-string map keys: a numeric key and a plain-enum key.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Slot {
    North,
    South,
}

#[model_schema()]
pub type SlotAliasKey = Slot;

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapKeys {
    pub by_number: HashMap<i32, String>,
    pub by_slot: HashMap<Slot, String>,
}

// ---------------------------------------------------------------------------------------------
// Dates and `as_number`, `ObjectId`.
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dates {
    pub plain_date: chrono::NaiveDate,
    #[model_schema_prop(as_number)]
    pub zoned_ms: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "object_id")]
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HasId {
    pub id: ObjectId,
}

// ---------------------------------------------------------------------------------------------
// Branded newtype and a non-branded bare tuple struct — both `@JvmInline value class`.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);

// ---------------------------------------------------------------------------------------------
// The `name` override moving a type's published Kotlin name.
// ---------------------------------------------------------------------------------------------

#[model_schema(name = "RenamedWidget")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Widget {
    pub label: String,
}

// ---------------------------------------------------------------------------------------------
// A generic struct: `kotlinx.serialization` covers this through its own compiler plugin, at the
// bare type parameter — no factory or converter argument for this module to thread through.
// ---------------------------------------------------------------------------------------------

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wrapper<T> {
    pub value: T,
}

// ---------------------------------------------------------------------------------------------
// A generic enum with a unit variant: the sealed base declares its parameters `out`, and the unit
// variant implements it at `Nothing` rather than repeating an unbound `T`.
// ---------------------------------------------------------------------------------------------

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Choice<T> {
    None,
    Value(T),
}

// ---------------------------------------------------------------------------------------------
// The same generic enum, adjacently tagged: the constructor-injected serializer class carries the
// tag/content dispatch exactly as the non-generic shape does.
// ---------------------------------------------------------------------------------------------

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum AdjacentChoice<T> {
    None,
    Value(T),
}

// ---------------------------------------------------------------------------------------------
// A generic untagged enum: the try-each-variant serializer names each subclass's own constructor
// serializer instead of a reified lookup.
// ---------------------------------------------------------------------------------------------

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UntaggedChoice<T> {
    Text(String),
    Value(T),
}

// ---------------------------------------------------------------------------------------------
// Two type parameters: one constructor serializer per parameter.
// ---------------------------------------------------------------------------------------------

#[model_schema(default_types(L = String, R = i64))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

// ---------------------------------------------------------------------------------------------
// A type parameter reached through a `Vec` and through an `Option`.
// ---------------------------------------------------------------------------------------------

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManyChoice<T> {
    List(Vec<T>),
    Maybe(Option<T>),
}

// ---------------------------------------------------------------------------------------------
// A generic tuple struct: the same constructor-injected serializer class, on the tuple-struct path.
// Named `GenericPair` rather than `Pair` to avoid shadowing `kotlin.Pair` in the emitted file.
// ---------------------------------------------------------------------------------------------

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericPair<T>(pub T, pub u32);

// ---------------------------------------------------------------------------------------------
// `#[serde(flatten)]`: a flattened struct, a flattened `Option<Struct>`, and a flattened map — each
// earns a generated merging `KSerializer` beside the ordinary `@Serializable` data class.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit {
    pub id: String,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionalAudit {
    pub id: String,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub stamp: Option<Stamp>,
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaggedRecord {
    pub id: String,
    #[serde(flatten)]
    pub tags: HashMap<String, String>,
}

// A generic struct with a flattened field: `Tagged<T>` flattens a non-generic sibling over its own
// bare type parameter; `Envelope<T>` flattens a generic sibling at that same parameter.
#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tagged<T> {
    pub id: T,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<T> {
    #[serde(flatten)]
    pub body: Wrapper<T>,
    pub id: String,
}

#[test]
fn test_every_declared_type_is_constructible() {
    let window_request = WindowRequest {
        conversation_id: "abc".to_owned(),
        limit: Some(10_i32),
    };
    assert_eq!(window_request.limit, Some(10_i32));

    let widths = Widths {
        arch_signed: 0,
        double: 0.0,
        flag: true,
        items: Vec::new(),
        large_signed: 0,
        letter: 'x',
        map: HashMap::new(),
        medium_signed: 0,
        medium_unsigned: 0,
        single: 0.0,
        small_signed: 0,
        small_unsigned: 0,
        text: String::new(),
        tiny_signed: 0,
        tiny_unsigned: 0,
    };
    assert_eq!(widths.text, "");

    let statuses = [Status::Active, Status::Inactive, Status::Pending];
    assert_eq!(statuses.len(), 3);

    let errors = [
        WindowError::NotFound,
        WindowError::RateLimited {
            retry_after_ms: 500,
        },
    ];
    assert_eq!(errors.len(), 2);

    let dynamic_values = [
        DynamicValue::Flag(true),
        DynamicValue::Nothing,
        DynamicValue::Number(7),
    ];
    assert_eq!(dynamic_values.len(), 3);

    let external_tagged = [
        ExternalTagged::Ping { nonce: 1_i32 },
        ExternalTagged::Pong { nonce: 2_i32 },
    ];
    assert_eq!(external_tagged.len(), 2);

    let date_values = [DateValue::Epoch(0), DateValue::Iso(String::new())];
    assert_eq!(date_values.len(), 2);

    let holds_tuple = HoldsTuple {
        coords: ("a".to_owned(), 7),
    };
    assert_eq!(holds_tuple.coords.1, 7);

    let slots = [Slot::North, Slot::South];
    assert_eq!(slots.len(), 2);

    let alias_key: SlotAliasKey = Slot::North;
    assert_eq!(alias_key, Slot::North);

    let map_keys = MapKeys {
        by_number: HashMap::from([(1_i32, "one".to_owned())]),
        by_slot: HashMap::from([(Slot::North, "n".to_owned())]),
    };
    assert_eq!(map_keys.by_number.len(), 1);

    let correlation_id = CorrelationId("abc-123".to_owned());
    assert_eq!(correlation_id.0, "abc-123");

    let widget = Widget {
        label: "a widget".to_owned(),
    };
    assert_eq!(widget.label, "a widget");

    let wrapper = Wrapper {
        value: "wrapped".to_owned(),
    };
    assert_eq!(wrapper.value, "wrapped");

    let choices = [Choice::Value("x".to_owned()), Choice::<String>::None];
    assert_eq!(choices.len(), 2);

    let audit = Audit {
        id: "a".to_owned(),
        stamp: Stamp {
            created_at: "2026-09-21T13:45:30Z".to_owned(),
            updated_at: None,
        },
    };
    assert_eq!(audit.id, "a");

    let optional_audit = OptionalAudit {
        id: "a".to_owned(),
        stamp: None,
    };
    assert_eq!(optional_audit.stamp, None);

    let tagged_record = TaggedRecord {
        id: "a".to_owned(),
        tags: HashMap::from([("color".to_owned(), "red".to_owned())]),
    };
    assert_eq!(tagged_record.tags.len(), 1);

    let simple_choices = [SimpleChoice::Value("x".to_owned()), SimpleChoice::None];
    assert_eq!(simple_choices.len(), 2);
}

#[test]
fn test_every_generic_enum_variant_is_constructible() {
    let adjacent_choices = [
        AdjacentChoice::Value("y".to_owned()),
        AdjacentChoice::<String>::None,
    ];
    assert_eq!(adjacent_choices.len(), 2);

    let untagged_choices = [
        UntaggedChoice::Text("t".to_owned()),
        UntaggedChoice::Value("z".to_owned()),
    ];
    assert_eq!(untagged_choices.len(), 2);

    let eithers = [
        Either::Left("l".to_owned()),
        Either::<String, i64>::Right(7),
    ];
    assert_eq!(eithers.len(), 2);

    let many_choices = [
        ManyChoice::List(vec!["a".to_owned()]),
        ManyChoice::Maybe(Some("b".to_owned())),
    ];
    assert_eq!(many_choices.len(), 2);

    let pair = GenericPair("p".to_owned(), 3_u32);
    assert_eq!(pair.1, 3_u32);

    let tagged = Tagged {
        id: "a".to_owned(),
        stamp: Stamp {
            created_at: "2026-09-21T13:45:30Z".to_owned(),
            updated_at: None,
        },
    };
    assert_eq!(tagged.id, "a");

    let envelope = Envelope {
        id: "e".to_owned(),
        body: Wrapper {
            value: "wrapped".to_owned(),
        },
    };
    assert_eq!(envelope.body.value, "wrapped");
}

#[test]
#[cfg(feature = "chrono")]
fn test_chrono_type_is_constructible() {
    let dates = Dates {
        plain_date: chrono::NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
        zoned_ms: chrono::DateTime::from_timestamp(0, 0).unwrap(),
    };
    assert_eq!(dates.plain_date.to_string(), "2026-09-21");
}

#[test]
#[cfg(feature = "object_id")]
fn test_object_id_type_is_constructible() {
    let has_id = HasId {
        id: ObjectId::new(),
    };
    assert_ne!(has_id.id.to_hex(), String::new());
}

#[test]
fn test_struct_rename_and_optional() {
    let kotlin = window_request_kotlin::kotlin_definition();
    assert!(kotlin.contains("@Serializable"), "got: {kotlin}");
    assert!(
        kotlin.contains("data class WindowRequest("),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("@SerialName(\"conversation_id\") val conversationId: String"),
        "got: {kotlin}"
    );
    assert!(kotlin.contains("val limit: Int? = null"), "got: {kotlin}");
    // The wire spelling already matches the Kotlin property, so no annotation is written for it.
    assert!(!kotlin.contains("@SerialName(\"limit\")"), "got: {kotlin}");
}

#[test]
fn test_unit_struct_is_an_object() {
    let ping = Ping;
    assert_eq!(ping, ping.clone());
    let kotlin = ping_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("@Serializable object Ping"),
        "got: {kotlin}"
    );
    assert!(!kotlin.contains("class Ping"), "got: {kotlin}");
}

#[test]
fn test_width_table() {
    let kotlin = widths_kotlin::kotlin_definition();
    for expected in [
        "val flag: Boolean",
        "val tinySigned: Byte",
        "val smallSigned: Short",
        "val mediumSigned: Int",
        "val largeSigned: Long",
        "val archSigned: Long",
        "val tinyUnsigned: UByte",
        "val smallUnsigned: UShort",
        "val mediumUnsigned: UInt",
        "val single: Float",
        "val double: Double",
        "val text: String",
        "val letter: String",
        "val items: List<Int>",
        "val map: Map<String, Int>",
    ] {
        assert!(
            kotlin.contains(expected),
            "missing {expected}, got: {kotlin}"
        );
    }
}

#[test]
#[cfg(feature = "serde")]
fn test_plain_enum() {
    let kotlin = status_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("@Serializable enum class Status"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("@SerialName(\"active\") Active"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("@SerialName(\"inactive\") Inactive"),
        "got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_internal_tagged_enum() {
    let kotlin = window_error_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("@JsonClassDiscriminator(\"kind\")"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("sealed interface WindowError"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "@SerialName(\"NotFound\") @Serializable data object WindowErrorNotFound : WindowError"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "@SerialName(\"RateLimited\") @Serializable data class WindowErrorRateLimited(@SerialName(\"retry_after_ms\") val retryAfterMs: Long) : WindowError"
        ),
        "got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_adjacent_tagged_enum() {
    let kotlin = dynamic_value_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("sealed interface DynamicValue"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("object DynamicValueSerializer : KSerializer<DynamicValue>"),
        "got: {kotlin}"
    );
    assert!(kotlin.contains("put(\"type\", \"Flag\")"), "got: {kotlin}");
    assert!(kotlin.contains("put(\"value\","), "got: {kotlin}");
}

#[test]
fn test_external_tagged_enum() {
    let kotlin = external_tagged_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("sealed interface ExternalTagged"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("object ExternalTaggedSerializer : KSerializer<ExternalTagged>"),
        "got: {kotlin}"
    );
    assert!(kotlin.contains("element is JsonPrimitive"), "got: {kotlin}");
    assert!(
        kotlin.contains("element.jsonObject.entries.single()"),
        "got: {kotlin}"
    );
}

#[test]
fn test_external_tagged_unit_variant_round_trip_source() {
    let kotlin = simple_choice_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("is SimpleChoiceNone -> JsonPrimitive(\"None\")"),
        "a unit variant still writes the bare string serde writes. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "val (tag, data) = if (element is JsonPrimitive) element.content to JsonNull \
             else element.jsonObject.entries.single().let { it.key to it.value }"
        ),
        "the reader branches on the decoded element's own shape rather than assuming an object. \
         got: {kotlin}"
    );
    assert!(
        kotlin.contains("\"None\" -> SimpleChoiceNone"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "\"Value\" -> SimpleChoiceValue(input.json.decodeFromJsonElement(serializer<String>(), data))"
        ),
        "got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_untagged_enum() {
    let kotlin = date_value_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("sealed interface DateValue"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("object DateValueSerializer : KSerializer<DateValue>"),
        "got: {kotlin}"
    );
    assert!(kotlin.contains("runCatching {"), "got: {kotlin}");
    assert!(kotlin.contains("recoverCatching {"), "got: {kotlin}");
    assert!(
        kotlin.contains("@JvmInline @Serializable value class DateValueEpoch(val value: Long)"),
        "got: {kotlin}"
    );
}

#[test]
fn test_tuple_field() {
    let kotlin = holds_tuple_kotlin::kotlin_definition();
    assert!(kotlin.contains("data class KotlinTuple"), "got: {kotlin}");
    assert!(kotlin.contains("buildJsonArray {"), "got: {kotlin}");
    assert!(kotlin.contains("val slot0: String"), "got: {kotlin}");
    assert!(kotlin.contains("val slot1: Long"), "got: {kotlin}");
}

#[test]
fn test_non_string_map_keys() {
    let kotlin = map_keys_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("val byNumber: Map<Int, String>"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("val bySlot: Map<Slot, String>"),
        "got: {kotlin}"
    );
}

#[test]
fn test_alias() {
    let kotlin = slot_alias_key_kotlin::kotlin_definition();
    // An alias with no `name` override still moves off its own Rust ident (the `Type` suffix),
    // and re-publishes under that ident so an earlier reference still resolves.
    assert!(
        kotlin.contains("typealias SlotAliasKeyType = Slot;"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("typealias SlotAliasKey = SlotAliasKeyType;"),
        "got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "chrono")]
fn test_dates() {
    let kotlin = dates_kotlin::kotlin_definition();
    assert!(kotlin.contains("val plainDate: String"), "got: {kotlin}");
    assert!(kotlin.contains("val zonedMs: Long"), "got: {kotlin}");
}

#[test]
#[cfg(feature = "object_id")]
fn test_object_id_bare() {
    let kotlin = has_id_kotlin::kotlin_definition();
    assert!(kotlin.contains("val id: ObjectId"), "got: {kotlin}");
}

#[test]
fn test_branded_newtype() {
    let kotlin = correlation_id_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("@JvmInline @Serializable value class CorrelationId(val value: String)"),
        "got: {kotlin}"
    );
}

#[test]
fn test_name_override() {
    let kotlin = widget_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("data class RenamedWidget(val label: String)"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("typealias Widget = RenamedWidget;"),
        "got: {kotlin}"
    );
}

#[test]
fn test_generic_struct() {
    let kotlin = wrapper_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("data class Wrapper<T>(val value: T)"),
        "got: {kotlin}"
    );
}

#[test]
fn test_generic_enum_unit_variant() {
    let kotlin = choice_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("sealed interface Choice<out T>"),
        "the sealed base declares its parameters `out`. got: {kotlin}"
    );
    assert!(
        kotlin.contains("@Serializable data class ChoiceValue<T>(val value: T) : Choice<T>"),
        "a data-carrying variant still implements the base at its own `T`. got: {kotlin}"
    );
    assert!(
        kotlin.contains("data object ChoiceNone : Choice<Nothing>"),
        "a unit variant implements the base at `Nothing` rather than an unbound `T`. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class ChoiceSerializer<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<Choice<T>>"
        ),
        "the generated serializer is a class over one constructor serializer per type parameter, \
         not the `object` a non-generic item keeps. got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_flatten_struct() {
    let kotlin = audit_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("@Serializable(with = AuditSerializer::class) data class Audit("),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("object AuditSerializer : KSerializer<Audit>"),
        "got: {kotlin}"
    );
    assert!(kotlin.contains(".jsonObject.forEach"), "got: {kotlin}");
}

#[test]
#[cfg(feature = "serde")]
fn test_flatten_optional_struct() {
    let kotlin = optional_audit_kotlin::kotlin_definition();
    assert!(
        kotlin.contains(
            "@Serializable(with = OptionalAuditSerializer::class) data class OptionalAudit("
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("elementNames"),
        "an absent flattened `Option` is told apart by its own declared keys. got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_flatten_map() {
    let kotlin = tagged_record_kotlin::kotlin_definition();
    assert!(
        kotlin.contains(
            "@Serializable(with = TaggedRecordSerializer::class) data class TaggedRecord("
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("filterKeys") || kotlin.contains("filter {"),
        "a flattened map takes whatever keys are left over. got: {kotlin}"
    );
}

#[test]
fn test_generic_dispatched_enum_serializer_class() {
    let kotlin = choice_kotlin::kotlin_definition();
    assert!(
        kotlin.contains(
            "@Serializable(with = ChoiceSerializer::class) sealed interface Choice<out T>"
        ),
        "the base names its serializer so a property of this type still resolves. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class ChoiceSerializer<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<Choice<T>>"
        ),
        "a generic item's serializer is a class over one constructor serializer per parameter, \
         not the singleton `object` a non-generic item keeps. got: {kotlin}"
    );
    assert!(
        kotlin
            .contains("put(\"Value\", output.json.encodeToJsonElement(tSerializer, value.value))"),
        "the payload's own serializer composes from the constructor argument. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "\"Value\" -> ChoiceValue(input.json.decodeFromJsonElement(tSerializer, data))"
        ),
        "got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_generic_adjacent_tagged_enum() {
    let kotlin = adjacent_choice_kotlin::kotlin_definition();
    assert!(
        kotlin.contains(
            "class AdjacentChoiceSerializer<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<AdjacentChoice<T>>"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "put(\"kind\", \"Value\"); put(\"data\", output.json.encodeToJsonElement(tSerializer, value.value))"
        ),
        "got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_generic_untagged_enum() {
    let kotlin = untagged_choice_kotlin::kotlin_definition();
    assert!(
        kotlin.contains(
            "@Serializable(with = UntaggedChoiceSerializer::class) sealed interface UntaggedChoice<out T>"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class UntaggedChoiceSerializer<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<UntaggedChoice<T>>"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "is UntaggedChoiceValue -> output.json.encodeToJsonElement(UntaggedChoiceValue.serializer(tSerializer), value)"
        ),
        "a subclass's own KSerializer is read off its companion `serializer(...)` rather than the \
         reified lookup, since the payload's `T` is unbound. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "input.json.decodeFromJsonElement(UntaggedChoiceValue.serializer(tSerializer), element)"
        ),
        "got: {kotlin}"
    );
}

#[test]
fn test_generic_enum_two_type_parameters() {
    let kotlin = either_kotlin::kotlin_definition();
    assert!(
        kotlin.contains(
            "class EitherSerializer<L: Any, R: Any>(private val lSerializer: KSerializer<L>, private val rSerializer: KSerializer<R>) : KSerializer<Either<L, R>>"
        ),
        "one constructor serializer per type parameter. got: {kotlin}"
    );
    assert!(
        kotlin.contains("put(\"Left\", output.json.encodeToJsonElement(lSerializer, value.value))"),
        "got: {kotlin}"
    );
    assert!(
        kotlin
            .contains("put(\"Right\", output.json.encodeToJsonElement(rSerializer, value.value))"),
        "got: {kotlin}"
    );
}

#[test]
fn test_generic_parameter_inside_collection() {
    let kotlin = many_choice_kotlin::kotlin_definition();
    assert!(
        kotlin.contains(
            "put(\"List\", output.json.encodeToJsonElement(ListSerializer(tSerializer), value.value))"
        ),
        "a `Vec<T>` field composes through `ListSerializer`. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "put(\"Maybe\", output.json.encodeToJsonElement(tSerializer.nullable, value.value))"
        ),
        "an `Option<T>` field composes through `.nullable`. got: {kotlin}"
    );
}

#[test]
fn test_generic_tuple_struct_serializer_class() {
    let kotlin = generic_pair_kotlin::kotlin_definition();
    assert!(
        kotlin.contains(
            "@Serializable(with = GenericPairSerializer::class) data class GenericPair<T>("
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class GenericPairSerializer<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<GenericPair<T>>"
        ),
        "the tuple-struct path earns the same generic serializer class as the enum shapes. got: {kotlin}"
    );
    assert!(
        kotlin.contains("add(output.json.encodeToJsonElement(tSerializer, value.slot0))"),
        "a slot reaching the type parameter composes it. got: {kotlin}"
    );
    assert!(
        kotlin.contains("add(output.json.encodeToJsonElement(serializer<UInt>(), value.slot1))"),
        "a slot reaching none of the parameters still resolves through the reified lookup. got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_generic_flatten_own_field() {
    let kotlin = tagged_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("@Serializable(with = TaggedSerializer::class) data class Tagged<T>("),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class TaggedSerializer<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<Tagged<T>>"
        ),
        "the flatten-merging serializer earns the same generic class as the other hand-rolled \
         serializers, not the `object` a non-generic flatten struct keeps. got: {kotlin}"
    );
    assert!(
        kotlin.contains("put(\"id\", output.json.encodeToJsonElement(tSerializer, value.id))"),
        "an own field typed `T` composes from the constructor serializer. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "val id = input.json.decodeFromJsonElement(tSerializer, obj.getValue(\"id\"))"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("output.json.encodeToJsonElement(serializer<Stamp>(), value.stamp)"),
        "the flattened non-generic sibling still resolves through the reified lookup. got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_generic_flatten_generic_sibling() {
    let kotlin = envelope_kotlin::kotlin_definition();
    assert!(
        kotlin.contains("@Serializable(with = EnvelopeSerializer::class) data class Envelope<T>("),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class EnvelopeSerializer<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<Envelope<T>>"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "output.json.encodeToJsonElement(Wrapper.serializer(tSerializer), value.body).jsonObject.forEach"
        ),
        "a flattened generic sibling reads its keys and value through its own companion serializer, \
         bound to the constructor argument. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "val bodyKeys = (Wrapper.serializer(tSerializer)).descriptor.elementNames.toSet()"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "val body = lenient.decodeFromJsonElement(Wrapper.serializer(tSerializer), obj)"
        ),
        "got: {kotlin}"
    );
}
