//! What the Swift backend writes for the read of a brand whose value is a `DateTime<Tz>`: the value
//! is read as the text chrono writes and parsed in a closure that throws, and the closure is called
//! with `try`. `service_schema_emitted_client_tests/run_swift_brand_reads.rs` compiles these
//! shapes under a Swift toolchain and reads a payload of each, and `run_swift_generic_dates.rs`
//! the brand of `generic_dates.rs`.

use core::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// How the date of a brand is parsed from the text it was read as.
const PARSED: &str = "try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath)";

/// A brand over a date.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PlainLandfall(pub DateTime<Utc>);

/// A brand over a list of dates, which counts them.
#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Landings(pub Vec<DateTime<Utc>>);

/// A brand over a list of optional dates.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Sightings(pub Vec<Option<DateTime<Utc>>>);

/// A brand over a list of dates that declares a parameter.
#[model_schema(no_display, default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Arrivals<Stowed> {
    pub marker: PhantomData<Stowed>,
    pub marks: Vec<DateTime<Utc>>,
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

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for text in written {
        assert!(swift.contains(text), "{text}\nin: {swift}");
    }
}

#[test]
fn each_brand_is_written_as_serde_writes_it() {
    let first = at(1_704_164_645_678);
    let second = at(1_709_208_000_100);
    assert_eq!(
        read_back(&PlainLandfall(first)),
        r#""2024-01-02T03:04:05.678Z""#
    );
    assert_eq!(
        read_back(&Landings(vec![first, second])),
        r#"["2024-01-02T03:04:05.678Z","2024-02-29T12:00:00.100Z"]"#
    );
    assert_eq!(
        read_back(&Sightings(vec![Some(first), None])),
        r#"["2024-01-02T03:04:05.678Z",null]"#
    );
    assert_eq!(
        read_back(&Arrivals::<String> {
            marker: PhantomData,
            marks: vec![first],
        }),
        r#"["2024-01-02T03:04:05.678Z"]"#
    );
}

#[test]
fn a_brand_calls_the_closure_that_reads_a_date_with_try() {
    for (swift, written) in [
        (
            plain_landfall_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode(String.self); \
             return try Self.swiftSchemaParseIso8601Date(wireValue, \
             codingPath: decoder.codingPath) }() }; "
                .to_owned(),
        ),
        (
            landings_swift::swift_definition(),
            format!(
                "self.value = try {{ let wireValue = try container.decode([String].self); \
                 return try wireValue.map {{ {PARSED} }} }}() }}; "
            ),
        ),
        (
            sightings_swift::swift_definition(),
            format!(
                "self.value = try {{ let wireValue = try container.decode([String?].self); \
                 return try wireValue.map {{ try $0.map {{ {PARSED} }} }} }}() }}; "
            ),
        ),
        (
            arrivals_swift::swift_definition(),
            format!(
                "self.value = try {{ let wireValue = try container.decode([String].self); \
                 return try wireValue.map {{ {PARSED} }} }}() }}; "
            ),
        ),
        (
            super::generic_dates::landfall_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode(String.self); \
             return try Self.swiftSchemaParseIso8601Date(wireValue, \
             codingPath: decoder.codingPath) }() }; "
                .to_owned(),
        ),
    ] {
        holds(&swift, &[written.as_str()]);
    }
}
