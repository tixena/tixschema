//! A payload serde refuses for its length, read by the emitted Swift types compiled with warnings as
//! errors: a fixed array `[T; N]` of another length, longer and shorter, at each level and in each
//! place a value is held, and a tuple of another length, a tuple struct's, a member's, a variant's
//! and the outer and the inner of a tuple that holds a tuple. Each is refused with
//! `DecodingError.dataCorrupted` in serde's words, at the coding path of the array, by
//! `init(swiftSchemaUnjudged:)` and by `init(from:)`. The fitting payloads, and a list of any
//! length, are read and written back as serde writes them, and an untagged enum takes the variant
//! serde takes. The fixed arrays and the tuples of `run_dart_refusals.rs` are declared where the
//! Dart client is built.

#![cfg(all(feature = "swift", feature = "dart", feature = "serde"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

use super::run_dart_refusals::{
    Clasping, Clenching, Fogging, Frosting, Grasping, Thawing, clasping_swift, clenching_swift,
    fogging_swift, frosting_swift, grasping_swift, thawing_swift,
};
use super::run_dart_validate::from_until;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What the run declares beside the emitted types: the function a member names, answering the
/// payload as it was handed, the read of a payload through `init(from:)` and through
/// `init(swiftSchemaUnjudged:)`, and what prints a row: the value read, written back with its
/// keys sorted, or the coding path and the description of what the read throws.
const HELPERS: &str = r#"
func asGiven(_ payload: Any) -> Any { payload }

protocol ReadUnjudged {
  init(swiftSchemaUnjudged decoder: Decoder) throws
}

struct Unjudged<Held: ReadUnjudged>: Decodable {
  let held: Held
  init(from decoder: Decoder) throws { held = try Held(swiftSchemaUnjudged: decoder) }
}

func place(_ path: [CodingKey]) -> String {
  path.map { $0.intValue.map(String.init) ?? $0.stringValue }.joined(separator: ".")
}

func written<Read: Encodable>(_ value: Read) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return String(data: try! encoder.encode(value), encoding: .utf8)!
}

func told<Read: Encodable>(_ at: String, _ read: () throws -> Read) {
  do {
    print("\(at): read \(written(try read()))")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(at): [\(place(context.codingPath))] \(context.debugDescription)")
  } catch {
    print("\(at): \(error)")
  }
}

func both<Read: Codable & ReadUnjudged>(_ at: Int, _ type: Read.Type, _ payload: String) {
  told("\(at)", { try JSONDecoder().decode(type, from: Data(payload.utf8)) })
  told("\(at), unjudged", {
    try JSONDecoder().decode(Unjudged<Read>.self, from: Data(payload.utf8)).held
  })
}
"#;

/// Every payload of another length, and the coding path and the description the Swift read
/// throws: a fixed array, longer and shorter, the inner and the outer level of a fixed array of
/// fixed arrays, in a tuple's slot, as a variant's lone slot, in a variant of several slots and as
/// a variant's named member, as a map's value under a key Swift keys natively and under one it
/// wraps, under an `Option` and under `nullable`, as the one slot of a tuple struct and of a brand,
/// of models a holder walks, corrected by a function first, filling a type parameter and under an
/// alias; and a tuple of another length,
/// longer and shorter, a tuple struct's, the inner and the outer of a tuple that holds a tuple, a
/// variant's, and one that holds a fixed array.
const ROWS: [Row; 27] = [
    Row {
        at: "pair",
        json: r#"{"pair":[1,2,3]}"#,
        read_as: "Clenching",
        serde: serde_told::<Clenching>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "pair",
        json: r#"{"pair":[1]}"#,
        read_as: "Clenching",
        serde: serde_told::<Clenching>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "grid.1",
        json: r#"{"grid":[[1,2],[3,4,5],[6,7]]}"#,
        read_as: "Clasping",
        serde: serde_told::<Clasping>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "grid",
        json: r#"{"grid":[[1,2],[3,4]]}"#,
        read_as: "Clasping",
        serde: serde_told::<Clasping>,
        thrown: "invalid length 2, expected 3",
    },
    Row {
        at: "held.1",
        json: r#"{"held":[1,[2,3,4]]}"#,
        read_as: "Cupping",
        serde: serde_told::<Cupping>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "Pair",
        json: r#"{"Pair":[1,2,3]}"#,
        read_as: "Pinching",
        serde: serde_told::<Pinching>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "Split.0",
        json: r#"{"Split":[[1],3]}"#,
        read_as: "Pinching",
        serde: serde_told::<Pinching>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "Named.pair",
        json: r#"{"Named":{"pair":[1,2,3]}}"#,
        read_as: "Pinching",
        serde: serde_told::<Pinching>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "by_name.a",
        json: r#"{"by_name":{"a":[1,2,3]},"by_number":{}}"#,
        read_as: "Clutching",
        serde: serde_told::<Clutching>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "by_number.7",
        json: r#"{"by_name":{},"by_number":{"7":[1]}}"#,
        read_as: "Clutching",
        serde: serde_told::<Clutching>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "pair",
        json: r#"{"pair":[1,2,3],"tied":null}"#,
        read_as: "Hugging",
        serde: serde_told::<Hugging>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "tied",
        json: r#"{"tied":[1]}"#,
        read_as: "Hugging",
        serde: serde_told::<Hugging>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "",
        json: "[1,2,3]",
        read_as: "Gauging",
        serde: serde_told::<Gauging>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "",
        json: "[1]",
        read_as: "Squeezing",
        serde: serde_told::<Squeezing>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "pair",
        json: r#"{"pair":[{"label":"a"},{"label":"b"},{"label":"c"}]}"#,
        read_as: "Cradling",
        serde: serde_told::<Cradling>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "pair",
        json: r#"{"pair":[1,2,3]}"#,
        read_as: "Wringing",
        serde: serde_told::<Wringing>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "pair",
        json: r#"{"pair":[1]}"#,
        read_as: "Wringing",
        serde: serde_told::<Wringing>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "yoked.held",
        json: r#"{"yoked":{"held":[1,2,3]}}"#,
        read_as: "Coupling",
        serde: serde_told::<Coupling>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "pair",
        json: r#"{"pair":[1]}"#,
        read_as: "Vising",
        serde: serde_told::<Vising>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "",
        json: r#"["b",1,2]"#,
        read_as: "Thawing",
        serde: serde_told::<Thawing>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "",
        json: r#"["b"]"#,
        read_as: "Thawing",
        serde: serde_told::<Thawing>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "reading.0",
        json: r#"{"reading":[["b",1,2],3]}"#,
        read_as: "Frosting",
        serde: serde_told::<Frosting>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "reading",
        json: r#"{"reading":[["b",1],3,4]}"#,
        read_as: "Frosting",
        serde: serde_told::<Frosting>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "reading",
        json: r#"{"reading":[["b",1]]}"#,
        read_as: "Frosting",
        serde: serde_told::<Frosting>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "Split",
        json: r#"{"Split":["b",1,2]}"#,
        read_as: "Fogging",
        serde: serde_told::<Fogging>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        at: "Split",
        json: r#"{"Split":["b"]}"#,
        read_as: "Fogging",
        serde: serde_told::<Fogging>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        at: "held",
        json: r#"{"held":[1,[2,3],4]}"#,
        read_as: "Cupping",
        serde: serde_told::<Cupping>,
        thrown: "invalid length 3, expected 2",
    },
];

/// Payloads each type reads, as serde reads them: a fixed array and a tuple from an array of
/// their length, in each place the rows above hold one, and a list from an array of any length.
const READ: [Readable; 22] = [
    Readable {
        json: r#"{"pair":[1,2]}"#,
        read_as: "Clenching",
        serde: serde_written::<Clenching>,
    },
    Readable {
        json: r#"{"grid":[[1,2],[3,4],[5,6]]}"#,
        read_as: "Clasping",
        serde: serde_written::<Clasping>,
    },
    Readable {
        json: r#"{"held":[1,2,3]}"#,
        read_as: "Grasping",
        serde: serde_written::<Grasping>,
    },
    Readable {
        json: r#"{"held":[]}"#,
        read_as: "Grasping",
        serde: serde_written::<Grasping>,
    },
    Readable {
        json: r#"{"held":[1,[2,3]]}"#,
        read_as: "Cupping",
        serde: serde_written::<Cupping>,
    },
    Readable {
        json: r#"{"Pair":[1,2]}"#,
        read_as: "Pinching",
        serde: serde_written::<Pinching>,
    },
    Readable {
        json: r#"{"Split":[[1,2],3]}"#,
        read_as: "Pinching",
        serde: serde_written::<Pinching>,
    },
    Readable {
        json: r#"{"Named":{"pair":[1,2]}}"#,
        read_as: "Pinching",
        serde: serde_written::<Pinching>,
    },
    Readable {
        json: r#"{"by_name":{"a":[1,2]},"by_number":{"7":[3,4]}}"#,
        read_as: "Clutching",
        serde: serde_written::<Clutching>,
    },
    Readable {
        json: r#"{"pair":[1,2],"tied":null}"#,
        read_as: "Hugging",
        serde: serde_written::<Hugging>,
    },
    Readable {
        json: r#"{"tied":[1,2]}"#,
        read_as: "Hugging",
        serde: serde_written::<Hugging>,
    },
    Readable {
        json: "[1,2]",
        read_as: "Gauging",
        serde: serde_written::<Gauging>,
    },
    Readable {
        json: "[1,2]",
        read_as: "Squeezing",
        serde: serde_written::<Squeezing>,
    },
    Readable {
        json: r#"{"pair":[{"label":"a"},{"label":"b"}]}"#,
        read_as: "Cradling",
        serde: serde_written::<Cradling>,
    },
    Readable {
        json: r#"{"pair":[1,2]}"#,
        read_as: "Wringing",
        serde: serde_written::<Wringing>,
    },
    Readable {
        json: r#"["b",1]"#,
        read_as: "Thawing",
        serde: serde_written::<Thawing>,
    },
    Readable {
        json: r#"{"reading":[["b",1],3]}"#,
        read_as: "Frosting",
        serde: serde_written::<Frosting>,
    },
    Readable {
        json: r#"{"Split":["b",1]}"#,
        read_as: "Fogging",
        serde: serde_written::<Fogging>,
    },
    Readable {
        json: "[1,2]",
        read_as: "Grappling",
        serde: serde_written::<Grappling>,
    },
    Readable {
        json: "[1,2,3]",
        read_as: "Grappling",
        serde: serde_written::<Grappling>,
    },
    Readable {
        json: r#"{"yoked":{"held":[1,2]}}"#,
        read_as: "Coupling",
        serde: serde_written::<Coupling>,
    },
    Readable {
        json: r#"{"pair":[1,2]}"#,
        read_as: "Vising",
        serde: serde_written::<Vising>,
    },
];

/// A fixed array in a slot of a tuple.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cupping {
    pub held: (u8, [u8; 2]),
}

/// A fixed array as a variant's named member, as its lone slot and in a variant of several slots.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Pinching {
    Named { pair: [u8; 2] },
    Pair([u8; 2]),
    Split([u8; 2], u8),
}

/// Fixed arrays as the values of a map keyed by text, which Swift keys natively, and of one keyed
/// by a number, which it reads through a wrapper.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Clutching {
    pub by_name: BTreeMap<String, [u8; 2]>,
    pub by_number: BTreeMap<u32, [u8; 2]>,
}

/// A fixed array under an `Option` whose key may be absent, and under one whose key is required.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hugging {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pair: Option<[u8; 2]>,
    #[model_schema_prop(nullable)]
    pub tied: Option<[u8; 2]>,
}

/// A tuple struct of one slot, which serde writes as the fixed array it holds.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gauging(pub [u8; 2]);

/// A brand over a fixed array, which writes no `Display`.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Squeezing(pub [u8; 2]);

/// A model with a rule, which the `validate()` of a holder walks.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tweezing {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A fixed array of models the holder's `validate()` walks.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cradling {
    pub pair: [Tweezing; 2],
}

/// A fixed array a function corrects first.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Wringing {
    #[model_schema_prop(preprocess = ["asGiven"])]
    pub pair: [u8; 2],
}

/// An untagged enum whose first variant holds a fixed array, which serde takes for an array of
/// its length alone, and whose second holds a list.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Grappling {
    Pair([u8; 2]),
    Spread(Vec<u8>),
}

/// A type that holds the value its parameter names.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Yoking<Held> {
    pub held: Held,
}

/// A fixed array that fills a type parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Coupling {
    pub yoked: Yoking<[u8; 2]>,
}

/// A fixed array under an alias.
#[model_schema()]
pub type Braced = [u8; 2];

/// A member typed with the alias of a fixed array.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Vising {
    pub pair: Braced,
}

/// One payload a type reads, the type and what serde writes back of it.
struct Readable {
    json: &'static str,
    /// The Swift type the payload is read as.
    read_as: &'static str,
    /// What serde writes back of the payload.
    serde: fn(&str) -> String,
}

/// One payload, the type it is read as, what serde answers of it, and the coding path and the
/// description the Swift read throws.
struct Row {
    /// The coding path the Swift read refuses the payload at, its steps joined by dots.
    at: &'static str,
    json: &'static str,
    /// The Swift type the payload is read as.
    read_as: &'static str,
    /// What serde answers of the payload.
    serde: fn(&str) -> String,
    /// The description of the `DecodingError.dataCorrupted` the Swift read throws.
    thrown: &'static str,
}

/// The payload as it was handed: the function [`Wringing`] names.
const fn as_given(payload: Value) -> Value {
    payload
}

/// serde's refusal of `json` as `T`, or `read` where it reads it.
fn serde_told<T>(json: &str) -> String
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json)
        .map_or_else(|refusal| refusal.to_string(), |_| "read".to_owned())
}

/// What serde writes back of `json` read as `T`, or its refusal.
fn serde_written<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |refusal| refusal.to_string(),
        |read| format!("read {}", serde_json::to_string(&read).unwrap_or_default()),
    )
}

/// What `swift -warnings-as-errors` prints of `main` over the emitted definitions of every type
/// the rows read, or `None` where no toolchain is reachable.
fn printed(main: &str) -> Option<String> {
    let mut parts = vec![
        "import Foundation".to_owned(),
        clenching_swift::swift_definition(),
        clasping_swift::swift_definition(),
        grasping_swift::swift_definition(),
        thawing_swift::swift_definition(),
        frosting_swift::swift_definition(),
        fogging_swift::swift_definition(),
        cupping_swift::swift_definition(),
        pinching_swift::swift_definition(),
        clutching_swift::swift_definition(),
        hugging_swift::swift_definition(),
        gauging_swift::swift_definition(),
        squeezing_swift::swift_definition(),
        tweezing_swift::swift_definition(),
        cradling_swift::swift_definition(),
        wringing_swift::swift_definition(),
        grappling_swift::swift_definition(),
        yoking_swift::swift_definition(),
        coupling_swift::swift_definition(),
        braced_swift::swift_definition(),
        vising_swift::swift_definition(),
        HELPERS.to_owned(),
    ];
    for read_as in [
        "Clenching",
        "Clasping",
        "Grasping",
        "Thawing",
        "Frosting",
        "Fogging",
        "Cupping",
        "Pinching",
        "Clutching",
        "Hugging",
        "Gauging",
        "Squeezing",
        "Cradling",
        "Wringing",
        "Grappling",
        "Coupling",
        "Vising",
    ] {
        parts.push(format!("extension {read_as}: ReadUnjudged {{}}"));
    }
    parts.push(main.to_owned());
    let source = parts.join("\n\n");
    let run = judged(
        "swift-refusals",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    )?;
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    Some(run.stdout)
}

/// Each payload serde refuses for its length is refused by the Swift read, judged and unjudged,
/// with `DecodingError.dataCorrupted` at the coding path of the array, in serde's words.
#[test]
fn an_array_of_another_length_is_refused_in_serdes_words() {
    let serde_refused: Vec<String> = ROWS.iter().map(|row| (row.serde)(row.json)).collect();
    assert_eq!(
        serde_refused,
        [
            "trailing characters at line 1 column 14",
            "invalid length 1, expected an array of length 2 at line 1 column 11",
            "trailing characters at line 1 column 21",
            "invalid length 2, expected an array of length 3 at line 1 column 21",
            "trailing characters at line 1 column 17",
            "trailing characters at line 1 column 14",
            "invalid length 1, expected an array of length 2 at line 1 column 13",
            "trailing characters at line 1 column 23",
            "trailing characters at line 1 column 22",
            "invalid length 1, expected an array of length 2 at line 1 column 34",
            "trailing characters at line 1 column 14",
            "invalid length 1, expected an array of length 2 at line 1 column 11",
            "trailing characters at line 1 column 6",
            "invalid length 1, expected an array of length 2 at line 1 column 3",
            "trailing characters at line 1 column 38",
            "invalid length 3, expected fewer elements in array at line 1 column 16",
            "invalid length 1, expected an array of length 2 at line 1 column 12",
            "trailing characters at line 1 column 23",
            "invalid length 1, expected an array of length 2 at line 1 column 11",
            "trailing characters at line 1 column 8",
            "invalid length 1, expected tuple struct Thawing with 2 elements at line 1 column 5",
            "trailing characters at line 1 column 20",
            "trailing characters at line 1 column 23",
            "invalid length 1, expected a tuple of size 2 at line 1 column 20",
            "trailing characters at line 1 column 17",
            "invalid length 1, expected tuple variant Fogging::Split with 2 elements at line 1 \
             column 14",
            "trailing characters at line 1 column 18",
        ],
        "serde no longer refuses each payload as it did"
    );
    let mut main = String::new();
    let mut expected = String::new();
    for (at, row) in ROWS.iter().enumerate() {
        let _ = writeln!(main, "both({at}, {}.self, #\"{}\"#)", row.read_as, row.json);
        let _ = writeln!(expected, "{at}: [{}] {}", row.at, row.thrown);
        let _ = writeln!(expected, "{at}, unjudged: [{}] {}", row.at, row.thrown);
    }
    let Some(printed) = printed(&main) else {
        return;
    };
    assert_eq!(printed, expected, "the Swift read is not the refusal above");
}

/// A fixed array and a tuple are read from an array of their length, and a list from an array of
/// any length, as serde reads each, and each is written back as serde writes it.
#[test]
fn an_array_of_its_length_is_read_and_written_back_as_serde_writes_it() {
    let serde_read: Vec<String> = READ.iter().map(|row| (row.serde)(row.json)).collect();
    assert_eq!(
        serde_read,
        [
            r#"read {"pair":[1,2]}"#,
            r#"read {"grid":[[1,2],[3,4],[5,6]]}"#,
            r#"read {"held":[1,2,3]}"#,
            r#"read {"held":[]}"#,
            r#"read {"held":[1,[2,3]]}"#,
            r#"read {"Pair":[1,2]}"#,
            r#"read {"Split":[[1,2],3]}"#,
            r#"read {"Named":{"pair":[1,2]}}"#,
            r#"read {"by_name":{"a":[1,2]},"by_number":{"7":[3,4]}}"#,
            r#"read {"pair":[1,2],"tied":null}"#,
            r#"read {"tied":[1,2]}"#,
            "read [1,2]",
            "read [1,2]",
            r#"read {"pair":[{"label":"a"},{"label":"b"}]}"#,
            r#"read {"pair":[1,2]}"#,
            r#"read ["b",1]"#,
            r#"read {"reading":[["b",1],3]}"#,
            r#"read {"Split":["b",1]}"#,
            "read [1,2]",
            "read [1,2,3]",
            r#"read {"yoked":{"held":[1,2]}}"#,
            r#"read {"pair":[1,2]}"#,
        ],
        "serde no longer reads each payload as it did"
    );
    let mut main = String::new();
    let mut expected = String::new();
    for (at, row) in READ.iter().enumerate() {
        let _ = writeln!(main, "both({at}, {}.self, #\"{}\"#)", row.read_as, row.json);
        let _ = writeln!(expected, "{at}: {}", serde_read[at]);
        let _ = writeln!(expected, "{at}, unjudged: {}", serde_read[at]);
    }
    let Some(printed) = printed(&main) else {
        return;
    };
    assert_eq!(printed, expected, "the Swift read is not serde's");
}

/// An untagged enum takes, for an array, the variant serde takes: the fixed array for an array of
/// its length, and the list for any other.
#[test]
fn an_untagged_enum_takes_the_fixed_array_for_an_array_of_its_length_alone() {
    let taken = |json: &str| match serde_json::from_str::<Grappling>(json).unwrap() {
        Grappling::Pair(_) => "Pair",
        Grappling::Spread(_) => "Spread",
    };
    let payloads = ["[1,2]", "[1,2,3]", "[1]"];
    let mut main = String::new();
    let mut expected = String::new();
    for payload in payloads {
        let _ = writeln!(
            main,
            "switch try! JSONDecoder().decode(Grappling.self, from: Data(\"{payload}\".utf8)) {{ \
             case .pair: print(\"{payload}: Pair\"); case .spread: print(\"{payload}: Spread\") }}"
        );
        let _ = writeln!(expected, "{payload}: {}", taken(payload));
    }
    assert_eq!(
        expected, "[1,2]: Pair\n[1,2,3]: Spread\n[1]: Spread\n",
        "serde no longer takes the variant it took"
    );
    let Some(printed) = printed(&main) else {
        return;
    };
    assert_eq!(printed, expected, "the Swift read takes another variant");
}

/// The README's sentence on the refusals of the Swift read shows payloads and messages the run
/// above measures.
#[test]
fn the_readme_shows_the_refusals_the_run_measures() {
    let section = from_until(
        include_str!("../../README.md"),
        "#### A Swift read refuses what `validate()` refuses",
        "### Literal Values",
    );
    for declared in [
        "`struct Clenching { pair: [u8; 2] }`",
        "`[[u8; 2]; 3]`",
        "`struct Thawing(String, u32)`",
        "`enum Grappling { Pair([u8; 2]), Spread(Vec<u8>) }`",
        "`[1,2,3]` reads `Spread`",
    ] {
        assert!(
            section.contains(declared),
            "the README no longer shows what the run reads: {declared}"
        );
    }
    for (read_as, json, thrown) in [
        (
            "Clenching",
            r#"{"pair":[1,2,3]}"#,
            "invalid length 3, expected 2",
        ),
        (
            "Clenching",
            r#"{"pair":[1]}"#,
            "invalid length 1, expected 2",
        ),
        ("Thawing", r#"["b",1,2]"#, "invalid length 3, expected 2"),
    ] {
        assert!(
            ROWS.iter()
                .any(|row| row.read_as == read_as && row.json == json && row.thrown == thrown),
            "no row refuses {json} as `{read_as}` with `{thrown}`"
        );
        assert!(
            section.contains(&format!("`{json}`")),
            "not in the README: {json}"
        );
        assert!(
            section.contains(&format!("`{thrown}`")),
            "not in the README: {thrown}"
        );
    }
}
