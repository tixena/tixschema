//! A bound written on a field is the recovering read's to list, in a BSON document as in a JSON
//! value: one issue per violation, at the value, in the bound's own words.
//!
//! A validator is published only where a schema surface is on, so a build with none lists nothing.

use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundedGroup {
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    human_name: Option<String>,
    #[model_schema_prop(minimum = 1)]
    level: i32,
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sort_field: Option<BoundedPath>,
    source: BoundedPath,
    #[model_schema_prop(minLength = 2, pattern = "^[a-z]+$")]
    tags: Vec<String>,
    #[model_schema_prop(maxLength = 3)]
    title: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct BoundedPath(String);

/// A stored group that breaks every bound `BoundedGroup` writes.
fn broken() -> Document {
    doc! {
        "humanName": "",
        "level": 0_i32,
        "sortField": "",
        "source": "Items",
        "tags": ["ok", "A"],
        "title": "long",
    }
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_broken_bound_is_listed_at_its_value_once_per_violation() {
    let mut calls = 0_u32;
    let read = BoundedGroup::from_bson_with(broken(), |_raw, _found| {
        calls += 1;
        bounded_group_schema::Verdict::Reject
    });
    assert_eq!(
        read.unwrap_err().to_string().lines().collect::<Vec<&str>>(),
        [
            "humanName: invalid: expected Optional(String), found String(\"\"): too short: minimum \
             length is 1, got 0",
            "level: invalid: expected I32, found Int32(0): too small: minimum is 1, got 0",
            "sortField: invalid: expected Model(\"BoundedPath\"), found String(\"\"): too short: \
             minimum length is 1, got 0",
            "tags[1]: invalid: expected String, found String(\"A\"): too short: minimum length is \
             2, got 1",
            "tags[1]: invalid: expected String, found String(\"A\"): does not match pattern \
             '^[a-z]+$'",
            "title: invalid: expected String, found String(\"long\"): too long: maximum length is \
             3, got 4",
        ]
    );
    assert_eq!(calls, 1);
}

/// The callback drops each value a bound refuses, and the read answers what is left.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_callback_that_removes_what_breaks_a_bound_recovers_the_read() {
    let stored = doc! {
        "humanName": "", "level": 1_i32, "sortField": "", "source": "Items", "tags": ["A"],
        "title": "ok",
    };
    let read = BoundedGroup::from_bson_with(stored, |raw, found| {
        for issue in found {
            if let bounded_group_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.remove_from_document(raw);
            }
        }
        bounded_group_schema::Verdict::Fixed
    });
    assert_eq!(
        read,
        Ok(BoundedGroup {
            human_name: None,
            level: 1,
            sort_field: None,
            source: BoundedPath("Items".to_owned()),
            tags: Vec::new(),
            title: "ok".to_owned(),
        })
    );
}

/// The bound stays off serde's own read, which reads the value that breaks it.
#[test]
fn plain_serde_reads_what_breaks_a_bound() {
    let read =
        BoundedGroup::deserialize(bson::Deserializer::new(Bson::Document(broken()))).unwrap();
    assert_eq!(read.sort_field, Some(BoundedPath(String::new())));
    assert_eq!(read.human_name, Some(String::new()));
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        let mut calls = 0_u32;
        let walked = BoundedGroup::from_bson_with(broken(), |_raw, _found| {
            calls += 1;
            bounded_group_schema::Verdict::Reject
        });
        assert_eq!(walked.unwrap(), read);
        assert_eq!(calls, 0);
    }
}
