//! What the Swift backend writes for a date held where no field of its own is written: a tuple's
//! slot, a map's value, a variant's slots and a type argument. Each is read as the text chrono
//! writes and turned into a `Date`, and written back as that text, through the helpers of the type
//! whose read and write it is in; a date under `as_number` in a slot is read and written as the
//! `Int64` serde writes. A date a field, a variant's member and a brand hold is read as it was.
//! `service_schema_emitted_client_tests/run_swift_held_dates.rs` compiles the models of
//! `held_dates_models.rs` under a Swift toolchain and reads a payload of each, and
//! `run_swift_generic_dates.rs` the models of `generic_dates.rs`.

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::generic_dates::{
    adjacent_tide_swift, chart_swift, tide_swift, untagged_tide_swift, waypoint_swift,
};
use super::held_dates_models::{
    AdjacentCountedTide, AdjacentPlainTide, Chime, Chimed, CountedTide, Pannier, Pendulum,
    PlainChart, PlainTide, StowedChart, StowedTicker, Ticker, Tock, UntaggedCountedTide,
    UntaggedPlainTide, adjacent_counted_tide_swift, adjacent_plain_tide_swift, chimed_swift,
    counted_tide_swift, first, plain_chart_swift, plain_tide_swift, second, stowed_chart_swift,
    stowed_ticker_swift, ticker_swift, tock_swift, untagged_counted_tide_swift,
    untagged_plain_tide_swift,
};

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

/// How the date the slot `slot0` was read as is turned into a `Date`.
const SLOT_READ: &str = "let wireSlot0 = try container.decode(String.self); \
                         slot0 = try Self.swiftSchemaParseIso8601Date(wireSlot0, \
                         codingPath: decoder.codingPath); ";

/// How the date of the slot `slot0` is written.
const SLOT_WRITTEN: &str = "try container.encode(Self.swiftSchemaFormatIso8601Date(slot0)); ";

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

/// The text of the struct `named` in `swift`, up to the next type or extension.
fn declared<'swift>(swift: &'swift str, named: &str) -> &'swift str {
    let opened = format!("public struct {named}");
    let found = swift
        .find(&format!("{opened}:"))
        .or_else(|| swift.find(&format!("{opened}<")));
    assert!(found.is_some(), "{named} in: {swift}");
    let at = found.unwrap();
    let rest = &swift[at + opened.len()..];
    let ends = ["public struct ", "public enum ", "extension "]
        .into_iter()
        .filter_map(|next| rest.find(next))
        .min()
        .unwrap_or(rest.len());
    &swift[at..at + opened.len() + ends]
}

/// [`PlainChart`] with [`first`] in each place a date is held.
fn plain_chart() -> PlainChart {
    PlainChart {
        at: second(),
        by_name: HashMap::from([("a".to_owned(), first())]),
        by_number: HashMap::from([(1, first())]),
        by_turn: HashMap::from([(-1, first())]),
        dated: Pannier { held: first() },
        marks_by_name: HashMap::from([("m".to_owned(), vec![first()])]),
        maybe_by_name: HashMap::from([("n".to_owned(), None)]),
        pair: (first(), 1),
        spans: (vec![first()], Some(second())),
        swung: Pendulum {
            first: vec![first()],
            second: None,
        },
    }
}

#[test]
fn each_model_is_written_as_serde_writes_it() {
    assert_eq!(
        read_back(&plain_chart()),
        r#"{"at":"2024-02-29T12:00:00.100Z","by_name":{"a":"2024-01-02T03:04:05.678Z"},"by_number":{"1":"2024-01-02T03:04:05.678Z"},"by_turn":{"-1":"2024-01-02T03:04:05.678Z"},"dated":{"held":"2024-01-02T03:04:05.678Z"},"marks_by_name":{"m":["2024-01-02T03:04:05.678Z"]},"maybe_by_name":{"n":null},"pair":["2024-01-02T03:04:05.678Z",1],"spans":[["2024-01-02T03:04:05.678Z"],"2024-02-29T12:00:00.100Z"],"swung":{"first":["2024-01-02T03:04:05.678Z"],"second":null}}"#
    );
    assert_eq!(
        read_back(&StowedChart {
            at: second(),
            by_name: HashMap::new(),
            by_number: HashMap::new(),
            dated: Chime {
                first: first(),
                second: "c".to_owned(),
            },
            held: "h".to_owned(),
            pair: (first(), "p".to_owned()),
        }),
        r#"{"at":"2024-02-29T12:00:00.100Z","by_name":{},"by_number":{},"dated":{"first":"2024-01-02T03:04:05.678Z","second":"c"},"held":"h","pair":["2024-01-02T03:04:05.678Z","p"]}"#
    );
    assert_eq!(
        read_back(&Tock(first(), 2)),
        r#"["2024-01-02T03:04:05.678Z",2]"#
    );
    assert_eq!(read_back(&Chimed(first())), r#""2024-01-02T03:04:05.678Z""#);
}

#[test]
fn each_variant_is_written_as_its_tagging_writes_it() {
    for (held, written) in [
        (
            read_back(&PlainTide::Logged { at: first() }),
            r#"{"Logged":{"at":"2024-01-02T03:04:05.678Z"}}"#,
        ),
        (
            read_back(&PlainTide::Paired(first(), 1)),
            r#"{"Paired":["2024-01-02T03:04:05.678Z",1]}"#,
        ),
        (read_back(&PlainTide::Pending(None)), r#"{"Pending":null}"#),
        (
            read_back(&PlainTide::Stamped(first())),
            r#"{"Stamped":"2024-01-02T03:04:05.678Z"}"#,
        ),
        (
            read_back(&AdjacentPlainTide::Paired(first(), 1)),
            r#"{"kind":"Paired","held":["2024-01-02T03:04:05.678Z",1]}"#,
        ),
        (
            read_back(&AdjacentPlainTide::Stamped(first())),
            r#"{"kind":"Stamped","held":"2024-01-02T03:04:05.678Z"}"#,
        ),
        (
            read_back(&UntaggedPlainTide::Stamped(first())),
            r#""2024-01-02T03:04:05.678Z""#,
        ),
    ] {
        assert_eq!(held, written);
    }
}

#[test]
fn a_date_under_as_number_in_a_slot_is_written_as_a_number() {
    for (held, written) in [
        (
            read_back(&CountedTide::Counted(first())),
            r#"{"Counted":1704164645678}"#,
        ),
        (
            read_back(&CountedTide::Maybe(Some(first()))),
            r#"{"Maybe":1704164645678}"#,
        ),
        (
            read_back(&CountedTide::Spanned(first(), 1)),
            r#"{"Spanned":[1704164645678,1]}"#,
        ),
        (
            read_back(&AdjacentCountedTide::Counted(first())),
            r#"{"kind":"Counted","held":1704164645678}"#,
        ),
        (
            read_back(&AdjacentCountedTide::Spanned(first(), 1)),
            r#"{"kind":"Spanned","held":[1704164645678,1]}"#,
        ),
        (read_back(&Ticker(first(), 1)), "[1704164645678,1]"),
        (
            read_back(&StowedTicker(first(), "t".to_owned())),
            r#"[1704164645678,"t"]"#,
        ),
    ] {
        assert_eq!(held, written);
    }
}

/// serde writes the date of an untagged variant under `as_number` as its text, where the Swift read
/// takes the number: the run leaves [`UntaggedCountedTide`] out for this.
#[test]
fn an_untagged_variant_writes_its_date_under_as_number_as_its_text() {
    assert_eq!(
        read_back(&UntaggedCountedTide::Counted(first())),
        r#""2024-01-02T03:04:05.678Z""#
    );
}

#[test]
fn a_tuple_reads_and_writes_the_date_of_a_slot_as_its_text() {
    let plain = plain_chart_swift::swift_definition();
    holds(
        declared(&plain, "PlainChartPairTuple"),
        &[
            "let slot0: Date; let slot1: UInt8; ".to_owned(),
            format!("{SLOT_READ}slot1 = try container.decode(UInt8.self); "),
            format!("{SLOT_WRITTEN}try container.encode(slot1); "),
        ],
    );
    holds(
        declared(&plain, "PlainChartSpansTuple"),
        &[
            "let slot0: [Date]; let slot1: Date?; ".to_owned(),
            "let wireSlot0 = try container.decode([String].self); slot0 = try wireSlot0.map { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) }; \
             let wireSlot1 = try container.decode(String?.self); slot1 = try wireSlot1.map { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) }; "
                .to_owned(),
            "try container.encode(slot0.map { Self.swiftSchemaFormatIso8601Date($0) }); \
             try container.encode(slot1.map { Self.swiftSchemaFormatIso8601Date($0) }); "
                .to_owned(),
        ],
    );
    holds(
        &plain,
        &[
            helpers("PlainChartPairTuple", STORED),
            helpers("PlainChartSpansTuple", STORED),
        ],
    );
    let stowed = stowed_chart_swift::swift_definition();
    holds(
        declared(&stowed, "StowedChartPairTuple"),
        &[
            "public struct StowedChartPairTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: Date; let slot1: Stowed; "
                .to_owned(),
            format!("{SLOT_READ}slot1 = try container.decode(Stowed.self); "),
            format!("{SLOT_WRITTEN}try container.encode(slot1); "),
        ],
    );
    holds(&stowed, &[helpers("StowedChartPairTuple", COMPUTED)]);
}

#[test]
fn a_tuple_struct_reads_and_writes_the_date_of_a_slot_as_its_text() {
    let plain = tock_swift::swift_definition();
    holds(
        declared(&plain, "TockTuple"),
        &[
            format!("{SLOT_READ}slot1 = try container.decode(UInt8.self); "),
            SLOT_WRITTEN.to_owned(),
        ],
    );
    holds(&plain, &[helpers("TockTuple", STORED)]);
    let stowed = waypoint_swift::swift_definition();
    holds(
        declared(&stowed, "WaypointTuple"),
        &[
            format!("{SLOT_READ}slot1 = try container.decode(Stowed.self); "),
            SLOT_WRITTEN.to_owned(),
        ],
    );
    holds(&stowed, &[helpers("WaypointTuple", COMPUTED)]);
}

#[test]
fn a_map_swift_keys_natively_turns_each_date_it_holds() {
    let plain = plain_chart_swift::swift_definition();
    holds(
        declared(&plain, "PlainChart"),
        &[
            "public let byName: [String: Date]; ".to_owned(),
            "public let byTurn: [Int: Date]; ".to_owned(),
            "public let marksByName: [String: [Date]]; public let maybeByName: [String: Date?]; "
                .to_owned(),
            "let wireByName = try container.decode([String: String].self, forKey: .byName); \
             self.byName = try wireByName.mapValues { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) }; "
                .to_owned(),
            "let wireByTurn = try container.decode([Int: String].self, forKey: .byTurn); \
             self.byTurn = try wireByTurn.mapValues { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) }; "
                .to_owned(),
            "let wireMarksByName = try container.decode([String: [String]].self, \
             forKey: .marksByName); self.marksByName = try wireMarksByName.mapValues { \
             try $0.map { try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) } }; "
                .to_owned(),
            "let wireMaybeByName = try container.decode([String: String?].self, \
             forKey: .maybeByName); self.maybeByName = try wireMaybeByName.mapValues { \
             try $0.map { try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) } }; "
                .to_owned(),
            "let wireByName = self.byName.mapValues { Self.swiftSchemaFormatIso8601Date($0) }; \
             try container.encode(wireByName, forKey: .byName); "
                .to_owned(),
            "let wireMarksByName = self.marksByName.mapValues { \
             $0.map { Self.swiftSchemaFormatIso8601Date($0) } }; "
                .to_owned(),
            "let wireMaybeByName = self.maybeByName.mapValues { \
             $0.map { Self.swiftSchemaFormatIso8601Date($0) } }; "
                .to_owned(),
        ],
    );
    holds(&plain, &[helpers("PlainChart", STORED)]);
    holds(
        &chart_swift::swift_definition(),
        &[
            "self.byName = try wireByName.mapValues { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) }; "
                .to_owned(),
            helpers("Chart", COMPUTED),
        ],
    );
}

#[test]
fn a_map_under_a_key_swift_wraps_reads_and_writes_each_date_it_holds() {
    let read = "let wireValue = try container.decode(String.self, forKey: codingKey); \
                result[key] = try Self.swiftSchemaParseIso8601Date(wireValue, \
                codingPath: decoder.codingPath) }; ";
    let written =
        "try container.encode(Self.swiftSchemaFormatIso8601Date(value), forKey: codingKey) } } };";
    for (swift, wrapper) in [
        (
            plain_chart_swift::swift_definition(),
            "PlainChartByNumberKeyedMap",
        ),
        (
            stowed_chart_swift::swift_definition(),
            "StowedChartByNumberKeyedMap",
        ),
        (chart_swift::swift_definition(), "ChartByNumberKeyedMap"),
    ] {
        holds(
            declared(&swift, wrapper),
            &[
                format!(
                    "public struct {wrapper}: Codable, Sendable {{ var storage: [UInt32: Date]; "
                ),
                read.to_owned(),
                written.to_owned(),
            ],
        );
        holds(&swift, &[helpers(wrapper, STORED)]);
    }
}

#[test]
fn a_variant_of_one_slot_reads_and_writes_its_date_under_each_tagging() {
    let parsed = "self = .stamped(try Self.swiftSchemaParseIso8601Date(wireStamped, \
                  codingPath: decoder.codingPath)); ";
    for (named, swift, formatter) in [
        ("PlainTide", plain_tide_swift::swift_definition(), STORED),
        ("Tide", tide_swift::swift_definition(), COMPUTED),
    ] {
        holds(
            &swift,
            &[
                "case stamped(Date); ".to_owned(),
                format!(
                    "case \"Stamped\": let wireStamped = try container.decode(String.self, \
                     forKey: key); {parsed}"
                ),
                "try container.encode(Self.swiftSchemaFormatIso8601Date(payload), \
                 forKey: SwiftSchemaExternalCodingKey(stringValue: \"Stamped\")!); "
                    .to_owned(),
                helpers(named, formatter),
            ],
        );
    }
    for (named, swift, formatter) in [
        (
            "AdjacentPlainTide",
            adjacent_plain_tide_swift::swift_definition(),
            STORED,
        ),
        (
            "AdjacentTide",
            adjacent_tide_swift::swift_definition(),
            COMPUTED,
        ),
    ] {
        holds(
            &swift,
            &[
                format!(
                    "case \"Stamped\": let wireStamped = try container.decode(String.self, \
                     forKey: .swiftSchemaContent); {parsed}"
                ),
                "try container.encode(Self.swiftSchemaFormatIso8601Date(payload), \
                 forKey: .swiftSchemaContent); "
                    .to_owned(),
                helpers(named, formatter),
            ],
        );
    }
    for (named, swift, formatter) in [
        (
            "UntaggedPlainTide",
            untagged_plain_tide_swift::swift_definition(),
            STORED,
        ),
        (
            "UntaggedTide",
            untagged_tide_swift::swift_definition(),
            COMPUTED,
        ),
    ] {
        holds(
            &swift,
            &[
                "private static func swiftSchemaReadStamped(_ decoder: Decoder) throws -> Date { \
                 let wireStamped = try String(from: decoder); \
                 return try Self.swiftSchemaParseIso8601Date(wireStamped, \
                 codingPath: decoder.codingPath) }; "
                    .to_owned(),
                format!(
                    "if let value = try? {named}.swiftSchemaReadStamped(decoder) {{ \
                     self = .stamped(value); return }}; "
                ),
                "case .stamped(let value): \
                 try Self.swiftSchemaFormatIso8601Date(value).encode(to: encoder); "
                    .to_owned(),
                helpers(named, formatter),
            ],
        );
    }
}

#[test]
fn an_optional_slot_reads_and_writes_its_date_as_its_text() {
    holds(
        &plain_tide_swift::swift_definition(),
        &[
            "case pending(Date?); ".to_owned(),
            "case \"Pending\": let wirePending = try container.decode(String?.self, forKey: key); \
             self = .pending(try wirePending.map { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) }); "
                .to_owned(),
            "try container.encode(payload.map { Self.swiftSchemaFormatIso8601Date($0) }, \
             forKey: SwiftSchemaExternalCodingKey(stringValue: \"Pending\")!); "
                .to_owned(),
        ],
    );
}

#[test]
fn a_variant_of_several_slots_reads_and_writes_the_date_of_a_slot_as_its_text() {
    for (swift, tuple, formatter) in [
        (
            plain_tide_swift::swift_definition(),
            "PlainTidePairedTuple",
            STORED,
        ),
        (
            adjacent_plain_tide_swift::swift_definition(),
            "AdjacentPlainTidePairedTuple",
            STORED,
        ),
        (tide_swift::swift_definition(), "TidePairedTuple", COMPUTED),
        (
            adjacent_tide_swift::swift_definition(),
            "AdjacentTidePairedTuple",
            COMPUTED,
        ),
    ] {
        holds(
            declared(&swift, tuple),
            &[SLOT_READ.to_owned(), SLOT_WRITTEN.to_owned()],
        );
        holds(&swift, &[helpers(tuple, formatter)]);
    }
}

#[test]
fn a_type_argument_holds_its_date_in_a_wrapper_that_reads_and_writes_its_text() {
    holds(
        &plain_chart_swift::swift_definition(),
        &[
            "public struct PannierArgument0Date: Codable, Sendable { public let value: Date; \
             public init(value: Date) { self.value = value }; \
             public init(swiftSchemaUnjudged decoder: Decoder) throws { \
             let container = try decoder.singleValueContainer(); \
             let wireValue = try container.decode(String.self); \
             self.value = try Self.swiftSchemaParseIso8601Date(wireValue, \
             codingPath: decoder.codingPath) }; "
                .to_owned(),
            "public func encode(to encoder: Encoder) throws { \
             var container = encoder.singleValueContainer(); \
             try container.encode(Self.swiftSchemaFormatIso8601Date(value)) }; \
             public func validate() -> [String] { [] } };"
                .to_owned(),
            helpers("PannierArgument0Date", STORED),
            "public struct PendulumArgument0Date: Codable, Sendable { public let value: [Date]; "
                .to_owned(),
            "let wireValue = try container.decode([String].self); \
             self.value = try wireValue.map { \
             try Self.swiftSchemaParseIso8601Date($0, codingPath: decoder.codingPath) } }; "
                .to_owned(),
            "public struct PendulumArgument1Date: Codable, Sendable { public let value: Date?; "
                .to_owned(),
            "let wireValue = try container.decode(String?.self); ".to_owned(),
            "public let dated: Pannier<PannierArgument0Date>; ".to_owned(),
            "public let swung: Pendulum<PendulumArgument0Date, PendulumArgument1Date>; ".to_owned(),
            "self.dated = try container.decode(Pannier<PannierArgument0Date>.self, \
             forKey: .dated); "
                .to_owned(),
            "try container.encode(self.dated, forKey: .dated); ".to_owned(),
        ],
    );
    holds(
        &stowed_chart_swift::swift_definition(),
        &[
            "public struct ChimeArgument0Date: Codable, Sendable { public let value: Date; "
                .to_owned(),
            helpers("ChimeArgument0Date", STORED),
            "public let dated: Chime<ChimeArgument0Date, Stowed>; ".to_owned(),
        ],
    );
}

#[test]
fn a_date_under_as_number_in_a_slot_is_read_and_written_as_the_number() {
    for (named, swift) in [
        ("CountedTide", counted_tide_swift::swift_definition()),
        (
            "AdjacentCountedTide",
            adjacent_counted_tide_swift::swift_definition(),
        ),
    ] {
        holds(
            &swift,
            &[
                "case counted(Int64); case maybe(Int64?); ".to_owned(),
                format!("{named}SpannedTuple: Codable, Sendable {{ let slot0: Int64; "),
                "slot0 = try container.decode(Int64.self); ".to_owned(),
                "try container.encode(slot0); ".to_owned(),
            ],
        );
        assert!(!swift.contains("extension "), "{swift}");
    }
    holds(
        &counted_tide_swift::swift_definition(),
        &[
            "case \"Counted\": self = .counted(try container.decode(Int64.self, forKey: key)); \
             case \"Maybe\": self = .maybe(try container.decode(Int64?.self, forKey: key)); "
                .to_owned(),
            "case .counted(let payload): var container = encoder.container(keyedBy: \
             SwiftSchemaExternalCodingKey.self); try container.encode(payload, \
             forKey: SwiftSchemaExternalCodingKey(stringValue: \"Counted\")!); "
                .to_owned(),
        ],
    );
    holds(
        &untagged_counted_tide_swift::swift_definition(),
        &[
            "if let value = try? Int64(from: decoder) { self = .counted(value); return }; "
                .to_owned(),
            "case .counted(let value): try value.encode(to: encoder); ".to_owned(),
        ],
    );
    for swift in [
        ticker_swift::swift_definition(),
        stowed_ticker_swift::swift_definition(),
    ] {
        holds(
            &swift,
            &[
                "let slot0: Int64; ".to_owned(),
                "slot0 = try container.decode(Int64.self); ".to_owned(),
                "try container.encode(slot0); ".to_owned(),
            ],
        );
        assert!(!swift.contains("extension "), "{swift}");
    }
}

#[test]
fn a_date_a_field_a_member_and_a_brand_hold_is_read_as_it_was() {
    holds(
        &plain_chart_swift::swift_definition(),
        &[
            "let wireAt = try container.decode(String.self, forKey: .at); \
             self.at = try Self.swiftSchemaParseIso8601Date(wireAt, codingPath: decoder.codingPath); "
                .to_owned(),
            "let wireAt = Self.swiftSchemaFormatIso8601Date(self.at); \
             try container.encode(wireAt, forKey: .at); "
                .to_owned(),
        ],
    );
    holds(
        &plain_tide_swift::swift_definition(),
        &[
            "public struct PlainTideLoggedPayload: Codable, Sendable { public let at: Date; "
                .to_owned(),
            "self.at = try Self.swiftSchemaParseIso8601Date(wireAt, codingPath: decoder.codingPath) }; "
                .to_owned(),
            "case \"Logged\": self = .logged(try PlainTideLoggedPayload(swiftSchemaUnjudged: \
             container.superDecoder(forKey: key))); "
                .to_owned(),
            helpers("PlainTideLoggedPayload", STORED),
        ],
    );
    holds(
        &chimed_swift::swift_definition(),
        &[
            "public struct Chimed: Codable, Sendable { public let value: Date; ".to_owned(),
            "let container = try decoder.singleValueContainer(); self.value = try { \
             let wireValue = try container.decode(String.self); \
             return try Self.swiftSchemaParseIso8601Date(wireValue, \
             codingPath: decoder.codingPath) }() }; "
                .to_owned(),
            "try container.encode(Self.swiftSchemaFormatIso8601Date(value)) }; ".to_owned(),
            helpers("Chimed", STORED),
        ],
    );
}
