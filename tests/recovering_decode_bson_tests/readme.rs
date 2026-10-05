//! The README's recovering-decode example, held as one text with the code that compiles, and run.
//!
//! This crate's lints order a struct's fields alphabetically and the README orders them for
//! reading, so `Record` is held member by member, each one whole.

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use record_schema::{Expected, Issue, Verdict};

use super::oid;

/// The callback the README declares, character for character.
const DECLARED_DECIDER: &str =
    "/// Repairs what older writers left in a `Record` row, inner versions included, and rejects
/// anything else.
fn repair_record(row: &mut Document, issues: &[Issue<Bson>]) -> Verdict {
    for issue in issues {
        if let Issue::Mistyped {
            path,
            expected: Expected::ObjectId,
            found: Bson::String(hex),
        } = issue
        {
            // the record's id, stored as text by rows written before it was an `ObjectId`
            let Ok(id) = ObjectId::parse_str(hex) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::ObjectId(id));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::DateTime(date),
            reason: _reason,
        } = issue
        {
            // the creation date, stored as a BSON date by a driver that writes dates natively
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(date.timestamp_millis())
            else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::Int64(millis),
            reason: _reason,
        } = issue
        {
            // the creation date, stored as epoch milliseconds
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(*millis) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::I32,
            found: Bson::String(text),
            reason: _reason,
        } = issue
        {
            // a version's number, stored as text: the path is `versions[1].number`
            let Ok(number) = text.parse::<i32>() else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::Int32(number));
        } else if let Issue::Unknown {
            path,
            found: _found,
        } = issue
        {
            // a key no type declares, on the record or on one of its versions
            path.remove_from_document(row);
        } else {
            return Verdict::Reject;
        }
    }
    Verdict::Fixed
}";

/// The line the README brings the callback's types into scope with.
const DECLARED_IMPORT: &str = "use record_schema::{Expected, Issue, Verdict};";

/// `Record` as the README declares it: what stands above its members, then each member in the
/// README's order.
const DECLARED_RECORD: [&str; 6] = [
    r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {"#,
    r#"    #[serde(rename = "recordId")]
    pub id: ObjectId,"#,
    "    pub name: String,",
    "    pub created_at: DateTime<Utc>,",
    "    pub versions: Vec<Version>,",
    r#"    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,"#,
];

/// The inner type the README declares, character for character.
const DECLARED_VERSION: &str = r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub number: i32,
}"#;

/// The README's table of what the callback is handed: member, path, expected and found.
const SHOWN_ISSUES: [&str; 5] = [
    r#"| `Mistyped` | `recordId` | `ObjectId` | `String("6a7cc592ca0574e6efdfe217")` |"#,
    "| `Invalid` | `createdAt` | `DateTime` | `DateTime(2025-10-04 17:46:40.0 +00:00:00)` |",
    r#"| `Invalid` | `versions[1].number` | `I32` | `String("2")` |"#,
    "| `Unknown` | `versions[1].draft` | | `Boolean(true)` |",
    r#"| `Unknown` | `_id` | | `ObjectId("6a7cc592ca0574e6efdfe299")` |"#,
];

/// The README's table of the `Record` the read returns.
const SHOWN_RESULT: [&str; 5] = [
    "| `id` | `6a7cc592ca0574e6efdfe217` |",
    "| `name` | `Loan` |",
    "| `created_at` | `2025-10-04T17:46:40+00:00` |",
    "| `versions` | `[1, 2]` |",
    "| `note` | `None` |",
];

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub created_at: DateTime<Utc>,
    #[serde(rename = "recordId")]
    pub id: ObjectId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub versions: Vec<Version>,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub number: i32,
}

/// Repairs what older writers left in a `Record` row, inner versions included, and rejects
/// anything else.
fn repair_record(row: &mut Document, issues: &[Issue<Bson>]) -> Verdict {
    for issue in issues {
        if let Issue::Mistyped {
            path,
            expected: Expected::ObjectId,
            found: Bson::String(hex),
        } = issue
        {
            // the record's id, stored as text by rows written before it was an `ObjectId`
            let Ok(id) = ObjectId::parse_str(hex) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::ObjectId(id));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::DateTime(date),
            reason: _reason,
        } = issue
        {
            // the creation date, stored as a BSON date by a driver that writes dates natively
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(date.timestamp_millis())
            else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::Int64(millis),
            reason: _reason,
        } = issue
        {
            // the creation date, stored as epoch milliseconds
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(*millis) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::I32,
            found: Bson::String(text),
            reason: _reason,
        } = issue
        {
            // a version's number, stored as text: the path is `versions[1].number`
            let Ok(number) = text.parse::<i32>() else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::Int32(number));
        } else if let Issue::Unknown {
            path,
            found: _found,
        } = issue
        {
            // a key no type declares, on the record or on one of its versions
            path.remove_from_document(row);
        } else {
            return Verdict::Reject;
        }
    }
    Verdict::Fixed
}

fn readme() -> &'static str {
    include_str!("../../README.md")
}

/// This file, read back so a pinned declaration can be held against the one that compiles.
fn source() -> &'static str {
    include_str!("readme.rs")
}

/// A declaration the README shows and this module compiles, held to being one text: it appears
/// here twice, as the constant the README is searched for and as what the compiler reads.
fn assert_declared_and_documented(pinned: &str) {
    assert_eq!(
        source().matches(pinned).count(),
        2,
        "this is pinned but no longer declared here character for character:\n{pinned}"
    );
    assert!(
        readme().contains(pinned),
        "the README no longer declares this verbatim:\n{pinned}"
    );
}

/// One issue as a row of the README's table, and never its `reason`, which is the `bson` library's
/// own wording.
fn shown(issue: &Issue<Bson>) -> String {
    if let Issue::Invalid {
        path,
        expected,
        found,
        reason: _reason,
    } = issue
    {
        format!("| `Invalid` | `{path}` | `{expected:?}` | `{found:?}` |")
    } else if let Issue::Mistyped {
        path,
        expected,
        found,
    } = issue
    {
        format!("| `Mistyped` | `{path}` | `{expected:?}` | `{found:?}` |")
    } else if let Issue::Unknown { path, found } = issue {
        format!("| `Unknown` | `{path}` | | `{found:?}` |")
    } else {
        String::from("an issue the README's table has no row for")
    }
}

#[test]
fn the_readme_declares_what_compiles_here() {
    for pinned in [DECLARED_DECIDER, DECLARED_IMPORT, DECLARED_VERSION]
        .into_iter()
        .chain(DECLARED_RECORD)
    {
        assert_declared_and_documented(pinned);
    }
    let record = format!("{}\n}}", DECLARED_RECORD.join("\n"));
    assert!(
        readme().contains(&record),
        "the README no longer declares this verbatim:\n{record}"
    );
}

#[test]
fn the_readme_decider_repairs_the_row_the_readme_stores() {
    let row_id = oid("6a7cc592ca0574e6efdfe299");
    let stored = doc! {
        "_id": row_id,
        "recordId": "6a7cc592ca0574e6efdfe217",
        "name": "Loan",
        "createdAt": bson::DateTime::from_millis(1_759_600_000_000),
        "versions": [{ "number": 1_i32 }, { "number": "2", "draft": true }],
    };
    let mut calls = 0_u32;
    let mut handed: Vec<String> = Vec::new();
    let record = Record::from_bson_with(stored, |held, issues| {
        calls += 1;
        handed = issues.iter().map(shown).collect();
        assert_eq!(held.get_object_id("_id").ok(), Some(row_id));
        repair_record(held, issues)
    })
    .unwrap();
    assert_eq!(calls, 1);

    // The README declares `id` first and this module declares it second, so the same issues are
    // handed over in another order.
    assert_eq!(handed.len(), SHOWN_ISSUES.len());
    for row in SHOWN_ISSUES {
        assert!(
            handed.iter().any(|issue| issue == row),
            "the callback was not handed {row}, got: {handed:?}"
        );
    }
    let table = SHOWN_ISSUES.join("\n");
    assert!(
        readme().contains(&table),
        "the README no longer shows these issues verbatim:\n{table}"
    );

    let numbers: Vec<i32> = record
        .versions
        .iter()
        .map(|version| version.number)
        .collect();
    let returned = [
        format!("| `id` | `{}` |", record.id),
        format!("| `name` | `{}` |", record.name),
        format!("| `created_at` | `{}` |", record.created_at.to_rfc3339()),
        format!("| `versions` | `{numbers:?}` |"),
        format!("| `note` | `{:?}` |", record.note),
    ];
    assert_eq!(returned, SHOWN_RESULT);
    let result = SHOWN_RESULT.join("\n");
    assert!(
        readme().contains(&result),
        "the README no longer shows this result verbatim:\n{result}"
    );
}

/// The two answers the stored row does not reach: a date stored as epoch milliseconds, and an
/// issue the callback repairs nothing for.
#[test]
fn the_readme_decider_reads_an_epoch_date_and_rejects_what_it_cannot_repair() {
    let written = doc! {
        "recordId": oid("6a7cc592ca0574e6efdfe217"),
        "name": "Loan",
        "createdAt": 1_759_600_000_000_i64,
        "versions": [],
    };
    let record = Record::from_bson_with(written, repair_record).unwrap();
    assert_eq!(record.created_at.timestamp_millis(), 1_759_600_000_000);

    let nameless = doc! {
        "recordId": oid("6a7cc592ca0574e6efdfe217"),
        "createdAt": "2025-10-04T17:46:40Z",
        "versions": [],
    };
    let refused = Record::from_bson_with(nameless, repair_record).unwrap_err();
    assert_eq!(refused.to_string(), "name: missing: expected String");
}
