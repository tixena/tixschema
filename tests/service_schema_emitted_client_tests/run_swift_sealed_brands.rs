//! A Swift brand is built through `init(validating:)` alone: `init(value:)` and the unjudged read
//! are `fileprivate`, so a second file of the module cannot reach either, and a tuple struct that
//! is no brand keeps its public `init(value:)`.

#![cfg(feature = "swift")]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::{checked_together, judged};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// A brand with a rule.
#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

/// A holder of the brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Label {
    pub text: LabelText,
}

/// A tuple struct that is no brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tag(String);

/// The generated types, as one file of the module.
fn generated() -> String {
    [
        "import Foundation".to_owned(),
        label_text_swift::swift_definition(),
        label_swift::swift_definition(),
        tag_swift::swift_definition(),
    ]
    .join("\n\n")
}

/// What checking `other` beside the generated file answered, or `None` where no toolchain is there.
fn checked_beside(other: &str) -> Option<(bool, String)> {
    let source = generated();
    checked_together(
        "swift-sealed-brands",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        &[("generated.swift", &source), ("other.swift", other)],
    )
    .map(|run| (run.succeeded, run.stderr))
}

/// A file of the module builds a brand through `init(validating:)` and a tuple struct through
/// `init(value:)`.
#[test]
fn a_second_file_builds_a_brand_through_validating_and_a_tuple_struct_through_value() {
    let Some((succeeded, said)) = checked_beside(
        "
func buildBrand() throws -> LabelText { try LabelText(validating: \"Shipping\") }
func buildTuple() -> Tag { Tag(value: \"x\") }
",
    ) else {
        return;
    };
    assert!(succeeded, "{said}");
}

/// A file of the module does not compile a call of a brand's `init(value:)`.
#[test]
fn a_second_file_cannot_build_a_brand_through_value() {
    let Some((succeeded, said)) = checked_beside(
        "
func build() -> LabelText { LabelText(value: \"\") }
",
    ) else {
        return;
    };
    assert!(!succeeded, "a brand was built through init(value:)");
    assert!(said.contains("incorrect argument label in call"), "{said}");
}

/// A file of the module does not compile a call of a brand's unjudged read.
#[test]
fn a_second_file_cannot_read_a_brand_unjudged() {
    let Some((succeeded, said)) = checked_beside(
        "
func read(_ decoder: Decoder) throws -> LabelText { try LabelText(swiftSchemaUnjudged: decoder) }
",
    ) else {
        return;
    };
    assert!(!succeeded, "a brand was read unjudged");
    assert!(said.contains("incorrect argument label in call"), "{said}");
}

/// `init(validating:)` throws the messages `validate()` gives, and a holder refuses the value its
/// brand refuses.
#[test]
fn validating_throws_the_messages_validate_gives() {
    let source = format!(
        "{}\n\n{}",
        generated(),
        r##"
do {
  print(try LabelText(validating: "Shipping").value)
  _ = try LabelText(validating: "")
} catch DecodingError.dataCorrupted(let context) {
  print(context.debugDescription)
}
do {
  _ = try JSONDecoder().decode(Label.self, from: Data(#"{"text":""}"#.utf8))
} catch DecodingError.dataCorrupted(let context) {
  print(context.codingPath.map { $0.stringValue }.joined(separator: "."))
  print(context.debugDescription)
}
"##
    );
    let Some(run) = judged(
        "swift-sealed-brands-run",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n{source}", run.stderr);
    let label = LabelDraft {
        text: LabelText::new("Shipping").unwrap(),
    };
    assert_eq!(label.text.as_inner(), "Shipping");
    assert_eq!(
        serde_json::to_string(&label).unwrap(),
        r#"{"text":"Shipping"}"#
    );
    assert_eq!(
        serde_json::to_string(&TagDraft("x".to_owned())).unwrap(),
        r#""x""#
    );
    let refused = LabelText::new("").unwrap_err().join("; ");
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "Shipping",
            refused.as_str(),
            "",
            &format!("'text': {refused}")
        ]
    );
}
