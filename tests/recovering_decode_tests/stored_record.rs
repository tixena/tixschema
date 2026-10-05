//! The record the design was written for: an id, a date, and inner model types, read from the JSON
//! values an older writer left behind.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

// The JSON schema of a type holding a `Version` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use super::version_schema;
use super::{Version, lines};

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    created_at: DateTime<Utc>,
    #[serde(rename = "recordId")]
    id: ObjectId,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    versions: Vec<Version>,
}

/// Ids and numbers held one by one, in a list, a map and an optional field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct CatalogByItem {
    counts: Vec<i32>,
    owners: HashMap<String, ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<ObjectId>,
    tags: Vec<ObjectId>,
}

/// A brand over an id: serde writes it as the id it holds.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct OwnerId(ObjectId);

/// A generic brand, which holds whatever fills its parameter.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Tagged<T>(T);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Owned {
    owner: OwnerId,
    tagged: Tagged<ObjectId>,
}

fn record(numbers: &[i32]) -> Record {
    Record {
        created_at: DateTime::from_timestamp_millis(1_759_600_000_000).unwrap(),
        id: ObjectId::parse_str("6a7cc592ca0574e6efdfe217").unwrap(),
        name: "Loan".to_owned(),
        note: None,
        versions: numbers.iter().map(|&number| Version { number }).collect(),
    }
}

/// Repairs what the cases below leave in a `Record`, and rejects anything else.
fn repair(raw: &mut Value, found: &[record_schema::Issue<Value>]) -> record_schema::Verdict {
    use record_schema::{Expected, Issue, Verdict};

    for issue in found {
        if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Value::Number(epoch),
            reason: _reason,
        } = issue
        {
            let Some(date) = epoch.as_i64().and_then(DateTime::from_timestamp_millis) else {
                return Verdict::Reject;
            };
            path.set_in_value(raw, Value::String(date.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::I32,
            found: Value::String(text),
            reason: _reason,
        } = issue
        {
            let Ok(number) = text.parse::<i32>() else {
                return Verdict::Reject;
            };
            path.set_in_value(raw, Value::from(number));
        } else if let Issue::Mistyped {
            path,
            expected: Expected::ObjectId,
            found: Value::String(hex),
        } = issue
        {
            path.set_in_value(raw, json!({ "$oid": hex }));
        } else if let Issue::Unknown {
            path,
            found: _found,
        } = issue
        {
            path.remove_from_value(raw);
        } else {
            return Verdict::Reject;
        }
    }
    Verdict::Fixed
}

/// Reads `stored` with `decide`, and returns the read, what `decide` was handed, and how many
/// times it ran.
fn read_with(
    stored: Value,
    decide: fn(&mut Value, &[record_schema::Issue<Value>]) -> record_schema::Verdict,
) -> (
    Result<Record, record_schema::Unrecovered<Value>>,
    Vec<String>,
    u32,
) {
    let mut seen: Vec<String> = Vec::new();
    let mut calls = 0_u32;
    let read = Record::from_value_with(stored, |raw, found| {
        calls += 1;
        seen = lines(&record_schema::Unrecovered {
            issues: found.to_vec(),
        });
        decide(raw, found)
    });
    (read, seen, calls)
}

fn reject(_raw: &mut Value, _found: &[record_schema::Issue<Value>]) -> record_schema::Verdict {
    record_schema::Verdict::Reject
}

/// Value 1: the happy path, written by serde itself.
#[test]
fn value_1_what_serde_wrote_is_read_and_the_decider_never_runs() {
    let stored = serde_json::to_value(record(&[1_i32, 2_i32])).unwrap();
    assert_eq!(
        stored,
        json!({
            "createdAt": "2025-10-04T17:46:40Z",
            "name": "Loan",
            "recordId": { "$oid": "6a7cc592ca0574e6efdfe217" },
            "versions": [{ "number": 1_i32 }, { "number": 2_i32 }],
        })
    );
    let (read, seen, calls) = read_with(stored, reject);
    assert_eq!(read, Ok(record(&[1_i32, 2_i32])));
    assert_eq!(seen, Vec::<String>::new());
    assert_eq!(calls, 0);
}

/// Value 2: an epoch where a date belongs, a number as text, and a key the type does not declare.
#[test]
fn value_2_three_problems_are_listed_and_fixed_in_one_call() {
    let stored = json!({
        "recordId": { "$oid": "6a7cc592ca0574e6efdfe217" },
        "name": "Loan",
        "createdAt": 1_759_600_000_000_i64,
        "versions": [{ "number": 1_i32 }, { "number": "2" }],
        "legacyField": true,
    });
    let (read, seen, calls) = read_with(stored, repair);
    assert_eq!(
        seen,
        [
            "createdAt: invalid: expected DateTime, found Number(1759600000000): invalid type: integer `1759600000000`, expected an RFC 3339 formatted date and time string",
            "versions[1].number: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "legacyField: unknown: found Bool(true)",
        ]
    );
    assert_eq!(read, Ok(record(&[1_i32, 2_i32])));
    assert_eq!(calls, 1);
}

/// Value 3: a required key missing, and the callback rejects the record.
#[test]
fn value_3_a_missing_key_is_rejected_with_the_list() {
    let stored = json!({
        "recordId": { "$oid": "6a7cc592ca0574e6efdfe217" },
        "createdAt": "2025-10-04T17:46:40Z",
        "versions": [],
    });
    let (read, _seen, calls) = read_with(stored, reject);
    assert_eq!(
        read,
        Err(record_schema::Unrecovered {
            issues: vec![record_schema::Issue::Missing {
                path: record_schema::Path(vec![record_schema::Segment::Key("name".to_owned())]),
                expected: record_schema::Expected::String,
            }],
        })
    );
    assert_eq!(calls, 1);
}

/// Value 4: one chance. The callback fixes the date but cannot fix the number, so the read fails
/// with the second walk's list.
#[test]
fn value_4_a_fix_that_leaves_one_problem_fails_the_read() {
    fn dates_only(
        raw: &mut Value,
        found: &[record_schema::Issue<Value>],
    ) -> record_schema::Verdict {
        for issue in found {
            if let record_schema::Issue::Invalid {
                path,
                expected: record_schema::Expected::DateTime,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_value(raw, json!("2025-10-04T17:46:40Z"));
            }
        }
        record_schema::Verdict::Fixed
    }

    let stored = json!({
        "recordId": { "$oid": "6a7cc592ca0574e6efdfe217" },
        "name": "Loan",
        "createdAt": 1_759_600_000_000_i64,
        "versions": [{ "number": "two" }],
    });
    let (read, seen, calls) = read_with(stored, dates_only);
    assert_eq!(seen.len(), 2);
    assert_eq!(
        read,
        Err(record_schema::Unrecovered {
            issues: vec![record_schema::Issue::Invalid {
                path: record_schema::Path(vec![
                    record_schema::Segment::Key("versions".to_owned()),
                    record_schema::Segment::Index(0),
                    record_schema::Segment::Key("number".to_owned()),
                ]),
                expected: record_schema::Expected::I32,
                found: json!("two"),
                reason: "invalid type: string \"two\", expected i32".to_owned(),
            }],
        })
    );
    assert_eq!(calls, 1);
}

/// Value 5: an id stored as its bare hex string. serde reads it, and it is not the form the field
/// writes.
#[test]
fn value_5_an_id_stored_as_text_is_mistyped() {
    let stored = json!({
        "recordId": "6a7cc592ca0574e6efdfe217",
        "name": "Loan",
        "createdAt": "2025-10-04T17:46:40Z",
        "versions": [],
    });
    serde_json::from_value::<Record>(stored.clone()).unwrap();
    let (read, seen, calls) = read_with(stored, repair);
    assert_eq!(
        seen,
        ["recordId: mistyped: expected ObjectId, found String(\"6a7cc592ca0574e6efdfe217\")"]
    );
    assert_eq!(read, Ok(record(&[])));
    assert_eq!(calls, 1);
}

/// serde reads the id held as text, so only the second walk's list can fail the read.
#[test]
fn an_id_stored_as_text_still_fails_after_fixed_while_it_is_left() {
    fn untouched(
        _raw: &mut Value,
        _found: &[record_schema::Issue<Value>],
    ) -> record_schema::Verdict {
        record_schema::Verdict::Fixed
    }

    let stored = json!({
        "createdAt": "2025-10-04T17:46:40Z",
        "name": "Loan",
        "recordId": "6a7cc592ca0574e6efdfe217",
        "versions": [],
    });
    serde_json::from_value::<Record>(stored.clone()).unwrap();
    let (read, seen, calls) = read_with(stored, untouched);
    assert_eq!(
        seen,
        ["recordId: mistyped: expected ObjectId, found String(\"6a7cc592ca0574e6efdfe217\")"]
    );
    assert_eq!(lines(&read.unwrap_err()), seen);
    assert_eq!(calls, 1);
}

/// Value 6: a record that decodes, but carries a key its model does not declare.
#[test]
fn value_6_an_undeclared_nested_key_reaches_the_decider() {
    let stored = json!({
        "recordId": { "$oid": "6a7cc592ca0574e6efdfe217" },
        "name": "Loan",
        "createdAt": "2025-10-04T17:46:40Z",
        "versions": [{ "number": 1_i32, "draft": true }],
    });
    serde_json::from_value::<Record>(stored.clone()).unwrap();
    let (read, seen, calls) = read_with(stored, reject);
    assert_eq!(seen, ["versions[0].draft: unknown: found Bool(true)"]);
    assert_eq!(
        read,
        Err(record_schema::Unrecovered {
            issues: vec![record_schema::Issue::Unknown {
                path: record_schema::Path(vec![
                    record_schema::Segment::Key("versions".to_owned()),
                    record_schema::Segment::Index(0),
                    record_schema::Segment::Key("draft".to_owned()),
                ]),
                found: json!(true),
            }],
        })
    );
    assert_eq!(calls, 1);
}

/// Each id and number is read on its own: an issue sits at the item, with the item's own type, and
/// a decider that sets each path repairs the record.
#[test]
fn a_plain_value_in_a_list_a_map_or_an_option_is_listed_and_fixed_at_its_own_path() {
    let stored = json!({
        "counts": [1_i32, "2"],
        "owners": { "alice": "6a7cc592ca0574e6efdfe219" },
        "parent": "6a7cc592ca0574e6efdfe21a",
        "tags": [{ "$oid": "6a7cc592ca0574e6efdfe217" }, "6a7cc592ca0574e6efdfe218"],
    });
    let mut seen: Vec<String> = Vec::new();
    let read = CatalogByItem::from_value_with(stored, |raw, found| {
        use catalog_by_item_schema::{Issue, Verdict};

        seen = lines(&catalog_by_item_schema::Unrecovered {
            issues: found.to_vec(),
        });
        for issue in found {
            if let Issue::Mistyped {
                path,
                expected: _expected,
                found: Value::String(hex),
            } = issue
            {
                path.set_in_value(raw, json!({ "$oid": hex }));
            } else if let Issue::Invalid {
                path,
                expected: _expected,
                found: Value::String(text),
                reason: _reason,
            } = issue
            {
                let Ok(number) = text.parse::<i32>() else {
                    return Verdict::Reject;
                };
                path.set_in_value(raw, Value::from(number));
            } else {
                return Verdict::Reject;
            }
        }
        Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            "counts[1]: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "owners.alice: mistyped: expected ObjectId, found String(\"6a7cc592ca0574e6efdfe219\")",
            "parent: mistyped: expected Optional(ObjectId), found String(\"6a7cc592ca0574e6efdfe21a\")",
            "tags[1]: mistyped: expected ObjectId, found String(\"6a7cc592ca0574e6efdfe218\")",
        ]
    );
    let id = |hex: &str| ObjectId::parse_str(hex).unwrap();
    assert_eq!(
        read,
        Ok(CatalogByItem {
            counts: vec![1_i32, 2_i32],
            owners: HashMap::from([("alice".to_owned(), id("6a7cc592ca0574e6efdfe219"))]),
            parent: Some(id("6a7cc592ca0574e6efdfe21a")),
            tags: vec![
                id("6a7cc592ca0574e6efdfe217"),
                id("6a7cc592ca0574e6efdfe218")
            ],
        })
    );
}

/// A field that holds something other than the list or the map it is written as is one issue, at
/// the field.
#[test]
fn a_list_or_map_field_held_as_something_else_is_one_issue_at_the_field() {
    let stored = json!({ "counts": [], "owners": [], "tags": "none" });
    let read = CatalogByItem::from_value_with(stored, |_raw, _found| {
        catalog_by_item_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "owners: invalid: expected Map(ObjectId), found Array []: not an object",
            "tags: invalid: expected Array(ObjectId), found String(\"none\"): not an array",
        ]
    );
}

/// A brand is read as the one value it holds, so an id held as text under one is `Mistyped` at
/// the brand's own path, expecting the id. Under a generic brand the id fills a parameter, and
/// nothing of a parameter's type is written back from a JSON value to compare.
#[test]
fn an_id_held_as_text_under_a_brand_is_mistyped_at_the_brand() {
    let stored = json!({
        "owner": "6a7cc592ca0574e6efdfe217",
        "tagged": "6a7cc592ca0574e6efdfe218",
    });
    let read = Owned::from_value_with(stored, |raw, found| {
        let seen = lines(&owned_schema::Unrecovered {
            issues: found.to_vec(),
        });
        assert_eq!(
            seen,
            ["owner: mistyped: expected ObjectId, found String(\"6a7cc592ca0574e6efdfe217\")"]
        );
        for issue in found {
            if let owned_schema::Issue::Mistyped {
                path,
                expected: owned_schema::Expected::ObjectId,
                found: Value::String(hex),
            } = issue
            {
                path.set_in_value(raw, json!({ "$oid": hex }));
            }
        }
        owned_schema::Verdict::Fixed
    });
    assert_eq!(
        read,
        Ok(Owned {
            owner: OwnerId(ObjectId::parse_str("6a7cc592ca0574e6efdfe217").unwrap()),
            tagged: Tagged(ObjectId::parse_str("6a7cc592ca0574e6efdfe218").unwrap()),
        })
    );

    let alone = OwnerId::from_value_with(json!("not-an-id"), |_raw, _found| {
        owner_id_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&alone.unwrap_err()),
        [
            "the value itself: invalid: expected ObjectId, found String(\"not-an-id\"): invalid value: string \"not-an-id\", expected 24-character, big-endian hex string"
        ]
    );
}
