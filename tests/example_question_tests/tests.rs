//! Tests of `?` and of the valid type in a `rust example` block.

use core::num::ParseIntError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::panic::catch_unwind;
use tixschema::model_schema;

/// A correlation id
/// ```rust example
/// CorrelationId::new("20e690df-a407-4468-adda-76d0b84913f5".to_owned())?
/// ```
#[model_schema(enable = ["jsonschema"], minLength = 1)]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(String);

/// A count read from text
/// ```rust example
/// CountedDraft { count: "42".parse::<u32>()? }
/// ```
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Counted {
    pub count: u32,
}

/// A role
/// ```rust example
/// SessionRole::Auditor
/// ```
/// ```rust example
/// SessionRole::Technician
/// ```
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum SessionRole {
    Auditor,
    Technician,
}

/// A rule result
/// ```rust example
/// let result: RuleResult = RuleResultDraft { passed: true, role: SessionRole::Technician }.into();
/// result
/// ```
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuleResult {
    pub passed: bool,
    pub role: SessionRole,
}

/// A shipment whose valid value an example builds
/// ```rust example
/// ShipmentDraft { weight: 3 }.validate()?
/// ```
/// ```rust example
/// ShipmentDraft { weight: 4 }
/// ```
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Shipment {
    #[model_schema_prop(minimum = 1)]
    pub weight: u32,
}

/// A value an example cannot build
/// ```rust example
/// BrokenDraft { count: "x".parse::<u32>()? }
/// ```
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Broken {
    pub count: u32,
}

/// A type of the author's that carries the name of the failure the macro generates.
pub struct ExampleFailure(pub u32);

impl ExampleFailure {
    /// # Errors
    /// The text is not a number.
    pub fn new(text: &str) -> Result<Self, ParseIntError> {
        text.parse().map(Self)
    }
}

/// A block that names a type called `ExampleFailure`
/// ```rust example
/// ShadowingDraft { held: ExampleFailure::new("7")?.0 }
/// ```
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Shadowing {
    pub held: u32,
}

#[test]
fn a_brand_block_may_use_a_question_mark() {
    assert_eq!(
        CorrelationId::schema_examples(),
        vec![json!("20e690df-a407-4468-adda-76d0b84913f5")]
    );
}

#[test]
fn a_question_mark_accepts_a_foreign_error() {
    assert_eq!(Counted::schema_examples(), vec![json!({ "count": 42_u32 })]);
}

#[test]
fn a_block_may_build_the_valid_type_or_the_draft() {
    assert_eq!(
        SessionRole::schema_examples(),
        vec![json!("Auditor"), json!("Technician")]
    );
    assert_eq!(
        RuleResult::schema_examples(),
        vec![json!({ "passed": true, "role": "Technician" }),]
    );
    assert_eq!(
        Shipment::schema_examples(),
        vec![json!({ "weight": 3_u32 }), json!({ "weight": 4_u32 })]
    );
}

#[test]
fn a_failed_question_mark_panics_with_the_example_and_the_error() {
    let failure = catch_unwind(Broken::schema_examples).unwrap_err();
    let text = failure
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        text,
        "example 1 of `Broken` failed: ParseIntError { kind: InvalidDigit }"
    );
}

#[test]
fn a_block_may_name_a_type_called_example_failure() {
    assert_eq!(Shadowing::schema_examples(), vec![json!({ "held": 7_u32 })]);
}
