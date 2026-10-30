//! A `Result` field in the recovering decode, walked into the arm it holds, an issue inside the arm
//! listed at its full path, and in the typed MongoDB paths, a `Field` of the whole value.

use logbook_schema::DecodeWith as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

/// A model with a bound of its own, read through its own walker.
#[model_schema(enable = ["decode_with"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Headline {
    #[model_schema_prop(minLength = 1)]
    pub title: String,
}

/// What a frontmatter that does not read holds.
#[model_schema(enable = ["decode_with"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Garbled {
    pub reason: String,
}

/// A frontmatter of a model in either arm, beside a `Result` of no model.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Logbook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<Result<Headline, Garbled>>,
    pub outcome: Result<u32, String>,
}

/// Each issue the recovering decode lists of `stored`, by its path and its kind.
fn listed(stored: Value) -> Vec<(String, &'static str, Option<logbook_schema::Expected>)> {
    let mut seen = Vec::new();
    let read = Logbook::from_value_with(stored, |_raw, found| {
        seen = found.to_vec();
        logbook_schema::Verdict::Reject
    });
    assert!(read.is_err() || seen.is_empty());
    seen.into_iter()
        .map(|issue| match issue {
            logbook_schema::Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } => (path.to_string(), "Invalid", Some(expected)),
            logbook_schema::Issue::Missing { path, expected } => {
                (path.to_string(), "Missing", Some(expected))
            }
            logbook_schema::Issue::Mistyped {
                path,
                expected,
                found: _found,
            } => (path.to_string(), "Mistyped", Some(expected)),
            logbook_schema::Issue::Unknown {
                path,
                found: _found,
            } => (path.to_string(), "Unknown", None),
            logbook_schema::Issue::NoVariant {
                path,
                found: _found,
                variants: _variants,
            } => (path.to_string(), "NoVariant", None),
            logbook_schema::Issue::Undescribed { reason: _reason } => {
                (String::new(), "Undescribed", None)
            }
        })
        .collect()
}

#[test]
fn an_issue_inside_an_arm_is_listed_at_its_full_path() {
    let in_ok = listed(json!({
        "frontmatter": { "Ok": { "title": "" } },
        "outcome": { "Ok": 7_u32 },
    }));
    assert_eq!(in_ok.len(), 1, "{in_ok:?}");
    assert_eq!(
        (in_ok[0].0.as_str(), in_ok[0].1),
        ("frontmatter.Ok.title", "Invalid")
    );
    let in_both = listed(json!({
        "frontmatter": { "Err": { "reason": 3_u32 } },
        "outcome": { "Ok": "seven" },
    }));
    let places: Vec<&str> = in_both.iter().map(|(place, _, _)| place.as_str()).collect();
    assert_eq!(
        places,
        ["frontmatter.Err.reason", "outcome.Ok"],
        "{in_both:?}"
    );
    assert_eq!(in_both[1].2, Some(logbook_schema::Expected::U32));
}

#[test]
fn an_object_of_both_keys_of_neither_or_of_another_is_one_issue_at_the_field() {
    for outcome in [
        json!({ "Ok": 7_u32, "Err": "x" }),
        json!({}),
        json!({ "Fine": 7_u32 }),
        json!(7_u32),
    ] {
        let issues = listed(json!({ "outcome": outcome }));
        assert_eq!(
            issues,
            [(
                "outcome".to_owned(),
                "Invalid",
                Some(logbook_schema::Expected::Variants(&["Ok", "Err"]))
            )],
            "for {outcome}"
        );
    }
}

#[test]
fn a_result_that_reads_lists_nothing() {
    let sound = json!({
        "frontmatter": { "Err": { "reason": "no title" } },
        "outcome": { "Err": "late" },
    });
    assert_eq!(
        Logbook::from_value_with(sound, |_raw, _found| logbook_schema::Verdict::Reject),
        Ok(Logbook {
            frontmatter: Some(Err(Garbled {
                reason: "no title".to_owned(),
            })),
            outcome: Err("late".to_owned()),
        })
    );
}

#[cfg(feature = "bson")]
#[test]
fn an_issue_inside_an_arm_of_a_bson_value_is_listed_at_its_full_path() {
    let stored = bson::doc! {
        "frontmatter": { "Ok": { "title": "" } },
        "outcome": { "Ok": 7_i32, "Err": "x" },
    };
    let mut seen = Vec::new();
    let read = Logbook::from_bson_with(stored, |_raw, found| {
        seen = found.to_vec();
        logbook_schema::Verdict::Reject
    });
    read.unwrap_err();
    let places: Vec<String> = seen
        .iter()
        .filter_map(|issue| {
            if let logbook_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                Some(path.to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(seen.len(), places.len(), "{seen:?}");
    assert_eq!(places, ["frontmatter.Ok.title", "outcome"], "{seen:?}");
}

#[cfg(feature = "mongodb")]
#[test]
fn a_result_is_a_field_of_one_whole_value() {
    let path: logbook_schema::Field<Logbook, Result<u32, String>> = Logbook::MONGO_FIELDS.outcome();
    let filter: bson::Document = path.eq(Ok(7)).unwrap().into();
    let written = bson::to_document(&json!({ "Ok": 7_u32 })).unwrap();
    assert_eq!(
        filter.get_document("outcome").unwrap().get_document("$eq"),
        Ok(&written),
        "{filter}"
    );
    let _: logbook_schema::OptionalField<Logbook, Result<Headline, Garbled>> =
        Logbook::MONGO_FIELDS.frontmatter();
}
