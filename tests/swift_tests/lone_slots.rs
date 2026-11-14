//! What the Swift backend writes for a tuple struct that declares one slot, which serde writes as
//! the slot's value: a struct of one `value`, read from its own decoder and written through the
//! value's own `encode(to:)`, the slot read unjudged where `validate()` walks what it holds and
//! reported under no step, in a type that declares a parameter and in one that declares none. A
//! struct that declares several slots wraps the tuple of the ones serde reads.
//! `service_schema_emitted_client_tests/run_swift_lone_slots.rs` compiles the models of
//! `lone_slots_models.rs` under a Swift toolchain and reads a payload of each.

use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::DateTime;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::lone_slots_models::{
    AdjacentStowDraft, Bale, BaleDraft, BalePairDraft, BaledDraft, BalesByNameDraft,
    BalesByNumberDraft, BalesDraft, Clipped, ClippedDraft, CountDraft, Girth, GirthedDraft,
    LabelsByNumberDraft, LadenDraft, MaybeBaleDraft, MonikerDraft, RiggerDraft, Sack, SackDraft,
    SackedDraft, SingleDraft, SlingDraft, SlungDraft, StowDraft, StowedByNameDraft,
    StowedByNumberDraft, StowedListDraft, StowedMaybeDraft, StowedPairDraft, StropDraft,
    TaggedStowDraft, ToteDraft, TwineDraft, Twined, UnskippedDraft, UntaggedStowDraft,
    bale_pair_swift, baled_swift, bales_by_name_swift, bales_by_number_swift, bales_swift,
    clipped_swift, count_swift, girthed_swift, labels_by_number_swift, laden_swift,
    maybe_bale_swift, moniker_swift, rigger_swift, sacked_swift, single_swift,
    stowed_by_name_swift, stowed_by_number_swift, stowed_list_swift, stowed_maybe_swift,
    stowed_pair_swift, strop_swift, twined_swift, unskipped_swift,
};
#[cfg(feature = "chrono")]
use super::lone_slots_models::{
    StowedStampDraft, TickedDraft, TickedMillisDraft, stowed_stamp_swift, ticked_millis_swift,
    ticked_swift,
};

/// What opens the read of a type alone.
const UNJUDGED: &str = "fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { ";

/// The write of every struct of one slot whose value Swift writes as it holds it: the value's own,
/// through the struct's encoder.
const WRITTEN: &str =
    "public func encode(to encoder: Encoder) throws { try value.encode(to: encoder) }; ";

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
    BaleDraft {
        label: "l".to_owned(),
    }
    .validate()
    .unwrap()
}

/// A sack labelled `l`, holding `7`.
fn sack() -> Sack<u8> {
    SackDraft {
        label: "l".to_owned(),
        stowed: 7,
    }
    .validate()
    .unwrap()
}

#[test]
fn serde_writes_a_struct_of_one_slot_as_the_slots_value() {
    for (written, expected) in [
        (read_back(&CountDraft(5)), "5"),
        (read_back(&MonikerDraft("a".to_owned())), r#""a""#),
        (read_back(&BaledDraft(bale())), r#"{"label":"l"}"#),
        (read_back(&BalesDraft(vec![bale()])), r#"[{"label":"l"}]"#),
        (read_back(&MaybeBaleDraft(Some(bale()))), r#"{"label":"l"}"#),
        (read_back(&MaybeBaleDraft(None)), "null"),
        (
            read_back(&BalesByNameDraft(HashMap::from([("n".to_owned(), bale())]))),
            r#"{"n":{"label":"l"}}"#,
        ),
        (
            read_back(&BalesByNumberDraft(HashMap::from([(1, bale())]))),
            r#"{"1":{"label":"l"}}"#,
        ),
        (
            read_back(&LabelsByNumberDraft(HashMap::from([(1, "a".to_owned())]))),
            r#"{"1":"a"}"#,
        ),
        (
            read_back(&BalePairDraft((bale(), 3))),
            r#"[{"label":"l"},3]"#,
        ),
        (read_back(&UnskippedDraft("s".to_owned())), r#""s""#),
        (read_back(&SingleDraft(5_u32)), "5"),
        (
            read_back(&SackedDraft(sack())),
            r#"{"label":"l","stowed":7}"#,
        ),
        (
            read_back(&StowedListDraft(vec!["a".to_owned()])),
            r#"["a"]"#,
        ),
        (read_back(&StowedMaybeDraft::<String>(None)), "null"),
        (
            read_back(&StowedByNameDraft(HashMap::from([("n".to_owned(), 1_u32)]))),
            r#"{"n":1}"#,
        ),
        (
            read_back(&StowedByNumberDraft(HashMap::from([(1, "a".to_owned())]))),
            r#"{"1":"a"}"#,
        ),
        (
            read_back(&StowedPairDraft((bale(), "a".to_owned()))),
            r#"[{"label":"l"},"a"]"#,
        ),
    ] {
        assert_eq!(written, expected);
    }
    // A slot serde skips is no item of the array it writes, and is read as its default.
    assert_eq!(read_back(&ClippedDraft("a".to_owned(), 0)), r#"["a"]"#);
    assert_eq!(serde_json::from_str::<Clipped>(r#"["a"]"#).unwrap().1, 0);
}

#[cfg(feature = "chrono")]
#[test]
fn serde_writes_a_date_in_one_slot_as_the_slots_value() {
    let date = DateTime::from_timestamp_millis(1_760_329_020_250).unwrap();
    assert_eq!(
        read_back(&TickedDraft(date)),
        r#""2025-10-13T04:17:00.250Z""#
    );
    assert_eq!(read_back(&TickedMillisDraft(date)), "1760329020250");
    assert_eq!(
        read_back(&StowedStampDraft((date, "a".to_owned()))),
        r#"["2025-10-13T04:17:00.250Z","a"]"#
    );
}

#[test]
fn a_holder_writes_a_struct_of_one_slot_as_the_slots_value() {
    let baled = || BaledDraft(bale()).into();
    let moniker = || MonikerDraft("a".to_owned()).validate().unwrap();
    let sacked = || SackedDraft(sack()).into();
    let counted = |count| CountDraft(count).validate().unwrap();
    let tote = ToteDraft {
        baled: baled(),
        counts: vec![counted(1)],
        moniker: moniker(),
        sacked: sacked(),
        single: SingleDraft(9).into(),
    };
    for (written, expected) in [
        (
            read_back(&tote),
            r#"{"baled":{"label":"l"},"counts":[1],"moniker":"a","sacked":{"label":"l","stowed":7},"single":9}"#,
        ),
        (
            read_back(&StowDraft::Counted(counted(5))),
            r#"{"Counted":5}"#,
        ),
        (
            read_back(&StowDraft::Listed(vec![baled()])),
            r#"{"Listed":[{"label":"l"}]}"#,
        ),
        (read_back(&StowDraft::Named(moniker())), r#"{"Named":"a"}"#),
        (
            read_back(&StowDraft::Packed(baled())),
            r#"{"Packed":{"label":"l"}}"#,
        ),
        (
            read_back(&StowDraft::Sacked(sacked())),
            r#"{"Sacked":{"label":"l","stowed":7}}"#,
        ),
        (
            read_back(&AdjacentStowDraft::Counted(counted(5))),
            r#"{"kind":"Counted","held":5}"#,
        ),
        (
            read_back(&AdjacentStowDraft::Listed(vec![baled()])),
            r#"{"kind":"Listed","held":[{"label":"l"}]}"#,
        ),
        (
            read_back(&AdjacentStowDraft::Named(moniker())),
            r#"{"kind":"Named","held":"a"}"#,
        ),
        (
            read_back(&AdjacentStowDraft::Packed(baled())),
            r#"{"kind":"Packed","held":{"label":"l"}}"#,
        ),
        (
            read_back(&AdjacentStowDraft::Sacked(sacked())),
            r#"{"kind":"Sacked","held":{"label":"l","stowed":7}}"#,
        ),
        (read_back(&UntaggedStowDraft::Counted(counted(5))), "5"),
        (
            read_back(&UntaggedStowDraft::Listed(vec![baled()])),
            r#"[{"label":"l"}]"#,
        ),
        (read_back(&UntaggedStowDraft::Named(moniker())), r#""a""#),
        (
            read_back(&UntaggedStowDraft::Packed(baled())),
            r#"{"label":"l"}"#,
        ),
        (
            read_back(&TaggedStowDraft::Packed(baled())),
            r#"{"kind":"Packed","label":"l"}"#,
        ),
        (
            read_back(&TaggedStowDraft::Sacked(sacked())),
            r#"{"kind":"Sacked","label":"l","stowed":7}"#,
        ),
    ] {
        assert_eq!(written, expected);
    }
}

/// A struct of one slot and a brand over a model write their value through their own encoder, so
/// flattened, the model's keys join the holder's object beside its own members, as serde writes
/// them.
#[test]
fn a_flattened_struct_of_one_slot_and_brand_join_their_keys_to_the_holder() {
    let bales = || BalesDraft(vec![bale()]);
    let laden = LadenDraft {
        baled: BaledDraft(bale()).into(),
        bales: bales().validate().unwrap(),
        carried_bales: SackDraft {
            label: "c".to_owned(),
            stowed: bales().validate().unwrap(),
        }
        .validate()
        .unwrap(),
        carried_count: SackDraft {
            label: "c".to_owned(),
            stowed: CountDraft(2).validate().unwrap(),
        }
        .validate()
        .unwrap(),
        carried_model: SackDraft {
            label: "c".to_owned(),
            stowed: BaledDraft(bale()).validate().unwrap(),
        }
        .validate()
        .unwrap(),
        carried_moniker: SackDraft {
            label: "c".to_owned(),
            stowed: MonikerDraft("a".to_owned()).validate().unwrap(),
        }
        .validate()
        .unwrap(),
        count: CountDraft(2).validate().unwrap(),
        counts_by_name: HashMap::new(),
        models: Vec::new(),
        models_by_name: HashMap::new(),
        monikers: Vec::new(),
        monikers_by_name: HashMap::new(),
        stacks: Vec::new(),
        stacks_by_name: HashMap::new(),
        twined: Twined::new(
            TwineDraft {
                twine: "t".to_owned(),
            }
            .validate()
            .unwrap(),
        )
        .unwrap(),
    };
    let written = read_back(&laden);
    for key in [r#"{"label":"l","#, r#""count":2,"#, r#""twine":"t"}"#] {
        assert!(written.contains(key), "{key} in: {written}");
    }
    holds(&baled_swift::swift_definition(), &[WRITTEN]);
    holds(&twined_swift::swift_definition(), &[WRITTEN]);
    holds(
        &laden_swift::swift_definition(),
        &[
            "try self.baled.encode(to: encoder); ",
            "try self.twined.encode(to: encoder) }",
        ],
    );
}

/// A struct of one slot and a brand over an externally tagged enum, flattened and beside a tag,
/// write a unit variant as an entry of its name over `null`, as serde writes it, told by the
/// wrapper's `value`.
#[test]
fn a_struct_of_one_slot_over_an_enum_writes_a_unit_variant_as_an_entry() {
    assert_eq!(
        read_back(&StropDraft {
            count: 1,
            slung: SlungDraft(SlingDraft::Slack.validate().unwrap()).into(),
        }),
        r#"{"count":1,"Slack":null}"#
    );
    assert_eq!(
        read_back(&GirthedDraft {
            count: 1,
            girth: Girth::new(SlingDraft::Slack.validate().unwrap()).unwrap(),
        }),
        r#"{"count":1,"Slack":null}"#
    );
    assert_eq!(
        read_back(&RiggerDraft::Slung(
            SlungDraft(SlingDraft::Slack.validate().unwrap()).into()
        )),
        r#"{"kind":"Slung","Slack":null}"#
    );
    let entry = "case .slack: var entry = encoder.container(keyedBy: SwiftSchemaAnyKey.self); try \
                 entry.encodeNil(forKey: SwiftSchemaAnyKey(stringValue: \"Slack\")!); ";
    holds(
        &strop_swift::swift_definition(),
        &[&format!(
            "switch self.slung.value.draft {{ {entry}default: try self.slung.encode(to: encoder) }}"
        )],
    );
    holds(
        &girthed_swift::swift_definition(),
        &[&format!(
            "switch self.girth.value.draft {{ {entry}default: try self.girth.encode(to: encoder) }}"
        )],
    );
    holds(
        &rigger_swift::swift_definition(),
        &[&format!(
            "switch payload.value.draft {{ {entry}default: try payload.encode(to: encoder) }}"
        )],
    );
}

/// The reproduction: a number, read from the decoder itself and written through its own
/// `encode(to:)`, with no tuple written for it. An integer, and a type parameter, is read through
/// the type named and mapped back to its own.
#[test]
fn a_struct_of_one_slot_reads_and_writes_the_value_of_its_slot() {
    for (swift, opened, read_as, fitted) in [
        (
            count_swift::swift_definition(),
            "public struct CountDraft: Codable, Sendable { public var value: UInt32; ",
            "UInt32",
            Some("SwiftSchemaFit<UInt32>.self).value"),
        ),
        (
            moniker_swift::swift_definition(),
            "public struct MonikerDraft: Codable, Sendable { public var value: String; ",
            "String",
            None,
        ),
        (
            unskipped_swift::swift_definition(),
            "public struct UnskippedDraft: Codable, Sendable { public var value: String; ",
            "String",
            None,
        ),
    ] {
        let decode = fitted.map_or_else(
            || format!("container.decode({read_as}.self)"),
            |fit| format!("container.decode({fit}"),
        );
        let read = format!(
            "{UNJUDGED}let container = try decoder.singleValueContainer(); \
             self.value = try {decode} }}; "
        );
        holds(&swift, &[opened, &read, WRITTEN]);
        assert!(!swift.contains("Tuple"), "got: {swift}");
        assert!(!swift.contains("unkeyedContainer"), "got: {swift}");
    }
}

/// A struct of one slot that holds its own parameter reads each value behind it with the reader a
/// holder hands in, bare, in a list, under an `Option` and as a map's value.
#[test]
fn a_slot_over_its_parameter_reads_each_value_behind_it_with_the_reader_handed_in() {
    for (swift, read) in [
        (
            single_swift::swift_definition(),
            "self.value = try swiftSchemaReaderOfStowed(decoder) }; ",
        ),
        (
            stowed_list_swift::swift_definition(),
            "var listed = try decoder.unkeyedContainer(); var items: [Stowed] = []; while \
             !listed.isAtEnd { items.append(try swiftSchemaReaderOfStowed(listed.superDecoder())) }; \
             self.value = items }; ",
        ),
        (
            stowed_maybe_swift::swift_definition(),
            "if try !decoder.singleValueContainer().decodeNil() { self.value = try \
             swiftSchemaReaderOfStowed(decoder) } else { self.value = nil } }; ",
        ),
        (
            stowed_by_name_swift::swift_definition(),
            "let keyed = try decoder.container(keyedBy: SwiftSchemaAnyKey.self); var entries: \
             [String: Stowed] = [:]; for entry in keyed.allKeys { entries[entry.stringValue] = \
             try swiftSchemaReaderOfStowed(keyed.superDecoder(forKey: entry)) }; self.value = entries \
             }; ",
        ),
    ] {
        holds(
            &swift,
            &[
                &format!(
                    "fileprivate init(swiftSchemaUnjudged decoder: Decoder, _ \
                     swiftSchemaReaderOfStowed: (Decoder) throws -> Stowed) throws {{ {read}"
                ),
                "fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { try \
                 self.init(swiftSchemaUnjudged: decoder, { try SwiftSchemaFit<Stowed>(from: \
                 $0).value }) }; ",
                WRITTEN,
            ],
        );
    }
}

/// A slot whose models `validate()` walks is read unjudged from the decoder itself: a model, a
/// list, an optional, a map Swift keys natively, one under a key it wraps, and a tuple.
#[test]
fn a_slot_validate_walks_is_read_unjudged_from_the_decoder_itself() {
    for (swift, read) in [
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
            stowed_pair_swift::swift_definition(),
            "self.value = try StowedPairTuple<Stowed>(swiftSchemaUnjudged: decoder) }; ",
        ),
    ] {
        holds(&swift, &[&format!("{UNJUDGED}{read}")]);
    }
    holds(
        &sacked_swift::swift_definition(),
        &[
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String], \
           _ swiftSchemaReaderOfStowed: (Decoder) throws -> Stowed) throws { self.value = try \
           Sack<Stowed>(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken, \
           swiftSchemaReaderOfStowed) }; fileprivate init(swiftSchemaUnjudged decoder: Decoder, \
           swiftSchemaWithout taken: [String]) throws { try self.init(swiftSchemaUnjudged: decoder, \
           swiftSchemaWithout: taken, { try SwiftSchemaFit<Stowed>(from: $0).value }) }; \
           fileprivate init(swiftSchemaUnjudged decoder: Decoder, _ swiftSchemaReaderOfStowed: \
           (Decoder) throws -> Stowed) throws { if !(decoder.codingPath.isEmpty && \
           decoder.userInfo[CodingUserInfoKey(rawValue: \"swiftSchemaFlat\")!] != nil) { for \
           unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys where \
           ![\"label\", \"stowed\"].contains(unknown.stringValue)",
        ],
    );
    holds(
        &baled_swift::swift_definition(),
        &[
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) \
           throws { self.value = try Bale(swiftSchemaUnjudged: decoder, swiftSchemaWithout: \
           taken) }; fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { if \
           !(decoder.codingPath.isEmpty && decoder.userInfo[CodingUserInfoKey(rawValue: \
           \"swiftSchemaFlat\")!] != nil) { for unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys where \
           ![\"label\"].contains(unknown.stringValue)",
        ],
    );
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
            "let written = LabelsByNumberKeyedMap(storage: value); try written.encode(to: \
             encoder) }; ",
        ],
    );
    holds(
        &stowed_by_number_swift::swift_definition(),
        &[
            "let container = try decoder.singleValueContainer(); self.value = try { \
             let wireValue = try container.decode(StowedByNumberKeyedMap<Stowed>.self); \
             return wireValue.storage }() }; ",
            "let written = StowedByNumberKeyedMap<Stowed>(storage: value); try \
             written.encode(to: encoder) }; ",
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
            "let written = Self.swiftSchemaFormatIso8601Date(value); try written.encode(to: \
             encoder) }; ",
            "extension TickedDraft { private static let swiftSchemaIso8601Formatter",
        ],
    );
    holds(
        &ticked_millis_swift::swift_definition(),
        &[
            "public var value: Int64; ",
            "let container = try decoder.singleValueContainer(); \
             self.value = try container.decode(SwiftSchemaFit<Int64>.self).value }; ",
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
            r#"for (index, held) in value.enumerated() { errors += held.validate().map { BalesDraft.swiftSchemaUnder("\(index)", $0) } }; if value.count < 1 { errors.append("too short: minimum length is 1, got \(value.count)") }"#,
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
            "var container = try decoder.unkeyedContainer(); if let count = container.count, \
             count != 1 { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
             container.codingPath, debugDescription: \"invalid length \\(count), expected 1\")) \
             }; slot0 = try container.decode(String.self);  }; ",
            r#"if slot0.unicodeScalars.count < 1 { errors.append("'0': too short: minimum length is 1, got \(slot0.unicodeScalars.count)") }"#,
            "public struct ClippedDraft: Codable, Sendable { public var value: ClippedTuple; ",
            "self.value = try ClippedTuple(swiftSchemaUnjudged: decoder) }; ",
            "public func validationIssues() -> [String] { var errors: [String] = []; \
             errors += value.validate(); return errors }",
        ],
    );
}
