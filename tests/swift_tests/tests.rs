//! Tests for the Swift type and `Codable`-codec backend (`swift` feature).
//!
//! Each `#[model_schema]` item earns `swift_definition()` inside a `{snake_case}_swift` module
//! beside it — the twin of `tests/dart_tests/tests.rs`, over Swift text instead of Dart text.
//! `u64`/`usize` are refused at expansion under this feature, so none of these fixtures use
//! them; the refusal itself is asserted in `src/model_schema/tests.rs`, where a real compile
//! error can be inspected without failing this file's own build.

use std::collections::HashMap;

#[cfg(feature = "mongodb")]
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use tixschema::model_schema;

// ---------------------------------------------------------------------------------------------
// Struct: the literal sample the task's acceptance criteria gives.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRequest {
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

// ---------------------------------------------------------------------------------------------
// Widths: every integer, the floats, strings, arrays, maps, optionals.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllWidths {
    pub a_bool: bool,
    pub a_char: char,
    pub a_f32: f32,
    pub a_f64: f64,
    pub a_i16: i16,
    pub a_i32: i32,
    pub a_i64: i64,
    pub a_i8: i8,
    pub a_isize: isize,
    pub a_string: String,
    pub a_u16: u16,
    pub a_u32: u32,
    pub a_u8: u8,
    pub array: Vec<String>,
    pub map: HashMap<String, i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<i32>,
}

// ---------------------------------------------------------------------------------------------
// Plain enum.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Active,
    InStock,
}

// ---------------------------------------------------------------------------------------------
// Externally tagged enum (serde's default once a variant carries data).
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExternalTagged {
    Bar(i64),
    Baz,
    Foo { a: String },
}

// ---------------------------------------------------------------------------------------------
// Internally tagged enum (`tag = "..."`, no `content`).
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InternalTagged {
    Foo { a: String },
    Reset,
}

// ---------------------------------------------------------------------------------------------
// Adjacently tagged enum (`tag = "...", content = "..."`).
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum AdjacentTagged {
    Cleared,
    Flag(bool),
}

// ---------------------------------------------------------------------------------------------
// Untagged enum.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Untagged {
    Count { count: i64 },
    Text { text: String },
}

// ---------------------------------------------------------------------------------------------
// Branded newtype (`#[serde(transparent)]`).
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);

// ---------------------------------------------------------------------------------------------
// Alias.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
pub type UserId = String;

// ---------------------------------------------------------------------------------------------
// Tuple field.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuplePoint {
    pub label: String,
    pub pair: (String, u32),
}

// ---------------------------------------------------------------------------------------------
// Non-string map key.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByU32 {
    pub by_u32: HashMap<u32, String>,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByBool {
    pub by_bool: HashMap<bool, String>,
}

#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Primary {
    Primary,
    Secondary,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByEnum {
    pub by_enum: HashMap<Primary, String>,
}

// ---------------------------------------------------------------------------------------------
// Generic struct.
// ---------------------------------------------------------------------------------------------

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub note: String,
    pub value: T,
}

// ---------------------------------------------------------------------------------------------
// `name` override: the Swift name moves, the ident re-publishes as a `typealias`.
// ---------------------------------------------------------------------------------------------

#[model_schema(name = "User")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
    pub id: String,
}

// ---------------------------------------------------------------------------------------------
// `nullable`: the key is always written, even for `nil` — needs a custom `encode(to:)`.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithNullable {
    #[model_schema_prop(nullable)]
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// `#[serde(flatten)]`.
// ---------------------------------------------------------------------------------------------

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlattenedExtra {
    pub reason: String,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithFlatten {
    #[serde(flatten)]
    pub extra: FlattenedExtra,
    pub id: String,
}

// ---------------------------------------------------------------------------------------------
// ObjectId (feature-gated).
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "mongodb")]
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: ObjectId,
}

// ---------------------------------------------------------------------------------------------
// Chrono (feature-gated).
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamps {
    pub date: chrono::NaiveDate,
    pub date_time: chrono::NaiveDateTime,
    pub instant: chrono::DateTime<chrono::Utc>,
    #[model_schema_prop(as_number)]
    pub instant_millis: chrono::DateTime<chrono::Utc>,
    pub time: chrono::NaiveTime,
}

// ---------------------------------------------------------------------------------------------
// Struct.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_struct_earns_codable_with_coding_keys() {
    let swift = window_request_swift::swift_definition();
    assert!(
        swift.contains("public struct WindowRequest: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(
        swift.contains("public let conversationId: String"),
        "got: {swift}"
    );
    assert!(swift.contains("public let limit: UInt32?"), "got: {swift}");
    assert!(
        swift.contains("case conversationId = \"conversation_id\""),
        "got: {swift}"
    );
    assert!(swift.contains("case limit"), "got: {swift}");
    // No hand-written codec: nothing in this struct needs one.
    assert!(!swift.contains("init(from decoder:"), "got: {swift}");
    assert!(!swift.contains("func encode(to encoder:"), "got: {swift}");
}

// ---------------------------------------------------------------------------------------------
// Widths.
// ---------------------------------------------------------------------------------------------

#[test]
fn every_width_maps_to_its_own_swift_type() {
    let swift = all_widths_swift::swift_definition();
    assert!(swift.contains("aBool: Bool"), "got: {swift}");
    assert!(swift.contains("aChar: String"), "got: {swift}");
    assert!(swift.contains("aF32: Float"), "got: {swift}");
    assert!(swift.contains("aF64: Double"), "got: {swift}");
    assert!(swift.contains("aI16: Int16"), "got: {swift}");
    assert!(swift.contains("aI32: Int32"), "got: {swift}");
    assert!(swift.contains("aI64: Int64"), "got: {swift}");
    assert!(swift.contains("aI8: Int8"), "got: {swift}");
    assert!(swift.contains("aIsize: Int"), "got: {swift}");
    assert!(swift.contains("aString: String"), "got: {swift}");
    assert!(swift.contains("aU16: UInt16"), "got: {swift}");
    assert!(swift.contains("aU32: UInt32"), "got: {swift}");
    assert!(swift.contains("aU8: UInt8"), "got: {swift}");
    assert!(swift.contains("array: [String]"), "got: {swift}");
    assert!(swift.contains("map: [String: Int32]"), "got: {swift}");
    assert!(swift.contains("optional: Int32?"), "got: {swift}");
}

// ---------------------------------------------------------------------------------------------
// Plain enum.
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn a_plain_enum_earns_a_string_raw_value_enum() {
    let swift = status_swift::swift_definition();
    assert!(
        swift.contains("public enum Status: String, Codable, Sendable"),
        "got: {swift}"
    );
    assert!(swift.contains("case active = \"active\""), "got: {swift}");
    assert!(swift.contains("case inStock = \"instock\""), "got: {swift}");
}

// ---------------------------------------------------------------------------------------------
// Externally tagged enum.
// ---------------------------------------------------------------------------------------------

#[test]
fn an_externally_tagged_enum_writes_the_payload_under_its_own_tag_key() {
    let swift = external_tagged_swift::swift_definition();
    assert!(
        swift.contains("public enum ExternalTagged: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(
        swift.contains("case foo(ExternalTaggedFooPayload)"),
        "got: {swift}"
    );
    assert!(swift.contains("case bar(Int64)"), "got: {swift}");
    assert!(swift.contains("case baz"), "got: {swift}");
    assert!(
        swift.contains("init(from decoder: Decoder) throws"),
        "got: {swift}"
    );
    assert!(
        swift.contains("func encode(to encoder: Encoder) throws"),
        "got: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// Internally tagged enum.
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn an_internally_tagged_enum_merges_the_tag_into_the_payloads_own_object() {
    let swift = internal_tagged_swift::swift_definition();
    assert!(
        swift.contains("public enum InternalTagged: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(
        swift.contains("case swiftSchemaTag = \"type\""),
        "got: {swift}"
    );
    assert!(
        swift.contains("case \"Foo\": self = .foo(try InternalTaggedFooPayload(from: decoder))")
    );
    assert!(
        swift.contains("case \"Reset\": self = .reset"),
        "got: {swift}"
    );
    assert!(
        swift.contains("try payload.encode(to: encoder)"),
        "got: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// Adjacently tagged enum.
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn an_adjacently_tagged_enum_writes_the_payload_under_its_own_content_key() {
    let swift = adjacent_tagged_swift::swift_definition();
    assert!(
        swift.contains("public enum AdjacentTagged: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(
        swift.contains("case swiftSchemaContent = \"value\", swiftSchemaTag = \"type\""),
        "got: {swift}"
    );
    assert!(
        swift.contains("case .cleared: try container.encode(\"Cleared\""),
        "got: {swift}"
    );
    assert!(
        swift.contains("case .flag(let payload): try container.encode(\"Flag\""),
        "got: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// Untagged enum.
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn an_untagged_enum_tries_each_member_in_turn() {
    let swift = untagged_swift::swift_definition();
    assert!(
        swift.contains("public enum Untagged: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(swift.contains("if let value = try? UntaggedCountPayload(from: decoder)"));
    assert!(swift.contains("if let value = try? UntaggedTextPayload(from: decoder)"));
    assert!(swift.contains("no untagged member matched"), "got: {swift}");
}

// ---------------------------------------------------------------------------------------------
// Branded newtype.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_branded_newtype_earns_a_single_value_container_codec() {
    let swift = correlation_id_swift::swift_definition();
    assert!(
        swift.contains("public struct CorrelationId: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(swift.contains("public let value: String"), "got: {swift}");
    assert!(
        swift.contains("decoder.singleValueContainer()"),
        "got: {swift}"
    );
    assert!(
        swift.contains("encoder.singleValueContainer()"),
        "got: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// Alias.
// ---------------------------------------------------------------------------------------------

#[test]
fn an_alias_earns_a_typealias() {
    let swift = user_id_swift::swift_definition();
    assert!(
        swift.contains("public typealias UserIdType = String;"),
        "got: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// Tuple field.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_tuple_field_earns_an_unkeyed_container_struct() {
    let swift = tuple_point_swift::swift_definition();
    assert!(
        swift.contains("struct TuplePointPairTuple: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(swift.contains("decoder.unkeyedContainer()"), "got: {swift}");
    assert!(swift.contains("encoder.unkeyedContainer()"), "got: {swift}");
    assert!(
        swift.contains("public let pair: TuplePointPairTuple"),
        "got: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// Non-string map key.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_numeric_map_key_earns_a_keyed_wrapper() {
    let swift = by_u32_swift::swift_definition();
    assert!(
        swift.contains("struct ByU32ByU32KeyedMap: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(
        swift.contains("public let byU32: [UInt32: String]"),
        "got: {swift}"
    );
    assert!(swift.contains("DynamicCodingKey"), "got: {swift}");
}

#[test]
fn a_boolean_map_key_earns_a_keyed_wrapper() {
    let swift = by_bool_swift::swift_definition();
    assert!(
        swift.contains("public let byBool: [Bool: String]"),
        "got: {swift}"
    );
    assert!(
        swift.contains(
            "guard let key = (wireKey == \"true\" ? true : wireKey == \"false\" ? false : nil) else"
        ),
        "the decode expression must be an Optional Bool, since it fills a `guard let` — an \
         unrecognized key text is refused exactly like a bad numeric key. Got: {swift}"
    );
}

#[test]
fn a_plain_enum_map_key_earns_a_keyed_wrapper() {
    let swift = by_enum_swift::swift_definition();
    assert!(
        swift.contains("public let byEnum: [Primary: String]"),
        "got: {swift}"
    );
    assert!(swift.contains("Primary(rawValue: wireKey)"), "got: {swift}");
}

// ---------------------------------------------------------------------------------------------
// Generic.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_generic_struct_binds_its_parameter_to_codable_and_sendable() {
    let swift = envelope_swift::swift_definition();
    assert!(
        swift.contains("public struct Envelope<T: Codable & Sendable>: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(swift.contains("public let value: T"), "got: {swift}");
}

// ---------------------------------------------------------------------------------------------
// `name` override and the ident's own re-publish as a `typealias`.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_name_override_moves_the_swift_name_and_republishes_the_ident() {
    let swift = user_data_swift::swift_definition();
    assert!(
        swift.contains("public struct User: Codable, Sendable"),
        "got: {swift}"
    );
    assert!(
        swift.contains("public typealias UserData = User;"),
        "got: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// `nullable`.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_nullable_field_always_writes_its_key_even_when_nil() {
    let swift = with_nullable_swift::swift_definition();
    assert!(swift.contains("public let note: String?"), "got: {swift}");
    assert!(
        swift.contains("try container.encode(note, forKey: .note)"),
        "got: {swift}"
    );
    assert!(
        !swift.contains("try container.encodeIfPresent(note"),
        "a required-nullable field must never be dropped: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// `#[serde(flatten)]`.
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn a_flattened_field_decodes_and_encodes_through_the_same_top_level_decoder() {
    let swift = with_flatten_swift::swift_definition();
    assert!(
        swift.contains("self.extra = try FlattenedExtra(from: decoder)"),
        "got: {swift}"
    );
    assert!(
        swift.contains("try extra.encode(to: encoder)"),
        "got: {swift}"
    );
}

// ---------------------------------------------------------------------------------------------
// ObjectId (feature-gated).
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "mongodb")]
#[test]
fn object_id_maps_to_a_bare_reference() {
    let swift = document_swift::swift_definition();
    assert!(swift.contains("public let id: ObjectId"), "got: {swift}");
}

// ---------------------------------------------------------------------------------------------
// Chrono (feature-gated).
// ---------------------------------------------------------------------------------------------

#[cfg(feature = "chrono")]
#[test]
fn naive_dates_are_plain_strings_and_datetime_is_date_with_a_fractional_second_formatter() {
    let swift = timestamps_swift::swift_definition();
    assert!(swift.contains("public let date: String"), "got: {swift}");
    assert!(
        swift.contains("public let dateTime: String"),
        "got: {swift}"
    );
    assert!(swift.contains("public let time: String"), "got: {swift}");
    assert!(swift.contains("public let instant: Date"), "got: {swift}");
    assert!(
        swift.contains("public let instantMillis: Int64"),
        "got: {swift}"
    );
    assert!(swift.contains("ISO8601DateFormatter"), "got: {swift}");
    assert!(swift.contains("withFractionalSeconds"), "got: {swift}");
}

// ---------------------------------------------------------------------------------------------
// Every fixture also round-trips through real `serde_json`, on top of the emitted Swift text
// each test above reads — the same double duty `tests/dart_tests/tests.rs` puts its own
// fixtures to by constructing one of each.
// ---------------------------------------------------------------------------------------------

#[test]
fn every_fixture_constructs_and_round_trips_through_serde_json() {
    let window_request = WindowRequest {
        conversation_id: "c-1".to_owned(),
        limit: Some(10),
    };
    let round_tripped: WindowRequest =
        serde_json::from_str(&serde_json::to_string(&window_request).unwrap()).unwrap();
    assert_eq!(
        round_tripped.conversation_id,
        window_request.conversation_id
    );

    let all_widths = AllWidths {
        a_bool: true,
        a_char: 'x',
        a_f32: 1.5,
        a_f64: 2.5,
        a_i16: 1,
        a_i32: 2,
        a_i64: 3,
        a_i8: 4,
        a_isize: 5,
        a_string: "s".to_owned(),
        a_u16: 1,
        a_u32: 2,
        a_u8: 3,
        array: vec!["a".to_owned()],
        map: HashMap::from([("k".to_owned(), 1_i32)]),
        optional: None,
    };
    serde_json::to_string(&all_widths).unwrap();

    serde_json::to_string(&Status::Active).unwrap();
    serde_json::to_string(&ExternalTagged::Foo { a: "x".to_owned() }).unwrap();
    serde_json::to_string(&InternalTagged::Foo { a: "x".to_owned() }).unwrap();
    serde_json::to_string(&AdjacentTagged::Flag(true)).unwrap();
    serde_json::to_string(&Untagged::Count { count: 1 }).unwrap();

    let correlation_id = CorrelationId("id-1".to_owned());
    let correlation_round_trip: CorrelationId =
        serde_json::from_str(&serde_json::to_string(&correlation_id).unwrap()).unwrap();
    assert_eq!(correlation_round_trip, correlation_id);

    let _user_id: UserId = "u-1".to_owned();

    serde_json::to_string(&TuplePoint {
        label: "pt".to_owned(),
        pair: ("a".to_owned(), 7),
    })
    .unwrap();
    serde_json::to_string(&ByU32 {
        by_u32: HashMap::from([(7, "seven".to_owned())]),
    })
    .unwrap();
    serde_json::to_string(&ByBool {
        by_bool: HashMap::from([(true, "yes".to_owned())]),
    })
    .unwrap();
    serde_json::to_string(&ByEnum {
        by_enum: HashMap::from([(Primary::Primary, "p".to_owned())]),
    })
    .unwrap();
    serde_json::to_string(&Envelope {
        note: "n1".to_owned(),
        value: "payload".to_owned(),
    })
    .unwrap();
    serde_json::to_string(&UserData {
        id: "u1".to_owned(),
    })
    .unwrap();
    serde_json::to_string(&WithNullable {
        note: Some("hi".to_owned()),
    })
    .unwrap();
    serde_json::to_string(&WithFlatten {
        extra: FlattenedExtra {
            reason: "r".to_owned(),
        },
        id: "id-1".to_owned(),
    })
    .unwrap();

    #[cfg(feature = "mongodb")]
    serde_json::to_string(&Document {
        id: ObjectId::new(),
    })
    .unwrap();

    #[cfg(feature = "chrono")]
    {
        let timestamps = Timestamps {
            date: chrono::NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
            date_time: chrono::NaiveDate::from_ymd_opt(2026, 9, 21)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            instant_millis: chrono::Utc::now(),
            instant: chrono::Utc::now(),
            time: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        };
        serde_json::to_string(&timestamps).unwrap();
    }
}
