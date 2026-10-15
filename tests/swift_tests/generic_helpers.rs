//! What the Swift backend writes for a tuple and for a map under a key Swift reads through a
//! wrapper, where the type holding it declares a parameter: the struct written for the tuple or the
//! map declares the parameters its members reach, under the bound the type declares them with, and
//! every type position names it at those parameters. A struct whose members reach none keeps the
//! text it had. `wrapped_maps.rs` pins how a map under a key Swift wraps is read and written in a
//! tuple's slot, a tuple struct's slot, a variant's slot and another map's value.
//! `service_schema_emitted_client_tests/run_swift_generic_helpers.rs` compiles these shapes under a
//! Swift toolchain and reads a payload of each, such a map in a slot and a list of such maps
//! included, but an alias of a tuple, which
//! `run_swift_renamed_generics.rs` reads, an alias of a map under a key Swift wraps, which that run
//! compiles and does not read, Swift reading it through its own `Dictionary` coding, a map keyed by
//! the parameter, which `run_swift_text_map_keys.rs` reads, and a brand, which
//! `run_swift_brand_reads.rs` reads.

use core::hash::Hash;
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A plain enum a map is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Deck {
    Lower,
    Upper,
}

/// A model with a rule, filled with a parameter.
#[model_schema(default_types(Contents = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cask<Contents> {
    pub contents: Contents,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// The tuple and the map of [`Stack`] that name no parameter, held by a type that declares none.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pile {
    pub by_count: HashMap<u32, u8>,
    pub marks: (u32, u8),
}

/// The parameter in a tuple, as the value of a map under a key Swift wraps, in a list of such maps,
/// in a model such a map holds, in a tuple such a map holds, in a map a tuple holds and in a tuple
/// a model is filled with; beside a tuple and a map that name none.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stack<Stowed> {
    pub by_cask: HashMap<u32, Cask<Stowed>>,
    pub by_count: HashMap<u32, u8>,
    pub by_number: HashMap<u32, Stowed>,
    pub cask: Cask<(Stowed, u8)>,
    pub listed: Vec<HashMap<u32, Stowed>>,
    pub marks: (u32, u8),
    pub numbered_pair: (HashMap<u32, Stowed>, u8),
    pub pair: (Stowed, u32),
    pub paired_by_number: HashMap<u32, (Stowed, u8)>,
}

/// The parameter as the key of a map, which Swift reads through a wrapper.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ranked<Stowed: Eq + Hash> {
    pub by_stowed: HashMap<Stowed, u32>,
}

/// A tuple struct whose slots are a tuple of the parameter and a number.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tally<Stowed>(pub (Stowed, u8), pub u32);

/// A tuple struct whose slot is a map of the parameter under a key Swift wraps.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Register<Stowed>(pub HashMap<u32, Stowed>, pub u8);

/// A brand over a map of the parameter under a key Swift wraps.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Hoard<Stowed>(pub HashMap<u32, Stowed>);

/// An alias of a tuple of the parameter.
#[model_schema(default_types(Stowed = String))]
pub type Pairing<Stowed> = (Stowed, u32);

/// An alias of a map of the parameter under a key Swift wraps.
#[model_schema(default_types(Stowed = String))]
pub type Numbering<Stowed> = HashMap<u32, Stowed>;

/// Each variant that holds a tuple or a map of the parameter, under no tag attribute.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Split<Stowed> {
    Halves(Stowed, u8),
    Keyed(HashMap<u32, Stowed>, u8),
    Numbered(HashMap<u32, Stowed>),
    Paired((Stowed, u32)),
    Stacked {
        by_number: HashMap<u32, Stowed>,
        pair: (Stowed, u32),
    },
}

/// The variant a tag alone takes, a variant with named members.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedSplit<Stowed> {
    Stacked {
        by_deck: HashMap<Deck, Stowed>,
        pair: (Stowed, u32),
    },
}

/// [`Split`] under a tag and a content key.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentSplit<Stowed> {
    Halves(Stowed, u8),
    Keyed(HashMap<u32, Stowed>, u8),
    Numbered(HashMap<u32, Stowed>),
    Paired((Stowed, u32)),
    Stacked {
        by_number: HashMap<u32, Stowed>,
        pair: (Stowed, u32),
    },
}

/// The variants an untagged enum takes: one slot and named members.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedSplit<Stowed> {
    Decked(HashMap<Deck, Stowed>),
    Paired((Stowed, u32)),
    Stacked {
        by_deck: HashMap<Deck, Stowed>,
        pair: (Stowed, u32),
    },
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

/// The declaration of the struct `named` in `swift`, up to the declaration after it.
fn declaration(swift: &str, named: &str) -> String {
    let opened = format!("public struct {named}: ");
    let (_, rest) = swift.split_once(&opened).unwrap();
    let body = rest.split(" public struct ").next().unwrap();
    format!("{opened}{body}")
}

/// A map of `held` under the key `1`.
fn at_one<Held>(held: Held) -> HashMap<u32, Held> {
    HashMap::from([(1, held)])
}

#[test]
fn each_struct_is_written_as_serde_writes_it() {
    let cask = Cask {
        contents: "c".to_owned(),
        label: "l".to_owned(),
    };
    let stack = Stack {
        by_cask: at_one(cask),
        by_count: at_one(2),
        by_number: at_one("n".to_owned()),
        cask: Cask {
            contents: ("t".to_owned(), 3),
            label: "l".to_owned(),
        },
        listed: vec![at_one("i".to_owned())],
        marks: (4, 5),
        numbered_pair: (at_one("p".to_owned()), 6),
        pair: ("q".to_owned(), 7),
        paired_by_number: at_one(("r".to_owned(), 8)),
    };
    assert_eq!(
        read_back(&stack),
        r#"{"by_cask":{"1":{"contents":"c","label":"l"}},"by_count":{"1":2},"by_number":{"1":"n"},"cask":{"contents":["t",3],"label":"l"},"listed":[{"1":"i"}],"marks":[4,5],"numbered_pair":[{"1":"p"},6],"pair":["q",7],"paired_by_number":{"1":["r",8]}}"#
    );
    assert_eq!(
        read_back(&Pile {
            by_count: at_one(2),
            marks: (3, 4),
        }),
        r#"{"by_count":{"1":2},"marks":[3,4]}"#
    );
    assert_eq!(
        read_back(&Ranked {
            by_stowed: HashMap::from([("s".to_owned(), 1)]),
        }),
        r#"{"by_stowed":{"s":1}}"#
    );
    assert_eq!(read_back(&Tally(("a".to_owned(), 1), 2)), r#"[["a",1],2]"#);
    assert_eq!(
        read_back(&Register(at_one("a".to_owned()), 2)),
        r#"[{"1":"a"},2]"#
    );
    assert_eq!(read_back(&Hoard(at_one("a".to_owned()))), r#"{"1":"a"}"#);
    let pairing: Pairing<String> = ("a".to_owned(), 1);
    assert_eq!(read_back(&pairing), r#"["a",1]"#);
    let numbering: Numbering<String> = at_one("a".to_owned());
    assert_eq!(read_back(&numbering), r#"{"1":"a"}"#);
}

#[test]
fn each_variant_is_written_as_its_tagging_writes_it() {
    let pair = ("b".to_owned(), 2);
    for (held, written) in [
        (Split::Halves("a".to_owned(), 1), r#"{"Halves":["a",1]}"#),
        (
            Split::Keyed(at_one("a".to_owned()), 2),
            r#"{"Keyed":[{"1":"a"},2]}"#,
        ),
        (
            Split::Numbered(at_one("a".to_owned())),
            r#"{"Numbered":{"1":"a"}}"#,
        ),
        (Split::Paired(pair.clone()), r#"{"Paired":["b",2]}"#),
        (
            Split::Stacked {
                by_number: at_one("a".to_owned()),
                pair: pair.clone(),
            },
            r#"{"Stacked":{"by_number":{"1":"a"},"pair":["b",2]}}"#,
        ),
    ] {
        assert_eq!(read_back(&held), written);
    }
    assert_eq!(
        read_back(&TaggedSplit::Stacked {
            by_deck: HashMap::from([(Deck::Upper, "a".to_owned())]),
            pair: pair.clone(),
        }),
        r#"{"kind":"Stacked","by_deck":{"Upper":"a"},"pair":["b",2]}"#
    );
    for (held, written) in [
        (
            AdjacentSplit::Halves("a".to_owned(), 1),
            r#"{"kind":"Halves","held":["a",1]}"#,
        ),
        (
            AdjacentSplit::Keyed(at_one("a".to_owned()), 2),
            r#"{"kind":"Keyed","held":[{"1":"a"},2]}"#,
        ),
        (
            AdjacentSplit::Numbered(at_one("a".to_owned())),
            r#"{"kind":"Numbered","held":{"1":"a"}}"#,
        ),
        (
            AdjacentSplit::Paired(pair.clone()),
            r#"{"kind":"Paired","held":["b",2]}"#,
        ),
        (
            AdjacentSplit::Stacked {
                by_number: at_one("a".to_owned()),
                pair: pair.clone(),
            },
            r#"{"kind":"Stacked","held":{"by_number":{"1":"a"},"pair":["b",2]}}"#,
        ),
    ] {
        assert_eq!(read_back(&held), written);
    }
    for (held, written) in [
        (
            UntaggedSplit::Decked(HashMap::from([(Deck::Lower, "a".to_owned())])),
            r#"{"Lower":"a"}"#,
        ),
        (UntaggedSplit::Paired(pair.clone()), r#"["b",2]"#),
        (
            UntaggedSplit::Stacked {
                by_deck: HashMap::from([(Deck::Lower, "a".to_owned())]),
                pair,
            },
            r#"{"by_deck":{"Lower":"a"},"pair":["b",2]}"#,
        ),
    ] {
        assert_eq!(read_back(&held), written);
    }
}

#[test]
fn the_tuple_and_the_map_of_a_field_declare_the_parameter_they_reach() {
    holds(
        &stack_swift::swift_definition(),
        &[
            "public struct StackPairTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: Stowed; let slot1: UInt32; ",
            "public init(slot0: Stowed, slot1: UInt32) { ",
            "public struct StackByNumberKeyedMap<Stowed: Codable & Sendable>: Codable, Sendable { \
             var storage: [UInt32: Stowed]; ",
            "init(storage: [UInt32: Stowed]) { self.storage = storage }; ",
            "result[key] = try container.decode(Stowed.self, forKey: codingKey) }; ",
            "public struct StackListedKeyedMap<Stowed: Codable & Sendable>: Codable, Sendable { \
             var storage: [UInt32: Stowed]; ",
            "public struct StackByCaskKeyedMap<Stowed: Codable & Sendable>: Codable, Sendable { \
             var storage: [UInt32: Cask<Stowed>]; ",
            "result[key] = try Cask<Stowed>(swiftSchemaUnjudged: \
             container.superDecoder(forKey: codingKey)) }; ",
            "public struct CaskArgument0Tuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: Stowed; let slot1: UInt8; ",
        ],
    );
}

#[test]
fn every_type_position_names_the_helper_at_its_parameters() {
    holds(
        &stack_swift::swift_definition(),
        &[
            "public struct Stack<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let byCask: [UInt32: Cask<Stowed>]; public let byCount: [UInt32: UInt8]; \
             public let byNumber: [UInt32: Stowed]; \
             public let cask: Cask<CaskArgument0Tuple<Stowed>>; \
             public let listed: [[UInt32: Stowed]]; public let marks: StackMarksTuple; \
             public let numberedPair: StackNumberedPairTuple<Stowed>; \
             public let pair: StackPairTuple<Stowed>; \
             public let pairedByNumber: [UInt32: StackPairedByNumberValueTuple<Stowed>]; ",
            "self.byCask = try StackByCaskKeyedMap<Stowed>(swiftSchemaUnjudged: \
             container.superDecoder(forKey: .byCask)).storage; ",
            "let wireByNumber = try container.decode(StackByNumberKeyedMap<Stowed>.self, \
             forKey: .byNumber); self.byNumber = wireByNumber.storage; ",
            "self.cask = try Cask<CaskArgument0Tuple<Stowed>>(swiftSchemaUnjudged: \
             container.superDecoder(forKey: .cask)); ",
            "let wireListed = try container.decode([StackListedKeyedMap<Stowed>].self, \
             forKey: .listed); self.listed = wireListed.map { $0.storage }; ",
            "self.pair = try container.decode(StackPairTuple<Stowed>.self, forKey: .pair); ",
            "let wireByCask = StackByCaskKeyedMap<Stowed>(storage: byCask); ",
            "let wireByNumber = StackByNumberKeyedMap<Stowed>(storage: byNumber); ",
            "let wireListed = listed.map { StackListedKeyedMap<Stowed>(storage: $0) }; ",
        ],
    );
}

#[test]
fn a_helper_held_by_another_declares_the_parameter_both_reach() {
    holds(
        &stack_swift::swift_definition(),
        &[
            "public struct StackPairedByNumberValueTuple<Stowed: Codable & Sendable>: Codable, \
             Sendable { let slot0: Stowed; let slot1: UInt8; ",
            "public struct StackPairedByNumberKeyedMap<Stowed: Codable & Sendable>: Codable, \
             Sendable { var storage: [UInt32: StackPairedByNumberValueTuple<Stowed>]; ",
            "result[key] = try container.decode(StackPairedByNumberValueTuple<Stowed>.self, \
             forKey: codingKey) }; ",
            "let wirePairedByNumber = try \
             container.decode(StackPairedByNumberKeyedMap<Stowed>.self, forKey: \
             .pairedByNumber); ",
            "public struct StackNumberedPairTupleSlot0KeyedMap<Stowed: Codable & Sendable>: \
             Codable, Sendable { var storage: [UInt32: Stowed]; ",
            "public struct StackNumberedPairTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: [UInt32: Stowed]; let slot1: UInt8; ",
            "self.numberedPair = try container.decode(StackNumberedPairTuple<Stowed>.self, \
             forKey: .numberedPair); ",
        ],
    );
}

#[test]
fn a_name_built_from_a_helper_s_name_carries_no_parameter() {
    let swift = stack_swift::swift_definition();
    for built in [">Slot", ">Value", ">KeyedMap", ">Tuple", ">Argument"] {
        assert!(!swift.contains(built), "{built}\nin: {swift}");
    }
}

#[test]
fn a_helper_whose_members_reach_no_parameter_keeps_its_text() {
    let stack = stack_swift::swift_definition();
    let pile = pile_swift::swift_definition();
    for (held, bare) in [
        ("StackMarksTuple", "PileMarksTuple"),
        ("StackByCountKeyedMap", "PileByCountKeyedMap"),
    ] {
        assert_eq!(
            declaration(&stack, held),
            declaration(&pile, bare).replace("Pile", "Stack")
        );
    }
    holds(
        &stack,
        &[
            "let wireByCount = try container.decode(StackByCountKeyedMap.self, forKey: .byCount); ",
            "self.marks = try container.decode(StackMarksTuple.self, forKey: .marks); ",
            "let wireByCount = StackByCountKeyedMap(storage: byCount); ",
        ],
    );
}

#[test]
fn the_key_of_a_map_reaches_the_parameter() {
    holds(
        &ranked_swift::swift_definition(),
        &[
            "public struct RankedByStowedKeyedMap<Stowed: Codable & Sendable>: Codable, Sendable { \
             var storage: [String: UInt32]; ",
            "public let byStowed: [String: UInt32]; ",
            "let wireByStowed = try container.decode(RankedByStowedKeyedMap<Stowed>.self, \
             forKey: .byStowed); ",
            "let wireByStowed = RankedByStowedKeyedMap<Stowed>(storage: byStowed); ",
        ],
    );
}

#[test]
fn the_tuple_of_a_tuple_struct_declares_the_parameter_its_slots_reach() {
    holds(
        &tally_swift::swift_definition(),
        &[
            "public struct TallyTupleSlot0Tuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: Stowed; let slot1: UInt8; ",
            "public struct TallyTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: TallyTupleSlot0Tuple<Stowed>; let slot1: UInt32; ",
            "slot0 = try container.decode(TallyTupleSlot0Tuple<Stowed>.self); ",
            "public struct Tally<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let value: TallyTuple<Stowed>; \
             public init(value: TallyTuple<Stowed>) { self.value = value }; \
             public init(swiftSchemaUnjudged decoder: Decoder) throws { \
             self.value = try TallyTuple<Stowed>(swiftSchemaUnjudged: decoder) }; ",
        ],
    );
    holds(
        &register_swift::swift_definition(),
        &[
            "public struct RegisterTupleSlot0KeyedMap<Stowed: Codable & Sendable>: Codable, \
             Sendable { var storage: [UInt32: Stowed]; ",
            "public struct RegisterTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: [UInt32: Stowed]; let slot1: UInt8; ",
            "public let value: RegisterTuple<Stowed>; ",
        ],
    );
}

#[test]
fn a_brand_and_an_alias_name_the_helper_at_its_parameters() {
    holds(
        &hoard_swift::swift_definition(),
        &[
            "public struct HoardKeyedMap<Stowed: Codable & Sendable>: Codable, Sendable { \
             var storage: [UInt32: Stowed]; ",
            "let wireValue = try container.decode(HoardKeyedMap<Stowed>.self); ",
            "try container.encode(HoardKeyedMap<Stowed>(storage: value)) }; ",
        ],
    );
    holds(
        &pairing_swift::swift_definition(),
        &[
            "public struct PairingTypeTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: Stowed; let slot1: UInt32; ",
            "public typealias PairingType<Stowed: Codable & Sendable> = \
             PairingTypeTuple<Stowed>;",
        ],
    );
    holds(
        &numbering_swift::swift_definition(),
        &["public struct NumberingTypeKeyedMap<Stowed: Codable & Sendable>: Codable, Sendable { \
           var storage: [UInt32: Stowed]; "],
    );
}

#[test]
fn a_variant_holds_its_helper_at_the_parameters_it_declares_under_no_tag_attribute() {
    holds(
        &split_swift::swift_definition(),
        &[
            "public struct SplitHalvesTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: Stowed; let slot1: UInt8; ",
            "public struct SplitKeyedTupleSlot0KeyedMap<Stowed: Codable & Sendable>: Codable, \
             Sendable { var storage: [UInt32: Stowed]; ",
            "public struct SplitKeyedTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: [UInt32: Stowed]; let slot1: UInt8; ",
            "public struct SplitNumberedValueKeyedMap<Stowed: Codable & Sendable>: Codable, \
             Sendable { var storage: [UInt32: Stowed]; ",
            "public struct SplitPairedValueTuple<Stowed: Codable & Sendable>: Codable, Sendable { \
             let slot0: Stowed; let slot1: UInt32; ",
            "public struct SplitStackedPayloadByNumberKeyedMap<Stowed: Codable & Sendable>: \
             Codable, Sendable { var storage: [UInt32: Stowed]; ",
            "public struct SplitStackedPayloadPairTuple<Stowed: Codable & Sendable>: Codable, \
             Sendable { let slot0: Stowed; let slot1: UInt32; ",
            "public enum Split<Stowed: Codable & Sendable>: Codable, Sendable { \
             case halves(SplitHalvesTuple<Stowed>); case keyed(SplitKeyedTuple<Stowed>); \
             case numbered([UInt32: Stowed]); case paired(SplitPairedValueTuple<Stowed>); \
             case stacked(SplitStackedPayload<Stowed>); ",
            "case \"Halves\": self = .halves(try SplitHalvesTuple<Stowed>(swiftSchemaUnjudged: \
             container.superDecoder(forKey: key))); \
             case \"Keyed\": self = .keyed(try SplitKeyedTuple<Stowed>(swiftSchemaUnjudged: \
             container.superDecoder(forKey: key))); ",
            "case \"Paired\": self = .paired(try \
             container.decode(SplitPairedValueTuple<Stowed>.self, forKey: key)); ",
            "let wireByNumber = try \
             container.decode(SplitStackedPayloadByNumberKeyedMap<Stowed>.self, forKey: \
             .byNumber); ",
        ],
    );
}

#[test]
fn a_variant_holds_its_helper_at_the_parameters_it_declares_under_a_tag_alone() {
    holds(
        &tagged_split_swift::swift_definition(),
        &[
            "public struct TaggedSplitStackedPayloadByDeckKeyedMap<Stowed: Codable & Sendable>: \
             Codable, Sendable { var storage: [Deck: Stowed]; ",
            "public struct TaggedSplitStackedPayloadPairTuple<Stowed: Codable & Sendable>: \
             Codable, Sendable { let slot0: Stowed; let slot1: UInt32; ",
            "public let byDeck: [Deck: Stowed]; \
             public let pair: TaggedSplitStackedPayloadPairTuple<Stowed>; ",
            "let wireByDeck = \
             TaggedSplitStackedPayloadByDeckKeyedMap<Stowed>(storage: byDeck); ",
        ],
    );
}

#[test]
fn a_variant_holds_its_helper_at_the_parameters_it_declares_under_a_tag_and_a_content_key() {
    holds(
        &adjacent_split_swift::swift_definition(),
        &[
            "public struct AdjacentSplitHalvesTuple<Stowed: Codable & Sendable>: Codable, \
             Sendable { ",
            "public struct AdjacentSplitKeyedTuple<Stowed: Codable & Sendable>: Codable, Sendable { ",
            "public struct AdjacentSplitNumberedValueKeyedMap<Stowed: Codable & Sendable>: \
             Codable, Sendable { ",
            "public struct AdjacentSplitPairedValueTuple<Stowed: Codable & Sendable>: Codable, \
             Sendable { ",
            "case halves(AdjacentSplitHalvesTuple<Stowed>); \
             case keyed(AdjacentSplitKeyedTuple<Stowed>); case numbered([UInt32: Stowed]); \
             case paired(AdjacentSplitPairedValueTuple<Stowed>); \
             case stacked(AdjacentSplitStackedPayload<Stowed>); ",
            "self = .halves(try AdjacentSplitHalvesTuple<Stowed>(swiftSchemaUnjudged: \
             container.superDecoder(forKey: .swiftSchemaContent))); ",
            "self = .paired(try container.decode(AdjacentSplitPairedValueTuple<Stowed>.self, \
             forKey: .swiftSchemaContent)); ",
        ],
    );
}

#[test]
fn a_variant_holds_its_helper_at_the_parameters_it_declares_untagged() {
    holds(
        &untagged_split_swift::swift_definition(),
        &[
            "public struct UntaggedSplitDeckedValueKeyedMap<Stowed: Codable & Sendable>: \
             Codable, Sendable { var storage: [Deck: Stowed]; ",
            "public struct UntaggedSplitPairedValueTuple<Stowed: Codable & Sendable>: Codable, \
             Sendable { ",
            "case decked([Deck: Stowed]); case paired(UntaggedSplitPairedValueTuple<Stowed>); \
             case stacked(UntaggedSplitStackedPayload<Stowed>); ",
            "if let value = try? UntaggedSplitPairedValueTuple<Stowed>(from: decoder) { \
             self = .paired(value); return }; ",
        ],
    );
}
