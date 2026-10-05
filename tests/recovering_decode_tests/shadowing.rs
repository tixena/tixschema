//! A flagged type whose fields name types their author called `Issue` and `Path`. The flag adds
//! types of those names to `{type}_schema`, and what that module already held goes on reading the
//! author's.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Issue {
    title: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Path {
    segments: Vec<String>,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Ticket {
    issue: Issue,
    path: Path,
    related: Vec<Issue>,
}

fn ticket() -> Ticket {
    Ticket {
        issue: Issue {
            title: "Loan".to_owned(),
        },
        path: Path {
            segments: vec!["a".to_owned()],
        },
        related: vec![Issue {
            title: "Other".to_owned(),
        }],
    }
}

#[test]
fn a_type_named_like_an_added_one_is_read_as_the_authors() {
    let mut calls = 0_u32;
    let stored = serde_json::to_value(ticket()).unwrap();
    let read = Ticket::from_value_with(stored, |_raw, _found| {
        calls += 1;
        ticket_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(ticket()));
    assert_eq!(calls, 0);
}

#[test]
fn an_issue_inside_a_type_named_issue_is_listed_at_its_path() {
    let stored = json!({
        "issue": { "title": 7_i32 },
        "path": { "segments": [] },
        "related": [{ "title": "Other", "draft": true }],
    });
    let read = Ticket::from_value_with(stored, |_raw, _found| ticket_schema::Verdict::Reject);
    assert_eq!(
        super::lines(&read.unwrap_err()),
        [
            "issue.title: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "related[0].draft: unknown: found Bool(true)",
        ]
    );
}

/// The author's `Issue` reads through the entry point the flag gave it, whose callback works with
/// the `Issue` the flag added beside it.
#[test]
fn the_authors_issue_and_the_added_issue_are_two_types() {
    let read = Issue::from_value_with(json!({}), |_raw, found| {
        assert_eq!(
            found,
            [issue_schema::Issue::Missing {
                path: issue_schema::Path(vec![issue_schema::Segment::Key("title".to_owned())]),
                expected: issue_schema::Expected::String,
            }]
        );
        issue_schema::Verdict::Reject
    });
    read.unwrap_err();
}
