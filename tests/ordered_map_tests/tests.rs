//! Tests of the order-keeping map and the list beside it: `IndexMap` and `IndexSet`, each held to
//! what its std twin gives on every surface.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod list_rule;

use std::collections::{HashMap, HashSet};
use std::hash::RandomState;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use tixschema::model_schema;

const DOCUMENT: &str = r#"{"data":{"zeta":"1","alpha":"2"},"labels":["b","a"]}"#;

/// [`Document`] as the README declares it, character for character.
const README_DECLARATION: &str = "#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub data: IndexMap<String, String>,
    pub labels: IndexSet<String>,
}";

const NESTED: &str = r#"{"by_group":{"odd":[3,1]},"hashed":{"zeta":1,"alpha":2},"listed":[{"zeta":1,"alpha":2}],"optional":{"zeta":1,"alpha":2},"tagged":[9,4]}"#;

const SLOTTED: &str = r#"{"counts":{"Opening":2,"Closing":1}}"#;

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub data: IndexMap<String, String>,
    pub labels: IndexSet<String>,
}

// `Document` written with the std types every surface already reads. A `//` comment: a doc
// comment is written into the outputs the two are compared by.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTwin {
    pub data: HashMap<String, String>,
    pub labels: HashSet<String>,
}

// Each type under what a field may hold it in, and each written with a hasher of its own.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nested {
    pub by_group: HashMap<String, IndexSet<u32>>,
    pub hashed: IndexMap<String, u32, RandomState>,
    pub listed: Vec<IndexMap<String, u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<IndexMap<String, u32>>,
    pub tagged: IndexSet<u32, RandomState>,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedTwin {
    pub by_group: HashMap<String, HashSet<u32>>,
    pub hashed: HashMap<String, u32, RandomState>,
    pub listed: Vec<HashMap<String, u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<HashMap<String, u32>>,
    pub tagged: HashSet<u32, RandomState>,
}

#[model_schema()]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Slot {
    Closing,
    Opening,
}

// A map keyed by a plain enum: its members are the object's keys.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slotted {
    pub counts: IndexMap<Slot, u32>,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlottedTwin {
    pub counts: HashMap<Slot, u32>,
}

/// `output`, which a twin gave, as the type it is the twin of would have given it.
#[cfg(any(
    feature = "typescript",
    feature = "zod",
    feature = "dart",
    feature = "swift",
    feature = "kotlin"
))]
fn under_its_own_name(output: &str) -> String {
    output.replace("Twin", "")
}

#[test]
fn a_document_is_written_back_in_the_order_it_was_read() {
    let read: Document = serde_json::from_str(DOCUMENT).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), DOCUMENT);
    let nested: Nested = serde_json::from_str(NESTED).unwrap();
    assert_eq!(serde_json::to_string(&nested).unwrap(), NESTED);
    let slotted: Slotted = serde_json::from_str(SLOTTED).unwrap();
    assert_eq!(serde_json::to_string(&slotted).unwrap(), SLOTTED);
}

/// The pin is in this file twice: as the constant the README is searched for, and as the
/// declaration the compiler reads.
#[test]
fn the_readme_declares_the_document_and_the_text_it_reads_and_writes_back() {
    let readme = include_str!("../../README.md");
    assert_eq!(
        include_str!("tests.rs").matches(README_DECLARATION).count(),
        2,
        "this is pinned but no longer declared here character for character:\n{README_DECLARATION}"
    );
    assert!(
        readme.contains(README_DECLARATION),
        "the README no longer declares this verbatim:\n{README_DECLARATION}"
    );
    assert_eq!(
        readme.matches(DOCUMENT).count(),
        2,
        "the README no longer reads this text and writes it back: {DOCUMENT}"
    );
}

/// The wire is one: what a type here writes, its std twin reads, and writes back as a document
/// this type reads.
#[test]
fn what_each_type_writes_its_std_twin_reads() {
    let document: DocumentTwin = serde_json::from_str(DOCUMENT).unwrap();
    assert_eq!(document.data.len(), 2);
    assert_eq!(document.labels.len(), 2);
    let back: Document = serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
    assert_eq!(back.data.get("zeta").map(String::as_str), Some("1"));

    let nested: NestedTwin = serde_json::from_str(NESTED).unwrap();
    assert_eq!(nested.hashed.get("alpha"), Some(&2));
    assert_eq!(nested.tagged.len(), 2);
    let again: Nested = serde_json::from_str(&serde_json::to_string(&nested).unwrap()).unwrap();
    assert_eq!(again.by_group.get("odd").map(IndexSet::len), Some(2));
    assert_eq!(again.listed.len(), 1);
    assert_eq!(again.optional.map(|held| held.len()), Some(2));

    let slotted: SlottedTwin = serde_json::from_str(SLOTTED).unwrap();
    assert_eq!(slotted.counts.get(&Slot::Opening), Some(&2));
    let reread: Slotted = serde_json::from_str(&serde_json::to_string(&slotted).unwrap()).unwrap();
    assert_eq!(reread.counts.get(&Slot::Closing), Some(&1));
}

#[cfg(feature = "typescript")]
#[test]
fn typescript_writes_each_as_its_std_twin() {
    let document = Document::ts_definition();
    for line in [
        "  data: Partial<Record<string, string>>;",
        "  labels: Array<string>;",
    ] {
        assert!(document.contains(line), "missing `{line}` in: {document}");
        assert!(
            include_str!("../../README.md")
                .lines()
                .any(|shown| shown == line),
            "the README no longer shows this member verbatim: {line}"
        );
    }
    assert_eq!(document, under_its_own_name(&DocumentTwin::ts_definition()));
    assert_eq!(
        Nested::ts_definition(),
        under_its_own_name(&NestedTwin::ts_definition())
    );
    let slotted = Slotted::ts_definition();
    assert!(
        slotted.contains("  counts: Partial<Record<Slot, number>>;"),
        "got: {slotted}"
    );
    assert_eq!(slotted, under_its_own_name(&SlottedTwin::ts_definition()));
}

#[cfg(feature = "zod")]
#[test]
fn zod_writes_each_as_its_std_twin() {
    let document = Document::zod_schema();
    for member in [
        "data: z.record(z.string(), z.string()),",
        "labels: z.array(z.string()).check((ctx) => {",
    ] {
        assert!(
            document.contains(member),
            "missing `{member}` in: {document}"
        );
    }
    assert_eq!(document, under_its_own_name(&DocumentTwin::zod_schema()));
    assert_eq!(
        Nested::zod_schema(),
        under_its_own_name(&NestedTwin::zod_schema())
    );
    let slotted = Slotted::zod_schema();
    assert!(
        slotted.contains("counts: z.partialRecord(Slot$Schema, "),
        "got: {slotted}"
    );
    assert_eq!(slotted, under_its_own_name(&SlottedTwin::zod_schema()));
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_of_each_is_its_std_twins() {
    assert_eq!(Document::json_schema(), DocumentTwin::json_schema());
    assert_eq!(Nested::json_schema(), NestedTwin::json_schema());
    assert_eq!(Slotted::json_schema(), SlottedTwin::json_schema());
    assert_eq!(
        Document::json_schema()["properties"]["data"],
        serde_json::json!({ "type": "object", "additionalProperties": { "type": "string" } })
    );
}

#[cfg(feature = "dart")]
#[test]
fn dart_writes_each_as_its_std_twin() {
    assert_eq!(
        document_dart::dart_definition(),
        under_its_own_name(&document_twin_dart::dart_definition())
    );
    assert_eq!(
        nested_dart::dart_definition(),
        under_its_own_name(&nested_twin_dart::dart_definition())
    );
    assert_eq!(
        slotted_dart::dart_definition(),
        under_its_own_name(&slotted_twin_dart::dart_definition())
    );
}

#[cfg(feature = "swift")]
#[test]
fn swift_writes_each_as_its_std_twin() {
    assert_eq!(
        document_swift::swift_definition(),
        under_its_own_name(&document_twin_swift::swift_definition())
    );
    assert_eq!(
        nested_swift::swift_definition(),
        under_its_own_name(&nested_twin_swift::swift_definition())
    );
    assert_eq!(
        slotted_swift::swift_definition(),
        under_its_own_name(&slotted_twin_swift::swift_definition())
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn kotlin_writes_each_as_its_std_twin() {
    assert_eq!(
        document_kotlin::kotlin_definition(),
        under_its_own_name(&document_twin_kotlin::kotlin_definition())
    );
    assert_eq!(
        nested_kotlin::kotlin_definition(),
        under_its_own_name(&nested_twin_kotlin::kotlin_definition())
    );
    assert_eq!(
        slotted_kotlin::kotlin_definition(),
        under_its_own_name(&slotted_twin_kotlin::kotlin_definition())
    );
}
