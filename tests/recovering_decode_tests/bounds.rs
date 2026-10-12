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
    #[serde(default)]
    labels: Vec<BoundedLabel>,
    #[model_schema_prop(minimum = 1, maximum = 9)]
    level: u32,
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sort_field: Option<BoundedPath>,
    source: BoundedPath,
    #[model_schema_prop(minLength = 1, maxLength = 2)]
    tags: Vec<String>,
    #[model_schema_prop(maxLength = 3, pattern = "^[a-z]+$")]
    title: String,
}

/// A label held to a rule of its own, which is where a rule for each item of a list is written.
/// A brand's rule is read with its schema surface, so a build with none writes none.
#[cfg_attr(
    any(feature = "typescript", feature = "zod", feature = "jsonschema"),
    model_schema(decode_with, minLength = 2)
)]
#[cfg_attr(
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema")),
    model_schema(decode_with)
)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct BoundedLabel(String);

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

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum BoundedSlots {
    Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
    Many(#[model_schema_prop(maxLength = 1)] Vec<String>),
    Pair(
        #[model_schema_prop(minLength = 2)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum BoundedSlotsAdjacent {
    Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
    Pair(
        #[model_schema_prop(minLength = 2)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
}

/// An untagged member that is a list, which the read counts to choose the variant.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum BoundedChoice {
    Few(#[model_schema_prop(maxLength = 2)] Vec<String>),
    Text(String),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct BoundedChooser {
    choice: BoundedChoice,
}

#[model_schema(decode_with, minLength = 1, maxLength = 5)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum BoundedByEnum {
    Label { text: String },
    Slot(String),
    Tags { tags: Vec<String> },
}

/// A stored group that breaks every bound `BoundedGroup` writes.
fn broken() -> Value {
    json!({
        "humanName": "",
        "level": 0_u32,
        "sortField": "",
        "source": "Items",
        "tags": ["ok", "A", "b"],
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
            "tags: invalid: expected Array(String), found Array [String(\"ok\"), String(\"A\"), \
             String(\"b\")]: too long: maximum length is 2, got 3",
            "title: invalid: expected String, found String(\"long\"): too long: maximum length is \
             3, got 4",
        ]
    );
    assert_eq!(calls, 1);
}

/// A list that breaks its bound is one issue at the list, whatever its items are.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_list_that_breaks_its_bound_is_one_issue_at_the_list() {
    let stored = json!({ "level": 1_u32, "source": "Items", "tags": [], "title": "ok" });
    let (read, calls) = rejected(stored);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "tags: invalid: expected Array(String), found Array []: too short: minimum length is 1, got 0"
        ]
    );
    assert_eq!(calls, 1);
}

/// A callback puts a list inside its bound at the issue's path, and the read answers it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_callback_that_sets_a_list_at_its_path_recovers_the_read() {
    use bounded_group_schema::{Issue, Verdict};

    let stored =
        json!({ "level": 1_u32, "source": "Items", "tags": ["a", "b", "c"], "title": "ok" });
    let read = BoundedGroup::from_value_with(stored, |raw, found| {
        let [
            Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            },
        ] = found
        else {
            return Verdict::Reject;
        };
        path.set_in_value(raw, json!(["a", "b"]));
        Verdict::Fixed
    });
    assert_eq!(read.unwrap().tags, ["a", "b"]);
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

#[test]
fn a_broken_bound_in_a_tagged_slot_is_listed_at_the_slot() {
    use bounded_slots_adjacent_schema::Verdict as AdjacentVerdict;
    use bounded_slots_schema::Verdict;

    let held = BoundedSlots::from_value_with(json!({ "Code": "ABC" }), |_, _| Verdict::Reject);
    assert_eq!(held, Ok(BoundedSlots::Code("ABC".to_owned())));

    let code = BoundedSlots::from_value_with(json!({ "Code": "abc" }), |_, _| Verdict::Reject);
    let pair =
        BoundedSlots::from_value_with(json!({ "Pair": ["a", 0_u32] }), |_, _| Verdict::Reject);
    let adjacent_code =
        BoundedSlotsAdjacent::from_value_with(json!({ "data": "abc", "kind": "Code" }), |_, _| {
            AdjacentVerdict::Reject
        });
    let adjacent_pair = BoundedSlotsAdjacent::from_value_with(
        json!({ "data": ["a", 0_u32], "kind": "Pair" }),
        |_, _| AdjacentVerdict::Reject,
    );
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        assert_eq!(
            lines(&code.unwrap_err()),
            [
                "Code: invalid: expected String, found String(\"abc\"): does not match pattern \
                 '^[A-Z]{3}$'"
            ]
        );
        assert_eq!(
            lines(&pair.unwrap_err()),
            [
                "Pair[0]: invalid: expected String, found String(\"a\"): too short: minimum \
                 length is 2, got 1",
                "Pair[1]: invalid: expected U32, found Number(0): too small: minimum is 1, got 0",
            ]
        );
        assert_eq!(
            lines(&adjacent_code.unwrap_err()),
            [
                "data: invalid: expected String, found String(\"abc\"): does not match pattern \
                 '^[A-Z]{3}$'"
            ]
        );
        assert_eq!(
            lines(&adjacent_pair.unwrap_err()),
            [
                "data[0]: invalid: expected String, found String(\"a\"): too short: minimum \
                 length is 2, got 1",
                "data[1]: invalid: expected U32, found Number(0): too small: minimum is 1, got 0",
            ]
        );
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(code, Ok(BoundedSlots::Code("abc".to_owned())));
        assert_eq!(pair, Ok(BoundedSlots::Pair("a".to_owned(), 0)));
        assert_eq!(
            adjacent_code,
            Ok(BoundedSlotsAdjacent::Code("abc".to_owned()))
        );
        assert_eq!(
            adjacent_pair,
            Ok(BoundedSlotsAdjacent::Pair("a".to_owned(), 0))
        );
    }
}

#[test]
fn a_list_in_a_variants_slot_is_listed_once_at_the_slot() {
    use bounded_slots_schema::Verdict;

    let held = BoundedSlots::from_value_with(json!({ "Many": ["only"] }), |_, _| Verdict::Reject);
    assert_eq!(held, Ok(BoundedSlots::Many(vec!["only".to_owned()])));
    let many = BoundedSlots::from_value_with(json!({ "Many": ["a", "b"] }), |_, _| Verdict::Reject);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        lines(&many.unwrap_err()),
        [
            "Many: invalid: expected Array(String), found Array [String(\"a\"), String(\"b\")]: \
             too long: maximum length is 1, got 2"
        ]
    );
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    assert_eq!(
        many,
        Ok(BoundedSlots::Many(vec!["a".to_owned(), "b".to_owned()]))
    );
}

#[test]
fn an_untagged_list_member_is_read_through_the_hook_that_counts_it() {
    use bounded_chooser_schema::Verdict;

    let few =
        BoundedChooser::from_value_with(json!({ "choice": ["a", "b"] }), |_, _| Verdict::Reject);
    assert_eq!(
        few,
        Ok(BoundedChooser {
            choice: BoundedChoice::Few(vec!["a".to_owned(), "b".to_owned()])
        })
    );
    let many = BoundedChooser::from_value_with(json!({ "choice": ["a", "b", "c"] }), |_, _| {
        Verdict::Reject
    });
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        lines(&many.unwrap_err()),
        [
            "choice: no variant: found Array [String(\"a\"), String(\"b\"), String(\"c\")], \
             tried Few, Text"
        ]
    );
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    assert!(
        many.is_ok(),
        "no surface hangs the hook, so the list is read"
    );
}

#[test]
fn a_bound_written_on_the_enum_is_listed_at_each_string_it_reaches() {
    use bounded_by_enum_schema::Verdict;

    let held = BoundedByEnum::from_value_with(json!({ "Slot": "abcde" }), |_, _| Verdict::Reject);
    assert_eq!(held, Ok(BoundedByEnum::Slot("abcde".to_owned())));
    let tags =
        BoundedByEnum::from_value_with(json!({ "Tags": { "tags": ["", "abcdef"] } }), |_, _| {
            Verdict::Reject
        });
    assert_eq!(
        tags,
        Ok(BoundedByEnum::Tags {
            tags: vec![String::new(), "abcdef".to_owned()]
        })
    );

    let label =
        BoundedByEnum::from_value_with(json!({ "Label": { "text": "" } }), |_, _| Verdict::Reject);
    let slot = BoundedByEnum::from_value_with(json!({ "Slot": "abcdef" }), |_, _| Verdict::Reject);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        assert_eq!(
            lines(&label.unwrap_err()),
            [
                "Label.text: invalid: expected String, found String(\"\"): too short: minimum \
                 length is 1, got 0"
            ]
        );
        assert_eq!(
            lines(&slot.unwrap_err()),
            [
                "Slot: invalid: expected String, found String(\"abcdef\"): too long: maximum \
                 length is 5, got 6"
            ]
        );
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(
            label,
            Ok(BoundedByEnum::Label {
                text: String::new()
            })
        );
        assert_eq!(slot, Ok(BoundedByEnum::Slot("abcdef".to_owned())));
    }
}

/// A callback sets the slot at the issue's path, and the read answers the repaired value.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_callback_that_sets_a_slot_at_its_path_recovers_the_read() {
    use bounded_slots_schema::{Issue, Verdict};

    let read = BoundedSlots::from_value_with(json!({ "Pair": ["ab", 0_u32] }), |raw, found| {
        let [
            Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            },
        ] = found
        else {
            return Verdict::Reject;
        };
        path.set_in_value(raw, json!(1_u32));
        Verdict::Fixed
    });
    assert_eq!(read, Ok(BoundedSlots::Pair("ab".to_owned(), 1)));
}

/// The callback drops each value a bound refuses, and the read answers what is left.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_callback_that_removes_what_breaks_a_bound_recovers_the_read() {
    let stored = json!({
        "humanName": "", "labels": ["A", "ok"], "level": 1_u32, "sortField": "", "source": "Items",
        "tags": ["A"], "title": "ok"
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
            labels: vec![BoundedLabel("ok".to_owned())],
            level: 1,
            sort_field: None,
            source: BoundedPath("Items".to_owned()),
            tags: vec!["A".to_owned()],
            title: "ok".to_owned(),
        })
    );
}

/// A title that breaks two bounds is two issues at one path, alike but for the reason.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn two_bounds_broken_by_one_value_are_repaired_once_at_its_path() {
    use bounded_group_schema::{Issue, Verdict};

    let stored = || json!({ "level": 1_u32, "source": "Items", "tags": ["ok"], "title": "LONG" });
    let (listed, _calls) = rejected(stored());
    assert_eq!(
        lines(&listed.unwrap_err()),
        [
            "title: invalid: expected String, found String(\"LONG\"): too long: maximum length is \
             3, got 4",
            "title: invalid: expected String, found String(\"LONG\"): does not match pattern \
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
    assert_eq!(set_once.unwrap().title, "aa");

    let set_for_each = BoundedGroup::from_value_with(stored(), |raw, found| {
        for issue in found {
            if let Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_value(raw, json!("ok"));
            }
        }
        Verdict::Fixed
    });
    assert_eq!(set_for_each.unwrap().title, "ok");
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
