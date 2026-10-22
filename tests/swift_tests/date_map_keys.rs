//! What the Swift backend writes for a map keyed by a date: the wrapper the map is read through
//! computes its formatter in a static property, binds each key the formatter reads with `guard`,
//! and writes each key with the same formatter. A wrapper of any other key is written as it was.
//! A list or an optional of such maps is read as one of the wrapper and handed on as each
//! wrapper's storage, with no `try`: the wrapper's own read is what parses a key.
//! `service_schema_emitted_client_tests/run_swift_date_map_keys.rs` compiles the models of
//! `date_map_keys_models.rs` under a Swift toolchain and reads a payload of each.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::date_map_keys_models::{
    AdjacentShift, AdjacentWatch, Chronicle, Daybook, Daybooks, Diary, Logbook, Sailing, Shift,
    StowedLogbook, TaggedShift, TaggedWatch, Tidetable, Timetable, UntaggedShift, UntaggedWatch,
    Watch, adjacent_shift_swift, adjacent_watch_swift, chronicle_swift, daybook_swift,
    daybooks_swift, diary_swift, first, logbook_swift, second, shift_swift, stowed_logbook_swift,
    tagged_shift_swift, tagged_watch_swift, tidetable_swift, timetable_swift, untagged_shift_swift,
    untagged_watch_swift, watch_swift,
};
use super::text_map_keys_models::{almanac_swift, ephemeris_swift, gazetteer_swift};
// The JSON Schema of `Roll` describes each value of its map keyed by a number by `Sailing`'s own.
#[cfg(feature = "jsonschema")]
use super::date_map_keys_models::sailing_schema;

/// The formatter a wrapper of a date key declares ahead of its coding key.
const FORMATTER: &str = "private static var isoFormatterForKeys: ISO8601DateFormatter { \
                         let f = ISO8601DateFormatter(); \
                         f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; \
                         return f }; ";

/// How a wrapper binds a date key, ahead of the entry it reads.
const DATE_KEY: &str = "let wireKey = codingKey.stringValue; \
                        guard let key = Self.isoFormatterForKeys.date(from: wireKey) else { \
                        throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
                        debugDescription: \"bad key \\(wireKey)\") }; result[key] = ";

/// How a wrapper writes a date key.
const WRITTEN_DATE_KEY: &str = "let wireKey = Self.isoFormatterForKeys.string(from: key); ";

/// A map under a key Swift parses, a key it holds as its text and a key `Bool` reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Roll {
    pub by_count: HashMap<u32, Sailing>,
    pub by_day: HashMap<NaiveDate, String>,
    pub flags: HashMap<bool, u8>,
}

/// A map under a key Swift parses, in a type that declares a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Muster<Stowed> {
    pub by_count: HashMap<u32, Stowed>,
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

/// A map keyed by [`first`] alone, holding `held`.
fn once<Held>(held: Held) -> HashMap<DateTime<Utc>, Held> {
    HashMap::from([(first(), held)])
}

/// The text of each wrapper of a map's keys in `swift`, up to the next type.
fn wrappers(swift: &str) -> Vec<&str> {
    swift
        .split("public struct ")
        .filter(|declared| {
            declared
                .split_once([':', '<'])
                .is_some_and(|(name, _)| name.ends_with("KeyedMap"))
        })
        .collect()
}

#[test]
fn each_model_is_written_as_serde_writes_it() {
    let sailing = || Sailing {
        label: "s".to_owned(),
    };
    let logged = r#"{"by_at":{"2024-01-02T03:04:05.678Z":1},"sailings":{"2024-01-02T03:04:05.678Z":{"label":"s"}}}"#;
    let shifted =
        r#"{"by_at":{"2024-01-02T03:04:05.678Z":"t"},"counts":{"2024-01-02T03:04:05.678Z":2}}"#;
    for (written, expected) in [
        (
            read_back(&Timetable {
                by_at: once(1),
                sailings: once(sailing()),
            }),
            logged.to_owned(),
        ),
        (
            read_back(&Tidetable {
                by_at: once("t".to_owned()),
                counts: once(2),
            }),
            shifted.to_owned(),
        ),
        (
            read_back(&Watch::Logged {
                by_at: once(1),
                sailings: once(sailing()),
            }),
            format!(r#"{{"Logged":{logged}}}"#),
        ),
        (
            read_back(&TaggedWatch::Logged {
                by_at: once(1),
                sailings: once(sailing()),
            }),
            format!(r#"{{"kind":"Logged",{}"#, &logged[1..]),
        ),
        (
            read_back(&AdjacentWatch::Logged {
                by_at: once(1),
                sailings: once(sailing()),
            }),
            format!(r#"{{"kind":"Logged","body":{logged}}}"#),
        ),
        (
            read_back(&UntaggedWatch::Logged {
                by_at: once(1),
                sailings: once(sailing()),
            }),
            logged.to_owned(),
        ),
        (
            read_back(&Shift::Logged {
                by_at: once("t".to_owned()),
                counts: once(2),
            }),
            format!(r#"{{"Logged":{shifted}}}"#),
        ),
        (
            read_back(&TaggedShift::Logged {
                by_at: once("t".to_owned()),
                counts: once(2),
            }),
            format!(r#"{{"kind":"Logged",{}"#, &shifted[1..]),
        ),
        (
            read_back(&AdjacentShift::Logged {
                by_at: once("t".to_owned()),
                counts: once(2),
            }),
            format!(r#"{{"kind":"Logged","body":{shifted}}}"#),
        ),
        (
            read_back(&UntaggedShift::Logged {
                by_at: once("t".to_owned()),
                counts: once(2),
            }),
            shifted.to_owned(),
        ),
        (
            read_back(&Daybook(once("d".to_owned()))),
            r#"{"2024-01-02T03:04:05.678Z":"d"}"#.to_owned(),
        ),
        (
            read_back(&Chronicle { by_at: once(3) }),
            r#"{"2024-01-02T03:04:05.678Z":3}"#.to_owned(),
        ),
        (
            read_back(&Diary(HashMap::from([(second(), "d".to_owned())]))),
            r#"{"2024-02-29T12:00:00.100Z":"d"}"#.to_owned(),
        ),
    ] {
        assert_eq!(written, expected);
    }
}

/// chrono writes no fraction for a date of whole seconds, and six or nine digits for a finer one:
/// the formatter reads no text of whole seconds, and keeps three digits of a longer fraction, so
/// no key of the run is such a date.
#[test]
fn a_date_of_whole_seconds_or_finer_than_milliseconds_is_written_as_chrono_writes_it() {
    let whole = DateTime::from_timestamp(1_704_164_645, 0).unwrap();
    let finer = DateTime::from_timestamp(1_704_164_645, 678_901_000).unwrap();
    assert_eq!(
        read_back(&Daybook(HashMap::from([(whole, "d".to_owned())]))),
        r#"{"2024-01-02T03:04:05Z":"d"}"#
    );
    assert_eq!(
        read_back(&Daybook(HashMap::from([(finer, "d".to_owned())]))),
        r#"{"2024-01-02T03:04:05.678901Z":"d"}"#
    );
}

/// Every wrapper of a date key computes its formatter, binds each key it reads with `guard`, one
/// read per wrapper and two for the wrapper of a map whose value is a model, and writes each key
/// with the formatter.
#[test]
fn each_wrapper_of_a_date_key_computes_its_formatter_and_reads_an_optional_key() {
    for (swift, wrapped, reads) in [
        (timetable_swift::swift_definition(), 2, 3),
        (tidetable_swift::swift_definition(), 2, 2),
        (watch_swift::swift_definition(), 2, 3),
        (tagged_watch_swift::swift_definition(), 2, 3),
        (adjacent_watch_swift::swift_definition(), 2, 3),
        (untagged_watch_swift::swift_definition(), 2, 3),
        (shift_swift::swift_definition(), 2, 2),
        (tagged_shift_swift::swift_definition(), 2, 2),
        (adjacent_shift_swift::swift_definition(), 2, 2),
        (untagged_shift_swift::swift_definition(), 2, 2),
        (daybook_swift::swift_definition(), 1, 1),
        (chronicle_swift::swift_definition(), 1, 1),
        (diary_swift::swift_definition(), 1, 1),
        (logbook_swift::swift_definition(), 5, 6),
        (stowed_logbook_swift::swift_definition(), 4, 4),
        (daybooks_swift::swift_definition(), 1, 1),
    ] {
        assert_eq!(wrappers(&swift).len(), wrapped, "{swift}");
        assert_eq!(swift.matches(FORMATTER).count(), wrapped, "{swift}");
        assert_eq!(swift.matches(DATE_KEY).count(), reads, "{swift}");
        assert_eq!(swift.matches(WRITTEN_DATE_KEY).count(), wrapped, "{swift}");
        assert!(
            !swift.contains("private let isoFormatterForKeys"),
            "{swift}"
        );
        assert!(!swift.contains("timeIntervalSince1970: 0"), "{swift}");
    }
}

#[test]
fn the_wrapper_of_a_date_key_is_written_whole() {
    holds(
        &timetable_swift::swift_definition(),
        &[
            "public struct TimetableSailingsKeyedMap: Codable, Sendable { \
           var storage: [Date: Sailing]; \
           private static var isoFormatterForKeys: ISO8601DateFormatter { \
           let f = ISO8601DateFormatter(); \
           f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; return f }; \
           private struct DynamicCodingKey: CodingKey { var stringValue: String; \
           init?(stringValue: String) { self.stringValue = stringValue }; \
           var intValue: Int? { nil }; init?(intValue: Int) { nil } }; \
           init(storage: [Date: Sailing]) { self.storage = storage }; \
           init(swiftSchemaUnjudged decoder: Decoder) throws { \
           let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
           var result: [Date: Sailing] = [:]; for codingKey in container.allKeys { \
           let wireKey = codingKey.stringValue; \
           guard let key = Self.isoFormatterForKeys.date(from: wireKey) else { \
           throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
           debugDescription: \"bad key \\(wireKey)\") }; \
           result[key] = try Sailing(swiftSchemaUnjudged: \
           container.superDecoder(forKey: codingKey)) }; \
           storage = result }; \
           public init(from decoder: Decoder) throws { \
           let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
           var result: [Date: Sailing] = [:]; for codingKey in container.allKeys { \
           let wireKey = codingKey.stringValue; \
           guard let key = Self.isoFormatterForKeys.date(from: wireKey) else { \
           throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
           debugDescription: \"bad key \\(wireKey)\") }; \
           result[key] = try container.decode(Sailing.self, forKey: codingKey) }; \
           storage = result }; \
           public func encode(to encoder: Encoder) throws { \
           var container = encoder.container(keyedBy: DynamicCodingKey.self); \
           for (key, value) in storage { \
           let wireKey = Self.isoFormatterForKeys.string(from: key); \
           guard let codingKey = DynamicCodingKey(stringValue: wireKey) else { continue }; \
           try container.encode(value, forKey: codingKey) } } }; ",
        ],
    );
    holds(
        &tidetable_swift::swift_definition(),
        &[
            "public struct TidetableByAtKeyedMap<Stowed: Codable & Sendable>: Codable, Sendable { \
             var storage: [Date: Stowed]; \
             private static var isoFormatterForKeys: ISO8601DateFormatter { \
             let f = ISO8601DateFormatter(); \
             f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; return f }; ",
            "public let byAt: [Date: Stowed]; ",
        ],
    );
}

/// A wrapper of a key Swift parses from its text declares no formatter, and is written as it was.
#[test]
fn a_wrapper_of_a_parsed_key_is_written_as_it_was() {
    assert_eq!(
        read_back(&Muster {
            by_count: HashMap::from([(7, "m".to_owned())]),
        }),
        r#"{"by_count":{"7":"m"}}"#
    );
    holds(
        &roll_swift::swift_definition(),
        &["public struct RollByCountKeyedMap: Codable, Sendable { \
             var storage: [UInt32: Sailing];  \
             private struct DynamicCodingKey: CodingKey { var stringValue: String; \
             init?(stringValue: String) { self.stringValue = stringValue }; \
             var intValue: Int? { nil }; init?(intValue: Int) { nil } }; \
             init(storage: [UInt32: Sailing]) { self.storage = storage }; \
             init(swiftSchemaUnjudged decoder: Decoder) throws { \
             let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
             var result: [UInt32: Sailing] = [:]; for codingKey in container.allKeys { \
             let wireKey = codingKey.stringValue; guard let key = UInt32(wireKey) else { \
             throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
             debugDescription: \"bad key \\(wireKey)\") }; \
             result[key] = try Sailing(swiftSchemaUnjudged: \
             container.superDecoder(forKey: codingKey)) }; \
             storage = result }; \
             public init(from decoder: Decoder) throws { \
             let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
             var result: [UInt32: Sailing] = [:]; for codingKey in container.allKeys { \
             let wireKey = codingKey.stringValue; guard let key = UInt32(wireKey) else { \
             throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
             debugDescription: \"bad key \\(wireKey)\") }; \
             result[key] = try container.decode(Sailing.self, forKey: codingKey) }; \
             storage = result }; \
             public func encode(to encoder: Encoder) throws { \
             var container = encoder.container(keyedBy: DynamicCodingKey.self); \
             for (key, value) in storage { let wireKey = String(key); \
             guard let codingKey = DynamicCodingKey(stringValue: wireKey) else { continue }; \
             try container.encode(value, forKey: codingKey) } } }; "],
    );
    holds(
        &muster_swift::swift_definition(),
        &[
            "public struct MusterByCountKeyedMap<Stowed: Codable & Sendable>: Codable, Sendable { \
             var storage: [UInt32: Stowed];  \
             private struct DynamicCodingKey: CodingKey { var stringValue: String; \
             init?(stringValue: String) { self.stringValue = stringValue }; \
             var intValue: Int? { nil }; init?(intValue: Int) { nil } }; \
             init(storage: [UInt32: Stowed]) { self.storage = storage }; \
             public init(from decoder: Decoder) throws { \
             let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
             var result: [UInt32: Stowed] = [:]; for codingKey in container.allKeys { \
             let wireKey = codingKey.stringValue; guard let key = UInt32(wireKey) else { \
             throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
             debugDescription: \"bad key \\(wireKey)\") }; \
             result[key] = try container.decode(Stowed.self, forKey: codingKey) }; \
             storage = result }; \
             public func encode(to encoder: Encoder) throws { \
             var container = encoder.container(keyedBy: DynamicCodingKey.self); \
             for (key, value) in storage { let wireKey = String(key); \
             guard let codingKey = DynamicCodingKey(stringValue: wireKey) else { continue }; \
             try container.encode(value, forKey: codingKey) } } }; ",
        ],
    );
}

/// A wrapper of a key Swift holds as its text, and of a flag, declares no formatter, and is
/// written as it was.
#[test]
fn a_wrapper_of_a_text_key_or_a_flag_is_written_as_it_was() {
    assert_eq!(
        read_back(&Roll {
            by_count: HashMap::from([(
                7,
                Sailing {
                    label: "s".to_owned()
                }
            )]),
            by_day: HashMap::from([(first().date_naive(), "d".to_owned())]),
            flags: HashMap::from([(true, 1)]),
        }),
        r#"{"by_count":{"7":{"label":"s"}},"by_day":{"2024-01-02":"d"},"flags":{"true":1}}"#
    );
    holds(
        &roll_swift::swift_definition(),
        &[
            "public struct RollByDayKeyedMap: Codable, Sendable { \
             var storage: [String: String];  \
             private struct DynamicCodingKey: CodingKey { var stringValue: String; \
             init?(stringValue: String) { self.stringValue = stringValue }; \
             var intValue: Int? { nil }; init?(intValue: Int) { nil } }; \
             init(storage: [String: String]) { self.storage = storage }; \
             public init(from decoder: Decoder) throws { \
             let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
             var result: [String: String] = [:]; for codingKey in container.allKeys { \
             let wireKey = codingKey.stringValue; let key = wireKey; \
             result[key] = try container.decode(String.self, forKey: codingKey) }; \
             storage = result }; \
             public func encode(to encoder: Encoder) throws { \
             var container = encoder.container(keyedBy: DynamicCodingKey.self); \
             for (key, value) in storage { let wireKey = key; \
             guard let codingKey = DynamicCodingKey(stringValue: wireKey) else { continue }; \
             try container.encode(value, forKey: codingKey) } } }; ",
            "public struct RollFlagsKeyedMap: Codable, Sendable { \
             var storage: [Bool: UInt8];  \
             private struct DynamicCodingKey: CodingKey { var stringValue: String; \
             init?(stringValue: String) { self.stringValue = stringValue }; \
             var intValue: Int? { nil }; init?(intValue: Int) { nil } }; \
             init(storage: [Bool: UInt8]) { self.storage = storage }; \
             public init(from decoder: Decoder) throws { \
             let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
             var result: [Bool: UInt8] = [:]; for codingKey in container.allKeys { \
             let wireKey = codingKey.stringValue; \
             guard let key = (wireKey == \"true\" ? true : wireKey == \"false\" ? false : nil) \
             else { \
             throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
             debugDescription: \"bad key \\(wireKey)\") }; \
             result[key] = try container.decode(UInt8.self, forKey: codingKey) }; \
             storage = result }; \
             public func encode(to encoder: Encoder) throws { \
             var container = encoder.container(keyedBy: DynamicCodingKey.self); \
             for (key, value) in storage { let wireKey = (key ? \"true\" : \"false\"); \
             guard let codingKey = DynamicCodingKey(stringValue: wireKey) else { continue }; \
             try container.encode(value, forKey: codingKey) } } }; ",
        ],
    );
}

/// A wrapper declares the formatter exactly where its key is a date.
#[test]
fn only_a_wrapper_of_a_date_key_declares_the_formatter() {
    let mut seen = 0_usize;
    for swift in [
        roll_swift::swift_definition(),
        muster_swift::swift_definition(),
        almanac_swift::swift_definition(),
        ephemeris_swift::swift_definition(),
        gazetteer_swift::swift_definition(),
        timetable_swift::swift_definition(),
        tidetable_swift::swift_definition(),
        watch_swift::swift_definition(),
        tagged_shift_swift::swift_definition(),
        diary_swift::swift_definition(),
    ] {
        for wrapper in wrappers(&swift) {
            seen += 1;
            assert_eq!(
                wrapper.contains("isoFormatterForKeys"),
                wrapper.contains("var storage: [Date: "),
                "{wrapper}"
            );
        }
    }
    assert_eq!(seen, 21);
}

/// A list or an optional of maps keyed by a date is read as one of the wrapper, whose own read
/// parses each key, and handed on as each wrapper's storage, with no `try` around a conversion
/// that throws nothing.
#[test]
fn a_list_or_an_optional_of_maps_keyed_by_a_date_hands_on_each_storage_with_no_try() {
    let sailing = Sailing {
        label: "s".to_owned(),
    };
    assert_eq!(
        read_back(&Logbook {
            listed: vec![once(1)],
            listed_or_none: vec![None, Some(once(2))],
            maybe: Some(once(3)),
            maybe_listed: None,
            sailing_lists: vec![once(sailing)],
        }),
        r#"{"listed":[{"2024-01-02T03:04:05.678Z":1}],"listed_or_none":[null,{"2024-01-02T03:04:05.678Z":2}],"maybe":{"2024-01-02T03:04:05.678Z":3},"sailing_lists":[{"2024-01-02T03:04:05.678Z":{"label":"s"}}]}"#
    );
    assert_eq!(
        read_back(&StowedLogbook {
            listed: Vec::new(),
            listed_or_none: Vec::new(),
            maybe: None,
            maybe_listed: Some(vec![once("t".to_owned())]),
        }),
        r#"{"listed":[],"listed_or_none":[],"maybe_listed":[{"2024-01-02T03:04:05.678Z":"t"}]}"#
    );
    assert_eq!(
        read_back(&Daybooks(vec![once("d".to_owned())])),
        r#"[{"2024-01-02T03:04:05.678Z":"d"}]"#
    );
    for (swift, held, parameter) in [
        (logbook_swift::swift_definition(), "Logbook", ""),
        (
            stowed_logbook_swift::swift_definition(),
            "StowedLogbook",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            &[
                &format!(
                    "let wireListed = try container.decode([{held}ListedKeyedMap{parameter}].self, \
                     forKey: .listed); self.listed = wireListed.map {{ $0.storage }}; "
                ),
                &format!(
                    "let wireListedOrNone = try \
                     container.decode([{held}ListedOrNoneKeyedMap{parameter}?].self, \
                     forKey: .listedOrNone); \
                     self.listedOrNone = wireListedOrNone.map {{ $0.map {{ $0.storage }} }}; "
                ),
                &format!(
                    "let wireMaybe = try \
                     container.decodeIfPresent({held}MaybeKeyedMap{parameter}.self, \
                     forKey: .maybe); self.maybe = wireMaybe.map {{ $0.storage }}; "
                ),
                &format!(
                    "let wireMaybeListed = try \
                     container.decodeIfPresent([{held}MaybeListedKeyedMap{parameter}].self, \
                     forKey: .maybeListed); \
                     self.maybeListed = wireMaybeListed.map {{ $0.map {{ $0.storage }} }}"
                ),
            ],
        );
    }
    holds(
        &logbook_swift::swift_definition(),
        &[
            "items.append(try LogbookSailingListsKeyedMap(swiftSchemaUnjudged: \
             listed.superDecoder()).storage) }; self.sailingLists = items",
        ],
    );
    holds(
        &daybooks_swift::swift_definition(),
        &[
            "self.value = try { let wireValue = try container.decode([DaybooksKeyedMap].self); \
             return wireValue.map { $0.storage } }() }; ",
        ],
    );
}
