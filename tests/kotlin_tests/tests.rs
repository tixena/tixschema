//! Tests for the Kotlin type and `kotlinx.serialization` codec backend (`kotlin` feature).
//!
//! Each `#[model_schema]` item earns `kotlin_definition()` inside a `{snake_case}_kotlin` module
//! beside it (never a direct inherent `impl` — see `features::kotlin::kotlin_module_tokens`).

use alloc::collections::BTreeMap;
use std::collections::HashMap;

#[cfg(feature = "mongodb")]
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use tixschema::model_schema;

#[cfg(feature = "serde")]
use super::corrected_options_models::{moused_kotlin, reeved_kotlin, wormed_kotlin};
use super::flattened_models::{
    BELTING, BOWLINING, BROACHING, Belting, Bending, Bowlining, Brailing, Broaching, CHAFING,
    CHOCKED, CLEATING, Cathead, Chafing, Chock, Cleating, CorrectedLocker, DRESSING, DRESSING_TAGS,
    Dipping, Dressing, ENSIGN, EmptiedLocker, Ensign, FLAG, FLAG_TAGS, FRAYING, Flag, Fraying,
    GAFFING, GONFALON, Gaffing, Gammoning, Gasketing, Gonfalon, HEAVING, HITCHING, Hawsepipe,
    Heaving, Hitching, Jibing, Knotting, LOCKER, Lanyarding, Luffing, MARLINE, Netting, Nipping,
    PAYLOAD, PENNANT_TAGS, Pennant, ProbeLocker, Quaying, Rousing, SCARFING, SCUDDING, SEMAPHORE,
    SNUBBED, STOWING, STROPPING, STRUCK, SWAYING, Scarfing, Scudding, Seizing, Semaphore, Sheeting,
    Slinging, Snubbing, Splicing, Stoppering, Stowing, Striking, Stropping, Swaying, Tacking,
    UNDER_AN_ALIAS, UNDER_TWO_NAMES, UNWRITTEN, WHELPING, Whelping, Whipping, Yawing, read_of,
};
#[cfg(feature = "serde")]
use super::flattened_models::{
    belting_kotlin, bending_kotlin, bowlining_kotlin, brailing_kotlin, broaching_kotlin,
    cathead_kotlin, chafing_kotlin, cleating_kotlin, corrected_locker_kotlin, dipping_kotlin,
    dressing_kotlin, emptied_locker_kotlin, flag_kotlin, fraying_kotlin, gaffing_kotlin,
    gammoning_kotlin, gasketing_kotlin, gonfalon_kotlin, hawsepipe_kotlin, heaving_kotlin,
    hitching_kotlin, knotting_kotlin, luffing_kotlin, netting_kotlin, nipping_kotlin,
    pennant_kotlin, probe_locker_kotlin, quaying_kotlin, rousing_kotlin, scarfing_kotlin,
    scudding_kotlin, seizing_kotlin, semaphore_kotlin, slinging_kotlin, splicing_kotlin,
    stoppering_kotlin, stowing_kotlin, striking_kotlin, swaying_kotlin, whelping_kotlin,
    whipping_kotlin, yawing_kotlin,
};
use super::read::{kept, serializers};
use super::tuple_variants_models::Labelling;
#[cfg(feature = "serde")]
use super::unrecorded_options_models::{
    bridling_kotlin, clapping_kotlin, fiddling_kotlin, grommeting_kotlin, slipping_kotlin,
    snagging_kotlin,
};

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowRequest {
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

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

#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Active,
    Inactive,
    Pending,
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum WindowError {
    NotFound,
    RateLimited { retry_after_ms: i64 },
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DynamicValue {
    Flag(bool),
    Nothing,
    Number(i64),
}

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

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DateValue {
    Epoch(i64),
    Iso(String),
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldsTuple {
    pub coords: (String, i64),
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping;

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

#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dates {
    pub plain_date: chrono::NaiveDate,
    #[model_schema_prop(as_number)]
    pub zoned_ms: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "mongodb")]
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HasId {
    pub id: ObjectId,
}

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);

#[model_schema(name = "RenamedWidget")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Widget {
    pub label: String,
}

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wrapper<T> {
    pub value: T,
}

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Choice<T> {
    None,
    Value(T),
}

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum AdjacentChoice<T> {
    None,
    Value(T),
}

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UntaggedChoice<T> {
    Text(String),
    Value(T),
}

#[model_schema(default_types(L = String, R = i64))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManyChoice<T> {
    List(Vec<T>),
    Maybe(Option<T>),
}

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericPair<T>(pub T, pub u32);

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

/// Members named after words the client languages reserve, and one none of them does.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReservedMembers {
    pub class: String,
    pub r#in: String,
    pub r#type: String,
}

/// A plain enum whose members lower-case to words Dart and Swift reserve.
#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReservedMember {
    Default,
    In,
    Other,
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
#[cfg(feature = "mongodb")]
fn test_object_id_type_is_constructible() {
    let has_id = HasId {
        id: ObjectId::new(),
    };
    assert_ne!(has_id.id.to_hex(), String::new());
}

/// `kotlin` with the number serializers it declares renamed `KotlinNumber1`, `KotlinNumber2`, in
/// the order they first appear: the counter that names them runs across the items of a crate.
pub fn renumbered(kotlin: &str) -> String {
    let Ok(pattern) = regex::Regex::new(r"KotlinNumber(\d+)") else {
        return kotlin.to_owned();
    };
    let mut order: Vec<String> = Vec::new();
    for found in pattern.captures_iter(kotlin) {
        let digits = found[1].to_owned();
        if !order.contains(&digits) {
            order.push(digits);
        }
    }
    pattern
        .replace_all(kotlin, |found: &regex::Captures<'_>| {
            let at = order
                .iter()
                .position(|digits| *digits == found[1])
                .unwrap_or(0);
            format!("KotlinNumber{}", at + 1)
        })
        .into_owned()
}

/// `kotlin` without the annotation that reads a number or a `bool` through its serializer, so a
/// test of the types a declaration writes reads them as the Kotlin types they are.
pub fn without_number_readers(kotlin: &str) -> String {
    regex::Regex::new(r"@Serializable\(with = KotlinNumber\d+::class\) ").map_or_else(
        |_| kotlin.to_owned(),
        |pattern| pattern.replace_all(kotlin, "").into_owned(),
    )
}

#[test]
fn test_struct_rename_and_optional() {
    let kotlin = without_number_readers(&renumbered(&window_request_kotlin::kotlin_definition()));
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
    let kotlin = renumbered(&ping_kotlin::kotlin_definition());
    assert!(
        kotlin.contains("@KeepGeneratedSerializer object Ping {"),
        "got: {kotlin}"
    );
    assert!(!kotlin.contains("class Ping"), "got: {kotlin}");
}

#[test]
fn test_width_table() {
    let kotlin = without_number_readers(&renumbered(&widths_kotlin::kotlin_definition()));
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
        "val letter: @Serializable(with = KotlinText",
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
    let kotlin = renumbered(&status_kotlin::kotlin_definition());
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
    let kotlin = without_number_readers(&renumbered(&window_error_kotlin::kotlin_definition()));
    assert!(
        kotlin.contains("val tag = (obj[\"kind\"] as? JsonPrimitive)?.content"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "@Serializable(with = WindowErrorSerializer::class) sealed interface WindowError"
        ),
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
    let kotlin = renumbered(&dynamic_value_kotlin::kotlin_definition());
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
    let kotlin = renumbered(&external_tagged_kotlin::kotlin_definition());
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
        kotlin.contains(
            "(element as? JsonObject ?: throw SerializationException(\"expected an object, found \" \
             + element)).entries.singleOrNull()"
        ),
        "got: {kotlin}"
    );
}

#[test]
fn test_external_tagged_unit_variant_round_trip_source() {
    let kotlin = renumbered(&simple_choice_kotlin::kotlin_definition());
    assert!(
        kotlin.contains("is SimpleChoiceNone -> JsonPrimitive(\"None\")"),
        "a unit variant still writes the bare string serde writes. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "val (tag, data) = if (element is JsonPrimitive) element.content to JsonNull \
             else (element as? JsonObject ?: throw SerializationException(\"expected an object, \
             found \" + element)).entries.singleOrNull()?.let { it.key to it.value } ?: throw \
             SerializationException(\"expected an object of one key, found \" + element);"
        ),
        "the reader branches on the decoded element's own shape rather than assuming an object. \
         got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "\"None\" -> if (data is JsonNull) SimpleChoiceNone else throw \
             SerializationException(\"SimpleChoice: \" + tag + \" takes no value\")"
        ),
        "a unit variant's entry reads it over `null` alone, as serde reads it. got: {kotlin}"
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
    let kotlin = without_number_readers(&renumbered(&date_value_kotlin::kotlin_definition()));
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
    let kotlin = renumbered(&holds_tuple_kotlin::kotlin_definition());
    assert!(kotlin.contains("data class KotlinTuple"), "got: {kotlin}");
    assert!(kotlin.contains("buildJsonArray {"), "got: {kotlin}");
    assert!(kotlin.contains("val slot0: String"), "got: {kotlin}");
    assert!(kotlin.contains("val slot1: Long"), "got: {kotlin}");
}

#[test]
fn test_non_string_map_keys() {
    let kotlin = renumbered(&map_keys_kotlin::kotlin_definition());
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
    let kotlin = renumbered(&slot_alias_key_kotlin::kotlin_definition());
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
    let kotlin = renumbered(&dates_kotlin::kotlin_definition());
    assert!(
        kotlin.contains("val plainDate: @Serializable(with = KotlinText"),
        "got: {kotlin}"
    );
    assert!(
        without_number_readers(&kotlin).contains("val zonedMs: Long"),
        "got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "mongodb")]
fn test_object_id_bare() {
    let kotlin = renumbered(&has_id_kotlin::kotlin_definition());
    assert!(kotlin.contains("val id: ObjectId"), "got: {kotlin}");
}

#[test]
fn test_branded_newtype() {
    let kotlin = renumbered(&correlation_id_kotlin::kotlin_definition());
    assert!(
        kotlin.contains(
            "@JvmInline @Serializable(with = CorrelationIdSerializer::class) \
             @KeepGeneratedSerializer value class CorrelationId(val value: String)"
        ),
        "got: {kotlin}"
    );
}

#[test]
fn test_name_override() {
    let kotlin = renumbered(&widget_kotlin::kotlin_definition());
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
    let kotlin = renumbered(&wrapper_kotlin::kotlin_definition());
    assert!(
        kotlin.contains("data class Wrapper<T>(val value: T)"),
        "got: {kotlin}"
    );
}

#[test]
fn test_generic_enum_unit_variant() {
    let kotlin = renumbered(&choice_kotlin::kotlin_definition());
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
            "class ChoiceUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<Choice<T>>"
        ),
        "the generated serializer is a class over one constructor serializer per type parameter, \
         not the `object` a non-generic item keeps. got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_flatten_struct() {
    let kotlin = renumbered(&audit_kotlin::kotlin_definition());
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
    let kotlin = renumbered(&optional_audit_kotlin::kotlin_definition());
    assert!(
        kotlin.contains(
            "@Serializable(with = OptionalAuditSerializer::class) data class OptionalAudit("
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("val stampKeys = setOf(\"created_at\", \"updated_at\");"),
        "an absent flattened `Option` is told apart by the keys serde reads it from. got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_flatten_map() {
    let kotlin = renumbered(&tagged_record_kotlin::kotlin_definition());
    assert!(
        kotlin.contains(
            "@Serializable(with = TaggedRecordSerializer::class) data class TaggedRecord("
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "val tags = obj.mapValues { (_, v) -> \
             input.json.decodeFromJsonElement(serializer<String>(), v) }"
        ),
        "a flattened map takes every key of the object. got: {kotlin}"
    );
}

/// The payload each struct of flattened members below writes and reads, and each struct with an
/// optional member beside a flattened one with that member absent.
#[test]
fn every_flattened_fixture_is_written_as_serde_writes_it() {
    let labelling = || Labelling {
        label: "n".to_owned(),
    };
    assert_eq!(
        serde_json::to_string(&Netting {
            rest: HashMap::from([("north".to_owned(), 2)]),
        })
        .unwrap(),
        r#"{"north":2}"#
    );
    assert_eq!(
        serde_json::to_string(&Knotting {
            labelling: labelling(),
            note: "a".to_owned(),
        })
        .unwrap(),
        r#"{"label":"n","note":"a"}"#
    );
    assert_eq!(
        serde_json::to_string(&Splicing {
            labelling: labelling(),
            spare: None
        })
        .unwrap(),
        r#"{"label":"n"}"#
    );
    assert_eq!(
        serde_json::to_string(&Seizing {
            labelling: labelling(),
            spare: None
        })
        .unwrap(),
        r#"{"label":"n","spare":null}"#
    );
}

/// What serde reads of each holder of a member flattened after a struct or beside a member of its
/// own: a function after a struct is handed the keys neither the struct nor its holder took, a
/// struct with a member serde never writes among them, the struct's own tag and the key of a
/// member it never reads among those it did not, and one ahead of a struct, or after a struct
/// that takes no key, every key. A struct after a struct, or beside a member of its holder's own,
/// that reads the same key is refused, and an optional one reads no value, nor does one whose tag
/// alone is in the object.
#[test]
fn serde_hands_a_flattened_member_the_keys_no_member_ahead_of_it_took() {
    let whipping: Whipping = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(whipping.tally.keys, ["north"]);
    let rousing: Rousing = serde_json::from_str(UNWRITTEN).unwrap();
    assert_eq!(rousing.tally.keys, ["north"]);
    assert_eq!(
        serde_json::from_str::<Gasketing>(UNWRITTEN)
            .unwrap_err()
            .to_string(),
        "missing field `label` at line 1 column 34"
    );
    let hitching: Hitching = serde_json::from_str(HITCHING).unwrap();
    assert_eq!(hitching.tally.keys, ["north"]);
    let bending: Bending = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(bending.extra.keys, ["label", "north"]);
    assert_eq!(bending.labelling.label, "n");
    let lanyarding: Lanyarding = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(lanyarding.tally.keys, ["label", "north"]);
    let brailing: Brailing = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(brailing.tally.keys, ["label", "north"]);
    assert_eq!(read_of::<Stropping>(STROPPING), STROPPING);
    for refused in [
        serde_json::from_str::<Stoppering>(PAYLOAD).unwrap_err(),
        serde_json::from_str::<Nipping>(PAYLOAD).unwrap_err(),
    ] {
        assert_eq!(
            refused.to_string(),
            "missing field `label` at line 1 column 23"
        );
    }
    let slinging: Slinging = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(slinging.spare, None);
    let dipping: Dipping = serde_json::from_str(ENSIGN).unwrap();
    assert_eq!(dipping.tally.keys, ["kind", "north"]);
    let gammoning: Gammoning = serde_json::from_str(MARLINE).unwrap();
    assert_eq!(gammoning.tally.keys, ["north", "spare"]);
    assert_eq!(gammoning.marline.spare, 0);
    let striking: Striking = serde_json::from_str(STRUCK).unwrap();
    assert_eq!(striking.ensign, None);
}

/// serde writes a struct's own tag ahead of the struct's members, whatever the order it was read
/// in, and so does a holder that flattens the struct ahead of a member of its own.
#[test]
fn serde_writes_a_structs_own_tag_ahead_of_its_members() {
    assert_eq!(
        [
            read_of::<Flag>(FLAG),
            read_of::<Gonfalon<String>>(GONFALON),
            read_of::<Semaphore>(SEMAPHORE),
            read_of::<Dressing>(DRESSING),
        ],
        [
            r#"{"kind":"Flag","label":"n"}"#,
            r#"{"kind":"Gonfalon","label":"n"}"#,
            r#"{"kind":"Semaphore","label":"n"}"#,
            r#"{"kind":"Ensign","label":"n","north":2}"#,
        ],
        "serde's write of each payload"
    );
}

/// serde reads a struct under its own tag with no tag and with any value under it, alone, with a
/// flattened member of its own and flattened ahead of a member of its holder's own, and writes the
/// struct's name back.
#[test]
fn serde_reads_a_structs_own_tag_whatever_it_holds_and_writes_the_name_back() {
    assert_eq!(
        FLAG_TAGS.map(read_of::<Flag>),
        [r#"{"kind":"Flag","label":"n"}"#; 4],
        "serde's write of each payload of `Flag`"
    );
    assert_eq!(
        PENNANT_TAGS.map(read_of::<Pennant>),
        [r#"{"kind":"Pennant","label":"n","note":"a"}"#; 4],
        "serde's write of each payload of `Pennant`"
    );
    assert_eq!(
        DRESSING_TAGS.map(read_of::<Dressing>),
        [r#"{"kind":"Ensign","label":"n","north":2}"#; 4],
        "serde's write of each payload of `Dressing`"
    );
}

/// The Zod schema of a struct under its own tag holds the tag as the literal of the struct's name,
/// which refuses each payload of [`FLAG_TAGS`], [`PENNANT_TAGS`] and [`DRESSING_TAGS`] but the
/// first.
#[test]
#[cfg(feature = "zod")]
fn zod_holds_a_structs_own_tag_as_the_literal_of_its_name() {
    for (zod, literal) in [
        (Flag::zod_schema(), "kind: z.literal(\"Flag\"),"),
        (Pennant::zod_schema(), "kind: z.literal(\"Pennant\"),"),
        (Ensign::zod_schema(), "kind: z.literal(\"Ensign\"),"),
    ] {
        assert!(zod.contains(literal), "no `{literal}` in: {zod}");
    }
}

/// The statement that refuses `object` where it holds no tag `kind` of the value `named`.
#[cfg(feature = "serde")]
fn tag_held(object: &str, named: &str) -> String {
    format!(
        "when ({object}[\"kind\"]) {{ JsonPrimitive(\"{named}\") -> {{}}; null -> throw \
         SerializationException(\"missing field `kind`\"); else -> throw \
         SerializationException(\"expected tag \\\"{named}\\\", found \" + {object}[\"kind\"]) }}"
    )
}

/// The unjudged serializer of a struct under its own tag refuses a payload that holds no tag, or
/// another value under it, after the correction of a payload where it has one, and moves the tag,
/// which the plugin writes after the constructor's properties, ahead of the members. A serializer
/// tixschema writes checks the object it reads every member from, and writes the tag first, and a
/// holder merges what the unjudged serializer writes ahead of its own members.
#[test]
#[cfg(feature = "serde")]
fn a_struct_under_its_own_tag_is_read_by_its_tag_and_written_with_the_tag_ahead() {
    let moved = "override fun transformSerialize(element: JsonElement): JsonElement = \
                 JsonObject(mapOf(\"kind\" to element.jsonObject.getValue(\"kind\")) + \
                 element.jsonObject)";
    let read = |named: &str| {
        format!(
            "override fun transformDeserialize(element: JsonElement): JsonElement = \
             element.also {{ whole: JsonElement -> if (whole is JsonObject) {} }}",
            tag_held("whole", named)
        )
    };
    for (kotlin, written) in [
        (
            renumbered(&flag_kotlin::kotlin_definition()),
            format!(
                "@OptIn(ExperimentalSerializationApi::class) object FlagUnjudged : \
                 JsonTransformingSerializer<Flag>(Flag.generatedSerializer()) {{ {}; {moved} }}",
                read("Flag")
            ),
        ),
        (
            renumbered(&gonfalon_kotlin::kotlin_definition()),
            format!(
                "@OptIn(ExperimentalSerializationApi::class) class \
                 GonfalonUnjudged<Held>(heldSerializer: KSerializer<Held>) : \
                 JsonTransformingSerializer<Gonfalon<Held>>(Gonfalon.generatedSerializer(\
                 heldSerializer)) {{ {}; {moved} }}",
                read("Gonfalon")
            ),
        ),
        (
            renumbered(&semaphore_kotlin::kotlin_definition()),
            format!(
                "-> val named = when (key) {{ \"name\" -> \"label\"; else -> key }}; if \
                 (put(named, held) != null) throw SerializationException(\"duplicate field \
                 `$named`\") }} }}) else whole }}.also {{ whole: JsonElement -> if (whole is \
                 JsonObject) {} }}; {moved} }}",
                tag_held("whole", "Semaphore")
            ),
        ),
        (
            renumbered(&pennant_kotlin::kotlin_definition()),
            "buildJsonObject { put(\"kind\", \"Pennant\"); \
             output.json.encodeToJsonElement(LabellingUnjudged, value.labelling)"
                .to_owned(),
        ),
        (
            renumbered(&pennant_kotlin::kotlin_definition()),
            format!(
                "val obj = input.decodeJsonElement().let {{ it as? JsonObject ?: throw \
                 SerializationException(\"expected an object, found \" + it) }}; {}; val \
                 labellingKeys",
                tag_held("obj", "Pennant")
            ),
        ),
        (
            renumbered(&dressing_kotlin::kotlin_definition()),
            "buildJsonObject { output.json.encodeToJsonElement(EnsignUnjudged, \
             value.ensign).jsonObject.forEach { (k, v) -> put(k, v) }; put(\"north\""
                .to_owned(),
        ),
    ] {
        assert!(kotlin.contains(&written), "no `{written}` in: {kotlin}");
    }
}

/// A flattened map, the one member of its struct, is read from every key of the object and written
/// entry by entry.
#[test]
#[cfg(feature = "serde")]
fn a_flattened_map_is_read_from_every_key_and_written_entry_by_entry() {
    let netting = renumbered(&netting_kotlin::kotlin_definition());
    for held in [
        "buildJsonObject { value.rest.forEach { (k, v) -> put(k, \
         output.json.encodeToJsonElement(KotlinNumber1, v)) } }",
        "val obj = input.decodeJsonElement().let { it as? JsonObject ?: throw \
         SerializationException(\"expected an object, found \" + it) }; val rest = obj.mapValues \
         { (_, v) -> input.json.decodeFromJsonElement(KotlinNumber1, v) }; return \
         Netting(rest = rest) }",
    ] {
        assert!(netting.contains(held), "no `{held}` in: {netting}");
    }
}

/// What serde reads of an optional map, the one member, from each of [`LOCKER`]: the map where
/// every entry reads as a count, and `None` where one does not, which it writes as no entry. Under
/// a function it reads the map from what the function answers: it refuses an entry that does not
/// read as a count, and reads `null` as `None`.
#[test]
fn serde_reads_an_optional_flattened_map_as_none_where_an_entry_does_not_read() {
    let read = LOCKER.map(|payload| serde_json::from_str::<ProbeLocker>(payload).unwrap().counts);
    assert_eq!(
        read,
        [
            Some(BTreeMap::new()),
            Some(BTreeMap::from([("north".to_owned(), 2)])),
            None,
            None,
            None
        ]
    );
    assert_eq!(
        LOCKER.map(read_of::<ProbeLocker>),
        ["{}", r#"{"north":2}"#, "{}", "{}", "{}"]
    );
    assert_eq!(
        LOCKER.map(read_of::<CorrectedLocker>),
        ["{}", r#"{"north":2}"#, "refused", "refused", "refused"]
    );
    let emptied = LOCKER.map(|payload| {
        serde_json::from_str::<EmptiedLocker>(payload)
            .unwrap()
            .counts
    });
    assert_eq!(emptied, [None, None, None, None, None]);
}

/// An optional map, the one member, writes no entry where it holds no value, and is read as
/// `null` wherever an entry does not read as its value. Under a function it is read from what the
/// function answers, and as `null` where that is `null`.
#[test]
#[cfg(feature = "serde")]
fn an_optional_flattened_map_is_read_as_null_where_an_entry_does_not_read() {
    let entries = "mapValues { (_, v) -> input.json.decodeFromJsonElement(KotlinNumber1, v) }";
    for (kotlin, held) in [
        (
            renumbered(&probe_locker_kotlin::kotlin_definition()),
            "buildJsonObject { value.counts?.forEach { (k, v) -> put(k, \
             output.json.encodeToJsonElement(KotlinNumber1, v)) } }"
                .to_owned(),
        ),
        (
            renumbered(&probe_locker_kotlin::kotlin_definition()),
            format!(
                "val counts = runCatching {{ obj.{entries} }}.getOrNull(); \
                 return ProbeLocker(counts = counts) }}"
            ),
        ),
        (
            renumbered(&corrected_locker_kotlin::kotlin_definition()),
            format!(
                "val counts = asGiven(obj).let {{ if (it is JsonNull) null else (it as? JsonObject \
                 ?: throw SerializationException(\"expected an object, found \" + it)).{entries} \
                 }}; return CorrectedLocker(counts = counts) }}"
            ),
        ),
        (
            renumbered(&emptied_locker_kotlin::kotlin_definition()),
            format!(
                "val counts = asNull(obj).let {{ if (it is JsonNull) null else (it as? JsonObject \
                 ?: throw SerializationException(\"expected an object, found \" + it)).{entries} \
                 }}; return EmptiedLocker(counts = counts) }}"
            ),
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
}

/// What serde reads of a map keyed by a plain enum, the one member, from each of [`CHOCKED`] and
/// [`SNUBBED`]: a key a variant is written under, its `rename` and the enum's `rename_all`
/// among them, written back as it was read, and a key no variant is written under refused, or
/// read as `None` where the map is optional.
#[test]
fn serde_reads_a_flattened_map_keyed_by_an_enum_under_the_names_its_variants_are_written_under() {
    assert_eq!(
        CHOCKED.map(read_of::<Cathead>),
        [
            "{}",
            r#"{"Port":2}"#,
            r#"{"Port":2,"Starboard":3}"#,
            "refused",
            "refused"
        ]
    );
    assert_eq!(
        serde_json::from_str::<Cathead>(CHOCKED[3])
            .unwrap_err()
            .to_string(),
        "unknown variant `Bow`, expected `Port` or `Starboard` at line 1 column 9"
    );
    let read = CHOCKED.map(|payload| serde_json::from_str::<Hawsepipe>(payload).unwrap().lines);
    assert_eq!(
        read,
        [
            Some(BTreeMap::new()),
            Some(BTreeMap::from([(Chock::Port, 2)])),
            Some(BTreeMap::from([(Chock::Port, 2), (Chock::Starboard, 3)])),
            None,
            None
        ]
    );
    assert_eq!(
        CHOCKED.map(read_of::<Hawsepipe>),
        [
            "{}",
            r#"{"Port":2}"#,
            r#"{"Port":2,"Starboard":3}"#,
            "{}",
            "{}"
        ]
    );
    assert_eq!(
        SNUBBED.map(read_of::<Snubbing>),
        [r#"{"bow":2}"#, r#"{"open_water":2}"#, "refused"]
    );
}

/// A flattened map keyed by a plain enum, the one member, writes each key as the text the key's
/// serializer writes and reads it back through that serializer, optional or not.
#[test]
#[cfg(feature = "serde")]
fn a_flattened_map_keyed_by_an_enum_writes_and_reads_each_key_through_its_serializer() {
    let entry_written = "forEach { (k, v) -> put(output.json.encodeToJsonElement(\
                         serializer<Chock>(), k).jsonPrimitive.content, \
                         output.json.encodeToJsonElement(KotlinNumber1, v)) }";
    let entries = "entries.associate { (k, v) -> input.json.decodeFromJsonElement(\
                   serializer<Chock>(), JsonPrimitive(k)) to \
                   input.json.decodeFromJsonElement(KotlinNumber1, v) }";
    for (kotlin, held) in [
        (
            renumbered(&cathead_kotlin::kotlin_definition()),
            format!("buildJsonObject {{ value.lines.{entry_written} }}"),
        ),
        (
            renumbered(&cathead_kotlin::kotlin_definition()),
            format!("val lines = obj.{entries}; return Cathead(lines = lines) }}"),
        ),
        (
            renumbered(&hawsepipe_kotlin::kotlin_definition()),
            format!("buildJsonObject {{ value.lines?.{entry_written} }}"),
        ),
        (
            renumbered(&hawsepipe_kotlin::kotlin_definition()),
            format!(
                "val lines = runCatching {{ obj.{entries} }}.getOrNull(); \
                 return Hawsepipe(lines = lines) }}"
            ),
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
}

/// An optional member beside a flattened one is written only where it holds a value, and one
/// under `nullable` always. A member that is not optional is written as it was.
#[test]
#[cfg(feature = "serde")]
fn an_optional_member_beside_a_flattened_one_is_written_only_where_it_holds_a_value() {
    let splicing = renumbered(&splicing_kotlin::kotlin_definition());
    assert!(
        splicing.contains(
            "{ output.json.encodeToJsonElement(LabellingUnjudged, value.labelling).jsonObject\
             .forEach { (k, v) -> put(k, v) }; if (value.spare != null) put(\"spare\", \
             output.json.encodeToJsonElement(KotlinNumber1.nullable, value.spare)) }"
        ),
        "got: {splicing}"
    );
    let seizing = renumbered(&seizing_kotlin::kotlin_definition());
    assert!(
        seizing.contains(
            "{ output.json.encodeToJsonElement(LabellingUnjudged, value.labelling).jsonObject\
             .forEach { (k, v) -> put(k, v) }; put(\"spare\", \
             output.json.encodeToJsonElement(KotlinNumber1.nullable, value.spare)) }"
        ),
        "got: {seizing}"
    );
    let knotting = renumbered(&knotting_kotlin::kotlin_definition());
    assert!(
        knotting.contains(
            "{ output.json.encodeToJsonElement(LabellingUnjudged, value.labelling).jsonObject\
             .forEach { (k, v) -> put(k, v) }; put(\"note\", \
             output.json.encodeToJsonElement(serializer<String>(), value.note)) }"
        ),
        "got: {knotting}"
    );
}

/// A member that names a function is read from the object without the keys its holder's own
/// members take and those each struct flattened ahead of it took, as serde hands it them, and a
/// member a function reads takes none of the keys it reads. A struct is read from the whole object
/// its holder reads, as TypeScript and Zod read it: after a struct that reads the same key, beside
/// a member of its holder's own, and optional, its own keys looked for in that object.
#[test]
#[cfg(feature = "serde")]
fn a_flattened_member_is_read_without_the_keys_taken_ahead_of_it() {
    let labelling = "val labellingKeys = setOf(\"label\"); val labelling = \
                     input.json.decodeFromJsonElement(LabellingUnjudged, \
                     JsonObject(obj.filterKeys { it in setOf(\"label\") }));";
    for (kotlin, held) in [
        (
            renumbered(&whipping_kotlin::kotlin_definition()),
            labelling.to_owned(),
        ),
        (
            renumbered(&whipping_kotlin::kotlin_definition()),
            "val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - labellingKeys)));"
                .to_owned(),
        ),
        (
            renumbered(&hitching_kotlin::kotlin_definition()),
            "val labelling = input.json.decodeFromJsonElement(LabellingUnjudged, \
             JsonObject(obj.filterKeys { it in setOf(\"label\") }));"
                .to_owned(),
        ),
        (
            renumbered(&hitching_kotlin::kotlin_definition()),
            "val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - setOf(\"note\") - labellingKeys)));"
                .to_owned(),
        ),
        (
            renumbered(&bending_kotlin::kotlin_definition()),
            "val extra = lenient.decodeFromJsonElement(BuntingUnjudged, withItsKeys(obj));"
                .to_owned(),
        ),
        (
            renumbered(&bending_kotlin::kotlin_definition()),
            labelling.to_owned(),
        ),
        (
            renumbered(&stoppering_kotlin::kotlin_definition()),
            labelling.to_owned(),
        ),
        (
            renumbered(&stoppering_kotlin::kotlin_definition()),
            "val twice = input.json.decodeFromJsonElement(LabellingUnjudged, \
             JsonObject(obj.filterKeys { it in setOf(\"label\") }));"
                .to_owned(),
        ),
        (
            renumbered(&nipping_kotlin::kotlin_definition()),
            "val labelling = input.json.decodeFromJsonElement(LabellingUnjudged, \
             JsonObject(obj.filterKeys { it in setOf(\"label\") }));"
                .to_owned(),
        ),
        (
            renumbered(&slinging_kotlin::kotlin_definition()),
            "val spare = if (obj.keys.any { it in spareKeys }) \
             input.json.decodeFromJsonElement(LabellingUnjudged, JsonObject(obj.filterKeys { it \
             in setOf(\"label\") })) else null;"
                .to_owned(),
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
}

/// A flattened struct takes the keys serde takes of it, whoever writes its serializer: those it
/// reads, where it has a member serde never writes too, and none where it has a flattened member
/// of its own. Neither its own tag nor a member serde never reads is among them, and an optional
/// one is looked for by them.
#[test]
#[cfg(feature = "serde")]
fn a_flattened_struct_takes_the_keys_serde_takes_of_it() {
    let hauled = "val hauledKeys = setOf(\"hidden\", \"label\"); val hauled = \
                  input.json.decodeFromJsonElement(UnwrittenUnjudged, \
                  JsonObject(obj.filterKeys { it in setOf(\"hidden\", \"label\") }));";
    for (kotlin, held) in [
        (renumbered(&rousing_kotlin::kotlin_definition()), hauled),
        (
            renumbered(&rousing_kotlin::kotlin_definition()),
            "val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - hauledKeys)));",
        ),
        (renumbered(&gasketing_kotlin::kotlin_definition()), hauled),
        (
            renumbered(&gasketing_kotlin::kotlin_definition()),
            "val labelling = input.json.decodeFromJsonElement(LabellingUnjudged, \
             JsonObject(obj.filterKeys { it in setOf(\"label\") }));",
        ),
        (
            renumbered(&brailing_kotlin::kotlin_definition()),
            "val sheetingKeys = setOf<String>(); val sheeting = \
             lenient.decodeFromJsonElement(SheetingUnjudged, obj);",
        ),
        (
            renumbered(&whipping_kotlin::kotlin_definition()),
            "val labellingKeys = setOf(\"label\");",
        ),
        (
            renumbered(&dipping_kotlin::kotlin_definition()),
            "val ensignKeys = setOf(\"label\"); val ensign = \
             input.json.decodeFromJsonElement(EnsignUnjudged, JsonObject(obj.filterKeys { it in \
             setOf(\"label\", \"kind\") })); val tallyKeys = setOf(\"keys\"); \
             val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - ensignKeys)));",
        ),
        (
            renumbered(&gammoning_kotlin::kotlin_definition()),
            "val marlineKeys = setOf(\"label\"); val marline = \
             input.json.decodeFromJsonElement(MarlineUnjudged, JsonObject(obj.filterKeys { it in \
             setOf(\"label\", \"spare\") })); val tallyKeys = setOf(\"keys\"); \
             val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - marlineKeys)));",
        ),
        (
            renumbered(&striking_kotlin::kotlin_definition()),
            "val ensignKeys = setOf(\"label\"); val ensign = if (obj.keys.any { it in \
             ensignKeys })",
        ),
    ] {
        assert!(kotlin.contains(held), "no `{held}` in: {kotlin}");
    }
}

/// serde reads an optional struct with a flattened member of its own wherever the struct reads
/// from the object, and as `None` wherever it does not.
#[test]
fn serde_reads_an_optional_struct_with_a_flattened_member_wherever_it_reads() {
    let read = |json: &str| serde_json::from_str::<Scarfing>(json).unwrap().sheeting;
    let held = Sheeting {
        labelling: Labelling {
            label: "n".to_owned(),
        },
    };
    assert_eq!(
        SCARFING.map(read),
        [Some(held), None, None],
        "serde no longer reads each payload as it did"
    );
}

/// An optional struct with a flattened member of its own takes no key, so it has none to be
/// looked for by: it is read wherever its read succeeds and is `null` wherever that read throws,
/// as serde reads it.
#[test]
#[cfg(feature = "serde")]
fn an_optional_struct_that_takes_no_key_is_read_wherever_its_read_succeeds() {
    let scarfing = renumbered(&scarfing_kotlin::kotlin_definition());
    let decoded = "val sheetingKeys = setOf<String>(); val sheeting = runCatching { \
                   lenient.decodeFromJsonElement(SheetingUnjudged, obj) }.getOrNull();";
    assert!(scarfing.contains(decoded), "no `{decoded}` in: {scarfing}");
}

/// An optional flattened member whose type the emitter recorded no keys for is looked for by the
/// elements its serializer's descriptor names, and where it names none, a serializer written by
/// hand, is read wherever its read succeeds and is `null` wherever that read throws: a struct
/// declared below its holder, an untagged enum and an enum under a tag key that names a function
/// on itself. Where the type's serializer judges, a read that throws is tried again with no
/// judgement, and the payload is refused where that read succeeds.
#[test]
#[cfg(feature = "serde")]
fn an_optional_type_of_no_recorded_key_is_read_where_its_descriptor_names_no_element() {
    let read = |prop: &str, serializer: &str, rest: &str| {
        let decoded = format!("lenient.decodeFromJsonElement({serializer}, {rest})");
        let tried = if serializer.ends_with("Unjudged") {
            format!("runCatching {{ {decoded} }}.getOrNull()")
        } else {
            format!(
                "runCatching {{ {decoded} }}.getOrElse {{ refused -> if (runCatching {{ \
                 lenient.decodeFromJsonElement(({serializer} as? Function0<*>)?.invoke() as? \
                 DeserializationStrategy<*> ?: {serializer}, {rest}) }}.isSuccess) throw refused \
                 else null }}"
            )
        };
        format!(
            "val {prop}Keys = ({serializer}).descriptor.elementNames.toSet(); val {prop} = if \
             ({prop}Keys.isEmpty()) {tried} else if ({rest}.keys.any {{ it in {prop}Keys }}) \
             {decoded} else null;"
        )
    };
    let beside_a_note = "JsonObject(obj - setOf(\"note\"))";
    for (kotlin, held) in [
        (
            renumbered(&fiddling_kotlin::kotlin_definition()),
            read("crowned", "serializer<Crowned>()", "obj"),
        ),
        (
            renumbered(&grommeting_kotlin::kotlin_definition()),
            read("gromet", "serializer<Gromet>()", "obj"),
        ),
        (
            renumbered(&slipping_kotlin::kotlin_definition()),
            read("slipped", "SlippedUnjudged", beside_a_note),
        ),
        (
            renumbered(&clapping_kotlin::kotlin_definition()),
            read("clapped", "ClappedUnjudged", beside_a_note),
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
}

/// An optional flattened enum under a tag key the emitter wrote is `null` where the whole object
/// its holder reads holds neither its tag nor its content key, and is read where it holds one, as
/// TypeScript and Zod read it, so a read that throws refuses the payload. One under a tag alone
/// takes the keys its descriptor names, and one under a tag and a content key takes both.
#[test]
#[cfg(feature = "serde")]
fn an_optional_enum_under_a_tag_key_is_looked_for_by_its_tag_and_its_content_key() {
    let read = |prop: &str, serializer: &str, taken: &str, keys: &str| {
        format!(
            "val {prop}Keys = {taken}; val {prop} = if (obj.keys.any {{ it in setOf({keys}) }}) \
             lenient.decodeFromJsonElement({serializer}, obj) else null;"
        )
    };
    for (kotlin, held) in [
        (
            renumbered(&snagging_kotlin::kotlin_definition()),
            read(
                "hitched",
                "HitchedUnjudged",
                "(HitchedUnjudged).descriptor.elementNames.toSet()",
                "\"kind\"",
            ),
        ),
        (
            renumbered(&bridling_kotlin::kotlin_definition()),
            read(
                "bridled",
                "BridledUnjudged",
                "setOf(\"kind\", \"held\")",
                "\"kind\", \"held\"",
            ),
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
}

/// An optional flattened member that names a function is read from what the function answers,
/// which is called first, and where that read throws is `null` where the object the function is
/// handed holds none of its type's keys, or no key at all where its serializer's descriptor names
/// none, and refuses the payload where it holds one.
#[test]
#[cfg(feature = "serde")]
fn an_optional_type_that_names_a_function_is_read_from_what_the_function_answers() {
    let read = |prop: &str, keys: &str, (function, serializer): (&str, &str), held: &str| {
        format!(
            "val {prop}Keys = {keys}; val {prop}Corrected = {function}(JsonObject(obj - \
             setOf(\"note\"))); val {prop} = runCatching {{ \
             lenient.decodeFromJsonElement({serializer}, {prop}Corrected) }}.getOrElse {{ if \
             ({held}) throw it else null }};"
        )
    };
    let any_key =
        |prop: &str| format!("JsonObject(obj - setOf(\"note\")).keys.any({prop}Keys::contains)");
    let tholepin = ("setOf(\"label\")", any_key("tholepin"));
    for (kotlin, held) in [
        (
            renumbered(&reeved_kotlin::kotlin_definition()),
            read(
                "tholepin",
                tholepin.0,
                ("asGiven", "TholepinUnjudged"),
                &tholepin.1,
            ),
        ),
        (
            renumbered(&wormed_kotlin::kotlin_definition()),
            read(
                "tholepin",
                tholepin.0,
                ("asNull", "TholepinUnjudged"),
                &tholepin.1,
            ),
        ),
        (
            renumbered(&moused_kotlin::kotlin_definition()),
            read(
                "frapped",
                "(FrappedUnjudged).descriptor.elementNames.toSet()",
                ("asGiven", "FrappedUnjudged"),
                &format!(
                    "if (frappedKeys.isEmpty()) JsonObject(obj - setOf(\"note\")).isNotEmpty() \
                     else {}",
                    any_key("frapped")
                ),
            ),
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
}

/// What serde reads of each holder of a flattened externally tagged enum: the first entry, in the
/// object's order, under a name a variant is read under, its alias included, is the enum's, and
/// every other entry is left to the members after it. A holder with no such entry is refused, and
/// an optional enum reads no value.
#[test]
fn serde_hands_a_flattened_enum_the_first_entry_under_a_variants_name() {
    let bay = || Tacking::Bay {
        label: "n".to_owned(),
    };
    let dock = || Tacking::Dock {
        label: "s".to_owned(),
    };
    let stowed: Vec<_> = STOWING
        .iter()
        .map(|payload| serde_json::from_str::<Stowing>(payload).map(|read| read.tacking))
        .collect();
    assert_eq!(
        stowed[..6]
            .iter()
            .map(|read| read.as_ref().ok())
            .collect::<Vec<_>>(),
        [
            Some(&bay()),
            Some(&bay()),
            Some(&bay()),
            Some(&bay()),
            Some(&Tacking::Calm),
            Some(&Tacking::Held("x".to_owned()))
        ]
    );
    assert_eq!(
        stowed[6].as_ref().unwrap_err().to_string(),
        "no variant of enum Tacking found in flattened data at line 1 column 12"
    );
    let belted = BELTING.map(|payload| serde_json::from_str::<Belting>(payload).unwrap().tacking);
    assert_eq!(belted, [Some(bay()), None, None]);
    let heaved = HEAVING.map(|payload| {
        let read: Heaving = serde_json::from_str(payload).unwrap();
        (read.tacking, read.tally.keys)
    });
    assert_eq!(
        heaved,
        [
            (bay(), vec!["north".to_owned()]),
            (bay(), vec!["Dock".to_owned(), "north".to_owned()]),
            (dock(), vec!["Bay".to_owned(), "north".to_owned()]),
            (bay(), vec!["Quay".to_owned(), "north".to_owned()]),
        ]
    );
    let cleated = CLEATING.map(|payload| {
        serde_json::from_str::<Cleating>(payload).map(|read| (read.jibing, read.tally.keys))
    });
    let north = || vec!["north".to_owned()];
    assert_eq!(
        cleated[..2]
            .iter()
            .map(|read| read.as_ref().ok())
            .collect::<Vec<_>>(),
        [
            Some(&(
                Jibing::Jetty {
                    label: "n".to_owned()
                },
                north()
            )),
            Some(&(
                Jibing::OpenBay {
                    label: "n".to_owned()
                },
                north()
            )),
        ]
    );
    assert_eq!(
        cleated[2].as_ref().unwrap_err().to_string(),
        "no variant of enum Jibing found in flattened data at line 1 column 35"
    );
    let scudding: Scudding = serde_json::from_str(SCUDDING).unwrap();
    assert_eq!(
        (
            scudding.hatch,
            scudding.labelling.label,
            scudding.tally.keys
        ),
        (bay(), "m".to_owned(), north())
    );
    let stowing: Stowing = serde_json::from_str(UNDER_AN_ALIAS[0]).unwrap();
    assert_eq!(stowing.tacking, dock());
    let heaving: Heaving = serde_json::from_str(UNDER_AN_ALIAS[1]).unwrap();
    assert_eq!((heaving.tacking, heaving.tally.keys), (dock(), north()));
    let luffing: Luffing = serde_json::from_str(STOWING[0]).unwrap();
    assert_eq!(
        luffing.quaying,
        Quaying::Bay {
            label: "n".to_owned()
        }
    );
}

/// What serde reads of a flattened enum under a tag and a content key, which it reads as a struct
/// of the two keys: a function after it is handed every other key, and a struct after it reads its
/// own. Of entries under two names of a flattened externally tagged enum, serde reads the first.
#[test]
fn serde_takes_both_keys_of_a_flattened_enum_under_a_content_key_and_the_first_of_two_entries() {
    let chafing: Chafing = serde_json::from_str(CHAFING).unwrap();
    assert_eq!(chafing.tally.keys, ["north"]);
    let frayed = FRAYING.map(|payload| {
        let read: Fraying = serde_json::from_str(payload).unwrap();
        read.labelling.label
    });
    assert_eq!(frayed, ["m", "m"]);
    let stowed =
        UNDER_TWO_NAMES.map(|payload| serde_json::from_str::<Stowing>(payload).unwrap().tacking);
    let named = |label: &str| label.to_owned();
    assert_eq!(
        stowed,
        [
            Tacking::Bay { label: named("n") },
            Tacking::Dock { label: named("s") },
            Tacking::Dock { label: named("s") }
        ]
    );
}

/// A flattened enum under a tag and a content key takes both keys: a function after it is handed
/// the object without them, and a struct after it reads the whole object.
#[test]
#[cfg(feature = "serde")]
fn a_flattened_enum_under_a_content_key_takes_both_keys_in_kotlin() {
    for (kotlin, held) in [
        (
            renumbered(&chafing_kotlin::kotlin_definition()),
            "val fairleadKeys = setOf(\"kind\", \"held\"); val fairlead = \
             lenient.decodeFromJsonElement(SnubberUnjudged, obj); val tallyKeys = setOf(\"keys\"); \
             val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - fairleadKeys)));",
        ),
        (
            renumbered(&fraying_kotlin::kotlin_definition()),
            "val labelling = input.json.decodeFromJsonElement(LabellingUnjudged, \
             JsonObject(obj.filterKeys { it in setOf(\"label\") })); if \
             (!input.json.configuration.ignoreUnknownKeys) (obj - setOf(\"kind\", \"held\") - \
             setOf(\"label\")).keys.firstOrNull()?.let { throw SerializationException(\"Encountered \
             an unknown key '\" + it + \"'\") };",
        ),
    ] {
        assert!(kotlin.contains(held), "no `{held}` in: {kotlin}");
    }
}

/// The read of the one entry of `rest` under one of `variants`' names, which a flattened enum
/// `named` held by `prop` is read from: entries under two names are refused, naming each variant
/// once.
fn one_entry(rest: &str, prop: &str, named: &str, variants: &[&[&str]]) -> String {
    let quoted = |names: &[&str]| {
        names
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let listed = quoted(&variants.concat());
    let found = variants
        .iter()
        .map(|names| {
            format!(
                "if ({prop}Entries.any {{ it.key in setOf({}) }}) \"{}\" else null",
                quoted(names),
                names[0]
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{rest}.entries.filter {{ it.key in setOf({listed}) }}.also {{ {prop}Entries -> if \
         ({prop}Entries.size > 1) throw SerializationException(\"more than one variant of enum \
         {named} found in flattened data: \" + listOfNotNull({found}).joinToString(\", \")) \
         }}.firstOrNull()"
    )
}

/// A flattened externally tagged enum is read from the one entry under a name a variant is read
/// under, its `rename`, the enum's `rename_all` and an alias included: the holder is refused
/// without one, and with entries under two, and an optional enum reads no value. Every member
/// flattened after it but a struct is handed the object without that entry. An enum declared below
/// its holder is read from the object without the holder's own keys, as before.
#[test]
#[cfg(feature = "serde")]
fn a_flattened_enum_is_read_from_the_one_entry_under_a_variants_name() {
    let tacking: &[&[&str]] = &[&["Bay"], &["Calm"], &["Dock", "Quay"], &["Held"]];
    let beside_a_note = one_entry(
        "JsonObject(obj - setOf(\"note\"))",
        "tacking",
        "Tacking",
        tacking,
    );
    for (kotlin, held) in [
        (
            renumbered(&stowing_kotlin::kotlin_definition()),
            format!(
                "val tackingEntry = {beside_a_note} ?: throw SerializationException(\"no variant \
                 of enum Tacking found in flattened data\"); val tacking = \
                 input.json.decodeFromJsonElement(TackingUnjudged, \
                 JsonObject(mapOf(tackingEntry.toPair())));"
            ),
        ),
        (
            renumbered(&belting_kotlin::kotlin_definition()),
            format!(
                "val tackingEntry = {beside_a_note}; val tacking = tackingEntry?.let {{ \
                 input.json.decodeFromJsonElement(TackingUnjudged, JsonObject(mapOf(it.toPair()))) \
                 }};"
            ),
        ),
        (
            renumbered(&cleating_kotlin::kotlin_definition()),
            format!(
                "val jibingEntry = {} ?: throw SerializationException(\"no variant of enum Jibing \
                 found in flattened data\");",
                one_entry("obj", "jibing", "Jibing", &[&["pier"], &["open_bay"]])
            ),
        ),
        (
            renumbered(&heaving_kotlin::kotlin_definition()),
            "val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - tackingEntry.key)));"
                .to_owned(),
        ),
        (
            renumbered(&scudding_kotlin::kotlin_definition()),
            "val labelling = input.json.decodeFromJsonElement(LabellingUnjudged, \
             JsonObject(obj.filterKeys { it in setOf(\"label\") }));"
                .to_owned(),
        ),
        (
            renumbered(&scudding_kotlin::kotlin_definition()),
            "val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - hatchEntry.key - labellingKeys)));"
                .to_owned(),
        ),
        (
            renumbered(&luffing_kotlin::kotlin_definition()),
            "val quaying = lenient.decodeFromJsonElement(serializer<Quaying>(), JsonObject(obj - \
             setOf(\"note\")));"
                .to_owned(),
        ),
    ] {
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    }
}

/// What serde writes of each holder of a flattened externally tagged enum: a variant that holds a
/// value as its entry, and a unit variant as its name over `null`, after the holder's own member,
/// the enum optional and under a function too. An optional enum that holds no value writes no
/// entry.
#[test]
fn serde_writes_a_flattened_enums_unit_variant_as_its_name_over_null() {
    let note = || "a".to_owned();
    for (tacking, expected) in [
        (
            Tacking::Bay {
                label: "n".to_owned(),
            },
            r#"{"note":"a","Bay":{"label":"n"}}"#,
        ),
        (Tacking::Calm, r#"{"note":"a","Calm":null}"#),
        (
            Tacking::Dock {
                label: "s".to_owned(),
            },
            r#"{"note":"a","Dock":{"label":"s"}}"#,
        ),
        (Tacking::Held("x".to_owned()), r#"{"note":"a","Held":"x"}"#),
    ] {
        let written = [
            serde_json::to_string(&Stowing {
                note: note(),
                tacking: tacking.clone(),
            }),
            serde_json::to_string(&Belting {
                note: note(),
                tacking: Some(tacking.clone()),
            }),
            serde_json::to_string(&Yawing {
                note: note(),
                tacking,
            }),
        ]
        .map(Result::unwrap);
        assert_eq!(written, [expected; 3]);
    }
    let belting = Belting {
        note: note(),
        tacking: None,
    };
    assert_eq!(serde_json::to_string(&belting).unwrap(), r#"{"note":"a"}"#);
}

/// A flattened externally tagged enum with a unit variant merges what it writes as serde writes
/// it: an object's entry as it is, and a bare name as that name over `null`, the enum optional and
/// under a function too. An enum with no unit variant merges its object as before.
#[test]
#[cfg(feature = "serde")]
fn a_flattened_enum_merges_a_unit_variant_as_its_name_over_null() {
    for (kotlin, merged) in [
        (
            renumbered(&stowing_kotlin::kotlin_definition()),
            "when (val written = output.json.encodeToJsonElement(TackingUnjudged, value.tacking)) \
             { is JsonPrimitive -> put(written.content, JsonNull); else -> \
             written.jsonObject.forEach { (k, v) -> put(k, v) } }",
        ),
        (
            renumbered(&belting_kotlin::kotlin_definition()),
            "value.tacking?.let { flattened -> when (val written = \
             output.json.encodeToJsonElement(TackingUnjudged, flattened)) { is JsonPrimitive -> \
             put(written.content, JsonNull); else -> written.jsonObject.forEach { (k, v) -> put(k, \
             v) } } }",
        ),
        (
            renumbered(&yawing_kotlin::kotlin_definition()),
            "when (val written = output.json.encodeToJsonElement(TackingUnjudged, value.tacking)) \
             { is JsonPrimitive -> put(written.content, JsonNull); else -> \
             written.jsonObject.forEach { (k, v) -> put(k, v) } }",
        ),
        (
            renumbered(&cleating_kotlin::kotlin_definition()),
            "output.json.encodeToJsonElement(JibingUnjudged, value.jibing).jsonObject.forEach { \
             (k, v) -> put(k, v) }",
        ),
    ] {
        assert!(kotlin.contains(merged), "no `{merged}` in: {kotlin}");
    }
}

/// What serde reads of each enum with a flattened member in a variant, written back: a map's
/// entries and a struct's keys at the variant's own level, under every tagging that builds one,
/// each written back where serde writes the variant's own members.
#[test]
fn serde_reads_a_variants_flattened_member_at_the_variants_own_level() {
    let read = [
        GAFFING.map(read_of::<Gaffing>).to_vec(),
        BOWLINING.map(read_of::<Bowlining>).to_vec(),
        SWAYING.map(read_of::<Swaying>).to_vec(),
        WHELPING.map(read_of::<Whelping>).to_vec(),
        BROACHING.map(read_of::<Broaching<u8>>).to_vec(),
    ]
    .concat();
    assert_eq!(
        read,
        [
            r#"{"Counted":{}}"#,
            r#"{"Counted":{"north":2}}"#,
            "refused",
            r#"{"Spare":{}}"#,
            r#"{"Spare":{"north":2}}"#,
            r#"{"Spare":{}}"#,
            r#"{"Labelled":{"label":"n","note":"a"}}"#,
            r#"{"Labelled":{"label":"n","note":"a"}}"#,
            r#"{"Labelled":{"label":"n","note":"a"}}"#,
            "refused",
            "refused",
            r#"{"kind":"Counted","held":{}}"#,
            r#"{"kind":"Counted","held":{"north":2}}"#,
            "refused",
            r#"{"kind":"Spare","held":{"north":2}}"#,
            r#"{"kind":"Spare","held":{}}"#,
            r#"{"kind":"Labelled","held":{"label":"n","note":"a"}}"#,
            "refused",
            r#"{"kind":"Tallied","held":{"label":"n","keys":["north"]}}"#,
            "refused",
            r#"{"label":"n","note":"a"}"#,
            r#"{"north":2}"#,
            "{}",
            "{}",
            "{}",
            r#"{"label":"n","note":"a"}"#,
            r#"{"kind":"Labelled","label":"n","note":"a"}"#,
            r#"{"kind":"Labelled","label":"n","note":"a"}"#,
            r#"{"kind":"Hoisted","label":"n"}"#,
            r#"{"kind":"Hoisted","label":"n"}"#,
            "refused",
            r#"{"Counted":{}}"#,
            r#"{"Counted":{"north":2}}"#,
            "refused",
        ],
        "serde no longer reads each payload as it did"
    );
}

/// A variant with a flattened member is read and written by a serializer the emitter writes, as a
/// struct with one is, under every tagging: the enum hands it the variant's object as it is, and
/// it reads each alias and calls each function of a member itself. A variant with no flattened
/// member keeps the plugin's serializer.
#[test]
#[cfg(feature = "serde")]
fn a_variant_with_a_flattened_member_is_read_by_a_serializer_of_its_own() {
    for (kotlin, written) in [
        (
            renumbered(&gaffing_kotlin::kotlin_definition()),
            "@Serializable(with = GaffingCountedUnjudged::class) data class \
             GaffingCounted(val counts: Map<String, UByte>) : Gaffing { override fun validate(): \
             List<String> = emptyList() } object GaffingCountedUnjudged : \
             KSerializer<GaffingCounted> {",
        ),
        (
            renumbered(&gaffing_kotlin::kotlin_definition()),
            "val counts = obj.mapValues { (_, v) -> \
             input.json.decodeFromJsonElement(KotlinNumber1, v) }; return \
             GaffingCounted(counts = counts) }",
        ),
        (
            renumbered(&gaffing_kotlin::kotlin_definition()),
            "\"Counted\" -> input.json.decodeFromJsonElement(serializer<GaffingCounted>(), data); \
             \"Labelled\" -> input.json.decodeFromJsonElement(serializer<GaffingLabelled>(), \
             data);",
        ),
        (
            renumbered(&gaffing_kotlin::kotlin_definition()),
            "val named = when (key) { \"remark\" -> \"note\"; else -> key }",
        ),
        (
            renumbered(&bowlining_kotlin::kotlin_definition()),
            "val tally = lenient.decodeFromJsonElement(BuntingUnjudged, \
             withItsKeys(JsonObject(obj - labellingKeys)));",
        ),
        (
            renumbered(&bowlining_kotlin::kotlin_definition()),
            "\"Tallied\" -> input.json.decodeFromJsonElement(serializer<BowliningTallied>(), \
             (data ?: throw SerializationException(\"missing field `held`\")))",
        ),
        (
            renumbered(&swaying_kotlin::kotlin_definition()),
            ".recoverCatching { \
             input.json.decodeFromJsonElement(serializer<SwayingAnnotated>(), element) as \
             Swaying }",
        ),
        (
            renumbered(&whelping_kotlin::kotlin_definition()),
            "@SerialName(\"Hoisted\") @Serializable(with = WhelpingHoistedUnjudged::class) data \
             class WhelpingHoisted(val labelling: Labelling) : Whelping",
        ),
        (
            renumbered(&whelping_kotlin::kotlin_definition()),
            "\"Hoisted\" -> input.json.decodeFromJsonElement(serializer<WhelpingHoisted>(), \
             obj);",
        ),
        (
            renumbered(&broaching_kotlin::kotlin_definition()),
            "@Serializable(with = BroachingCountedUnjudged::class) data class \
             BroachingCounted<Count>(val counts: Map<String, Count>) : Broaching<Count> { \
             override fun validate(): List<String> = emptyList() } class \
             BroachingCountedUnjudged<Count: Any>(private val countSerializer: \
             KSerializer<Count>) : KSerializer<BroachingCounted<Count>> {",
        ),
        (
            renumbered(&quaying_kotlin::kotlin_definition()),
            "@Serializable data class QuayingBay(val label: String) : Quaying {",
        ),
    ] {
        assert!(kotlin.contains(written), "no `{written}` in: {kotlin}");
    }
    let gaffing = renumbered(&gaffing_kotlin::kotlin_definition());
    assert_eq!(
        gaffing.matches("\"remark\" -> \"note\"").count(),
        1,
        "the enum reads no alias of a variant that reads its own: {gaffing}"
    );
    let bowlining = renumbered(&bowlining_kotlin::kotlin_definition());
    assert_eq!(
        bowlining.matches("withItsKeys(").count(),
        1,
        "the enum calls no function of a variant that calls its own: {bowlining}"
    );
}

#[test]
fn test_generic_dispatched_enum_serializer_class() {
    let kotlin = renumbered(&choice_kotlin::kotlin_definition());
    assert!(
        kotlin.contains(
            "@Serializable(with = ChoiceSerializer::class) sealed interface Choice<out T>"
        ),
        "the base names its serializer so a property of this type still resolves. got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class ChoiceUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<Choice<T>>"
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
    let kotlin = renumbered(&adjacent_choice_kotlin::kotlin_definition());
    assert!(
        kotlin.contains(
            "class AdjacentChoiceUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<AdjacentChoice<T>>"
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
    let kotlin = renumbered(&untagged_choice_kotlin::kotlin_definition());
    assert!(
        kotlin.contains(
            "@Serializable(with = UntaggedChoiceSerializer::class) sealed interface UntaggedChoice<out T>"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class UntaggedChoiceUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<UntaggedChoice<T>>"
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
    let kotlin = renumbered(&either_kotlin::kotlin_definition());
    assert!(
        kotlin.contains(
            "class EitherUnjudged<L: Any, R: Any>(private val lSerializer: KSerializer<L>, private val rSerializer: KSerializer<R>) : KSerializer<Either<L, R>>"
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
    let kotlin = renumbered(&many_choice_kotlin::kotlin_definition());
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
    let kotlin = renumbered(&generic_pair_kotlin::kotlin_definition());
    assert!(
        kotlin.contains(
            "@Serializable(with = GenericPairSerializer::class) data class GenericPair<T>("
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class GenericPairUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<GenericPair<T>>"
        ),
        "the tuple-struct path earns the same generic serializer class as the enum shapes. got: {kotlin}"
    );
    assert!(
        kotlin.contains("add(output.json.encodeToJsonElement(tSerializer, value.slot0))"),
        "a slot reaching the type parameter composes it. got: {kotlin}"
    );
    assert!(
        kotlin.contains("add(output.json.encodeToJsonElement(KotlinNumber1, value.slot1))"),
        "a slot reaching none of the parameters still resolves through the reified lookup. got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_generic_flatten_own_field() {
    let kotlin = renumbered(&tagged_kotlin::kotlin_definition());
    assert!(
        kotlin.contains("@Serializable(with = TaggedSerializer::class) data class Tagged<T>("),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class TaggedUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<Tagged<T>>"
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
            "val id = input.json.decodeFromJsonElement(tSerializer, (obj[\"id\"] ?: throw \
             SerializationException(\"missing field `id`\")))"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains("output.json.encodeToJsonElement(StampUnjudged, value.stamp)"),
        "the flattened non-generic sibling is read and written by its unjudged serializer. got: {kotlin}"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_generic_flatten_generic_sibling() {
    let kotlin = renumbered(&envelope_kotlin::kotlin_definition());
    assert!(
        kotlin.contains("@Serializable(with = EnvelopeSerializer::class) data class Envelope<T>("),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "class EnvelopeUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : KSerializer<Envelope<T>>"
        ),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "output.json.encodeToJsonElement(WrapperUnjudged(tSerializer), value.body).jsonObject.forEach"
        ),
        "a flattened generic sibling reads its keys and value through its unjudged serializer, \
         bound to the constructor argument. got: {kotlin}"
    );
    assert!(
        kotlin.contains("val bodyKeys = setOf(\"value\")"),
        "got: {kotlin}"
    );
    assert!(
        kotlin.contains(
            "val body = input.json.decodeFromJsonElement(WrapperUnjudged(tSerializer), \
             JsonObject(obj.filterKeys { it in setOf(\"value\") }))"
        ),
        "a flattened struct is read from the whole object its holder reads. got: {kotlin}"
    );
}

#[test]
fn a_member_named_after_a_hard_keyword_is_written_between_backticks() {
    assert_eq!(
        renumbered(&reserved_members_kotlin::kotlin_definition()),
        format!(
            "{} data class ReservedMembers(val `class`: String, val `in`: String, val type: \
             String) {{ fun validate(): List<String> = emptyList() }} {}",
            kept("ReservedMembers"),
            serializers("ReservedMembers")
        )
    );
    assert_eq!(
        renumbered(&reserved_member_kotlin::kotlin_definition()),
        "@Serializable enum class ReservedMember { Default, In, Other; fun validate(): \
         List<String> = emptyList() }"
    );
    assert_eq!(
        serde_json::to_value(ReservedMembers {
            class: "c".to_owned(),
            r#in: "i".to_owned(),
            r#type: "t".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "class": "c", "in": "i", "type": "t" }),
        "the keys serde writes, which the definition above reads and writes"
    );
    assert_eq!(
        serde_json::to_value([
            ReservedMember::Default,
            ReservedMember::In,
            ReservedMember::Other
        ])
        .unwrap(),
        serde_json::json!(["Default", "In", "Other"])
    );
}
