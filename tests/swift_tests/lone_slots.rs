//! What the Swift backend writes for a tuple struct that declares one slot, which serde writes as
//! the slot's value: a struct of one `value`, read from its own decoder and written through a
//! single-value container, the slot read unjudged where `validate()` walks what it holds and
//! reported under no step, in a type that declares a parameter and in one that declares none. A
//! struct that declares several slots wraps the tuple of the ones serde reads.
//! `service_schema_emitted_client_tests/run_swift_lone_slots.rs` compiles the models of
//! `lone_slots_models.rs` under a Swift toolchain and reads a payload of each.

use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::DateTime;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[cfg(feature = "chrono")]
use super::lone_slots_models::{
    StowedStamp, Ticked, TickedMillis, stowed_stamp_swift, ticked_millis_swift, ticked_swift,
};
use super::lone_slots_models::{
    AdjacentStow, Bale, BalePair, Baled, Bales, BalesByName, BalesByNumber, Clipped, Count,
    LabelsByNumber, MaybeBale, Moniker, Sack, Sacked, Single, Stow, StowedByName, StowedByNumber,
    StowedList, StowedMaybe, StowedPair, TaggedStow, Tote, UntaggedStow, Unskipped,
    bale_pair_swift, baled_swift, bales_by_name_swift, bales_by_number_swift, bales_swift,
    clipped_swift, count_swift, labels_by_number_swift, maybe_bale_swift, moniker_swift,
    sacked_swift, single_swift, stowed_by_name_swift, stowed_by_number_swift, stowed_list_swift,
    stowed_maybe_swift, stowed_pair_swift, unskipped_swift,
};

/// What opens the read of a type alone.
const UNJUDGED: &str = "public init(swiftSchemaUnjudged decoder: Decoder) throws { ";

/// The write of every struct of one slot whose value Swift writes as it holds it.
const WRITTEN: &str = "public func encode(to encoder: Encoder) throws { \
     var container = encoder.singleValueContainer(); try container.encode(value) }; ";

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

/// A bale labelled `l`.
fn bale() -> Bale {
    Bale {
        label: "l".to_owned(),
    }
}

/// A sack labelled `l`, holding `7`.
fn sack() -> Sack<u8> {
    Sack {
        label: "l".to_owned(),
        stowed: 7,
    }
}

#[test]
fn serde_writes_a_struct_of_one_slot_as_the_slots_value() {
    for (written, expected) in [
        (read_back(&Count(5)), "5"),
        (read_back(&Moniker("a".to_owned())), r#""a""#),
        (read_back(&Baled(bale())), r#"{"label":"l"}"#),
        (read_back(&Bales(vec![bale()])), r#"[{"label":"l"}]"#),
        (read_back(&MaybeBale(Some(bale()))), r#"{"label":"l"}"#),
        (read_back(&MaybeBale(None)), "null"),
        (
            read_back(&BalesByName(HashMap::from([("n".to_owned(), bale())]))),
            r#"{"n":{"label":"l"}}"#,
        ),
        (
            read_back(&BalesByNumber(HashMap::from([(1, bale())]))),
            r#"{"1":{"label":"l"}}"#,
        ),
        (
            read_back(&LabelsByNumber(HashMap::from([(1, "a".to_owned())]))),
            r#"{"1":"a"}"#,
        ),
        (read_back(&BalePair((bale(), 3))), r#"[{"label":"l"},3]"#),
        (read_back(&Unskipped("s".to_owned())), r#""s""#),
        (read_back(&Single(5_u32)), "5"),
        (read_back(&Sacked(sack())), r#"{"label":"l","stowed":7}"#),
        (read_back(&StowedList(vec!["a".to_owned()])), r#"["a"]"#),
        (read_back(&StowedMaybe::<String>(None)), "null"),
        (
            read_back(&StowedByName(HashMap::from([("n".to_owned(), 1_u32)]))),
            r#"{"n":1}"#,
        ),
        (
            read_back(&StowedByNumber(HashMap::from([(1, "a".to_owned())]))),
            r#"{"1":"a"}"#,
        ),
        (
            read_back(&StowedPair((bale(), "a".to_owned()))),
            r#"[{"label":"l"},"a"]"#,
        ),
    ] {
        assert_eq!(written, expected);
    }
    // A slot serde skips is no item of the array it writes, and is read as its default.
    assert_eq!(read_back(&Clipped("a".to_owned(), 0)), r#"["a"]"#);
    assert_eq!(serde_json::from_str::<Clipped>(r#"["a"]"#).unwrap().1, 0);
}

#[cfg(feature = "chrono")]
#[test]
fn serde_writes_a_date_in_one_slot_as_the_slots_value() {
    let date = DateTime::from_timestamp_millis(1_760_329_020_250).unwrap();
    assert_eq!(read_back(&Ticked(date)), r#""2025-10-13T04:17:00.250Z""#);
    assert_eq!(read_back(&TickedMillis(date)), "1760329020250");
    assert_eq!(
        read_back(&StowedStamp((date, "a".to_owned()))),
        r#"["2025-10-13T04:17:00.250Z","a"]"#
    );
}

#[test]
fn a_holder_writes_a_struct_of_one_slot_as_the_slots_value() {
    let tote = Tote {
        baled: Baled(bale()),
        counts: vec![Count(1)],
        moniker: Moniker("a".to_owned()),
        sacked: Sacked(sack()),
        single: Single(9),
    };
    for (written, expected) in [
        (
            read_back(&tote),
            r#"{"baled":{"label":"l"},"counts":[1],"moniker":"a","sacked":{"label":"l","stowed":7},"single":9}"#,
        ),
        (read_back(&Stow::Counted(Count(5))), r#"{"Counted":5}"#),
        (
            read_back(&Stow::Listed(vec![Baled(bale())])),
            r#"{"Listed":[{"label":"l"}]}"#,
        ),
        (
            read_back(&Stow::Named(Moniker("a".to_owned()))),
            r#"{"Named":"a"}"#,
        ),
        (
            read_back(&Stow::Packed(Baled(bale()))),
            r#"{"Packed":{"label":"l"}}"#,
        ),
        (
            read_back(&Stow::Sacked(Sacked(sack()))),
            r#"{"Sacked":{"label":"l","stowed":7}}"#,
        ),
        (
            read_back(&AdjacentStow::Counted(Count(5))),
            r#"{"kind":"Counted","held":5}"#,
        ),
        (
            read_back(&AdjacentStow::Listed(vec![Baled(bale())])),
            r#"{"kind":"Listed","held":[{"label":"l"}]}"#,
        ),
        (
            read_back(&AdjacentStow::Named(Moniker("a".to_owned()))),
            r#"{"kind":"Named","held":"a"}"#,
        ),
        (
            read_back(&AdjacentStow::Packed(Baled(bale()))),
            r#"{"kind":"Packed","held":{"label":"l"}}"#,
        ),
        (
            read_back(&AdjacentStow::Sacked(Sacked(sack()))),
            r#"{"kind":"Sacked","held":{"label":"l","stowed":7}}"#,
        ),
        (read_back(&UntaggedStow::Counted(Count(5))), "5"),
        (
            read_back(&UntaggedStow::Listed(vec![Baled(bale())])),
            r#"[{"label":"l"}]"#,
        ),
        (
            read_back(&UntaggedStow::Named(Moniker("a".to_owned()))),
            r#""a""#,
        ),
        (
            read_back(&UntaggedStow::Packed(Baled(bale()))),
            r#"{"label":"l"}"#,
        ),
        (
            read_back(&TaggedStow::Packed(Baled(bale()))),
            r#"{"kind":"Packed","label":"l"}"#,
        ),
        (
            read_back(&TaggedStow::Sacked(Sacked(sack()))),
            r#"{"kind":"Sacked","label":"l","stowed":7}"#,
        ),
    ] {
        assert_eq!(written, expected);
    }
}

/// The reproduction: a number, read from the decoder itself and written through a single-value
/// container, with no tuple written for it.
#[test]
fn a_struct_of_one_slot_reads_and_writes_the_value_of_its_slot() {
    for (swift, opened, read_as) in [
        (
            count_swift::swift_definition(),
            "public struct Count: Codable, Sendable { public let value: UInt32; ",
            "UInt32",
        ),
        (
            moniker_swift::swift_definition(),
            "public struct Moniker: Codable, Sendable { public let value: String; ",
            "String",
        ),
        (
            unskipped_swift::swift_definition(),
            "public struct Unskipped: Codable, Sendable { public let value: String; ",
            "String",
        ),
        (
            single_swift::swift_definition(),
            "public struct Single<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let value: Stowed; ",
            "Stowed",
        ),
        (
            stowed_list_swift::swift_definition(),
            "public struct StowedList<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let value: [Stowed]; ",
            "[Stowed]",
        ),
        (
            stowed_maybe_swift::swift_definition(),
            "public struct StowedMaybe<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let value: Stowed?; ",
            "Stowed?",
        ),
        (
            stowed_by_name_swift::swift_definition(),
            "public struct StowedByName<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let value: [String: Stowed]; ",
            "[String: Stowed]",
        ),
    ] {
        let read = format!(
            "{UNJUDGED}let container = try decoder.singleValueContainer(); \
             self.value = try container.decode({read_as}.self) }}; "
        );
        holds(&swift, &[opened, &read, WRITTEN]);
        assert!(!swift.contains("Tuple"), "got: {swift}");
        assert!(!swift.contains("unkeyedContainer"), "got: {swift}");
    }
}

/// A slot whose models `validate()` walks is read unjudged from the decoder itself: a model, a
/// list, an optional, a map Swift keys natively, one under a key it wraps, and a tuple.
#[test]
fn a_slot_validate_walks_is_read_unjudged_from_the_decoder_itself() {
    for (swift, read) in [
        (
            baled_swift::swift_definition(),
            "self.value = try Bale(swiftSchemaUnjudged: decoder) }; ",
        ),
        (
            bales_swift::swift_definition(),
            "var listed = try decoder.unkeyedContainer(); var items: [Bale] = []; \
             while !listed.isAtEnd { items.append(try Bale(swiftSchemaUnjudged: listed.superDecoder())) }; \
             self.value = items }; ",
        ),
        (
            maybe_bale_swift::swift_definition(),
            "if try !decoder.singleValueContainer().decodeNil() { \
             self.value = try Bale(swiftSchemaUnjudged: decoder) } else { self.value = nil } }; ",
        ),
        (
            bales_by_name_swift::swift_definition(),
            "let keyed = try decoder.container(keyedBy: SwiftSchemaAnyKey.self); \
             var entries: [String: Bale] = [:]; for entry in keyed.allKeys { \
             entries[entry.stringValue] = try Bale(swiftSchemaUnjudged: keyed.superDecoder(forKey: entry)) }; \
             self.value = entries }; ",
        ),
        (
            bales_by_number_swift::swift_definition(),
            "self.value = try BalesByNumberKeyedMap(swiftSchemaUnjudged: decoder).storage }; ",
        ),
        (
            bale_pair_swift::swift_definition(),
            "self.value = try BalePairTuple(swiftSchemaUnjudged: decoder) }; ",
        ),
        (
            sacked_swift::swift_definition(),
            "self.value = try Sack<Stowed>(swiftSchemaUnjudged: decoder) }; ",
        ),
        (
            stowed_pair_swift::swift_definition(),
            "self.value = try StowedPairTuple<Stowed>(swiftSchemaUnjudged: decoder) }; ",
        ),
    ] {
        holds(&swift, &[&format!("{UNJUDGED}{read}")]);
    }
}

/// A map under a key Swift wraps, which `validate()` walks nothing of, is read and written through
/// the wrapper, with and without a parameter.
#[test]
fn a_slot_swift_converts_is_read_and_written_as_the_type_serde_writes() {
    holds(
        &labels_by_number_swift::swift_definition(),
        &[
            "let container = try decoder.singleValueContainer(); self.value = try { \
             let wireValue = try container.decode(LabelsByNumberKeyedMap.self); \
             return wireValue.storage }() }; ",
            "try container.encode(LabelsByNumberKeyedMap(storage: value)) }; ",
        ],
    );
    holds(
        &stowed_by_number_swift::swift_definition(),
        &[
            "let container = try decoder.singleValueContainer(); self.value = try { \
             let wireValue = try container.decode(StowedByNumberKeyedMap<Stowed>.self); \
             return wireValue.storage }() }; ",
            "try container.encode(StowedByNumberKeyedMap<Stowed>(storage: value)) }; ",
        ],
    );
}

/// A date is read and written as the text serde writes, and a date under `as_number` as the
/// number it writes, as a slot of several is.
#[cfg(feature = "chrono")]
#[test]
fn a_date_in_one_slot_is_read_and_written_as_serde_writes_it() {
    holds(
        &ticked_swift::swift_definition(),
        &[
            "let container = try decoder.singleValueContainer(); self.value = try { \
             let wireValue = try container.decode(String.self); \
             return try Self.swiftSchemaParseIso8601Date(wireValue, codingPath: decoder.codingPath) }() }; ",
            "try container.encode(Self.swiftSchemaFormatIso8601Date(value)) }; ",
            "extension Ticked { private static let swiftSchemaIso8601Formatter",
        ],
    );
    holds(
        &ticked_millis_swift::swift_definition(),
        &[
            "public let value: Int64; ",
            "let container = try decoder.singleValueContainer(); \
             self.value = try container.decode(Int64.self) }; ",
            WRITTEN,
        ],
    );
    holds(
        &stowed_stamp_swift::swift_definition(),
        &["self.value = try container.decode(StowedStampTuple<Stowed>.self) }; "],
    );
}

/// `validate()` reports the one slot under no step: its own bound, and each model it holds.
#[test]
fn the_one_slot_is_reported_under_no_step() {
    for (swift, rule) in [
        (
            count_swift::swift_definition(),
            r#"if value < 1 { errors.append("too small: minimum is 1, got \(value)") }"#,
        ),
        (
            moniker_swift::swift_definition(),
            r#"if value.unicodeScalars.count < 1 { errors.append("too short: minimum length is 1, got \(value.unicodeScalars.count)") }"#,
        ),
        (
            baled_swift::swift_definition(),
            "errors += value.validate(); return errors",
        ),
        (
            bales_swift::swift_definition(),
            r#"if value.count < 1 { errors.append("too short: minimum length is 1, got \(value.count)") }; for (index, held) in value.enumerated() { errors += held.validate().map { Bales.swiftSchemaUnder("\(index)", $0) } }"#,
        ),
        (
            maybe_bale_swift::swift_definition(),
            "if let held = value { errors += held.validate() }",
        ),
        (
            stowed_list_swift::swift_definition(),
            r#"if value.count < 1 { errors.append("too short: minimum length is 1, got \(value.count)") }"#,
        ),
    ] {
        holds(&swift, &[rule]);
    }
}

/// A struct that declares two slots, one of which serde skips, wraps the tuple of the other, and
/// reports it under its position in the array serde writes.
#[test]
fn a_struct_of_several_slots_wraps_the_tuple_of_the_ones_serde_reads() {
    holds(
        &clipped_swift::swift_definition(),
        &[
            "public struct ClippedTuple: Codable, Sendable { let slot0: String; ",
            "var container = try decoder.unkeyedContainer(); \
             slot0 = try container.decode(String.self);  }; ",
            r#"if slot0.unicodeScalars.count < 1 { errors.append("'0': too short: minimum length is 1, got \(slot0.unicodeScalars.count)") }"#,
            "public struct Clipped: Codable, Sendable { public let value: ClippedTuple; ",
            "self.value = try ClippedTuple(swiftSchemaUnjudged: decoder) }; ",
            "public func validate() -> [String] { var errors: [String] = []; \
             errors += value.validate(); return errors }",
        ],
    );
}
