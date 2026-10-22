//! A `literal` is a rule: `validate()` refuses every other value, and so does the read that runs
//! it, in each place a member is.

#![cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]

use alloc::borrow::Cow;
use core::fmt::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::literal_models::{
    Answered, Assay, Hallmark, Ingot, LITERAL_READS, literal_report, names_each_violation,
    serde_read,
};

/// A literal through each wrapper a member reaches it by: a fixed array, a box, a borrow, a path,
/// an `Option` of a list of `Option`s and a list of lists.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Reached<'held> {
    #[model_schema_prop(literal = "x")]
    pub boxed: Box<str>,
    #[model_schema_prop(literal = "x")]
    #[serde(borrow)]
    pub cow: Cow<'held, str>,
    #[model_schema_prop(literal = 3)]
    pub grid: Vec<Vec<u16>>,
    #[model_schema_prop(literal = "x")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nested: Option<Vec<Option<String>>>,
    #[model_schema_prop(literal = "x")]
    pub pair: [String; 2],
    #[model_schema_prop(literal = "x")]
    pub path: PathBuf,
}

/// Serde's read answers each payload of the table, in the words `validate()` reports each
/// violation in.
#[test]
fn the_read_refuses_each_value_other_than_its_literal() {
    let mut printed = String::new();
    for row in &LITERAL_READS {
        match serde_read(row.held_by, row.payload) {
            Ok(()) => {
                let _ = writeln!(printed, "{}: read", row.name);
            }
            Err(refusal) => {
                let _ = writeln!(printed, "{}: refused", row.name);
                if names_each_violation(row) {
                    for violation in refusal.split("; ") {
                        let _ = writeln!(printed, "    {violation}");
                    }
                } else {
                    assert_eq!(
                        refusal, "data did not match any variant of untagged enum Ingot",
                        "serde refuses {} in other words",
                        row.name
                    );
                }
            }
        }
    }
    assert_eq!(printed, literal_report(names_each_violation));
}

/// `validate()` reports of a value built in code what the read refuses the payload with.
#[test]
fn validate_refuses_a_value_built_with_another_value() {
    let other = Hallmark {
        fineness: 916,
        grade: "britannia".to_owned(),
        marks: vec!["lion".to_owned(), "anchor".to_owned()],
        offset: Some(4_i32),
        purity: 0.75,
        stamped: false,
        weight: 2.5,
    };
    let said: Vec<String> = match LITERAL_READS[2].answered {
        Answered::Refused(said) => said.iter().map(|line| (*line).to_owned()).collect(),
        Answered::Read | Answered::Unmatched => Vec::new(),
    };
    assert_eq!(other.validate(), Err(said));

    let declared = Hallmark {
        fineness: 925,
        grade: "sterling".to_owned(),
        marks: Vec::new(),
        offset: None,
        purity: 0.5,
        stamped: true,
        weight: 1.25,
    };
    assert_eq!(declared.validate(), Ok(()));
    assert_eq!(
        Assay::Weighed(30, false).validate(),
        Err(vec![
            "'Weighed.0': not the literal 31, got 30".to_owned(),
            "'Weighed.1': not the literal true, got false".to_owned(),
        ])
    );
}

/// The member of an untagged enum is held to its literal as it is read, which chooses the
/// variant.
#[test]
fn a_literal_chooses_the_variant_of_an_untagged_enum() {
    assert_eq!(
        serde_json::from_str::<Ingot>(r#"{"metal":"silver","weight":1}"#).unwrap(),
        Ingot::Silver {
            metal: "silver".to_owned(),
            weight: 1,
        }
    );
    assert_eq!(
        serde_json::from_str::<Ingot>(r#"{"metal":"gold","weight":1}"#).unwrap(),
        Ingot::Gold {
            metal: "gold".to_owned(),
            weight: 1,
        }
    );
}

/// A literal is reached through every wrapper, and each item of a list holds it under its index.
#[test]
fn a_literal_is_held_through_each_wrapper_and_by_each_item() {
    let other = Reached {
        boxed: "y".into(),
        cow: Cow::Borrowed("z"),
        grid: vec![vec![3, 4]],
        nested: Some(vec![None, Some("w".to_owned())]),
        pair: ["x".to_owned(), "v".to_owned()],
        path: PathBuf::from("p"),
    };
    assert_eq!(
        other.validate(),
        Err(vec![
            "'boxed': not the literal \"x\", got y".to_owned(),
            "'cow': not the literal \"x\", got z".to_owned(),
            "'grid.0.1': not the literal 3, got 4".to_owned(),
            "'nested.1': not the literal \"x\", got w".to_owned(),
            "'pair.1': not the literal \"x\", got v".to_owned(),
            "'path': not the literal \"x\", got p".to_owned(),
        ])
    );
    let held = Reached {
        boxed: "x".into(),
        cow: Cow::Borrowed("x"),
        grid: vec![vec![3], Vec::new()],
        nested: None,
        pair: ["x".to_owned(), "x".to_owned()],
        path: PathBuf::from("x"),
    };
    assert_eq!(held.validate(), Ok(()));
    assert_eq!(
        serde_json::from_str::<Reached<'_>>(
            r#"{"boxed":"x","cow":"x","grid":[[3]],"nested":[null,"y"],"pair":["x","x"],"path":"x"}"#
        )
        .unwrap_err()
        .to_string(),
        "'nested.1': not the literal \"x\", got y"
    );
}
