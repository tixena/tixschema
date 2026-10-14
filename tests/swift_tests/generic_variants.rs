//! What the Swift backend writes for a variant with named members of an enum that declares a
//! parameter: the struct written for the members declares the parameters they name, under the
//! bound the enum declares them with, and the case holds the struct at those parameters. A variant
//! whose members name none keeps its struct with no parameter. What a read answers is run under a
//! Swift toolchain in `service_schema_emitted_client_tests/run_swift_serde_default.rs`.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A model filled with a parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Keg<Stowed> {
    pub contents: Stowed,
}

/// The parameter in a list and as a model's argument in a map's value, under a tag, beside a
/// variant with no member and one whose members name no parameter.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Hold<Stowed> {
    Closed {
        deck: u32,
    },
    Empty,
    Kegged {
        by_dock: HashMap<String, Keg<Stowed>>,
    },
    Laden {
        cargo: Vec<Stowed>,
        deck: u32,
    },
}

/// Two parameters, a variant naming both and one naming the second alone, under no tag
/// attribute.
#[model_schema(default_types(Stowed = String, Marked = u32))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Lading<Stowed, Marked> {
    Both { marked: Marked, stowed: Stowed },
    Marks { marks: Vec<Marked> },
}

/// [`Lading`] under a tag and a content key.
#[model_schema(default_types(Stowed = String, Marked = u32))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentLading<Stowed, Marked> {
    Both { marked: Marked, stowed: Stowed },
    Marks { marks: Vec<Marked> },
}

/// [`Lading`] under no tag.
#[model_schema(default_types(Stowed = String, Marked = u32))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedLading<Stowed, Marked> {
    Both { marked: Marked, stowed: Stowed },
    Marks { marks: Vec<Marked> },
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
fn each_variant_is_written_as_its_tagging_writes_it() {
    let kegged = Hold::Kegged {
        by_dock: HashMap::from([(
            "d".to_owned(),
            Keg {
                contents: "x".to_owned(),
            },
        )]),
    };
    for (held, written) in [
        (Hold::Closed { deck: 1 }, r#"{"kind":"Closed","deck":1}"#),
        (Hold::Empty, r#"{"kind":"Empty"}"#),
        (
            kegged,
            r#"{"kind":"Kegged","by_dock":{"d":{"contents":"x"}}}"#,
        ),
        (
            Hold::Laden {
                cargo: vec!["c".to_owned()],
                deck: 2,
            },
            r#"{"kind":"Laden","cargo":["c"],"deck":2}"#,
        ),
    ] {
        assert_eq!(read_back(&held), written);
    }
    assert_eq!(
        read_back(&Lading::Both {
            marked: 3_u32,
            stowed: "s".to_owned()
        }),
        r#"{"Both":{"marked":3,"stowed":"s"}}"#
    );
    assert_eq!(
        read_back(&Lading::<String, u32>::Marks { marks: vec![1] }),
        r#"{"Marks":{"marks":[1]}}"#
    );
    assert_eq!(
        read_back(&AdjacentLading::Both {
            marked: 3_u32,
            stowed: "s".to_owned()
        }),
        r#"{"kind":"Both","held":{"marked":3,"stowed":"s"}}"#
    );
    assert_eq!(
        read_back(&AdjacentLading::<String, u32>::Marks { marks: vec![1] }),
        r#"{"kind":"Marks","held":{"marks":[1]}}"#
    );
    assert_eq!(
        read_back(&UntaggedLading::Both {
            marked: 3_u32,
            stowed: "s".to_owned()
        }),
        r#"{"marked":3,"stowed":"s"}"#
    );
    assert_eq!(
        read_back(&UntaggedLading::<String, u32>::Marks { marks: vec![1] }),
        r#"{"marks":[1]}"#
    );
}

#[test]
fn the_struct_of_a_variant_declares_the_parameter_its_members_name() {
    let swift = hold_swift::swift_definition();
    holds(
        &swift,
        &[
            "public struct HoldLadenPayload<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let cargo: [Stowed]; public let deck: UInt32; ",
            "public struct HoldKeggedPayload<Stowed: Codable & Sendable>: Codable, Sendable { \
             public let byDock: [String: Keg<Stowed>]; ",
            "public enum Hold<Stowed: Codable & Sendable>: Codable, Sendable { \
             case closed(HoldClosedPayload); case empty; \
             case kegged(HoldKeggedPayload<Stowed>); case laden(HoldLadenPayload<Stowed>); ",
            "case \"Kegged\": self = .kegged(try HoldKeggedPayload<Stowed>(swiftSchemaUnjudged: \
             decoder)); case \"Laden\": self = .laden(try \
             HoldLadenPayload<Stowed>(swiftSchemaUnjudged: decoder)); ",
        ],
    );
}

#[test]
fn the_struct_of_a_variant_whose_members_name_no_parameter_declares_none() {
    let swift = hold_swift::swift_definition();
    holds(
        &swift,
        &[
            "public struct HoldClosedPayload: Codable, Sendable { public let deck: UInt32; ",
            "case \"Closed\": self = .closed(try HoldClosedPayload(swiftSchemaUnjudged: \
             decoder)); ",
        ],
    );
}

#[test]
fn the_struct_of_a_variant_declares_only_the_parameters_its_members_name() {
    holds(
        &lading_swift::swift_definition(),
        &[
            "public struct LadingBothPayload<Stowed: Codable & Sendable, Marked: Codable & \
             Sendable>: Codable, Sendable { public let marked: Marked; public let stowed: \
             Stowed; ",
            "public struct LadingMarksPayload<Marked: Codable & Sendable>: Codable, Sendable { \
             public let marks: [Marked]; ",
            "public enum Lading<Stowed: Codable & Sendable, Marked: Codable & Sendable>: \
             Codable, Sendable { case both(LadingBothPayload<Stowed, Marked>); \
             case marks(LadingMarksPayload<Marked>); ",
            "case \"Both\": self = .both(try LadingBothPayload<Stowed, \
             Marked>(swiftSchemaUnjudged: container.superDecoder(forKey: key))); ",
            "case \"Marks\": self = .marks(try \
             LadingMarksPayload<Marked>(swiftSchemaUnjudged: container.superDecoder(forKey: \
             key))); ",
        ],
    );
}

#[test]
fn each_tagging_holds_the_struct_of_a_variant_at_the_parameters_it_declares() {
    holds(
        &adjacent_lading_swift::swift_definition(),
        &[
            "public struct AdjacentLadingMarksPayload<Marked: Codable & Sendable>: Codable, \
             Sendable { ",
            "case both(AdjacentLadingBothPayload<Stowed, Marked>); \
             case marks(AdjacentLadingMarksPayload<Marked>); ",
            "self = .marks(try \
             AdjacentLadingMarksPayload<Marked>(swiftSchemaUnjudged: \
             container.superDecoder(forKey: .swiftSchemaContent)))",
        ],
    );
    holds(
        &untagged_lading_swift::swift_definition(),
        &[
            "public struct UntaggedLadingMarksPayload<Marked: Codable & Sendable>: Codable, \
             Sendable { ",
            "case both(UntaggedLadingBothPayload<Stowed, Marked>); \
             case marks(UntaggedLadingMarksPayload<Marked>); ",
            "if let value = try? UntaggedLadingMarksPayload<Marked>(swiftSchemaUnjudged: \
             decoder) { self = .marks(value); return }; ",
        ],
    );
}
