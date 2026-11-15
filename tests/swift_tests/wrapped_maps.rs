//! What the Swift backend writes for a map under a key Swift reads through a wrapper, a number, a
//! plain enum and a `bool`, held where no field of its own is written: a tuple's slot, a tuple
//! struct's slot, another map's value, a variant's one slot and the tuple of a variant of several
//! slots. Each is read as the wrapper and handed on as its storage, and written as the wrapper
//! over the value, under each tagging, in a type that declares a parameter and in one that declares
//! none; a map whose values `validate()` walks is read through the wrapper's unjudged read.
//! `service_schema_emitted_client_tests/run_swift_wrapped_maps.rs` compiles the models of
//! `wrapped_maps_models.rs` under a Swift toolchain and reads a payload of each.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::wrapped_maps_models::{
    AdjacentDrawerDraft, AdjacentStowedDrawerDraft, CabinetDraft, CubbyDraft, DrawerDraft, Latch,
    Satchel, SatchelDraft, StowedCabinetDraft, StowedCubbyDraft, StowedDrawerDraft,
    TaggedDrawerDraft, TaggedStowedDrawerDraft, UntaggedDrawerDraft, UntaggedStowedDrawerDraft,
    adjacent_drawer_swift, adjacent_stowed_drawer_swift, cabinet_swift, cubby_swift, drawer_swift,
    stowed_cabinet_swift, stowed_cubby_swift, stowed_drawer_swift, tagged_drawer_swift,
    tagged_stowed_drawer_swift, untagged_drawer_swift, untagged_stowed_drawer_swift,
};

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

/// Each of `written` in `swift`, with `@held` filled with `held` and `@p` with `parameter`. A
/// template that reads another way where the type declares a parameter holds both, the second
/// after a `\u{1}`: for each such type, after its name and a `\u{3}`, the whole statement that hands
/// each reader to the wrapper, the statements of the types apart by a `\u{2}`.
fn holds(swift: &str, held: &str, parameter: &str, written: &[&str]) {
    for template in written {
        let (plain, with_readers) = template.split_once('\u{1}').unwrap_or((template, template));
        let chosen = if parameter.is_empty() {
            plain
        } else {
            with_readers
                .split('\u{2}')
                .filter_map(|entry| entry.split_once('\u{3}'))
                .find_map(|(named, read)| (named == held).then_some(read))
                .unwrap_or(with_readers)
        };
        let text = chosen.replace("@held", held).replace("@p", parameter);
        assert!(swift.contains(&text), "{text}\nin: {swift}");
    }
}

/// A map of `held` under the key `1`.
fn at_one<Held>(held: Held) -> HashMap<u32, Held> {
    HashMap::from([(1, held)])
}

/// A map of `held` under `true`.
fn at_true<Held>(held: Held) -> HashMap<bool, Held> {
    HashMap::from([(true, held)])
}

/// A map of `held` under [`Latch::Inner`].
fn at_inner<Held>(held: Held) -> HashMap<Latch, Held> {
    HashMap::from([(Latch::Inner, held)])
}

/// A satchel labelled `l`.
fn satchel() -> Satchel {
    SatchelDraft {
        label: "l".to_owned(),
    }
    .validate()
    .unwrap()
}

/// The text `a`.
fn text() -> String {
    "a".to_owned()
}

#[test]
fn each_struct_is_written_as_serde_writes_it() {
    assert_eq!(
        read_back(&CubbyDraft {
            flags: (at_true(text()), 1),
            flags_by_number: at_one(at_true(text())),
            latches: (at_inner(text()), 2),
            latches_by_latch: at_inner(at_inner(text())),
            numbers: (at_one(text()), 3),
            numbers_by_name: HashMap::from([("n".to_owned(), at_one(text()))]),
            satchels: (at_one(satchel()), 4),
            satchels_by_name: HashMap::from([("s".to_owned(), at_one(satchel()))]),
        }),
        r#"{"flags":[{"true":"a"},1],"flags_by_number":{"1":{"true":"a"}},"latches":[{"Inner":"a"},2],"latches_by_latch":{"Inner":{"Inner":"a"}},"numbers":[{"1":"a"},3],"numbers_by_name":{"n":{"1":"a"}},"satchels":[{"1":{"label":"l"}},4],"satchels_by_name":{"s":{"1":{"label":"l"}}}}"#
    );
    assert_eq!(
        read_back(&StowedCubbyDraft {
            flags: (at_true(text()), 1),
            flags_by_number: at_one(at_true(text())),
            latches: (at_inner(text()), 2),
            latches_by_latch: at_inner(at_inner(text())),
            numbers: (at_one(text()), text()),
            numbers_by_name: HashMap::from([("n".to_owned(), at_one(text()))]),
            satchels: (at_one(satchel()), text()),
            satchels_by_name: HashMap::new(),
        }),
        r#"{"flags":[{"true":"a"},1],"flags_by_number":{"1":{"true":"a"}},"latches":[{"Inner":"a"},2],"latches_by_latch":{"Inner":{"Inner":"a"}},"numbers":[{"1":"a"},"a"],"numbers_by_name":{"n":{"1":"a"}},"satchels":[{"1":{"label":"l"}},"a"],"satchels_by_name":{}}"#
    );
    let written = r#"[{"1":"a"},{"true":"a"},{"Inner":"a"},{"1":{"label":"l"}}]"#;
    assert_eq!(
        read_back(&CabinetDraft(
            at_one(text()),
            at_true(text()),
            at_inner(text()),
            at_one(satchel())
        )),
        written
    );
    assert_eq!(
        read_back(&StowedCabinetDraft(
            at_one(text()),
            at_true(text()),
            at_inner(text()),
            at_one(satchel())
        )),
        written
    );
}

#[test]
fn each_variant_is_written_as_its_tagging_writes_it() {
    for (held, written) in [
        (
            read_back(&DrawerDraft::Bagged(at_one(satchel()))),
            r#"{"Bagged":{"1":{"label":"l"}}}"#,
        ),
        (
            read_back(&DrawerDraft::Flagged(at_true(text()))),
            r#"{"Flagged":{"true":"a"}}"#,
        ),
        (
            read_back(&DrawerDraft::Keyed(
                at_one(text()),
                at_true(text()),
                at_inner(text()),
            )),
            r#"{"Keyed":[{"1":"a"},{"true":"a"},{"Inner":"a"}]}"#,
        ),
        (
            read_back(&StowedDrawerDraft::Numbered(at_one(text()))),
            r#"{"Numbered":{"1":"a"}}"#,
        ),
        (
            read_back(&StowedDrawerDraft::Packed(at_true(satchel()), text())),
            r#"{"Packed":[{"true":{"label":"l"}},"a"]}"#,
        ),
        (
            read_back(&AdjacentDrawerDraft::Keyed(
                at_one(text()),
                at_true(text()),
                at_inner(text()),
            )),
            r#"{"kind":"Keyed","held":[{"1":"a"},{"true":"a"},{"Inner":"a"}]}"#,
        ),
        (
            read_back(&AdjacentStowedDrawerDraft::Numbered(at_one(text()))),
            r#"{"kind":"Numbered","held":{"1":"a"}}"#,
        ),
        (
            read_back(&UntaggedDrawerDraft::Latched(at_inner(text()))),
            r#"{"Inner":"a"}"#,
        ),
        (
            read_back(&UntaggedStowedDrawerDraft::Paired((at_inner(text()), 1))),
            r#"[{"Inner":"a"},1]"#,
        ),
        (
            read_back(&UntaggedDrawerDraft::Sealed(at_inner(satchel()))),
            r#"{"Inner":{"label":"l"}}"#,
        ),
        (
            read_back(&TaggedDrawerDraft::Stacked {
                latches_by_latch: at_inner(at_inner(text())),
                pair: (at_inner(text()), 1),
                sealed: (at_inner(satchel()), 2),
            }),
            r#"{"kind":"Stacked","latches_by_latch":{"Inner":{"Inner":"a"}},"pair":[{"Inner":"a"},1],"sealed":[{"Inner":{"label":"l"}},2]}"#,
        ),
        (
            read_back(&TaggedStowedDrawerDraft::Stacked {
                latches_by_latch: HashMap::new(),
                pair: (at_inner(text()), 1),
                sealed: (HashMap::new(), 2),
            }),
            r#"{"kind":"Stacked","latches_by_latch":{},"pair":[{"Inner":"a"},1],"sealed":[{},2]}"#,
        ),
    ] {
        assert_eq!(held, written);
    }
}

/// serde reads an untagged enum and a variant under a tag alone from what it buffers, where it
/// refuses a number key and a `bool` key written as text, and reads an empty map: the models key
/// every map there by a plain enum.
#[test]
fn serde_refuses_a_number_key_and_a_bool_key_it_buffers() {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Numbered {
        Held(HashMap<u32, String>),
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Flagged {
        Held(HashMap<bool, String>),
    }
    #[derive(Deserialize)]
    #[serde(tag = "kind")]
    enum Tagged {
        Held { by_number: HashMap<u32, String> },
    }
    assert!(serde_json::from_str::<Numbered>("{}").is_ok_and(|Numbered::Held(map)| map.is_empty()));
    assert!(serde_json::from_str::<Numbered>(r#"{"1":"a"}"#).is_err());
    assert!(serde_json::from_str::<Flagged>("{}").is_ok_and(|Flagged::Held(map)| map.is_empty()));
    assert!(serde_json::from_str::<Flagged>(r#"{"true":"a"}"#).is_err());
    assert!(
        serde_json::from_str::<Tagged>(r#"{"kind":"Held","by_number":{}}"#)
            .is_ok_and(|Tagged::Held { by_number }| by_number.is_empty())
    );
    assert!(serde_json::from_str::<Tagged>(r#"{"kind":"Held","by_number":{"1":"a"}}"#).is_err());
}

#[test]
fn a_tuple_reads_and_writes_a_map_in_its_slot_through_the_wrapper() {
    for (swift, held, parameter) in [
        (cubby_swift::swift_definition(), "Cubby", ""),
        (
            stowed_cubby_swift::swift_definition(),
            "StowedCubby",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireSlot0 = try container.decode(@heldFlagsTupleSlot0KeyedMap@p.self); slot0 = wireSlot0.storage; \u{1}StowedCubby\u{3}slot0 = try StowedCubbyFlagsTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try container.decode(SwiftSchemaFit<UInt8>.self).value;",
                "try container.encode(@heldFlagsTupleSlot0KeyedMap@p(storage: slot0)); ",
                "let wireSlot0 = try container.decode(@heldLatchesTupleSlot0KeyedMap@p.self); slot0 = wireSlot0.storage; \u{1}StowedCubby\u{3}slot0 = try StowedCubbyLatchesTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try container.decode(SwiftSchemaFit<UInt8>.self).value;",
                "try container.encode(@heldLatchesTupleSlot0KeyedMap@p(storage: slot0)); ",
                "let wireSlot0 = try container.decode(@heldNumbersTupleSlot0KeyedMap@p.self); slot0 = wireSlot0.storage; \u{1}StowedCubby\u{3}slot0 = try StowedCubbyNumbersTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try swiftSchemaReaderOfStowed(container.superDecoder());",
                "try container.encode(@heldNumbersTupleSlot0KeyedMap@p(storage: slot0)); ",
                "slot0 = try @heldSatchelsTupleSlot0KeyedMap(swiftSchemaUnjudged: \
                 container.superDecoder()).storage; ",
                "try container.encode(@heldSatchelsTupleSlot0KeyedMap(storage: slot0)); ",
            ],
        );
    }
    holds(
        &cubby_swift::swift_definition(),
        "",
        "",
        &[
            "public struct CubbyNumbersTuple: Codable, Sendable { let slot0: [UInt32: String]; \
           let slot1: UInt8; ",
        ],
    );
    holds(
        &stowed_cubby_swift::swift_definition(),
        "",
        "",
        &[
            "public struct StowedCubbyFlagsTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: [Bool: Stowed]; let slot1: UInt8; ",
        ],
    );
}

#[test]
fn a_tuple_struct_reads_and_writes_a_map_in_each_slot_through_the_wrapper() {
    for (swift, held, parameter) in [
        (cabinet_swift::swift_definition(), "Cabinet", ""),
        (
            stowed_cabinet_swift::swift_definition(),
            "StowedCabinet",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireSlot0 = try container.decode(@heldTupleSlot0KeyedMap@p.self); slot0 = wireSlot0.storage; let wireSlot1 = try container.decode(@heldTupleSlot1KeyedMap@p.self); slot1 = wireSlot1.storage; let wireSlot2 = try container.decode(@heldTupleSlot2KeyedMap@p.self); slot2 = wireSlot2.storage; slot3 = try @heldTupleSlot3KeyedMap(swiftSchemaUnjudged: container.superDecoder()).storage; \u{1}StowedCabinet\u{3}slot0 = try StowedCabinetTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try StowedCabinetTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot2 = try StowedCabinetTupleSlot2KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage;\u{2}StowedCellar\u{3}var listed = try container.nestedUnkeyedContainer(); var items: [[UInt32: Stowed]] = []; while !listed.isAtEnd { items.append(try StowedCellarTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: listed.superDecoder(), swiftSchemaReaderOfStowed).storage) }; slot0 = items; var listed1 = try container.nestedUnkeyedContainer(); var items1: [[Bool: Stowed]?] = []; while !listed1.isAtEnd { if try !listed1.decodeNil() { items1.append(try StowedCellarTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: listed1.superDecoder(), swiftSchemaReaderOfStowed).storage) } else { items1.append(nil) } }; slot1 = items1; if try !container.decodeNil() { var listed2 = try container.nestedUnkeyedContainer(); var items2: [[Rung: Stowed]] = []; while !listed2.isAtEnd { items2.append(try StowedCellarTupleSlot2KeyedMap<Stowed>(swiftSchemaUnjudged: listed2.superDecoder(), swiftSchemaReaderOfStowed).storage) }; slot2 = items2 } else { slot2 = nil };",
                "try container.encode(@heldTupleSlot0KeyedMap@p(storage: slot0)); \
                 try container.encode(@heldTupleSlot1KeyedMap@p(storage: slot1)); \
                 try container.encode(@heldTupleSlot2KeyedMap@p(storage: slot2)); \
                 try container.encode(@heldTupleSlot3KeyedMap(storage: slot3)); ",
            ],
        );
    }
}

#[test]
fn a_map_reads_and_writes_a_map_it_holds_through_the_wrapper() {
    for (swift, held, parameter) in [
        (cubby_swift::swift_definition(), "Cubby", ""),
        (
            stowed_cubby_swift::swift_definition(),
            "StowedCubby",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireValue = try container.decode(@heldFlagsByNumberValueKeyedMap@p.self, \
                 forKey: codingKey); result[key] = wireValue.storage }; ",
                "try container.encode(@heldFlagsByNumberValueKeyedMap@p(storage: value), \
                 forKey: codingKey) } } };",
                "let wireValue = try container.decode(@heldLatchesByLatchValueKeyedMap@p.self, \
                 forKey: codingKey); result[key] = wireValue.storage }; ",
                "try container.encode(@heldLatchesByLatchValueKeyedMap@p(storage: value), \
                 forKey: codingKey) } } };",
                "let wireNumbersByName = try container.decode([String: @heldNumbersByNameValueKeyedMap@p].self, forKey: .numbersByName); self.numbersByName = wireNumbersByName.mapValues { $0.storage }; \u{1}StowedCubby\u{3}let keyed = try container.nestedContainer(keyedBy: SwiftSchemaAnyKey.self, forKey: .numbersByName); var entries: [String: [UInt32: Stowed]] = [:]; for entry in keyed.allKeys { entries[entry.stringValue] = try StowedCubbyNumbersByNameValueKeyedMap<Stowed>(swiftSchemaUnjudged: keyed.superDecoder(forKey: entry), swiftSchemaReaderOfStowed).storage }; self.numbersByName = entries;",
                "let wireNumbersByName = self.numbersByName.mapValues { \
                 @heldNumbersByNameValueKeyedMap@p(storage: $0) }; ",
                "entries[entry.stringValue] = try \
                 @heldSatchelsByNameValueKeyedMap(swiftSchemaUnjudged: \
                 keyed.superDecoder(forKey: entry)).storage }; \u{1}StowedCubby\u{3}\
                 entries1[entry1.stringValue] = try \
                 @heldSatchelsByNameValueKeyedMap(swiftSchemaUnjudged: \
                 keyed1.superDecoder(forKey: entry1)).storage }; self.satchelsByName = \
                 entries1 }; ",
                "let wireSatchelsByName = self.satchelsByName.mapValues { \
                 @heldSatchelsByNameValueKeyedMap(storage: $0) }; ",
            ],
        );
    }
}

#[test]
fn a_variant_of_one_slot_reads_and_writes_its_map_through_the_wrapper_under_each_tagging() {
    for (swift, held, parameter) in [
        (drawer_swift::swift_definition(), "Drawer", ""),
        (
            stowed_drawer_swift::swift_definition(),
            "StowedDrawer",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "case \"Bagged\": self = .bagged(try \
                 @heldBaggedValueKeyedMap(swiftSchemaUnjudged: \
                 container.superDecoder(forKey: key)).storage); ",
                "case \"Flagged\": let wireFlagged = try container.decode(@heldFlaggedValueKeyedMap@p.self, forKey: key); self = .flagged(wireFlagged.storage); \u{1}AdjacentStowedDrawer\u{3}case \"Flagged\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .flagged(try AdjacentStowedDrawerFlaggedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage); case \"Keyed\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .keyed(try AdjacentStowedDrawerKeyedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed));\u{2}StowedDrawer\u{3}case \"Flagged\": self = .flagged(try StowedDrawerFlaggedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage); case \"Keyed\": self = .keyed(try StowedDrawerKeyedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed));",
                "case \"Latched\": let wireLatched = try container.decode(@heldLatchedValueKeyedMap@p.self, forKey: key); self = .latched(wireLatched.storage); \u{1}AdjacentStowedDrawer\u{3}case \"Latched\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .latched(try AdjacentStowedDrawerLatchedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage); case \"Numbered\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .numbered(try AdjacentStowedDrawerNumberedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage);\u{2}StowedDrawer\u{3}case \"Latched\": self = .latched(try StowedDrawerLatchedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage); case \"Numbered\": self = .numbered(try StowedDrawerNumberedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage);\u{2}UntaggedStowedDrawer\u{3}if let value = try? UntaggedStowedDrawerLatchedValueKeyedMap<Stowed>(swiftSchemaUnjudged: decoder, swiftSchemaReaderOfStowed).storage { self = .latched(value); return };",
                "case \"Numbered\": let wireNumbered = try container.decode(@heldNumberedValueKeyedMap@p.self, forKey: key); self = .numbered(wireNumbered.storage); \u{1}AdjacentStowedDrawer\u{3}case \"Numbered\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .numbered(try AdjacentStowedDrawerNumberedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage); case \"Packed\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .packed(try AdjacentStowedDrawerPackedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed));\u{2}StowedDrawer\u{3}case \"Numbered\": self = .numbered(try StowedDrawerNumberedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage); case \"Packed\": self = .packed(try StowedDrawerPackedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed));",
                "try container.encode(@heldBaggedValueKeyedMap(storage: payload), \
                 forKey: SwiftSchemaExternalCodingKey(stringValue: \"Bagged\")!); ",
                "try container.encode(@heldFlaggedValueKeyedMap@p(storage: payload), \
                 forKey: SwiftSchemaExternalCodingKey(stringValue: \"Flagged\")!); ",
                "try container.encode(@heldLatchedValueKeyedMap@p(storage: payload), \
                 forKey: SwiftSchemaExternalCodingKey(stringValue: \"Latched\")!); ",
                "try container.encode(@heldNumberedValueKeyedMap@p(storage: payload), \
                 forKey: SwiftSchemaExternalCodingKey(stringValue: \"Numbered\")!); ",
            ],
        );
    }
    for (swift, held, parameter) in [
        (
            adjacent_drawer_swift::swift_definition(),
            "AdjacentDrawer",
            "",
        ),
        (
            adjacent_stowed_drawer_swift::swift_definition(),
            "AdjacentStowedDrawer",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "case \"Bagged\": _ = try container.decodeNil(forKey: .swiftSchemaContent); \
                 self = .bagged(try @heldBaggedValueKeyedMap(swiftSchemaUnjudged: \
                 container.superDecoder(forKey: .swiftSchemaContent)).storage); ",
                "case \"Flagged\": let wireFlagged = try container.decode(@heldFlaggedValueKeyedMap@p.self, forKey: .swiftSchemaContent); self = .flagged(wireFlagged.storage); \u{1}AdjacentStowedDrawer\u{3}case \"Flagged\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .flagged(try AdjacentStowedDrawerFlaggedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage); case \"Keyed\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .keyed(try AdjacentStowedDrawerKeyedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed));\u{2}StowedDrawer\u{3}case \"Flagged\": self = .flagged(try StowedDrawerFlaggedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage); case \"Keyed\": self = .keyed(try StowedDrawerKeyedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed));",
                "case \"Numbered\": let wireNumbered = try container.decode(@heldNumberedValueKeyedMap@p.self, forKey: .swiftSchemaContent); self = .numbered(wireNumbered.storage); \u{1}AdjacentStowedDrawer\u{3}case \"Numbered\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .numbered(try AdjacentStowedDrawerNumberedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage); case \"Packed\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .packed(try AdjacentStowedDrawerPackedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed));\u{2}StowedDrawer\u{3}case \"Numbered\": self = .numbered(try StowedDrawerNumberedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage); case \"Packed\": self = .packed(try StowedDrawerPackedTuple<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed));",
                "try container.encode(@heldBaggedValueKeyedMap(storage: payload), \
                 forKey: .swiftSchemaContent); ",
                "try container.encode(@heldFlaggedValueKeyedMap@p(storage: payload), \
                 forKey: .swiftSchemaContent); ",
                "try container.encode(@heldNumberedValueKeyedMap@p(storage: payload), \
                 forKey: .swiftSchemaContent); ",
            ],
        );
    }
    for (swift, held, parameter) in [
        (
            untagged_drawer_swift::swift_definition(),
            "UntaggedDrawer",
            "",
        ),
        (
            untagged_stowed_drawer_swift::swift_definition(),
            "UntaggedStowedDrawer",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireLatched = try @heldLatchedValueKeyedMap@p(from: decoder); return wireLatched.storage }; \u{1}AdjacentStowedDrawer\u{3}self = .latched(try AdjacentStowedDrawerLatchedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage); case \"Numbered\": _ = try container.decodeNil(forKey: .swiftSchemaContent); self = .numbered(try AdjacentStowedDrawerNumberedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: .swiftSchemaContent), swiftSchemaReaderOfStowed).storage);\u{2}StowedDrawer\u{3}case \"Latched\": self = .latched(try StowedDrawerLatchedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage); case \"Numbered\": self = .numbered(try StowedDrawerNumberedValueKeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(forKey: key), swiftSchemaReaderOfStowed).storage);\u{2}UntaggedStowedDrawer\u{3}if let value = try? UntaggedStowedDrawerLatchedValueKeyedMap<Stowed>(swiftSchemaUnjudged: decoder, swiftSchemaReaderOfStowed).storage { self = .latched(value); return };",
                "if let value = try? @heldSealedValueKeyedMap(swiftSchemaUnjudged: \
                 decoder).storage { self = .sealed(value); return }; ",
                "case .latched(let value): try \
                 @heldLatchedValueKeyedMap@p(storage: value).encode(to: encoder); ",
                "case .sealed(let value): try \
                 @heldSealedValueKeyedMap(storage: value).encode(to: encoder); ",
            ],
        );
    }
}

#[test]
fn the_tuple_of_a_variant_of_several_slots_reads_and_writes_each_map_through_the_wrapper() {
    for (swift, held, parameter) in [
        (drawer_swift::swift_definition(), "Drawer", ""),
        (
            stowed_drawer_swift::swift_definition(),
            "StowedDrawer",
            "<Stowed>",
        ),
        (
            adjacent_drawer_swift::swift_definition(),
            "AdjacentDrawer",
            "",
        ),
        (
            adjacent_stowed_drawer_swift::swift_definition(),
            "AdjacentStowedDrawer",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireSlot0 = try container.decode(@heldKeyedTupleSlot0KeyedMap@p.self); slot0 = wireSlot0.storage; let wireSlot1 = try container.decode(@heldKeyedTupleSlot1KeyedMap@p.self); slot1 = wireSlot1.storage; let wireSlot2 = try container.decode(@heldKeyedTupleSlot2KeyedMap@p.self); slot2 = wireSlot2.storage; \u{1}AdjacentStowedDrawer\u{3}slot0 = try AdjacentStowedDrawerKeyedTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try AdjacentStowedDrawerKeyedTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot2 = try AdjacentStowedDrawerKeyedTupleSlot2KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage;\u{2}StowedDrawer\u{3}slot0 = try StowedDrawerKeyedTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try StowedDrawerKeyedTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot2 = try StowedDrawerKeyedTupleSlot2KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage;",
                "try container.encode(@heldKeyedTupleSlot0KeyedMap@p(storage: slot0)); \
                 try container.encode(@heldKeyedTupleSlot1KeyedMap@p(storage: slot1)); \
                 try container.encode(@heldKeyedTupleSlot2KeyedMap@p(storage: slot2)); ",
                "slot0 = try @heldPackedTupleSlot0KeyedMap(swiftSchemaUnjudged: \
                 container.superDecoder()).storage; ",
                "try container.encode(@heldPackedTupleSlot0KeyedMap(storage: slot0)); ",
            ],
        );
    }
}

#[test]
fn named_members_read_and_write_a_map_in_a_tuple_through_the_wrapper_under_each_tagging() {
    for (swift, held, parameter) in [
        (drawer_swift::swift_definition(), "Drawer", ""),
        (
            stowed_drawer_swift::swift_definition(),
            "StowedDrawer",
            "<Stowed>",
        ),
        (
            adjacent_drawer_swift::swift_definition(),
            "AdjacentDrawer",
            "",
        ),
        (
            adjacent_stowed_drawer_swift::swift_definition(),
            "AdjacentStowedDrawer",
            "<Stowed>",
        ),
        (
            untagged_drawer_swift::swift_definition(),
            "UntaggedDrawer",
            "",
        ),
        (
            untagged_stowed_drawer_swift::swift_definition(),
            "UntaggedStowedDrawer",
            "<Stowed>",
        ),
        (tagged_drawer_swift::swift_definition(), "TaggedDrawer", ""),
        (
            tagged_stowed_drawer_swift::swift_definition(),
            "TaggedStowedDrawer",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireSlot0 = try container.decode(@heldStackedPayloadPairTupleSlot0KeyedMap@p.self); slot0 = wireSlot0.storage; \u{1}AdjacentStowedDrawer\u{3}slot0 = try AdjacentStowedDrawerStackedPayloadPairTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try AdjacentStowedDrawerStackedPayloadPairTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage;\u{2}StowedDrawer\u{3}slot0 = try StowedDrawerStackedPayloadPairTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try StowedDrawerStackedPayloadPairTupleSlot1KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage;\u{2}TaggedStowedDrawer\u{3}slot0 = try TaggedStowedDrawerStackedPayloadPairTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try container.decode(SwiftSchemaFit<UInt8>.self).value;\u{2}UntaggedStowedDrawer\u{3}slot0 = try UntaggedStowedDrawerStackedPayloadPairTupleSlot0KeyedMap<Stowed>(swiftSchemaUnjudged: container.superDecoder(), swiftSchemaReaderOfStowed).storage; slot1 = try container.decode(SwiftSchemaFit<UInt8>.self).value;",
                "try container.encode(@heldStackedPayloadPairTupleSlot0KeyedMap@p(storage: \
                 slot0)); ",
            ],
        );
    }
    for (swift, held, parameter) in [
        (tagged_drawer_swift::swift_definition(), "TaggedDrawer", ""),
        (
            tagged_stowed_drawer_swift::swift_definition(),
            "TaggedStowedDrawer",
            "<Stowed>",
        ),
    ] {
        holds(
            &swift,
            held,
            parameter,
            &[
                "let wireValue = try \
                 container.decode(@heldStackedPayloadLatchesByLatchValueKeyedMap@p.self, \
                 forKey: codingKey); result[key] = wireValue.storage }; ",
                "try container.encode(@heldStackedPayloadLatchesByLatchValueKeyedMap@p(storage: \
                 value), forKey: codingKey) } } };",
                "slot0 = try @heldStackedPayloadSealedTupleSlot0KeyedMap(swiftSchemaUnjudged: \
                 container.superDecoder()).storage; ",
                "try container.encode(@heldStackedPayloadSealedTupleSlot0KeyedMap(storage: \
                 slot0)); ",
            ],
        );
    }
}

#[test]
fn validate_walks_the_models_a_map_holds_in_each_position() {
    holds(
        &cubby_swift::swift_definition(),
        "",
        "",
        &[
            "for (key, held) in slot0.sorted(by: { $0.key < $1.key }) { errors += \
             held.validate().map { CubbySatchelsTuple.swiftSchemaUnder(\"0.\\(key)\", $0) } }; ",
            "errors += satchels.validate().map { CubbyDraft.swiftSchemaUnder(\"satchels\", $0) }; ",
        ],
    );
    holds(
        &drawer_swift::swift_definition(),
        "",
        "",
        &[
            "case .bagged(let payload): for (key, held) in payload.sorted(by: { $0.key < $1.key }) \
             { errors += held.validate().map { \
             DrawerDraft.swiftSchemaUnder(\"Bagged.\\(key)\", $0) } }; ",
            "case .packed(let payload): errors += payload.validate().map { \
             DrawerDraft.swiftSchemaUnder(\"Packed\", $0) }; ",
        ],
    );
    holds(
        &untagged_drawer_swift::swift_definition(),
        "",
        "",
        &[
            "case .sealed(let payload): for (key, held) in payload.sorted(by: { \
           $0.key.rawValue < $1.key.rawValue }) { errors += held.validate().map { \
           UntaggedDrawerDraft.swiftSchemaUnder(\"\\(key.rawValue)\", $0) } }; ",
        ],
    );
}
