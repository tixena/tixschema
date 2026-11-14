//! A Swift model type is a pair: the draft, which code builds and changes, and the valid type,
//! which only the draft's `validate()` and a read make. A second file of the module cannot build
//! the valid type, and every refusal keeps the text it had.

#![cfg(feature = "swift")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::{checked_together, judged};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What the run prints: the refusals of `validate()` and of a read, and a valid value read and
/// written as its draft.
const MAIN: &str = r##"
let draft = ShipmentDraft(label: try LabelText(validating: "Shipping"), parcelCount: 200)
do { _ = try draft.validate() } catch DecodingError.dataCorrupted(let context) {
  print(context.debugDescription)
}
do {
  _ = try JSONDecoder().decode(Shipment.self, from: Data(#"{"label":"Shipping","parcelCount":200}"#.utf8))
} catch DecodingError.dataCorrupted(let context) {
  print(context.debugDescription)
}
let shipment = try ShipmentDraft(label: try LabelText(validating: "Shipping"), parcelCount: 3).validate()
print(shipment.parcelCount)
print(shipment.draft.parcelCount)
let encoder = JSONEncoder()
encoder.outputFormatting = [.sortedKeys]
print(String(decoding: try encoder.encode(shipment), as: UTF8.self))
print(String(decoding: try encoder.encode(shipment.draft), as: UTF8.self))
let manifest = try JSONDecoder().decode(Manifest.self, from: Data(#"{"shipment":{"label":"a","parcelCount":1},"extra":[]}"#.utf8))
print(manifest.shipment.parcelCount)
do {
  _ = try JSONDecoder().decode(Manifest.self, from: Data(#"{"shipment":{"label":"a","parcelCount":101},"extra":[]}"#.utf8))
} catch DecodingError.dataCorrupted(let context) {
  print(context.codingPath.map { $0.stringValue }.joined(separator: "."))
  print(context.debugDescription)
}
print(try JSONDecoder().decode(Leg.self, from: Data(#"{"kind":"Voyage"}"#.utf8)).draft)
print(try JSONDecoder().decode(Lane.self, from: Data(#""Air""#.utf8)))
print(try JSONDecoder().decode(Tag.self, from: Data(#""x""#.utf8)).value)
print(try JSONDecoder().decode(Crate<String>.self, from: Data(#"{"content":"x"}"#.utf8)).content)
let holder = try JSONDecoder().decode(Holder.self, from: Data(#"{"draft":7}"#.utf8))
print(holder.draft_)
print(holder.draft.draft_)
print(String(decoding: try encoder.encode(holder), as: UTF8.self))
"##;

/// A brand with a rule.
#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

/// A struct with a member under a rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shipment {
    pub label: LabelText,
    #[model_schema_prop(maximum = 100)]
    pub parcel_count: u32,
}

/// A holder of the struct, which names it by the valid type.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub extra: Vec<Shipment>,
    pub shipment: Shipment,
}

/// A plain enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Lane {
    Air,
    Sea,
}

/// A tagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Leg {
    Flight { code: String },
    Voyage,
}

/// A tuple struct of one slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tag(String);

/// A struct with a member named after the method a draft declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Audit {
    pub validation_issues: Vec<String>,
}

/// A struct with a member named after the valid type's own property.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Holder {
    pub draft: u32,
}

/// A generic struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate<T> {
    pub content: T,
}

/// The generated types, as one file of the module.
fn generated() -> String {
    [
        "import Foundation".to_owned(),
        label_text_swift::swift_definition(),
        shipment_swift::swift_definition(),
        manifest_swift::swift_definition(),
        lane_swift::swift_definition(),
        leg_swift::swift_definition(),
        tag_swift::swift_definition(),
        crate_swift::swift_definition(),
        audit_swift::swift_definition(),
        holder_swift::swift_definition(),
    ]
    .join("\n\n")
}

/// What checking `other` beside the generated file answered, or `None` where no toolchain is there.
fn checked_beside(other: &str) -> Option<(bool, String)> {
    let source = generated();
    checked_together(
        "swift-valid-types",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        &[("generated.swift", &source), ("other.swift", other)],
    )
    .map(|run| (run.succeeded, run.stderr))
}

/// A file of the module builds each draft, and reads a valid value through its draft.
#[test]
fn a_second_file_builds_a_draft_and_validates_it() {
    let Some((succeeded, said)) = checked_beside(
        "
func build() throws -> Shipment {
    var draft = ShipmentDraft(label: try LabelText(validating: \"Shipping\"), parcelCount: 3)
    draft.parcelCount = 4
    return try draft.validate()
}
func read(_ shipment: Shipment) -> UInt32 { shipment.parcelCount }
func back(_ shipment: Shipment) -> ShipmentDraft { shipment.draft }
func lane() -> Lane { Lane.air }
func leg() throws -> Leg { try LegDraft.voyage.validate() }
func tag() throws -> Tag { try TagDraft(value: \"x\").validate() }
func crate() throws -> Crate<String> { try CrateDraft(content: \"x\").validate() }
func held() throws -> UInt32 { try HolderDraft(draft_: 1).validate().draft_ }
func audit() throws -> [String] { try AuditDraft(validationIssues_: []).validate().validationIssues_ }
",
    ) else {
        return;
    };
    assert!(succeeded, "{said}");
}

/// A file of the module does not compile a call of the valid type's initializer.
#[test]
fn a_second_file_cannot_build_the_valid_type() {
    let Some((succeeded, said)) = checked_beside(
        "
func build(_ draft: ShipmentDraft) -> Shipment { Shipment(draft: draft) }
",
    ) else {
        return;
    };
    assert!(!succeeded, "a valid type was built from a draft");
    assert!(
        said.contains("incorrect argument label in call (have 'draft:', expected 'from:')"),
        "{said}"
    );
}

/// A file of the module does not compile a change of a member of the valid type.
#[test]
fn a_second_file_cannot_change_a_member_of_the_valid_type() {
    let Some((succeeded, said)) = checked_beside(
        "
func change(_ shipment: inout Shipment) { shipment.parcelCount = 1 }
",
    ) else {
        return;
    };
    assert!(!succeeded, "a member of a valid type was changed");
    assert!(
        said.contains("cannot assign to property: 'shipment' is immutable"),
        "{said}"
    );
}

/// A draft that breaks a rule throws the messages the read throws, joined by a semicolon, and a
/// valid value reads and writes as its draft does.
#[test]
fn validate_throws_the_messages_the_read_throws() {
    let source = format!("{}\n\n{}", generated(), MAIN);
    let Some(run) = judged(
        "swift-valid-types-run",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n{source}", run.stderr);
    let shipment = ShipmentDraft {
        label: LabelText::new("Shipping").unwrap(),
        parcel_count: 3,
    };
    let refused = ShipmentDraft {
        parcel_count: 200,
        ..shipment.clone()
    }
    .validation_issues()
    .join("; ");
    let written = serde_json::to_string(&shipment).unwrap();
    let manifest = ManifestDraft {
        shipment: shipment.validate().unwrap(),
        extra: Vec::new(),
    };
    assert_eq!(
        serde_json::to_string(&manifest).unwrap(),
        r#"{"extra":[],"shipment":{"label":"Shipping","parcelCount":3}}"#
    );
    assert_eq!(
        serde_json::to_string(&HolderDraft { draft: 7 }).unwrap(),
        r#"{"draft":7}"#
    );
    let tag = TagDraft("x".to_owned());
    let crated = CrateDraft {
        content: "x".to_owned(),
    };
    let lane = Lane::Air;
    let leg = LegDraft::Voyage;
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            refused.as_str(),
            refused.as_str(),
            "3",
            "3",
            written.as_str(),
            written.as_str(),
            "1",
            "",
            "'shipment.parcelCount': too large: maximum is 100, got 101",
            "voyage",
            "air",
            "x",
            "x",
            "7",
            "7",
            r#"{"draft":7}"#
        ]
    );
    assert_eq!(
        serde_json::to_string(&AuditDraft {
            validation_issues: Vec::new()
        })
        .unwrap(),
        r#"{"validationIssues":[]}"#
    );
    assert_eq!(serde_json::to_string(&lane).unwrap(), r#""Air""#);
    assert_eq!(serde_json::to_string(&leg).unwrap(), r#"{"kind":"Voyage"}"#);
    assert_eq!(serde_json::to_string(&tag).unwrap(), r#""x""#);
    assert_eq!(
        serde_json::to_string(&crated).unwrap(),
        r#"{"content":"x"}"#
    );
}
