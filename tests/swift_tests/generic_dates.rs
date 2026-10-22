//! What the Swift backend writes for the formatter a `DateTime<Tz>` member is read and written
//! through: Swift refuses a static stored property in a type that declares a parameter, so such a
//! type computes its formatter, and a type that declares none keeps the one it stores.
//! `service_schema_emitted_client_tests/run_swift_generic_dates.rs` compiles these shapes under a
//! Swift toolchain and reads a payload of each the Swift read reads as serde does.

use core::marker::PhantomData;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The formatter a type that declares a parameter computes on each call.
const COMPUTED: &str = "private static var swiftSchemaIso8601Formatter: ISO8601DateFormatter { \
                        let f = ISO8601DateFormatter(); \
                        f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; \
                        return f }; ";

/// The formatter a type that declares no parameter stores.
const STORED: &str = "private static let swiftSchemaIso8601Formatter: ISO8601DateFormatter = { \
                      let f = ISO8601DateFormatter(); \
                      f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; \
                      return f }(); ";

/// A date in a type that declares no parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sighting {
    pub at: DateTime<Utc>,
}

/// A date, one under `as_number`, a list of dates and an optional date, beside the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Voyage<Stowed> {
    pub at: DateTime<Utc>,
    #[model_schema_prop(as_number)]
    pub at_millis: DateTime<Utc>,
    pub held: Stowed,
    pub marks: Vec<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seen: Option<DateTime<Utc>>,
}

/// A brand over a date that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Landfall<Stowed> {
    pub at: DateTime<Utc>,
    pub marker: PhantomData<Stowed>,
}

/// A tuple struct of a date and the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Waypoint<Stowed>(pub DateTime<Utc>, pub Stowed);

/// A date in a tuple and as the value of a map, beside the parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Chart<Stowed> {
    pub by_name: HashMap<String, DateTime<Utc>>,
    pub by_number: HashMap<u32, DateTime<Utc>>,
    pub pair: (DateTime<Utc>, Stowed),
    pub paired_by_number: HashMap<u32, (DateTime<Utc>, Stowed)>,
}

/// A date in a variant with named members that name the parameter and in one whose members name
/// none, in a variant of several slots and in a variant of one, under no tag attribute.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Tide<Stowed> {
    Logged { at: DateTime<Utc>, held: Stowed },
    Noted { at: DateTime<Utc> },
    Paired(DateTime<Utc>, Stowed),
    Stamped(DateTime<Utc>),
}

/// The variants a tag alone takes, those with named members.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedTide<Stowed> {
    Logged { at: DateTime<Utc>, held: Stowed },
    Noted { at: DateTime<Utc> },
}

/// [`Tide`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentTide<Stowed> {
    Logged { at: DateTime<Utc>, held: Stowed },
    Noted { at: DateTime<Utc> },
    Paired(DateTime<Utc>, Stowed),
    Stamped(DateTime<Utc>),
}

/// The variants an untagged enum takes: named members and one slot.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedTide<Stowed> {
    Logged { at: DateTime<Utc>, held: Stowed },
    Noted { at: DateTime<Utc> },
    Stamped(DateTime<Utc>),
}

/// The date `millis` milliseconds after the epoch.
const fn at(millis: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(millis).unwrap()
}

/// The JSON serde writes for `held`, read back by serde as it was written.
fn read_back<Held>(held: &Held) -> String
where
    Held: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(held).unwrap();
    let read: Held = serde_json::from_str(&written).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), written);
    written
}

/// The helpers written for `named`, opened by `formatter` and its twin with no fraction, which
/// reads a date of whole seconds.
fn helpers(named: &str, formatter: &str) -> String {
    let whole = formatter
        .replace(
            "swiftSchemaIso8601Formatter",
            "swiftSchemaWholeIso8601Formatter",
        )
        .replace(
            "[.withInternetDateTime, .withFractionalSeconds]",
            "[.withInternetDateTime]",
        );
    format!(
        "extension {named} {{ {formatter}{whole}private static func \
         swiftSchemaParseIso8601Date(_ text: String, codingPath: [CodingKey]) throws -> Date {{ "
    )
}

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[String]) {
    for text in written {
        assert!(swift.contains(text.as_str()), "{text}\nin: {swift}");
    }
}

#[test]
fn each_model_is_written_as_serde_writes_it() {
    let first = at(1_704_164_645_678);
    let second = at(1_709_208_000_100);
    assert_eq!(
        read_back(&Sighting { at: first }),
        r#"{"at":"2024-01-02T03:04:05.678Z"}"#
    );
    assert_eq!(
        read_back(&Voyage {
            at: first,
            at_millis: second,
            held: "a".to_owned(),
            marks: vec![first, second],
            seen: Some(second),
        }),
        r#"{"at":"2024-01-02T03:04:05.678Z","at_millis":1709208000100,"held":"a","marks":["2024-01-02T03:04:05.678Z","2024-02-29T12:00:00.100Z"],"seen":"2024-02-29T12:00:00.100Z"}"#
    );
    assert_eq!(
        read_back(&Landfall::<String> {
            at: first,
            marker: PhantomData,
        }),
        r#""2024-01-02T03:04:05.678Z""#
    );
    assert_eq!(
        read_back(&Waypoint(first, "a".to_owned())),
        r#"["2024-01-02T03:04:05.678Z","a"]"#
    );
    assert_eq!(
        read_back(&Chart {
            by_name: HashMap::from([("n".to_owned(), first)]),
            by_number: HashMap::from([(1, first)]),
            pair: (first, "a".to_owned()),
            paired_by_number: HashMap::from([(2, (second, "b".to_owned()))]),
        }),
        r#"{"by_name":{"n":"2024-01-02T03:04:05.678Z"},"by_number":{"1":"2024-01-02T03:04:05.678Z"},"pair":["2024-01-02T03:04:05.678Z","a"],"paired_by_number":{"2":["2024-02-29T12:00:00.100Z","b"]}}"#
    );
}

#[test]
fn each_variant_is_written_as_its_tagging_writes_it() {
    let first = at(1_704_164_645_678);
    for (held, written) in [
        (
            Tide::Logged {
                at: first,
                held: "a".to_owned(),
            },
            r#"{"Logged":{"at":"2024-01-02T03:04:05.678Z","held":"a"}}"#,
        ),
        (
            Tide::Noted { at: first },
            r#"{"Noted":{"at":"2024-01-02T03:04:05.678Z"}}"#,
        ),
        (
            Tide::Paired(first, "a".to_owned()),
            r#"{"Paired":["2024-01-02T03:04:05.678Z","a"]}"#,
        ),
        (
            Tide::Stamped(first),
            r#"{"Stamped":"2024-01-02T03:04:05.678Z"}"#,
        ),
    ] {
        assert_eq!(read_back(&held), written);
    }
    for (held, written) in [
        (
            TaggedTide::Logged {
                at: first,
                held: "a".to_owned(),
            },
            r#"{"kind":"Logged","at":"2024-01-02T03:04:05.678Z","held":"a"}"#,
        ),
        (
            TaggedTide::Noted { at: first },
            r#"{"kind":"Noted","at":"2024-01-02T03:04:05.678Z"}"#,
        ),
    ] {
        assert_eq!(read_back(&held), written);
    }
    for (held, written) in [
        (
            AdjacentTide::Logged {
                at: first,
                held: "a".to_owned(),
            },
            r#"{"kind":"Logged","held":{"at":"2024-01-02T03:04:05.678Z","held":"a"}}"#,
        ),
        (
            AdjacentTide::Noted { at: first },
            r#"{"kind":"Noted","held":{"at":"2024-01-02T03:04:05.678Z"}}"#,
        ),
        (
            AdjacentTide::Paired(first, "a".to_owned()),
            r#"{"kind":"Paired","held":["2024-01-02T03:04:05.678Z","a"]}"#,
        ),
        (
            AdjacentTide::Stamped(first),
            r#"{"kind":"Stamped","held":"2024-01-02T03:04:05.678Z"}"#,
        ),
    ] {
        assert_eq!(read_back(&held), written);
    }
    for (held, written) in [
        (
            UntaggedTide::Logged {
                at: first,
                held: "a".to_owned(),
            },
            r#"{"at":"2024-01-02T03:04:05.678Z","held":"a"}"#,
        ),
        (
            UntaggedTide::Noted { at: first },
            r#"{"at":"2024-01-02T03:04:05.678Z"}"#,
        ),
        (
            UntaggedTide::Stamped(first),
            r#""2024-01-02T03:04:05.678Z""#,
        ),
    ] {
        assert_eq!(read_back(&held), written);
    }
}

#[test]
fn a_type_that_declares_no_parameter_keeps_the_formatter_it_stores() {
    let swift = sighting_swift::swift_definition();
    holds(&swift, &[helpers("Sighting", STORED)]);
    assert!(!swift.contains("static var"), "got: {swift}");
}

#[test]
fn a_struct_that_declares_a_parameter_computes_its_formatter() {
    holds(
        &voyage_swift::swift_definition(),
        &[
            "public struct Voyage<Stowed: Codable & Sendable>: Codable, Sendable { ".to_owned(),
            helpers("Voyage", COMPUTED),
            "self.atMillis = try container.decode(Int64.self, forKey: .atMillis); ".to_owned(),
            "private static func swiftSchemaFormatIso8601Date(_ date: Date) -> String { \
             swiftSchemaIso8601Formatter.string(from: date) } };"
                .to_owned(),
        ],
    );
}

#[test]
fn a_brand_that_declares_a_parameter_computes_its_formatter() {
    holds(
        &landfall_swift::swift_definition(),
        &[
            "public struct Landfall<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let value: Date; "
                .to_owned(),
            helpers("Landfall", COMPUTED),
        ],
    );
}

#[test]
fn the_struct_of_a_variant_computes_its_formatter_where_it_declares_a_parameter() {
    for (named, swift) in [
        ("Tide", tide_swift::swift_definition()),
        ("TaggedTide", tagged_tide_swift::swift_definition()),
        ("AdjacentTide", adjacent_tide_swift::swift_definition()),
        ("UntaggedTide", untagged_tide_swift::swift_definition()),
    ] {
        holds(
            &swift,
            &[
                format!(
                    "public struct {named}LoggedPayload<Stowed: Codable & Sendable>: Codable, \
                     Sendable {{ "
                ),
                helpers(&format!("{named}LoggedPayload"), COMPUTED),
                format!("public struct {named}NotedPayload: Codable, Sendable {{ "),
                helpers(&format!("{named}NotedPayload"), STORED),
            ],
        );
    }
}

#[test]
fn no_type_that_declares_a_parameter_stores_a_formatter() {
    for swift in [
        voyage_swift::swift_definition(),
        landfall_swift::swift_definition(),
        waypoint_swift::swift_definition(),
        chart_swift::swift_definition(),
        tide_swift::swift_definition(),
        tagged_tide_swift::swift_definition(),
        adjacent_tide_swift::swift_definition(),
        untagged_tide_swift::swift_definition(),
    ] {
        for declared in swift.split("public struct ").skip(1) {
            let ends = declared.find([':', '<']).unwrap();
            if declared[ends..].starts_with('<') {
                let stored = format!("extension {} {{ {STORED}", &declared[..ends]);
                assert!(!swift.contains(&stored), "{stored}\nin: {swift}");
            }
        }
    }
}
