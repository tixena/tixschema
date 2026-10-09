//! A bound written on a field is the recovering read's to list. serde reads a value that breaks
//! one, so the walker runs the field's validator on what it read: one issue per violation, at the
//! value, which is what a callback repairs a stored value by.
//!
//! A validator is published only where a schema surface is on, so a build with none lists nothing.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::lines;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundedGroup {
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    human_name: Option<String>,
    #[model_schema_prop(minimum = 1, maximum = 9)]
    level: u32,
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

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum BoundedShape {
    Circle {
        #[model_schema_prop(minimum = 1)]
        radius: u32,
    },
    Label {
        #[model_schema_prop(minLength = 1)]
        text: String,
    },
}

/// A stored group that breaks every bound `BoundedGroup` writes.
fn broken() -> Value {
    json!({
        "humanName": "",
        "level": 0_u32,
        "sortField": "",
        "source": "Items",
        "tags": ["ok", "A"],
        "title": "long",
    })
}

/// Reads `stored` with a callback that rejects, and answers the read with how often it ran.
fn rejected(
    stored: Value,
) -> (
    Result<BoundedGroup, bounded_group_schema::Unrecovered<Value>>,
    u32,
) {
    let mut calls = 0_u32;
    let read = BoundedGroup::from_value_with(stored, |_raw, _found| {
        calls = calls.saturating_add(1);
        bounded_group_schema::Verdict::Reject
    });
    (read, calls)
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_broken_bound_is_listed_at_its_value_once_per_violation() {
    let (read, calls) = rejected(broken());
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "humanName: invalid: expected Optional(String), found String(\"\"): too short: minimum \
             length is 1, got 0",
            "level: invalid: expected U32, found Number(0): too small: minimum is 1, got 0",
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

#[test]
fn a_broken_bound_in_a_tagged_variant_is_listed() {
    let circle =
        BoundedShape::from_value_with(json!({ "kind": "Circle", "radius": 0_u32 }), |_, _| {
            bounded_shape_schema::Verdict::Reject
        });
    let held = BoundedShape::from_value_with(json!({ "kind": "Label", "text": "a" }), |_, _| {
        bounded_shape_schema::Verdict::Reject
    });
    assert_eq!(
        held,
        Ok(BoundedShape::Label {
            text: "a".to_owned()
        })
    );
    let label = BoundedShape::from_value_with(json!({ "kind": "Label", "text": "" }), |_, _| {
        bounded_shape_schema::Verdict::Reject
    });
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        assert_eq!(
            lines(&circle.unwrap_err()),
            ["radius: invalid: expected U32, found Number(0): too small: minimum is 1, got 0"]
        );
        assert_eq!(
            lines(&label.unwrap_err()),
            [
                "text: invalid: expected String, found String(\"\"): too short: minimum length is \
                 1, got 0"
            ]
        );
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(circle, Ok(BoundedShape::Circle { radius: 0 }));
        assert_eq!(
            label,
            Ok(BoundedShape::Label {
                text: String::new()
            })
        );
    }
}

/// The callback drops each value a bound refuses, and the read answers what is left.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_callback_that_removes_what_breaks_a_bound_recovers_the_read() {
    let stored = json!({
        "humanName": "", "level": 1_u32, "sortField": "", "source": "Items", "tags": ["A"], "title": "ok"
    });
    let read = BoundedGroup::from_value_with(stored, |raw, found| {
        for issue in found {
            if let bounded_group_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.remove_from_value(raw);
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

/// A tag that breaks two bounds is two issues at one path, alike but for the reason.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn two_bounds_broken_by_one_value_are_repaired_once_at_its_path() {
    use bounded_group_schema::{Issue, Verdict};

    let stored =
        || json!({ "level": 1_u32, "source": "Items", "tags": ["A", "ok"], "title": "ok" });
    let (listed, _calls) = rejected(stored());
    assert_eq!(
        lines(&listed.unwrap_err()),
        [
            "tags[0]: invalid: expected String, found String(\"A\"): too short: minimum length is \
             2, got 1",
            "tags[0]: invalid: expected String, found String(\"A\"): does not match pattern \
             '^[a-z]+$'",
        ]
    );

    let set_once = BoundedGroup::from_value_with(stored(), |raw, found| {
        let [
            Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            },
            _pattern,
        ] = found
        else {
            return Verdict::Reject;
        };
        path.set_in_value(raw, json!("aa"));
        Verdict::Fixed
    });
    assert_eq!(set_once.unwrap().tags, ["aa", "ok"]);

    let removed_for_each = BoundedGroup::from_value_with(stored(), |raw, found| {
        for issue in found {
            if let Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.remove_from_value(raw);
            }
        }
        Verdict::Fixed
    });
    assert_eq!(removed_for_each.unwrap().tags, Vec::<String>::new());
}

#[test]
fn a_value_inside_every_bound_never_reaches_the_callback() {
    let stored = json!({
        "humanName": "h", "level": 9_u32, "sortField": "a.b", "source": "Items", "tags": ["ok"],
        "title": "ok",
    });
    let (read, calls) = rejected(stored);
    assert_eq!(
        read.unwrap().sort_field,
        Some(BoundedPath("a.b".to_owned()))
    );
    assert_eq!(calls, 0);
}

/// The bound stays off serde's own read, which reads the value that breaks it.
#[test]
fn plain_serde_reads_what_breaks_a_bound() {
    let read: BoundedGroup = serde_json::from_value(broken()).unwrap();
    assert_eq!(read.sort_field, Some(BoundedPath(String::new())));
    assert_eq!(read.human_name, Some(String::new()));
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        let (walked, calls) = rejected(broken());
        assert_eq!(walked.unwrap(), read);
        assert_eq!(calls, 0);
    }
}
